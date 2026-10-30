//! A window into a list: only the slots a viewport can see.
//!
//! iced gives you a `scrollable` that reports its [`Viewport`], and nothing that
//! turns a viewport into *which slots to build*. That gap is the whole of this
//! module.
//!
//! # What each piece is for
//!
//! | type | the problem it solves |
//! | --- | --- |
//! | [`Scroll`] | iced only ever hands you a [`Viewport`] as a *message*; `view` needs those numbers *now*, so they are kept as state |
//! | [`Slice`] | "which slots this frame, and how much padding above and below" — four numbers from one piece of arithmetic |
//! | [`Slots`] | the windowing itself (`slice` / `reveal` / `anchor`), written once for both kinds of height |
//! | [`Fixed`] | heights are a constant, so there is no measurement and no feedback loop |
//! | [`Measured`] | heights arrive later from a `sensor`, so they are a cache over estimates |
//!
//! # The invariant
//!
//! ```text
//! scrollable.content_bounds().height() == slots.total()
//! ```
//!
//! Every absolute offset in iced is clamped against the content height, so if
//! the two disagree the scrollbar is right and everything else is quietly
//! wrong: `scroll_to` lands somewhere else, and the window drifts one row
//! further off with every notch.
//!
//! The only way to build the padding is [`Slice::surround`], and the only way
//! to get a [`Slice`] is from a [`Slots`]. Both spacers therefore come from the
//! same arithmetic as the range they surround, and the invariant is a property
//! of the types instead of a comment.
//!
//! # Why the app holds the offset
//!
//! `scrollable` keeps its offset private and only tells you about it through
//! `on_scroll` — a message, delivered after the frame that scrolled. So a
//! windowed list keeps the numbers itself ([`Scroll`]) and passes them while
//! building.
//!
//! That copy has one trap, and it is not obvious: `scrollable::scroll_to` sets
//! the widget's offset **without** notifying `on_scroll` (`scrollable::State`'s
//! `scroll_to` only stores the value). After a programmatic scroll, an app that
//! only listens would render the old window forever. That is why [`Scroll::to`]
//! exists: it records the offset *and* returns the payload
//! `operation::scroll_to` takes, so the two can only move together.

use std::ops::Range;

use iced_core::{Element, Pixels, Size};
use iced_widget::Column;
use iced_widget::scrollable::{AbsoluteOffset, Viewport};
use iced_widget::space;

/// The stored counterpart of a [`Viewport`].
///
/// A [`Viewport`] is a snapshot handed to `on_scroll`; this is the same two
/// numbers kept as state, because `view` needs them *now* and the message
/// arrives later.
///
/// Two writers, both named after where the news came from:
///
/// * [`Scroll::update`] — from `on_scroll` (a wheel turn, a drag, a touch).
/// * [`Scroll::resized`] — from a `sensor`, because `on_scroll` does not fire
///   until something actually scrolls, and `reveal` needs the height before
///   that.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Scroll {
    y: f32,
    height: f32,
}

impl Scroll {
    /// A scroll that has not been told anything yet: top, no height.
    pub const fn new() -> Self {
        Self {
            y: 0.0,
            height: 0.0,
        }
    }

    /// A scroll at a known position, mostly for tests and for restoring state.
    pub const fn at(y: f32, height: f32) -> Self {
        Self { y, height }
    }

    /// Where the viewport sits, in pixels from the content's top.
    pub fn y(&self) -> f32 {
        self.y
    }

    /// How tall the viewport is, in pixels.
    pub fn height(&self) -> f32 {
        self.height
    }

    /// The payload of `on_scroll`.
    pub fn update(&mut self, viewport: &Viewport) {
        self.y = viewport.absolute_offset().y;
        self.height = viewport.bounds().height;
    }

    /// The height of the viewport, without touching the offset.
    ///
    /// For a `sensor` that reports its own size: the height is known from the
    /// first frame, the offset is not, and overwriting one with the other would
    /// throw away a scroll that has already happened.
    pub fn resized(&mut self, height: f32) {
        self.height = height.max(0.0);
    }

