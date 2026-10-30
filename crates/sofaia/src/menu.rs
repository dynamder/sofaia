//! A simulated context menu.
//!
//! "Simulated" is doing real work in that sentence: the panel appears in the
//! middle of the window instead of at the pointer, because a target is an
//! address ("the third row"), never a coordinate. Positioning a menu at the
//! cursor is the day the target has to learn about a `Point` — and the day it
//! stops surviving a resize.
//!
//! The menu is three `Stack` layers, and each one has exactly one job:
//!
//! ```text
//! panel    the card: opaque (OpaqueExt), its items are aimed (AimExt)
//! scrim    dims everything, and answers presses aimed at Target::Scrim
//! base     the app: it never sees a press while the scrim is up
//! ```
//!
//! That scrim is what makes "click outside to close" work without a single line
//! of event forwarding. It is an aimed target like any other, and `Stack` asks
//! it in the right order — panel first, then scrim, then the app.
//!
//! The menu owns *two* regions, so it owns one small union: [`Target`]. That is
//! the only union in the application. A union per component is fine — a union
//! for the whole application is what forces a central table of meanings.
//!
//! Most of the menu is filler, and the filler is instructive: a placeholder has
//! **no target**, so nothing can be aimed at it and no meaning has to be
//! invented for it. It still cannot be clicked *through*, because the card is
//! opaque — a press on a placeholder lands on the card and stops there. That is
//! exactly how a disabled menu item behaves, and it costs zero lines of policy.
//!
//! # Why only the entrance is animated
//!
//! Opening starts a [`Track`], and the panel arrives by two means of the same
//! value: the scrim's alpha and the card's size. Neither touches layout — a card
//! drawn at 96% still occupies 100% of its box — so nothing underneath moves
//! while it arrives.
//!
//! Closing is immediate, and that is deliberate: a fade-out would leave the
//! scrim over a listing that is already listening for presses, and a layer that
//! dims without blocking is worse than no layer at all. An exit animation needs
//! a state that is *not* the open one and *not* the closed one; ericed ships no
//! policy for that, and this menu does not invent one.

use std::time::Duration;

use iced::time::Instant;
use iced::widget::{column, container, space, text};
use iced::{Element, Fill};

use ericed::motion::{self, Curve, Track};
use ericed::style::Tokens;
use ericed::{Aim, AimExt, Meaning, OpaqueExt};

use crate::AppMsg;
use crate::intent::{Choice, Intent, Row};
use crate::style::container::RowInput;
use crate::style::{self, StyleExt};

/// How long the panel takes to arrive.
const ENTRANCE: Duration = Duration::from_millis(160);

/// How small the panel starts, as a fraction of its size.
///
/// Small enough to read as "it came in", large enough that the text keeps its
/// shape: the card is *drawn* smaller while being *laid out* at full size, so
/// nothing re-wraps on the way.
const POP: f32 = 0.96;

/// A region of the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    /// A clickable line of the panel, by its index in `LINES`.
    Item(usize),
    /// The dimming layer that covers everything else.
    Scrim,
}

/// What pointing at a region of the menu means.
///
/// The same [`Aim::Press`] that *focuses* a row of the listing *chooses* a menu
/// line and *dismisses* the scrim. That difference used to be three arms of one
/// central table; now it is one impl, next to the thing it describes.
impl Meaning<Intent> for Target {
    fn meaning(self, aim: Aim) -> Option<Intent> {
        match (self, aim) {
            // No cursor inside a menu to move first: a press is a decision.
            (Self::Item(index), Aim::Press) => Some(Intent::Choose(Choice(index))),
            // The first press already chose it, and there are no submenus.
            (Self::Item(_), Aim::DoubleClick | Aim::RightPress) => None,
            // The empty area means "close what is open" and nothing else.
            (Self::Scrim, Aim::Press) => Some(Intent::Dismiss),
            (Self::Scrim, Aim::DoubleClick | Aim::RightPress) => None,
        }
    }
}

