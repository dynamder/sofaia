//! Aimed mouse areas.
//!
//! `mouse_area` already performs the hit test: it publishes a message only when
//! a press lands inside its own bounds, which is exactly a hit test with the
//! target filled in. What it leaves to every caller are three decisions that
//! are easy to get wrong, and all three have one right answer:
//!
//! 1. `on_press` and `on_double_click` must name the **same** target. Copied
//!    and edited, they silently disagree and a double click opens the row next
//!    to the one it focused.
//! 2. The area must be **opaque** to the layers below it
//!    (`.interaction(..) == None` makes a `Stack` treat it as a filter).
//! 3. It must wrap the node whose bounds **are** the row. Wrapping a `text`
//!    gives a hit box the size of the glyphs.
//!
//! [`AimExt::aimed`] decides those three and nothing else. What a gesture
//! *means* is the target's own business — see [`crate::Meaning`].

use iced_core::mouse;
use iced_widget::{Container, MouseArea, mouse_area};

/// Which gesture happened, with no target attached.
///
/// One variant per `mouse_area` callback that carries a target:
///
/// | variant | callback |
/// | --- | --- |
/// | `Press` | `on_press` |
/// | `DoubleClick` | `on_double_click` |
/// | `RightPress` | `on_right_press` |
///
/// [`crate::Meaning`] is implemented *per target* and matched exhaustively over
/// this enum, so adding a gesture here breaks every consumer's build instead of
/// silently falling into a `_ =>` arm somewhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Aim {
    /// The left button went down.
    Press,
    /// A second press landed on the same target inside the platform's
    /// double-click window.
    DoubleClick,
    /// The right button went down.
    RightPress,
}

/// A gesture that already knows what it was aimed at.
///
/// The target type belongs to the caller: this crate never sees it. Keeping it
/// generic is what lets a consumer make the target their own type, and what
/// keeps the *meaning* of a gesture out of this crate.
///
/// The gesture kind is a separate [`Aim`] rather than three variants of this
/// type, so there is exactly one definition of "which gestures exist" instead
/// of one per target type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Aimed<T> {
    pub aim: Aim,
    pub target: T,
}

impl<T> Aimed<T> {
    pub fn new(aim: Aim, target: T) -> Self {
        Self { aim, target }
    }

    /// Splits the gesture into the two halves [`crate::interpret`] needs.
    pub fn into_parts(self) -> (Aim, T) {
        (self.aim, self.target)
    }
}

/// Turns a container's bounds into a mouse area aimed at a target.
///
/// One method, on purpose. Everything that has a unique right answer is decided
/// here; everything that is a policy — what the gesture means, hover, release,
/// the wheel — stays in the application, where it can be argued about.
///
/// Implemented on [`Container`] only: the row's bounds *are* the container's
/// bounds. Aiming at an arbitrary `Element` would silently make the hit box
/// whatever the inner widget happens to be.
pub trait AimExt<'a, M: 'a> {
    /// Wraps this container in a [`MouseArea`] that reports every aimed
    /// gesture as `M::from(Aimed::new(aim, target))`.
    ///
    /// The default [`mouse::Interaction`] is [`mouse::Interaction::Idle`]: not
    /// `None`, so the area blocks the layers below, but not a hand either — a
    /// row is not a link. Callers who want another cursor can chain
    /// `.interaction(..)` afterwards; it overwrites this default.
    fn aimed<T: Clone + 'a>(self, target: T) -> MouseArea<'a, M>
    where
        M: From<Aimed<T>>;
}

impl<'a, M: 'a> AimExt<'a, M> for Container<'a, M> {
    fn aimed<T: Clone + 'a>(self, target: T) -> MouseArea<'a, M>
    where
        M: From<Aimed<T>>,
    {
        mouse_area(self)
            .interaction(mouse::Interaction::Idle)
            .on_press(M::from(Aimed::new(Aim::Press, target.clone())))
            .on_double_click(M::from(Aimed::new(Aim::DoubleClick, target.clone())))
            .on_right_press(M::from(Aimed::new(Aim::RightPress, target)))
    }
}
