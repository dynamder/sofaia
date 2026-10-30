//! A typed visual language for iced widgets.
//!
//! iced styles are closures from a theme to a `Style` struct. Flexible, and
//! completely unreadable at a call site: `text(..).style(|theme| ..)` says
//! nothing about *what* the text is. This module splits the two halves:
//!
//! * a **kind** — the application's type — names what a widget *is*;
//! * a **recipe** — this module's trait — says how that kind paints;
//! * [`StyleExt::styled`] does the wiring, once per widget type.
//!
//! The kinds stay in the application, because "a sidebar" and "a preview" only
//! mean something there, and a library that guessed them would be a theme, not
//! a tool. What lives here is the machinery plus [`Tokens`]: the palette every
//! recipe reads, derived from `Theme` so that a recipe never touches raw
//! colours.

pub mod ext;
pub mod recipe;
pub mod tokens;

pub use ext::StyleExt;
pub use recipe::{ButtonRecipe, ContainerRecipe, Frame, RuleRecipe, TextRecipe};
pub use tokens::Tokens;
