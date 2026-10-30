//! The wheel's sign convention, written down once.

use iced_core::mouse::ScrollDelta;

/// Which way a wheel turned.
///
/// Only the sign survives. iced reports a magnitude — `Lines { y: 3.0 }` for a
/// mouse notch, pixels for a trackpad — and the intent this eventually produces
/// is "one row", which has no magnitude to carry it. What *is* worth writing
/// down is the sign: iced treats a **positive `y`** as the wheel being pushed
/// away from the user, which is why `Scrollable` scrolls by `-delta`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WheelDirection {
    Up,
    Down,
}

impl WheelDirection {
    /// Reads the direction out of a raw scroll delta.
    ///
    /// Returns [`None`] when the wheel reported no vertical motion at all
    /// (a horizontal-only trackpad gesture, for instance).
    pub fn read(delta: ScrollDelta) -> Option<Self> {
        let y = match delta {
            ScrollDelta::Lines { y, .. } | ScrollDelta::Pixels { y, .. } => y,
        };

        if y > 0.0 {
            Some(Self::Up)
        } else if y < 0.0 {
            Some(Self::Down)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_y_is_up() {
        assert_eq!(
            WheelDirection::read(ScrollDelta::Lines { x: 0.0, y: 1.0 }),
            Some(WheelDirection::Up)
        );
        assert_eq!(
            WheelDirection::read(ScrollDelta::Lines { x: 0.0, y: -3.0 }),
            Some(WheelDirection::Down)
        );
    }

    #[test]
    fn pixels_keep_the_same_sign() {
        assert_eq!(
            WheelDirection::read(ScrollDelta::Pixels { x: 0.0, y: 0.5 }),
            Some(WheelDirection::Up)
        );
        assert_eq!(
            WheelDirection::read(ScrollDelta::Pixels { x: 12.0, y: -0.5 }),
            Some(WheelDirection::Down)
        );
    }

    #[test]
    fn no_vertical_motion_is_not_a_gesture() {
        assert_eq!(
            WheelDirection::read(ScrollDelta::Pixels { x: 4.0, y: 0.0 }),
            None
        );
    }
}
