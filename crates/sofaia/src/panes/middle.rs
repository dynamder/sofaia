use std::path::{Path, PathBuf};
use std::time::Duration;

use iced::time::Instant;
use iced::widget::operation;
use iced::widget::scrollable::Viewport;
use iced::widget::{
    column, container, mouse_area, responsive, row, scrollable, sensor, text, text_input,
};
use iced::{Alignment, Element, Length, Size, Task, Vector};

use ericed::list::{self, Scroll, Slice, Slots};
use ericed::motion::{self, Curve, Track};
use ericed::{Aim, AimExt, Aimed, Meaning, WheelDirection, wheel_area};

use crate::fs::{self, Entry};
use crate::intent::{Intent, Motion, Row};
use crate::style::container::RowInput;
use crate::style::{self, StyleExt};

/// How long a folder takes to slide in.
const SLIDE: Duration = Duration::from_millis(220);

/// The viewport assumed to exist until a `sensor` reports the real one.
///
/// A windowed list cannot draw anything before it knows how tall the window is,
/// and only the widget tree knows that. The real height arrives as a `Resized`
/// message during the first redraw — before the first frame is drawn, so no one
/// sees this — but assuming *nothing* would make the first layout one row tall,
/// which headless tests then have to work around.
const VIEWPORT: f32 = 600.0;

/// What pointing at a row means.
///
/// The listing's rules, next to the listing. Three gestures, three goals, and
/// nothing else can arrive here: the target type is `Row`, so a menu item or the
/// scrim can never be mistaken for one.
impl Meaning<Intent> for Row {
    fn meaning(self, aim: Aim) -> Option<Intent> {
        match aim {
            // A press points at a row; it does not open anything.
            Aim::Press => Some(Intent::Focus(self)),
            // `mouse_area` publishes the press before this, so the cursor is
            // already here and `Open` can stay target-less (and therefore be
            // the same intent `l` produces).
            Aim::DoubleClick => Some(Intent::Open),
            Aim::RightPress => Some(Intent::Menu(self)),
        }
    }
}

pub struct MiddlePane {
    path: PathBuf,
    /// Everything the folder contains, in sort order.
    entries: Vec<Entry>,
    /// The indices the search lets through.
    ///
    /// Row `n` of the listing is `entries[visible[n]]`, so the cursor, the
    /// hovered row and every `Row` target are positions in the *filtered* list.
    /// One index space instead of two is what keeps a filtered listing from
    /// needing a second set of bounds checks.
    visible: Vec<usize>,
    query: String,
    cursor: usize,
    /// The row the pointer is over, if any.
    ///
    /// Appearance, not model: it never reaches an [`Intent`] and never changes
    /// the cursor. It is kept here, next to `cursor`, because both answer the
    /// same question — *which row* — and splitting them across two owners is
    /// how they fall out of sync.
    hovered: Option<usize>,
    error: Option<String>,
    /// Where the viewport is, as the app sees it.
    ///
    /// iced only reports this through `on_scroll`, so it is kept as state; and a
    /// programmatic scroll has to write it here as well, which is what
    /// [`Scroll::to`] is for.
    scroll: Scroll,
    /// The slide after a folder change: 0 off-screen, 1 arrived.
    slide: Track,
    /// Which side the current folder arrived from: `1.0` for a child (it comes
    /// in from the right), `-1.0` for a parent.
    arrival: f32,
}

#[derive(Debug, Clone)]
pub enum MiddlePaneMsg {
    EntryActivated(usize),
    EntriesLoaded(Result<Vec<Entry>, String>),
    /// The pointer entered a row, or left the listing.
    Hover(Option<usize>),
    /// A gesture the user aimed at a row.
    ///
    /// The pane can see the target — it laid the rows out — but what the
    /// gesture *means* is [`Row`]'s own impl above, applied by the application.
    /// `update` never receives this variant.
    Aimed(Aimed<Row>),
    /// The scrollable's viewport, which is the only way iced reports an offset.
    Scrolled(Viewport),
    /// The listing's own size, from a `sensor`.
    ///
    /// Needed because `on_scroll` does not fire until something scrolls, and
    /// `reveal` has to know how tall the viewport is before the first notch.
    Resized(Size),
    /// A wheel turn, claimed by `ericed::wheel_area`.
    ///
    /// The pane does not decide what it means: there is no target, so it goes up
    /// to the ambient grammar like any other device event.
    Wheel(WheelDirection),
    /// The search box changed.
    QueryChanged(String),
}

