use std::path::{Path, PathBuf};

use iced::widget::{column, container, mouse_area, text};
use iced::{Element, Task};

use ericed::{Aim, AimExt, Aimed, Meaning};

use crate::fs::{self, Entry};
use crate::intent::{Intent, Motion, Row};
use crate::style::container::RowInput;
use crate::style::{self, StyleExt};

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
    entries: Vec<Entry>,
    cursor: usize,
    /// The row the pointer is over, if any.
    ///
    /// Appearance, not model: it never reaches an [`Intent`] and never changes
    /// the cursor. It is kept here, next to `cursor`, because both answer the
    /// same question — *which row* — and splitting them across two owners is
    /// how they fall out of sync.
    hovered: Option<usize>,
    error: Option<String>,
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
        Self {
            path,
            entries,
            cursor: 0,
            hovered: None,
            error: None,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn cursor_entry(&self) -> Option<&Entry> {
        self.entries.get(self.cursor)
    }

    pub fn hovered(&self) -> Option<usize> {
        self.hovered
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Moves the cursor by a motion, stopping at either end.
    ///
    /// The pane owns the list, so it also owns the clamping; the intent only
    /// says which direction the user meant.
    pub fn move_cursor(&mut self, motion: Motion) {
        if self.entries.is_empty() {
            return;
        }

        let last = self.entries.len() - 1;

        self.cursor = match motion {
            Motion::Next => (self.cursor + 1).min(last),
            Motion::Previous => self.cursor.saturating_sub(1),
        };
    }

    /// Puts the cursor on a row, if that row still exists.
    ///
    /// A target was produced by a layout that may be a frame old — an
    /// asynchronous read can have replaced the listing in between — so an index
    /// can be out of range. That is a no-op, not a panic.
    pub fn focus(&mut self, row: Row) {
        if row.0 < self.entries.len() {
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

        self.navigate(parent)
    }

    pub fn update(&mut self, message: MiddlePaneMsg) -> Task<MiddlePaneMsg> {
        match message {
            MiddlePaneMsg::EntryActivated(index) => {
                let Some(entry) = self.entries.get(index) else {
                    return Task::none();
                };

                if !entry.item_type().is_folder() {
                    self.cursor = index;

                    return Task::none();
                }

                let path = entry.path().to_path_buf();

                self.navigate(path)
            }
            MiddlePaneMsg::EntriesLoaded(Ok(entries)) => {
                self.entries = entries;
                self.cursor = 0;
                self.hovered = None;
                self.error = None;

                Task::none()
            }
            MiddlePaneMsg::EntriesLoaded(Err(error)) => {
                self.error = Some(error);

                Task::none()
            }
            MiddlePaneMsg::Hover(row) => {
                self.hovered = row;

                Task::none()
            }
            // Routed to the application by `AppMsg::from`; the pane has no
            // opinion about what a gesture means.
            MiddlePaneMsg::Aimed(_) => Task::none(),
        }
    }

    /// Switches to another folder and starts reading it.
    ///
    /// The listing is cleared first so the view never shows the previous
    /// folder's contents under the new path.
    fn navigate(&mut self, path: PathBuf) -> Task<MiddlePaneMsg> {
        self.path = path.clone();
        self.entries.clear();
        self.cursor = 0;
        self.hovered = None;
        self.error = None;

        load(path)
    }

    pub fn view(&self) -> Element<'_, MiddlePaneMsg> {
        let mut list = column![
            container(text(self.path.display().to_string()).styled(style::text::caption()))
                .styled(style::container::header()),
            style::rule::horizontal(),
        ];

        if let Some(error) = &self.error {
            list = list.push(
                container(text(error.clone()).styled(style::text::error()))
                    .styled(style::container::row(RowInput::default())),
            );
        }

        let mut rows = column![];

        for (index, entry) in self.entries.iter().enumerate() {
            let input = RowInput {
                hovered: self.hovered == Some(index),
                cursor: index == self.cursor,
                selected: false,
            };

            rows = rows.push(
                container(text(entry.label()).styled(style::text::body()))
                    .styled(style::container::row(input))
                    // One target, three gestures: the press and the double
                    // click of the same click cannot disagree about the row.
                    .aimed(Row(index))
                    .on_enter(MiddlePaneMsg::Hover(Some(index))),
            );
        }

        // `enter` is per row (a row knows itself); `exit` is per listing,
        // because leaving needs no identity. Hanging `on_exit` on every row
        // instead would leave the last row lit while the pointer sits on the
        // header or the empty space below the list.
        list.push(mouse_area(rows).on_exit(MiddlePaneMsg::Hover(None)))
            .into()
    }
}

fn load(path: PathBuf) -> Task<MiddlePaneMsg> {
    Task::perform(fs::read_dir(path), |result| {
        MiddlePaneMsg::EntriesLoaded(result.map_err(|error| error.to_string()))
    })
}
