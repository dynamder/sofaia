//! The wheel's sign convention, written down once — and the area that claims it.
//!
//! iced has `mouse_area` for clicks and nothing for the wheel: a `Scrollable`
//! under the pointer simply takes the wheel and scrolls itself, which is what
//! you want for a document and not what you want for a keyboard-driven list.
//!
//! [`WheelArea`] is the missing counterpart. It claims the wheel over its own
//! bounds, turns it into a [`WheelDirection`] and hands it to the application,
//! so a list can decide that a notch means *one row* without the application
//! having to match on `mouse::Event::WheelScrolled` itself — and without the
//! silent surprise of that match going quiet the moment a `Scrollable` is in
//! the tree.
//!
//! It claims the wheel by capturing the event, which is enough to stop a
//! `Scrollable` below it: `scrollable` returns early when the event has already
//! been captured. Nothing else is touched — presses still reach the scrollbar,
//! so dragging it keeps working, and so do touch gestures.

use iced_core::layout;
use iced_core::mouse::{self, ScrollDelta};
use iced_core::overlay;
use iced_core::renderer;
use iced_core::widget::tree::{self, Tree};
use iced_core::widget::{self, Widget};
use iced_core::{Clipboard, Element, Event, Layout, Length, Rectangle, Shell, Size, Vector};

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

/// An area that claims the wheel and reports where it turned.
///
/// Build one with [`wheel_area`].
pub struct WheelArea<'a, Message, Theme = iced_widget::Theme, Renderer = iced_widget::Renderer>
where
    Renderer: iced_core::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    on_wheel: Box<dyn Fn(WheelDirection) -> Message + 'a>,
}

/// Claims the wheel over `content`, publishing a message for each turn.
///
/// The handler is a constructor argument rather than a chained method on
/// purpose: a `wheel_area` that captures the wheel with no handler would make
/// everything below it dead to the wheel — a silent failure, and the kind that
/// takes an afternoon to find.
///
/// ```no_run
/// # #[derive(Clone)]
/// # enum Message {
/// #     Wheel(ericed::WheelDirection),
/// # }
/// # fn example<'a, Theme, Renderer>(
/// #     list: iced_core::Element<'a, Message, Theme, Renderer>,
/// # ) -> iced_core::Element<'a, Message, Theme, Renderer>
/// # where
/// #     Theme: 'a,
/// #     Renderer: iced_core::Renderer + 'a,
/// # {
/// ericed::wheel_area(list, Message::Wheel).into()
/// # }
/// ```
pub fn wheel_area<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    on_wheel: impl Fn(WheelDirection) -> Message + 'a,
) -> WheelArea<'a, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer,
{
    WheelArea {
        content: content.into(),
        on_wheel: Box::new(on_wheel),
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for WheelArea<'_, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer,
{
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event
            && cursor.is_over(layout.bounds())
            && let Some(direction) = WheelDirection::read(*delta)
        {
            shell.publish((self.on_wheel)(direction));
            // Claiming it is the whole point: a `Scrollable` below sees the
            // captured status and leaves the event alone.
            shell.capture_event();

            return;
        }

        self.content.as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

impl<'a, Message, Theme, Renderer> From<WheelArea<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced_core::Renderer + 'a,
{
    fn from(area: WheelArea<'a, Message, Theme, Renderer>) -> Self {
        Self::new(area)
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