    /// Records a programmatic scroll and returns the payload to hand to
    /// `iced::widget::operation::scroll_to`.
    ///
    /// Call it in the same `update` that starts the task. Doing the two
    /// separately is how a windowed list ends up drawing one window while the
    /// scrollbar shows another — iced will not tell you, because `scroll_to`
    /// does not run `on_scroll`.
    pub fn to(&mut self, y: f32) -> AbsoluteOffset<Option<f32>> {
        self.y = y.max(0.0);

        AbsoluteOffset {
            x: None,
            y: Some(self.y),
        }
    }
}

/// The slots a viewport can see, plus the padding that keeps the scrollbar
/// honest.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slice {
    start: usize,
    end: usize,
    before: f32,
    after: f32,
    total: f32,
}

impl Slice {
    /// Which slots to build, in order.
    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }

    /// How many slots to build.
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Whether the viewport shows nothing at all.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The height that belongs above the built slots.
    pub fn before(&self) -> f32 {
        self.before
    }

    /// The height that belongs below them.
    ///
    /// Derived from the same total as [`Slice::before`], which is why the two
    /// cannot disagree with the range between them.
    pub fn after(&self) -> f32 {
        self.after
    }

    /// The height the scrollable's content must have.
    pub fn total(&self) -> f32 {
        self.total
    }

    /// Wraps the built slots in exactly the padding this slice needs.
    ///
    /// The spacing is zero on purpose: a [`Column`] adds its spacing *between
    /// every pair*, so a spacing here would add two more gaps that no one
    /// counted, and the content would grow by `2 * spacing` on every rebuild.
    pub fn surround<'a, Message, Theme, Renderer>(
        &self,
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Element<'a, Message, Theme, Renderer>
    where
        Message: 'a,
        Theme: 'a,
        Renderer: iced_core::Renderer + 'a,
    {
        Column::with_children(Vec::from([
            space().height(self.before).into(),
            content.into(),
            space().height(self.after).into(),
        ]))
        .spacing(0.0)
        .into()
    }
}

/// A list whose slots can be indexed, measured and placed.
///
/// Two implementations, and one difference that matters: [`Fixed`] knows every
/// height up front, [`Measured`] learns them. Everything here is written in
/// terms of `top`/`height`, so the windowing, the reveal and the anchor are
/// shared.
pub trait Slots {
    /// How many slots there are.
    fn len(&self) -> usize;

    /// Whether there are none.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The height of everything, which the scrollable's content must match.
    fn total(&self) -> f32;

    /// Where a slot starts, in pixels from the content's top.
    fn top(&self, slot: usize) -> f32;

    /// How tall a slot is.
    fn height(&self, slot: usize) -> f32;

    /// The slots visible in a viewport at `y` with the given `height`.
    fn slice_at(&self, y: f32, height: f32) -> Slice;

    /// The same, from the numbers the app keeps.
    fn slice(&self, scroll: &Scroll) -> Slice {
        self.slice_at(scroll.y(), scroll.height())
    }

    /// The largest offset that still shows content, never negative.
    fn max_scroll(&self, height: f32) -> f32 {
        (self.total() - height).max(0.0)
    }

    /// The offset that moves a slot into view with the least scrolling.
    ///
    /// Returns the current offset when the slot is already visible, and the
    /// current offset when the viewport height is still unknown — guessing
    /// there would jump the list to its end on the first frame.
    fn reveal(&self, slot: usize, scroll: &Scroll) -> f32 {
        if scroll.height() <= 0.0 || self.is_empty() {
            return scroll.y();
        }

        let slot = slot.min(self.len() - 1);
        let top = self.top(slot);
        let bottom = top + self.height(slot);

        let view_top = scroll.y();
        let view_bottom = view_top + scroll.height();

        let target = if top < view_top {
            top
        } else if bottom > view_bottom {
            bottom - scroll.height()
        } else {
            view_top
        };

        target.clamp(0.0, self.max_scroll(scroll.height()))
    }

    /// The offset that puts `slot`'s top `dy` pixels below the viewport's top.
    ///
    /// This is the anchor form, for keeping a reader's place when content is
    /// inserted above them: record `(slot, dy)` before the change, ask for the
    /// same pair afterwards with `slot` shifted by however many slots appeared
    /// before it.
    fn anchor(&self, slot: usize, dy: f32, scroll: &Scroll) -> f32 {
        if self.is_empty() {
            return 0.0;
        }

        let target = self.top(slot.min(self.len() - 1)) - dy;

        target.clamp(0.0, self.max_scroll(scroll.height()))
    }
}

