//! What pointing at something means.
//!
//! A `mouse_area` can only tell you *which* target was hit and *what* the
//! gesture was. It cannot know what that means: the same right-click opens a
//! menu on a row and does nothing on a panel. That knowledge belongs to the
//! component that owns the target, and a trait is how it gets there.
//!
//! So the interpretation is a method on the *target type*, not a table over
//! every target kind:
//!
//! ```text
//! before: one match over (gesture × target variant), far from both
//! after:  one impl per component, next to the component
//! ```
//!
//! This module is only the seam. It knows neither the target nor the meaning
//! type — `I` is the application's own vocabulary — so it can live in a library
//! while the rules stay next door to the components they describe.

use crate::Aim;
use crate::Aimed;

/// What a target means when it is aimed at.
///
/// Implemented by the *target*, one impl per component, each exhaustive over
/// [`Aim`]. Two consequences follow, and both are the point:
///
/// * adding a gesture to [`Aim`] breaks every component's build, so the
///   decision is forced instead of defaulted;
/// * the same gesture means whatever each component says it means, without a
///   central table knowing that components exist.
///
/// `I` is the consumer's meaning type — an `Intent`, a `Task`, whatever the
/// application dispatches on. This crate never names it.
pub trait Meaning<I> {
    fn meaning(self, aim: Aim) -> Option<I>;
}

/// The bridge: the only place that switches on the gesture, and it does so
/// generically, so it never has to grow.
pub fn interpret<I, T: Meaning<I>>(aimed: Aimed<T>) -> Option<I> {
    let (aim, target) = aimed.into_parts();

    target.meaning(aim)
}