impl From<Aimed<Row>> for MiddlePaneMsg {
    fn from(aimed: Aimed<Row>) -> Self {
        Self::Aimed(aimed)
    }
}

impl MiddlePane {
    pub fn new(path: PathBuf) -> (Self, Task<MiddlePaneMsg>) {
        let pane = Self::with_entries(path.clone(), Vec::new());

        (pane, load(path))
    }

    pub fn with_entries(path: PathBuf, entries: Vec<Entry>) -> Self {
        let visible = (0..entries.len()).collect();

        Self {
            path,
            entries,
            visible,
            query: String::new(),
            cursor: 0,
            hovered: None,
            error: None,
            scroll: Scroll::at(0.0, VIEWPORT),
            // Settled at the end, so nothing slides on startup.
            slide: Track::new(1.0),
            arrival: 1.0,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn cursor_entry(&self) -> Option<&Entry> {
        self.row_entry(self.cursor)
    }

    pub fn hovered(&self) -> Option<usize> {
        self.hovered
    }

    pub fn len(&self) -> usize {
        self.visible.len()
    }

    pub fn is_empty(&self) -> bool {
        self.visible.is_empty()
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn scroll(&self) -> Scroll {
        self.scroll
    }

    /// Whether the slide is still running.
    pub fn is_moving(&self) -> bool {
        self.slide.is_moving()
    }

    /// Advances the slide. Called once per frame message.
    pub fn tick(&mut self, now: Instant) {
        self.slide.tick(now);
    }

    /// The row `row` of the filtered listing.
    fn row_entry(&self, row: usize) -> Option<&Entry> {
        self.visible
            .get(row)
            .and_then(|&index| self.entries.get(index))
    }

    /// Moves the cursor by a motion, stopping at either end.
    ///
    /// The pane owns the list, so it also owns the clamping; the intent only
    /// says which direction the user meant.
    pub fn move_cursor(&mut self, motion: Motion) {
        if self.visible.is_empty() {
            return;
        }

        let last = self.visible.len() - 1;

        self.cursor = match motion {
            Motion::Next => (self.cursor + 1).min(last),
            Motion::Previous => self.cursor.saturating_sub(1),
        };
    }

    /// Moves the view by one row, leaving the cursor where it is.
    ///
    /// The wheel's intent. It clamps against the same total the window does, so
    /// the scrollbar and the offset cannot disagree about where the end is.
    pub fn scroll_by(&mut self, motion: Motion) -> Task<MiddlePaneMsg> {
        let row = style::container::row_height();
        let path = list::fixed(row, self.visible.len());

        let step = match motion {
            Motion::Next => row,
            Motion::Previous => -row,
        };

        let target = (self.scroll.y() + step).clamp(0.0, path.max_scroll(self.scroll.height()));

        if (target - self.scroll.y()).abs() < f32::EPSILON {
            return Task::none();
        }

        operation::scroll_to(Self::list_id(), self.scroll.to(target))
    }

    /// Scrolls the least amount that puts the cursor back in the window.
    pub fn reveal_cursor(&mut self) -> Task<MiddlePaneMsg> {
        let path = list::fixed(style::container::row_height(), self.visible.len());
        let target = path.reveal(self.cursor, &self.scroll);

        if (target - self.scroll.y()).abs() < 0.5 {
            return Task::none();
        }

        operation::scroll_to(Self::list_id(), self.scroll.to(target))
    }

    /// Puts the cursor in the search box.
    ///
    /// Focusing the box is what makes iced ask the window for an input method,
    /// which is the only way a Chinese folder name can be typed at all.
    pub fn focus_search(&self) -> Task<MiddlePaneMsg> {
        operation::focus(Self::search_id())
    }

    /// Forgets the query, if there is one.
    ///
    /// `Escape` while the box is focused is the box's own business (it
    /// unfocuses itself); this is the *unfocused* case, where the listing is
    /// still filtered and the user expects the same key to end that.
    pub fn clear_search(&mut self) -> Task<MiddlePaneMsg> {
        if self.query.is_empty() {
            return Task::none();
        }

        self.query.clear();
        self.refilter()
    }

    /// Returns the view to the top: the mirror **and** the widget.
    ///
    /// `scroll_to` does not run `on_scroll`, so resetting only the mirror would
    /// leave the listing built from one offset and drawn at another. The symptom
    /// looks like a stray scroll: the new content is translated by whatever the
    /// previous one was scrolled to, so it jumps up before settling.
    fn reset_view(&mut self) -> Task<MiddlePaneMsg> {
        operation::scroll_to(Self::list_id(), self.scroll.to(0.0))
    }

    /// Puts the cursor on a row, if that row still exists.
    ///
    /// A target was produced by a layout that may be a frame old — an
    /// asynchronous read can have replaced the listing in between — so an index
    /// can be out of range. That is a no-op, not a panic.
    pub fn focus(&mut self, row: Row) {
        if row.0 < self.visible.len() {
            self.cursor = row.0;
        }
    }

    /// Enters the folder under the cursor, if there is one.
    pub fn open_cursor(&mut self) -> Task<MiddlePaneMsg> {
        self.update(MiddlePaneMsg::EntryActivated(self.cursor))
    }

    /// Goes to the parent folder, if there is one.
    pub fn parent(&mut self) -> Task<MiddlePaneMsg> {
        let Some(parent) = self.path.parent().map(Path::to_path_buf) else {
            return Task::none();
        };

        self.navigate(parent, -1.0)
    }

    pub fn update(&mut self, message: MiddlePaneMsg) -> Task<MiddlePaneMsg> {
        match message {
            MiddlePaneMsg::EntryActivated(index) => {
                let Some(entry) = self.row_entry(index) else {
                    return Task::none();
                };

                if !entry.item_type().is_folder() {
                    self.cursor = index;

                    return Task::none();
                }

                let path = entry.path().to_path_buf();

                self.navigate(path, 1.0)
            }
            MiddlePaneMsg::EntriesLoaded(Ok(entries)) => {
                self.entries = entries;
                self.refilter()
            }
            MiddlePaneMsg::EntriesLoaded(Err(error)) => {
                self.error = Some(error);

                Task::none()
            }
            MiddlePaneMsg::Hover(row) => {
                self.hovered = row;

                Task::none()
            }
            MiddlePaneMsg::Scrolled(viewport) => {
                self.scroll.update(&viewport);

                // A row that scrolls out of the window never reports `on_exit`,
                // so the highlight would stay lit on a row nobody is over.
                self.hovered = None;

                Task::none()
            }
            MiddlePaneMsg::Resized(size) => {
                self.scroll.resized(size.height);

                Task::none()
            }
            MiddlePaneMsg::QueryChanged(query) => {
                self.query = query;
                self.refilter()
            }
            // Routed to the application by `AppMsg::from`: the pane has no
            // opinion about what a gesture means, and no target to attach a
            // wheel turn to.
            MiddlePaneMsg::Aimed(_) | MiddlePaneMsg::Wheel(_) => Task::none(),
        }
    }

    /// Switches to another folder and starts reading it.
    ///
    /// The listing is cleared at once, so the view never shows the previous
    /// folder's contents under the new path. What makes the change legible is
    /// the slide: the new listing is drawn arriving from the side the folder
    /// lives on.
    fn navigate(&mut self, path: PathBuf, arrival: f32) -> Task<MiddlePaneMsg> {
        // Nothing to slide over an empty listing — and nothing to slide into an
        // empty pane would look like a page that fell in from nowhere.
        if !self.visible.is_empty() {
            self.arrival = arrival;
            self.slide = Track::new(0.0).curve(Curve::out(SLIDE));
            self.slide.to(1.0);
        }

        self.path = path.clone();
        self.entries.clear();
        self.visible.clear();
        self.cursor = 0;
        self.hovered = None;
        self.error = None;
        self.query.clear();

        // No `Scroll::at` here: the scrollable still holds the previous folder's
        // offset, so the reset has to reach it as well.
        Task::batch([self.reset_view(), load(path)])
    }

    /// Recomputes which entries the query lets through.
    ///
    /// The cursor goes back to the first match — a query that keeps the old
    /// index would point at an unrelated row — and the view returns to the top.
    /// The viewport height survives, because it is a property of the window and
    /// not of the list.
    fn refilter(&mut self) -> Task<MiddlePaneMsg> {
        let needle = self.query.trim().to_lowercase();

        self.visible = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| needle.is_empty() || entry.name().to_lowercase().contains(&needle))
            .map(|(index, _)| index)
            .collect();

        self.cursor = 0;
        self.hovered = None;
        self.reset_view()
    }

    fn list_id() -> iced::widget::Id {
        iced::widget::Id::new("listing")
    }

    fn search_id() -> iced::widget::Id {
        iced::widget::Id::new("listing-search")
    }

    pub fn view(&self) -> Element<'_, MiddlePaneMsg> {
        responsive(|size| {
            let offset = self.arrival * (1.0 - self.slide.value()) * size.width;

            // Drawn, not laid out: the listing keeps its width while it moves, so
            // nothing re-wraps and the selection box keeps its shape. `pin` would
            // have laid the pane out at `width - offset` on every frame, which is
            // a resize wearing an animation's clothes.
            let listing: Element<'_, MiddlePaneMsg> =
                motion::translate(self.body(), Vector::new(offset, 0.0)).into();

            listing
        })
        .into()
    }

    fn body(&self) -> Element<'_, MiddlePaneMsg> {
        let path = list::fixed(style::container::row_height(), self.visible.len());
        let window = path.slice(&self.scroll);

        let listing = wheel_area(
            sensor(
                scrollable(window.surround(self.rows(window)))
                    .id(Self::list_id())
                    .on_scroll(MiddlePaneMsg::Scrolled)
                    .height(Length::Fill),
            )
            .on_resize(MiddlePaneMsg::Resized),
            MiddlePaneMsg::Wheel,
        );

        let mut body = column![self.header(), style::rule::horizontal()];

        if let Some(error) = &self.error {
            body = body.push(
                container(text(error.clone()).styled(style::text::error()))
                    .styled(style::container::row(RowInput::default())),
            );
        }

        // `enter` is per row (a row knows itself); `exit` is per listing,
        // because leaving needs no identity. Hanging `on_exit` on every row
        // instead would leave the last row lit while the pointer sits on the
        // header or the empty space below the list.
        body.push(mouse_area(listing).on_exit(MiddlePaneMsg::Hover(None)))
            .height(Length::Fill)
            .into()
    }