/// The offset itself, clamped into the content.
///
/// Shared by both implementations, because "the window may not start past the
/// end" is the same rule for both — and forgetting it is how a windowed list
/// shows a blank page after the window grows.
fn clamp_offset(y: f32, height: f32, max_scroll: f32) -> f32 {
    if height <= 0.0 {
        // The viewport has not been measured yet: take the offset at its word
        // rather than clamping it against a zero-height window.
        y.max(0.0)
    } else {
        y.clamp(0.0, max_scroll)
    }
}

fn empty_slice(total: f32) -> Slice {
    Slice {
        start: 0,
        end: 0,
        before: 0.0,
        after: total,
        total,
    }
}

/// A list of slots that all have the same height.
///
/// No measurement, so no feedback loop and no drift: the scrollbar, the reveal
/// arithmetic and the content height are exact from the first frame.
///
/// Build one with [`fixed`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fixed {
    row: f32,
    len: usize,
    overscan: usize,
}

/// A list of `len` slots, each `row` pixels tall.
///
/// `row` is the height of every slot, and the same number the row's own
/// `Length::Fixed` must use — the two are the invariant, so they should come
/// from one function.
pub fn fixed(row: impl Into<Pixels>, len: usize) -> Fixed {
    Fixed {
        row: row.into().0.max(0.0),
        len,
        overscan: 0,
    }
}

impl Fixed {
    /// Slots to build beyond the visible ones, on both sides.
    ///
    /// Not an optimisation: a viewport rarely starts on a slot boundary, so the
    /// partially visible slots at either end have to be in the tree.
    pub fn overscan(self, slots: usize) -> Self {
        Self {
            overscan: slots,
            ..self
        }
    }

    /// How many slots fit in a viewport of this height.
    pub fn visible(&self, height: f32) -> usize {
        if self.row <= 0.0 {
            return 0;
        }

        (height / self.row).ceil() as usize + 1
    }
}

impl Slots for Fixed {
    fn len(&self) -> usize {
        self.len
    }

    fn total(&self) -> f32 {
        self.row * self.len as f32
    }

    fn top(&self, slot: usize) -> f32 {
        self.row * slot as f32
    }

    fn height(&self, _slot: usize) -> f32 {
        self.row
    }

    fn slice_at(&self, y: f32, height: f32) -> Slice {
        let total = self.total();

        if self.len == 0 || self.row <= 0.0 {
            return empty_slice(total);
        }

        let y = clamp_offset(y, height, self.max_scroll(height));
        let first = ((y / self.row).floor() as usize).min(self.len - 1);

        let start = first.saturating_sub(self.overscan);
        let end = (first + self.visible(height) + self.overscan).min(self.len);

        Slice {
            start,
            end,
            before: self.top(start),
            after: total - self.top(end),
            total,
        }
    }
}

/// A list of slots whose heights are discovered by laying them out.
///
/// Every slot starts at `estimate` and is corrected by [`Measured::measure`],
/// which is what a `sensor` reports once the slot has been rendered. That makes
/// the heights a *cache* rather than a fact, and three rules keep the cache
/// from fighting the layout:
///
/// 1. **Measuring never changes a slot that is already measured.** Unknown
///    slots move; known ones do not, so the table converges instead of
///    oscillating between frames.
/// 2. **The window is positioned by the prefix sum as it stands.** Unmeasured
///    slots are estimates, so the scrollbar is approximate there — that is the
///    price, and correcting it by re-laying-out is how you get the oscillation
///    back.
/// 3. **A measurement is only valid at one width.** A `sensor` reports the size
///    it was laid out at, and text re-wraps when that changes, so
///    [`Measured::measure`] takes the whole [`Size`] and invalidates the table
///    itself when the width moves. Nobody has to remember that on resize.
///
/// Build one with [`measured`].
#[derive(Debug, Clone, PartialEq)]
pub struct Measured {
    len: usize,
    overscan: usize,
    estimate: f32,
    heights: Vec<f32>,
    measured: Vec<bool>,
    /// The width the heights were measured at, once anything has been measured.
    width: Option<f32>,
    /// `prefix[i]` is the top of slot `i`; one longer than `heights`.
    prefix: Vec<f32>,
    /// The lowest index whose prefix may be stale; past `len` means "clean".
    dirty: usize,
}

