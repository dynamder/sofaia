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

use iced::widget::{column, container, space, text};
use iced::{Element, Fill};

use ericed::{Aim, AimExt, Meaning, OpaqueExt};

use crate::AppMsg;
use crate::intent::{Choice, Intent, Row};
use crate::style::container::RowInput;
use crate::style::{self, StyleExt};

/// A region of the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    /// A clickable line of the panel, by its index in [`LINES`].
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
        Self { open: None }
    }

    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    pub fn open(&mut self, row: Row, title: String) {
        self.open = Some(Open { row, title });
    }

    pub fn close(&mut self) {
        self.open = None;
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
    pub fn scrim(&self) -> Element<'_, AppMsg> {
        container(space())
            .styled(style::container::scrim())
            .aimed(Target::Scrim)
            .into()
    }

    /// The panel, centred.
    ///
    /// The outer container is deliberately *not* opaque: it is the full-size
    /// layer that positions the card, and a press beside the card must reach the
    /// scrim. The card itself is opaque, so a press on its padding lands
    /// nowhere instead of falling through to the list.
    pub fn panel(&self) -> Element<'_, AppMsg> {
        let title = self.title().unwrap_or_default();

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

        let card = container(card).styled(style::container::menu()).opaque();

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
