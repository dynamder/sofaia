//! ERgonomic ICED: the parts every iced application writes again.
//!
//! This crate has one rule, and everything in it follows from that rule:
//!
//! > **Nothing here knows anything about the application using it.**
//!
//! No target type, no intent type, no pane names, no widths, no colours. What
//! it does know is the *shape* of six problems every iced application meets:
//!
//! | module | the problem | what it fixes |
//! | --- | --- | --- |
//! | [`style`] | `text(..).style(\|theme\| …)` says nothing about what the text *is* | a typed kind + a recipe, wired once |
//! | [`AimExt`] | `on_press` and `on_double_click` must name the same thing | one target, one constructor |
//! | [`Meaning`] | a gesture means different things on different components | the rule lives on the target type, not in a table |
//! | [`OpaqueExt`] | a panel is not a filter, but iced treats it like one | opacity as an explicit marker |
//! | [`WheelDirection`] | positive `y` means *up*, and nobody remembers | the convention, written once |
//! | [`WheelArea`] | the wheel is claimed by whatever `Scrollable` is under the pointer | the same claim, reported as a direction instead |
//! | [`list`] | a list is longer than the window it is drawn in | the slice, its padding and the offset, from one piece of arithmetic |
//!
//! # The aimed half, in three pieces
//!
//! ```text
//! Aimed<T>     a widget reports (Aim, T) — a fact, with no meaning
//! Meaning<I>   "T means I when aimed at" — implemented by the component
//! interpret    the bridge; generic, so it never grows
//! ```
//!
//! # What is deliberately *not* here
//!
//! Anything with a policy:
//!
//! * which gesture means which action — that is [`Meaning`], implemented by the
//!   application;
//! * what a button release means — iced does not capture it, so it cannot be
//!   owned by a widget library;
//! * what a row looks like — that is a visual language, and this crate only
//!   ships the machinery for one;
//! * **how tall a row is.** [`list::Fixed`] is told, [`list::Measured`] is
//!   told an estimate; neither guesses. A row's height is a design decision,
//!   and a library that guesses it is a library that disagrees with the theme;
//! * **blanket `From` impls.** A blanket `impl<T, M: From<T>> From<Aimed<T>> for
//!   M` would make every application's own adapter impossible. The `From` space
//!   is left empty on purpose.
//!
//! # Three mouse facts that types cannot express
//!
//! A library is also its documentation, so these are here rather than nowhere:
//!
//! * `on_release` never captures, and only publishes while the pointer is still
//!   inside the bounds. A "button is down" flag must be cleared from the event
//!   subscription, never from the widget tree.
//! * Hit testing ignores clipping: `mouse_area` drops the `viewport` it is
//!   handed. Inside a `scrollable` the cursor is levitated by the container
//!   instead; nothing else will save you.
//! * A `Stack` asks its children top-down, so an `on_enter` on the upper layer
//!   arrives **before** the lower layer's `on_exit` — the opposite order from a
//!   `Column`. Never rely on that order; give each layer its own concern.

pub mod list;
pub mod motion;
pub mod style;

mod aim;
mod meaning;
mod opaque;
mod wheel;

pub use aim::{Aim, AimExt, Aimed};
pub use meaning::{Meaning, interpret};
pub use opaque::OpaqueExt;
pub use wheel::{WheelArea, WheelDirection, wheel_area};