/// What a menu item does once chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Open,
    Dismiss,
}

impl Action {
    fn label(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::Dismiss => "Dismiss",
        }
    }
}

/// One line of the panel.
///
/// The menu is data, so a [`Choice`] index has something to mean. Only
/// [`Line::Action`] lines are aimed at; the rest is there to look like a menu,
/// and "not being a target" is what makes them inert without any extra rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Line {
    Action(Action),
    Placeholder(&'static str),
    Separator,
}

const LINES: &[Line] = &[
    Line::Action(Action::Open),
    Line::Placeholder("Open in new window"),
    Line::Placeholder("Reveal in parent folder"),
    Line::Separator,
    Line::Placeholder("Rename"),
    Line::Placeholder("Copy path"),
    Line::Placeholder("Move to…"),
    Line::Placeholder("Delete"),
    Line::Separator,
    Line::Action(Action::Dismiss),
];

pub struct Menu {
    open: Option<Open>,
    /// The entrance: 0 while the panel is arriving, 1 once it has.
    ///
    /// A plain [`Track`] in the model, like the listing's slide: `update`
    /// advances it, `view` reads it, and nothing inside a widget reads a clock.
    entrance: Track,
}

struct Open {
    /// The row the menu is about. Kept even if the cursor wanders: a menu is a
    /// snapshot of a target, not a live view of the cursor.
    row: Row,
    title: String,
}

impl Default for Menu {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu {
    pub fn new() -> Self {
        Self {
            open: None,
            // Settled: a menu that is not on screen has nothing to animate.
            entrance: Track::new(1.0),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    pub fn open(&mut self, row: Row, title: String) {
        self.open = Some(Open { row, title });

        // A fresh track each time, because the previous entrance finished at 1:
        // reusing it would pop the panel in at full size on the second opening.
        self.entrance = Track::new(0.0).curve(Curve::out(ENTRANCE));
        self.entrance.to(1.0);
    }

    pub fn close(&mut self) {
        self.open = None;
    }

    /// Advances the entrance. Called once per frame message, while it moves.
    pub fn tick(&mut self, now: Instant) {
        self.entrance.tick(now);
    }

    /// Whether the panel is still arriving.
    ///
    /// A closed menu answers `false` even if its track was interrupted: nothing
    /// is on screen, so nothing needs a frame. That check is what stops a menu
    /// dismissed mid-entrance from keeping the window redrawing.
    pub fn is_moving(&self) -> bool {
        self.is_open() && self.entrance.is_moving()
    }

    pub fn row(&self) -> Option<Row> {
        self.open.as_ref().map(|open| open.row)
    }

    pub fn title(&self) -> Option<&str> {
        self.open.as_ref().map(|open| open.title.as_str())
    }

    /// What the line at `index` would do, if a menu is open at all.
    ///
    /// Placeholders and separators answer [`None`]: even if a `Choice` for one
    /// of them could exist, it would have nothing to do.
    pub fn action(&self, index: usize) -> Option<Action> {
        self.open.as_ref()?;

        match LINES.get(index)? {
            Line::Action(action) => Some(*action),
            Line::Placeholder(_) | Line::Separator => None,
        }
    }

    /// The dimming layer. It covers the whole window and answers presses aimed
    /// at [`Target::Scrim`], which is how a click outside closes the menu
    /// without the list underneath ever seeing it.
    ///
    /// The dimming is a palette, not a layer: iced has no subtree opacity, so
    /// the entrance fades `Tokens::scrim` — already translucent — rather than
    /// the drawing. The layer still covers everything from the first frame,
    /// which is why a click in the corner dismisses a menu that is still
    /// arriving.
    pub fn scrim(&self) -> Element<'_, AppMsg> {
        let entrance = self.entrance.value();

        container(space())
            .styled_with(style::container::scrim(), move |theme| {
                Tokens::from_theme(theme).fade(entrance)
            })
            .aimed(Target::Scrim)
            .into()
    }

    /// The panel, centred.
    ///
    /// The outer container is deliberately *not* opaque: it is the full-size
    /// layer that positions the card, and a press beside the card must reach the
    /// scrim. The card itself is opaque, so a press on its padding lands
    /// nowhere instead of falling through to the list.
    ///
    /// The card is drawn at `POP`…1 of its size while it arrives. It is not
    /// laid out that way: `motion::scale` moves pixels, not boxes, so the card
    /// under the pointer is the card that was laid out, and hit testing follows
    /// the drawing.
    pub fn panel(&self) -> Element<'_, AppMsg> {
        let title = self.title().unwrap_or_default();
        let entrance = self.entrance.value();

        let mut card = column![
            container(text(title.to_owned()).styled(style::text::caption()))
                .styled(style::container::header()),
            style::rule::horizontal(),
        ];

        for (index, line) in LINES.iter().enumerate() {
            card = card.push(match line {
                Line::Action(action) => item(action.label(), Some(Target::Item(index))),
                Line::Placeholder(label) => item(label, None),
                Line::Separator => style::rule::horizontal().into(),
            });
        }

        card = card.push(
            container(text("Esc closes this menu").styled(style::text::caption()))
                .styled(style::container::header()),
        );

        let card = container(card)
            .styled_with(style::container::menu(), move |theme| {
                Tokens::from_theme(theme).fade(entrance)
            })
            .opaque();

        let card = motion::scale(card, POP + (1.0 - POP) * entrance);

        container(card).center_x(Fill).center_y(Fill).into()
    }
}

/// A line of the panel: aimed at when it has a target, muted and inert when it
/// does not.
fn item(label: &str, target: Option<Target>) -> Element<'_, AppMsg> {
    let text = match target {
        Some(_) => text(label).styled(style::text::body()),
        None => text(label).styled(style::text::muted()),
    };

