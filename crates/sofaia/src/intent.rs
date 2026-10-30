//! What the user asked for.
//!
//! Every device grammar ends here: [`crate::keys`] for the keyboard and
//! [`crate::gesture`] for the ambient half of the mouse. The aimed half does not
//! pass through a grammar at all — a component says what pointing at *it* means,
//! through [`Meaning`](ericed::Meaning), and produces one of these intents
//! directly.
//!
//! ```text
//! keys.rs      Event::Keyboard ──► Chord ──┐
//!                                          ├──► Intent ──► App::dispatch
//! gesture.rs   WheelDirection  ────────────┤
//! components   Aimed<Row/…>    ────────────┘   (via ericed::Meaning)
//! ```
//!
//! Two rules keep the vocabulary small:
//!
//! 1. An intent names a goal, never a device. There is no `Click` or `Key`
//!    here: `Open` is `Open` whether `l`, a double click or a menu item
//!    produced it.
//! 2. An intent is terminal. Applying one changes state or starts a `Task`, and
//!    never produces another intent, so [`crate::AppMsg::Intent`] cannot loop.
//!
//! The payload types are the vocabulary, and they are narrow on purpose: only a
//! [`Row`] can be focused, only a [`Choice`] can be chosen. Writing
//! `Intent::Focus(Choice(0))` does not compile, which is the type-level version
//! of a rule that used to be a `_ => None` arm in a match.

/// Where the cursor should go, relative to where it is now.
///
/// The pane owns the list, so it owns the clamping: a motion says which way to
/// go, not where to stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Motion {
    Next,
    Previous,
}

/// A row of the middle pane's listing.
///
/// An *address*: pressing one points at it, and there is a cursor that moves.
/// The index is a position, not an identity — an asynchronous read can replace
/// the listing underneath it, so applying one must bounds-check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Row(pub usize);

/// A menu item.
///
/// A *choice*: pressing one commits it. There is no cursor inside a menu to move
/// first, which is why the same gesture that focuses a row chooses an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Choice(pub usize);

/// What the user asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    /// Move the cursor one row.
    Move { motion: Motion },
    /// Scroll the view one row, without moving the cursor.
    ///
    /// The wheel's contribution. It is the one pointer gesture with no target,
    /// so it cannot arrive through [`Meaning`](ericed::Meaning); and now that
    /// the listing is windowed, "one row further down the *view*" is a
    /// different request from "one row further down the *list*".
    Scroll { motion: Motion },
    /// Put the cursor on a row.
    ///
    /// The mouse's contribution: an absolute address instead of a relative
    /// step. Pointing at something is still not the same as opening it.
    Focus(Row),
    /// Enter the folder under the cursor.
    Open,
    /// Go to the parent folder.
    Parent,
    /// Show the context menu for a row.
    Menu(Row),
    /// Put the cursor in the search box.
    ///
    /// The only intent that needs the window's input method: a folder full of
    /// Chinese names cannot be searched for without one, and iced enables it
    /// per focused widget rather than per window.
    Search,
    /// Commit a choice, such as a menu item.
    Choose(Choice),
    /// Close whatever is modal. Reachable from both devices: a press on the
    /// scrim, or `Escape`.
    Dismiss,
}