    fn header(&self) -> Element<'_, MiddlePaneMsg> {
        container(
            row![
                text(self.path.display().to_string()).styled(style::text::caption()),
                text_input("search this folder", &self.query)
                    .id(Self::search_id())
                    .on_input(MiddlePaneMsg::QueryChanged)
                    .width(Length::Fixed(180.0)),
            ]
            .spacing(style::container::gap())
            .align_y(Alignment::Center),
        )
        .styled(style::container::header())
        .into()
    }

    /// Only the rows the viewport can see, and only those are built.
    fn rows(&self, window: Slice) -> Element<'_, MiddlePaneMsg> {
        let mut rows = column![];

        for index in window.range() {
            let Some(entry) = self.row_entry(index) else {
                continue;
            };

            let input = RowInput {
                hovered: self.hovered == Some(index),
                cursor: index == self.cursor,
                selected: false,
            };

            rows = rows.push(
                container(text(entry.label()).styled(style::text::body()))
                    .styled(style::container::row(input))
                    // One target, three gestures: the press and the double click
                    // of the same click cannot disagree about the row.
                    .aimed(Row(index))
                    .on_enter(MiddlePaneMsg::Hover(Some(index))),
            );
        }

        rows.into()
    }
}

fn load(path: PathBuf) -> Task<MiddlePaneMsg> {
    Task::perform(fs::read_dir(path), |result| {
        MiddlePaneMsg::EntriesLoaded(result.map_err(|error| error.to_string()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::{DataType, ItemType};

    const ROW: f32 = 28.0;

    fn entries(count: usize) -> Vec<Entry> {
        let root = PathBuf::from("/home/user");

        (0..count)
            .map(|index| {
                Entry::new(
                    root.join(format!("file{index}.txt")),
                    ItemType::File(DataType::Text),
                )
            })
            .collect()
    }

    fn pane(count: usize) -> MiddlePane {
        MiddlePane::with_entries(PathBuf::from("/home/user"), entries(count))
    }

    #[test]
    fn a_query_leaves_only_the_matching_rows() {
        let mut pane = pane(20);

        let _ = pane.update(MiddlePaneMsg::QueryChanged("file1".into()));

        // file1, file10 … file19.
        assert_eq!(pane.len(), 11);
        assert_eq!(pane.query(), "file1");
        // The first match is where the listing starts again.
        assert_eq!(pane.cursor(), 0);
        assert_eq!(
            pane.cursor_entry().map(Entry::name).as_deref(),
            Some("file1.txt")
        );
    }

    #[test]
    fn an_empty_query_brings_everything_back() {
        let mut pane = pane(5);

        let _ = pane.update(MiddlePaneMsg::QueryChanged("file3".into()));
        assert_eq!(pane.len(), 1);

        let _ = pane.update(MiddlePaneMsg::QueryChanged(String::new()));
        assert_eq!(pane.len(), 5);
    }

    #[test]
    fn a_query_that_matches_nothing_leaves_an_empty_listing() {
        let mut pane = pane(5);

        let _ = pane.update(MiddlePaneMsg::QueryChanged("nothing".into()));

        assert!(pane.is_empty());
        assert_eq!(pane.cursor_entry().map(Entry::name), None);
        // The cursor cannot point past the end.
        pane.move_cursor(Motion::Next);
        assert_eq!(pane.cursor(), 0);
    }

    #[test]
    fn the_wheel_scrolls_the_view_and_clamps_at_the_edges() {
        let mut pane = pane(100);
        // A viewport that shows ten rows; the test has no window to measure
        // one, so it is told.
        pane.scroll.resized(10.0 * ROW);

        let _ = pane.scroll_by(Motion::Next);
        assert_eq!(pane.scroll().y(), ROW);
        // The cursor did not move: a notch is a scroll.
        assert_eq!(pane.cursor(), 0);

        let _ = pane.scroll_by(Motion::Previous);
        assert_eq!(pane.scroll().y(), 0.0);

        // Past the top, and past the bottom.
        let _ = pane.scroll_by(Motion::Previous);
        assert_eq!(pane.scroll().y(), 0.0);

        for _ in 0..200 {
            let _ = pane.scroll_by(Motion::Next);
        }

        assert_eq!(
            pane.scroll().y(),
            100.0 * ROW - 10.0 * ROW,
            "the end of the content, not one row further"
        );
    }

    #[test]
    fn revealing_the_cursor_moves_the_view_by_one_row_at_a_time() {
        let mut pane = pane(100);
        pane.scroll.resized(10.0 * ROW);

        // Rows 0..10 fit, so the cursor starts visible and nothing moves.
        let _ = pane.reveal_cursor();
        assert_eq!(pane.scroll().y(), 0.0);

        for _ in 0..10 {
            pane.move_cursor(Motion::Next);
        }

        // Row 10 is the first one below the window: put its bottom on the
        // window's bottom, which is one row of offset.
        let _ = pane.reveal_cursor();
        assert_eq!(pane.scroll().y(), ROW);

        // And again: already visible, so the offset holds.
        let _ = pane.reveal_cursor();
        assert_eq!(pane.scroll().y(), ROW);
    }

    #[test]
    fn changing_folders_slides_the_new_listing_in() {
        let mut pane = pane(5);
        assert!(!pane.is_moving());
        assert_eq!(pane.slide.value(), 1.0, "nothing moves on startup");

        let _ = pane.parent();

        // Going up: the new listing arrives from the left.
        assert_eq!(pane.arrival, -1.0);
        assert!(pane.is_moving());
        assert_eq!(pane.slide.value(), 0.0, "it starts off-screen");
        assert!(pane.is_empty(), "the new folder starts empty");
        assert_eq!(pane.path(), Path::new("/home"));

        let start = Instant::now();
        pane.tick(start);

        assert!(pane.is_moving(), "it has to be drawn at least once");

        pane.tick(start + SLIDE);

        assert!(!pane.is_moving());
        assert_eq!(pane.slide.value(), 1.0, "and it ends where it belongs");
    }

    #[test]
    fn entering_a_folder_slides_the_other_way() {
        let root = PathBuf::from("/home/user");
        let mut pane = MiddlePane::with_entries(
            root.clone(),
            vec![Entry::new(root.join("src"), ItemType::Folder)],
        );

        let _ = pane.open_cursor();

        // Going down: the new listing arrives from the right.
        assert_eq!(pane.arrival, 1.0);
        assert!(pane.is_moving());
    }
}
