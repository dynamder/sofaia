//! The keyboard grammar.
//!
//! iced hands the program a stream of raw key events. This module is the only
//! place that turns those keys into something the application understands:
//!
//! ```text
//! Event::Keyboard ──► Chord      (read, at the edge, once)
//!                 ──► Intent     (fed here, by the state machine)
//!                 ──► App::dispatch  (applied to the panes)
//! ```
//!
//! [`Chord`] exists so the grammar never has to look at iced's [`Key`] or
//! [`Modifiers`] again: `Shift` is already applied and `Ctrl` is normalized.
//! [`Intent`] exists so the rest of the program never has to know which key
//! produced it.
//!
//! Nothing here touches a window, a widget, or a `Task`, which is what makes
//! the whole keymap testable without a UI.

use iced::keyboard::key::Named;
use iced::keyboard::{Key, Modifiers};

use crate::intent::{Intent, Motion};

/// A normalized key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Chord {
    /// A character, with `Shift` already applied by the platform (`g` vs `G`).
    Char(char),
    /// `Ctrl` (or `Cmd` on macOS) plus a character, lowercased.
    Ctrl(char),
    /// A named key, such as `Escape`, `Enter`, or an arrow key.
    Named(Named),
}

impl Chord {
    /// Reads a [`Chord`] out of a raw iced key.
    ///
    /// Callers should pass `modified_key` (which has `Shift` applied but not
    /// `Ctrl`), so `Shift+g` arrives as [`Chord::Char`]`('G')` and `Ctrl+d` as
    /// [`Chord::Ctrl`]`('d')`.
    pub fn read(key: &Key, modifiers: Modifiers) -> Option<Self> {
        match key {
            Key::Character(text) => {
                let character = text.chars().next()?;

                if modifiers.control() {
                    Some(Self::Ctrl(character.to_ascii_lowercase()))
                } else {
                    Some(Self::Char(character))
                }
            }
            Key::Named(named) => Some(Self::Named(*named)),
            Key::Unidentified => None,
        }
    }
}

/// The key state machine.
///
/// Today every chord completes a sequence immediately, so this holds no state.
/// It is a struct rather than a free function because the next steps (counts,
/// `gg`, operators) need somewhere to keep a half-finished sequence, and that
/// state belongs next to the rules that read it.
#[derive(Debug, Default, Clone, Copy)]
pub struct ChordState;

impl ChordState {
    /// Feeds one key press.
    ///
    /// Returns [`None`] when the key is unbound or when it only started a
    /// sequence that is not finished yet.
    pub fn feed(&mut self, chord: Chord) -> Option<Intent> {
        match chord {
            Chord::Char('j') => Some(Intent::Move {
                motion: Motion::Next,
            }),
            Chord::Char('k') => Some(Intent::Move {
                motion: Motion::Previous,
            }),
            Chord::Char('h') => Some(Intent::Parent),
            Chord::Char('l') => Some(Intent::Open),
            // The keyboard's contribution to dismissal: `Escape` is the one key
            // that means nothing on its own and exactly one thing when
            // something is modal, which is the shape of `Dismiss`.
            Chord::Named(Named::Escape) => Some(Intent::Dismiss),
            _ => None,
        }
    }
}