    let row = container(text).styled(style::container::row(RowInput::default()));

    match target {
        Some(target) => row.aimed(target).into(),
        None => row.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(millis: u64) -> Instant {
        Instant::now() + Duration::from_millis(millis)
    }

    #[test]
    fn a_closed_menu_holds_no_frames() {
        let menu = Menu::new();

        assert!(!menu.is_moving());
        assert_eq!(menu.entrance.value(), 1.0, "nothing moves before it opens");
    }

    #[test]
    fn the_panel_arrives_from_smaller_and_stops_when_it_gets_there() {
        let mut menu = Menu::new();

        menu.open(Row(1), "Projects".to_owned());

        // Declared, so there is something to do…
        assert!(menu.is_moving());
        // …but no frame has passed, so nothing has moved yet.
        assert_eq!(menu.entrance.value(), 0.0);

        let start = at(0);
        menu.tick(start);

        assert!(menu.is_moving(), "it has to be drawn at least once");

        menu.tick(start + ENTRANCE);

        assert!(!menu.is_moving());
        assert_eq!(menu.entrance.value(), 1.0, "and it ends where it belongs");
    }

    #[test]
    fn the_entrance_restarts_from_nothing_every_time() {
        let mut menu = Menu::new();

        menu.open(Row(0), "a".to_owned());
        let start = at(0);
        menu.tick(start);
        menu.tick(start + ENTRANCE);
        assert_eq!(menu.entrance.value(), 1.0);

        menu.close();
        menu.open(Row(2), "b".to_owned());

        assert_eq!(menu.entrance.value(), 0.0, "the second opening starts over");
        assert!(menu.is_moving());
    }

    #[test]
    fn dismissing_an_arriving_menu_stops_the_frames() {
        let mut menu = Menu::new();

        menu.open(Row(0), "a".to_owned());
        menu.tick(at(0));

        // Closed mid-entrance: the track is unfinished, but the menu is gone —
        // a dimming layer that keeps asking for frames is a bug, not an exit
        // animation.
        menu.close();

        assert!(!menu.is_open());
        assert!(!menu.is_moving());
    }
}