/// A list of `len` slots, each assumed to be `estimate` pixels tall until a
/// measurement says otherwise.
pub fn measured(estimate: impl Into<Pixels>, len: usize) -> Measured {
    let estimate = estimate.into().0.max(0.0);

    Measured {
        len,
        overscan: 0,
        estimate,
        heights: vec![estimate; len],
        measured: vec![false; len],
        width: None,
        prefix: (0..=len).map(|i| estimate * i as f32).collect(),
        dirty: len + 1,
    }
}

impl Measured {
    /// Slots to build beyond the visible ones. See [`Fixed::overscan`].
    pub fn overscan(mut self, slots: usize) -> Self {
        self.overscan = slots;
        self
    }

    /// What a slot claims to be: measured if known, estimated if not.
    pub fn height_of(&self, slot: usize) -> f32 {
        self.heights.get(slot).copied().unwrap_or(self.estimate)
    }

    /// Whether a slot has been measured at the current width.
    pub fn is_measured(&self, slot: usize) -> bool {
        self.measured.get(slot).copied().unwrap_or(false)
    }

    /// The width every measurement so far was taken at.
    pub fn width(&self) -> Option<f32> {
        self.width
    }

    /// Records a slot's real size, as reported by a `sensor`.
    ///
    /// Returns whether the content height changed — the app needs that to know
    /// whether the anchor it is holding still means anything.
    ///
    /// A different width from the last measurement invalidates every height
    /// first, because a re-wrap changes all of them, not just this one.
    pub fn measure(&mut self, slot: usize, size: Size) -> bool {
        if slot >= self.len {
            return false;
        }

        let width = size.width.round();

        if self.width.is_some_and(|known| known.round() != width) {
            self.heights.fill(self.estimate);
            self.measured.fill(false);
            self.dirty = 0;
        }

        self.width = Some(size.width);

        let total_before = self.total();

        self.heights[slot] = size.height.max(0.0);
        self.measured[slot] = true;
        self.dirty = self.dirty.min(slot);
        self.rebuild_prefix();

        self.total() != total_before
    }

    /// Inserts `slots` unmeasured slots at the front.
    ///
    /// For loading older items above what is already shown: the app shifts any
    /// anchor it holds by the same count, and then asks [`Slots::anchor`] for
    /// the offset that keeps the reader in place.
    ///
    /// A block grid only survives this if the number of inserted slots is a
    /// multiple of the block size — otherwise every block boundary moves, and
    /// every measurement with it.
    pub fn prepend(&mut self, slots: usize) {
        if slots == 0 {
            return;
        }

        self.len += slots;
        self.heights = core::iter::repeat_n(self.estimate, slots)
            .chain(self.heights.drain(..))
            .collect();
        self.measured = core::iter::repeat_n(false, slots)
            .chain(self.measured.drain(..))
            .collect();

        // The old prefix sum is shifted by `slots * estimate`, but the measured
        // slots keep their heights, so it has to be rebuilt rather than
        // re-filled with the estimate.
        self.prefix.clear();
        self.prefix.push(0.0);

        for i in 0..self.len {
            self.prefix.push(self.prefix[i] + self.heights[i]);
        }

        self.dirty = self.len + 1;
    }

    /// Recomputes the prefix sum from the lowest stale index.
    ///
    /// Measuring is O(1) plus this, and this is O(n) in the worst case (a
    /// measurement near the front). In practice measurements arrive for the
    /// slots that are visible, and a batch of them costs one pass — but if a
    /// profile ever points here, this is the line to replace with a Fenwick
    /// tree.
    fn rebuild_prefix(&mut self) {
        if self.dirty > self.len {
            return;
        }

        for i in self.dirty.max(1)..=self.len {
            self.prefix[i] = self.prefix[i - 1] + self.heights[i - 1];
        }

        self.dirty = self.len + 1;
    }

    /// The slot that owns the pixel at `y`.
    fn slot_at(&self, y: f32) -> usize {
        if self.len == 0 {
            return 0;
        }

        let after = self.prefix.partition_point(|&edge| edge <= y.max(0.0));

        after.saturating_sub(1).min(self.len - 1)
    }
}

impl Slots for Measured {
    fn len(&self) -> usize {
        self.len
    }

    fn total(&self) -> f32 {
        self.prefix[self.len]
    }

