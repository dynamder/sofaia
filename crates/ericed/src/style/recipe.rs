//! What a kind has to answer to become a widget.

use iced_core::{Length, Padding};
use iced_widget::{button, container, rule, text};

use crate::style::Tokens;

/// The box a container kind asks for.
///
/// Layout belongs with the paint: "a sidebar" is *both* a surface colour and a
/// 200px column, and splitting those two decisions across two files is how
/// layouts drift.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub padding: Padding,
    pub width: Length,
    pub height: Length,
    pub clip: bool,
}

/// A container kind: a layout box plus a paint.
pub trait ContainerRecipe {
    fn frame(&self) -> Frame;
    fn paint(&self, tokens: &Tokens) -> container::Style;
}

/// A text kind: a size and a colour.
pub trait TextRecipe {
    fn size(&self) -> f32;
    fn paint(&self, tokens: &Tokens) -> text::Style;
}

/// A rule (divider) kind.
pub trait RuleRecipe {
    fn paint(&self, tokens: &Tokens) -> rule::Style;
}

/// A button kind, painted per interaction status.
///
/// The status is handed over instead of being collapsed into a colour: a button
/// that dims on hover and one that lifts on hover are both legitimate, and only
/// the kind knows which it is.
pub trait ButtonRecipe {
    fn paint(&self, tokens: &Tokens, status: button::Status) -> button::Style;
}
