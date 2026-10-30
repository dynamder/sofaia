//! Opaque layers.
//!
//! A `Stack` asks its layers top-down and stops at the first one that captures
//! the event. That handles *presses*. It does not handle *hover*: `on_enter`
//! never captures, so a cursor over a panel still lights up the rows below it.
//!
//! What actually stops the layers below is a non-[`None`] [`mouse::Interaction`]
//! reported at the cursor position — the `Stack` then levitates the cursor for
//! everything underneath. This is worth repeating because it is the number one
//! stacked-widget bug in iced:
//!
//! > **A background colour does not block clicks. Only `mouse_interaction` does.**
//!
//! A `container` with an opaque fill still reports `Interaction::None`, so a
//! panel built out of containers is a filter: clicks and hover pass straight
//! through it. [`OpaqueExt::opaque`] marks the region as solid with
//! [`mouse::Interaction::Idle`] — a real interaction value, i.e. opaque to the
//! layers below, but the default arrow rather than a hand.

use iced_core::mouse;
use iced_widget::{Container, MouseArea, mouse_area};

/// Marks a container as solid to everything below it, without giving it a
/// gesture of its own.
///
/// Use this for panels, scrims and cards — anywhere the *only* thing that
/// should happen is "the layers underneath do not see the pointer". If the
/// region also has a gesture, use [`AimExt::aimed`](crate::AimExt::aimed)
/// instead: it is already opaque.
pub trait OpaqueExt<'a, M: 'a> {
    fn opaque(self) -> MouseArea<'a, M>;
}

impl<'a, M: 'a> OpaqueExt<'a, M> for Container<'a, M> {
    fn opaque(self) -> MouseArea<'a, M> {
        mouse_area(self).interaction(mouse::Interaction::Idle)
    }
}
