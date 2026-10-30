//! The ambient half of the pointer grammar.
//!
//! The mouse reaches the application through two different roads, and only one
//! of them is a grammar:
//!
//! ```text
//! layout ──► Aimed<Row>  ──► ericed::Meaning ──► Intent     (the component decides)
//! wheel_area ──► WheelDirection ──► scroll ──► Intent       (this file)
//! ```
//!
//! An aimed gesture has a target, and the target's *type* knows what it means —
//! a row is focused, a menu item is chosen, the scrim is dismissed. That is
//! [`Meaning`](ericed::Meaning), implemented next to each component, and there
//! is deliberately no table here that would have to know every component.
//!
//! What is left is the gesture with no target at all: the wheel. It used to
//! arrive through the event subscription, which meant it went silent the moment
//! a `scrollable` was under the pointer — the component claimed the wheel
//! before the subscription ever saw it. [`ericed::WheelArea`] fixes the
//! ownership: the listing itself reports the wheel, so this grammar is the only
//! thing that decides what a notch means, and the wheel works because the list
//! is windowed rather than by accident.
//!
//! A `mouse_area` can report more than this state can hold. The missing state is
//! worth naming because it is the same shape [`ChordState`](crate::keys::ChordState)
//! will grow: a drag needs the button and the anchor row, a range selection
//! needs that anchor to survive across messages, and both belong here. The
//! *targets* are resolved by the widget tree, which is why this holds less than
//! the keyboard grammar has to.

use ericed::WheelDirection;

use crate::intent::{Intent, Motion};

/// What a wheel turn means: one row of the view, in the direction it turned.
///
/// The *view*, not the cursor: a notch is a scroll, and `j`/`k` are the cursor.
/// Only the direction is kept, so there is nothing to remember between events
/// yet. A high-resolution trackpad reports fractions of a row; accumulating
/// them is the first piece of state this module would gain, and the natural
/// home for it is `WheelArea`'s own tree state rather than here.
pub fn scroll(direction: WheelDirection) -> Intent {
    Intent::Scroll {
        motion: match direction {
            WheelDirection::Up => Motion::Previous,
            WheelDirection::Down => Motion::Next,
        },
    }
}