    fn top(&self, slot: usize) -> f32 {
        self.prefix[slot.min(self.len)]
    }

    fn height(&self, slot: usize) -> f32 {
        self.height_of(slot)
    }

    fn slice_at(&self, y: f32, height: f32) -> Slice {
        let total = self.total();

        if self.len == 0 {
            return empty_slice(total);
        }

        let y = clamp_offset(y, height, self.max_scroll(height));
        let first = self.slot_at(y);
        let last = self.slot_at(y + height.max(0.0));

        let start = first.saturating_sub(self.overscan);
        let end = (last + 1 + self.overscan).min(self.len);

        Slice {
            start,
            end,
            before: self.top(start),
            after: total - self.top(end),
            total,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROW: f32 = 24.0;

    #[test]
    fn scroll_records_what_the_writer_and_the_view_agree_on() {
        let mut scroll = Scroll::new();
        assert_eq!(scroll.y(), 0.0);
        assert_eq!(scroll.height(), 0.0);

        let payload = scroll.to(120.0);

        assert_eq!(scroll.y(), 120.0);
        assert_eq!(payload.y, Some(120.0));
        assert_eq!(payload.x, None);
    }

    #[test]
    fn resizing_keeps_the_offset_and_on_scroll_keeps_both() {
        let mut scroll = Scroll::at(240.0, 480.0);

        scroll.resized(600.0);

        assert_eq!(scroll.y(), 240.0);
        assert_eq!(scroll.height(), 600.0);
    }

    #[test]
    fn fixed_slices_the_viewport() {
        let path = fixed(ROW, 100);
        let slice = path.slice_at(240.0, 480.0);

        // 240/24 = slot 10, 480/24 = 20 rows, plus one for the partial one.
        assert_eq!(slice.range(), 10..31);
        assert_eq!(slice.before(), 240.0);
        assert_eq!(slice.total(), 2400.0);
        assert_eq!(slice.before() + slice.after() + 21.0 * ROW, 2400.0);
    }

    #[test]
    fn overscan_widens_both_sides_without_moving_the_invariant() {
        let path = fixed(ROW, 100).overscan(2);
        let slice = path.slice_at(240.0, 480.0);

        assert_eq!(slice.range(), 8..33);
        assert_eq!(slice.before(), 8.0 * ROW);
        assert_eq!(slice.total(), 2400.0);
        assert_eq!(slice.before() + slice.after() + 25.0 * ROW, 2400.0);
    }

    #[test]
    fn fixed_clamps_at_the_end() {
        let path = fixed(ROW, 100);
        let slice = path.slice_at(100_000.0, 480.0);

        // Clamped to the last screenful: the window never starts past the end
        // and never goes blank.
        assert_eq!(slice.range().end, 100);
        assert_eq!(slice.after(), 0.0);
        assert_eq!(
            slice.before() + slice.after() + slice.len() as f32 * ROW,
            path.total()
        );
    }

    #[test]
    fn an_offset_past_the_end_is_clamped_rather_than_blank() {
        let path = fixed(ROW, 10);

        // A window that grew leaves the kept offset past the content.
        let slice = path.slice_at(10_000.0, 480.0);

        assert!(!slice.is_empty());
        assert_eq!(slice.range().end, 10);
    }

    #[test]
    fn an_empty_list_still_reports_its_total() {
        let path = fixed(ROW, 0);
        let slice = path.slice_at(0.0, 480.0);

        assert!(slice.is_empty());
        assert_eq!(slice.total(), 0.0);
        assert_eq!(slice.before(), 0.0);
        assert_eq!(slice.after(), 0.0);
    }

    #[test]
    fn reveal_moves_the_minimum() {
        let path = fixed(ROW, 100);
        let scroll = Scroll::at(240.0, 480.0);

        // Already visible: no movement.
        assert_eq!(path.reveal(15, &scroll), 240.0);
        // Above the viewport: scroll up to its top.
        assert_eq!(path.reveal(3, &scroll), 72.0);
        // Below: put its bottom on the viewport's bottom.
        assert_eq!(path.reveal(40, &scroll), 41.0 * ROW - 480.0);
    }

    #[test]
    fn reveal_refuses_to_guess_without_a_height() {
        let path = fixed(ROW, 100);
        let scroll = Scroll::at(240.0, 0.0);

        // Not 0.0, and not the end of the list: nothing moves.
        assert_eq!(path.reveal(90, &scroll), 240.0);
    }

    #[test]
    fn anchor_puts_a_slot_at_a_known_depth() {
        let path = fixed(ROW, 100);
        let scroll = Scroll::at(240.0, 480.0);

        // Slot 20 is at 480; asking for it 10px down asks for offset 470.
        assert_eq!(path.anchor(20, 10.0, &scroll), 470.0);
        assert_eq!(path.anchor(0, 10.0, &scroll), 0.0);
        assert_eq!(path.anchor(99, 0.0, &scroll), 2400.0 - 480.0);
    }

    #[test]
    fn measured_starts_at_the_estimate() {
        let path = measured(40.0, 10);

        assert_eq!(path.total(), 400.0);
        assert_eq!(path.top(3), 120.0);
        assert_eq!(path.height_of(3), 40.0);
        assert!(!path.is_measured(3));
        assert_eq!(path.width(), None);
    }

    #[test]
    fn measuring_reports_whether_the_total_moved() {
        let mut path = measured(40.0, 10);

        assert!(path.measure(2, Size::new(600.0, 100.0)));
        assert_eq!(path.height_of(2), 100.0);
        assert_eq!(path.top(3), 2.0 * 40.0 + 100.0);
        assert_eq!(path.total(), 10.0 * 40.0 + 60.0);
        assert_eq!(path.width(), Some(600.0));

        // The same measurement again changes nothing.
        assert!(!path.measure(2, Size::new(600.0, 100.0)));
        assert_eq!(path.total(), 460.0);
    }

    #[test]
    fn a_new_width_invalidates_every_height() {
        let mut path = measured(40.0, 4);
        path.measure(1, Size::new(600.0, 100.0));
        assert_eq!(path.height_of(1), 100.0);

        // A rebuilt layout at a new width re-wraps every slot, so every
        // measurement is suspect — including the one being reported now.
        path.measure(2, Size::new(400.0, 70.0));

        assert_eq!(path.height_of(1), 40.0);
        assert_eq!(path.height_of(2), 70.0);
        assert!(!path.is_measured(1));
        assert_eq!(path.width(), Some(400.0));
    }

    #[test]
    fn measured_finds_the_slot_under_a_pixel() {
        let mut path = measured(40.0, 5);
        path.measure(1, Size::new(600.0, 100.0));

        assert_eq!(path.slot_at(0.0), 0);
        assert_eq!(path.slot_at(39.0), 0);
        assert_eq!(path.slot_at(40.0), 1);
        assert_eq!(path.slot_at(139.0), 1);
        assert_eq!(path.slot_at(140.0), 2);
        // Past the end clamps to the last slot.
        assert_eq!(path.slot_at(10_000.0), 4);
    }

    #[test]
    fn measured_slice_and_reveal_agree_with_the_prefix_sum() {
        let mut path = measured(40.0, 20).overscan(1);
        path.measure(5, Size::new(600.0, 80.0));

        let scroll = Scroll::at(path.top(5), 200.0);
        let slice = path.slice(&scroll);

        assert_eq!(slice.total(), path.total());
        assert_eq!(slice.before(), path.top(slice.range().start));
        assert_eq!(slice.after(), path.total() - path.top(slice.range().end));
        assert!(slice.range().contains(&5));
        assert_eq!(path.reveal(5, &scroll), scroll.y());
    }

    #[test]
    fn prepend_shifts_measurements_and_keeps_the_shapes_valid() {
        let mut path = measured(40.0, 4);
        path.measure(0, Size::new(600.0, 100.0));
        assert_eq!(path.total(), 4.0 * 40.0 + 60.0);

        path.prepend(2);

        assert_eq!(path.len(), 6);
        assert_eq!(path.total(), 6.0 * 40.0 + 60.0);
        assert_eq!(path.top(0), 0.0);
        assert!(!path.is_measured(0));
        // The slot that was measured is still the same slot, two places later.
        assert!(path.is_measured(2));
        assert_eq!(path.height_of(2), 100.0);
        assert_eq!(path.top(3), 2.0 * 40.0 + 100.0);
        assert_eq!(path.slot_at(0.0), 0);
        assert_eq!(path.slice_at(0.0, 80.0).total(), path.total());
    }
}
