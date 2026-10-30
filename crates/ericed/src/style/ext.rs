//! The wiring from a kind to a widget.
//!
//! [`StyleExt`] is generic over the kind rather than carrying an associated
//! type, because an associated type fixes *one* kind family per widget type
//! forever. Generic means a container can be painted by any kind that answers
//! [`ContainerRecipe`], including one defined in a crate this one has never
//! heard of.
//!
//! # Why the palette is a parameter and not a field
//!
//! A kind says what a widget *is*; the palette says what it looks like *this
//! frame*. Those are different questions, and only the second one changes while
//! something moves.
//!
//! It also has to be the palette, because iced cannot draw a subtree at reduced
//! opacity: `Renderer` offers layers and transformations, not alpha, and
//! `with_layer` takes a `Rectangle` and nothing else. A fade therefore cannot be
//! a property of the subtree — it has to be a palette whose colours are faded,
//! which is why the palette has to be injectable:
//!
//! ```ignore
//! container(card).styled_with(kind, move |theme| {
//!     Tokens::from_theme(theme).fade(track.value())
//! })
//! ```
//!
//! [`styled`](StyleExt::styled) is the same call with the theme's own palette, so
//! a view that is not moving looks exactly as it did before this existed.

use iced_core::Theme;
use iced_widget::{Button, Container, Rule, Text};

use crate::style::Tokens;
use crate::style::recipe::{ButtonRecipe, ContainerRecipe, RuleRecipe, TextRecipe};

/// Paints a widget from a kind.
///
/// `'a` is the lifetime of the widget and of the palette function: a
/// `styled_with` closure may borrow or copy whatever it needs — a `Track`'s
/// value, a shared handle, a theme.
pub trait StyleExt<'a, K> {
    /// Paints from the palette the theme itself provides.
    fn styled(self, kind: K) -> Self
    where
        Self: Sized,
    {
        self.styled_with(kind, Tokens::from_theme)
    }

    /// Paints from a palette derived from the theme.
    ///
    /// The derivation runs where iced runs a style: once per draw, with the
    /// theme in hand. That is the same place `iced`'s own `.style(..)` closure
    /// runs, so nothing new happens at paint time — the closure simply decides
    /// the palette instead of the recipe doing it.
    fn styled_with(self, kind: K, tokens: impl Fn(&Theme) -> Tokens + 'a) -> Self;
}

impl<'a, M: 'a, K: ContainerRecipe + Copy + 'a> StyleExt<'a, K> for Container<'a, M> {
    fn styled_with(self, kind: K, tokens: impl Fn(&Theme) -> Tokens + 'a) -> Self {
        let frame = kind.frame();

        self.padding(frame.padding)
            .width(frame.width)
            .height(frame.height)
            .clip(frame.clip)
            .style(move |theme: &Theme| kind.paint(&tokens(theme)))
    }
}

impl<'a, K: TextRecipe + Copy + 'a> StyleExt<'a, K> for Text<'a> {
    fn styled_with(self, kind: K, tokens: impl Fn(&Theme) -> Tokens + 'a) -> Self {
        self.size(kind.size())
            .style(move |theme: &Theme| kind.paint(&tokens(theme)))
    }
}

impl<'a, K: RuleRecipe + Copy + 'a> StyleExt<'a, K> for Rule<'a> {
    fn styled_with(self, kind: K, tokens: impl Fn(&Theme) -> Tokens + 'a) -> Self {
        self.style(move |theme: &Theme| kind.paint(&tokens(theme)))
    }
}

impl<'a, M: 'a, K: ButtonRecipe + Copy + 'a> StyleExt<'a, K> for Button<'a, M> {
    fn styled_with(self, kind: K, tokens: impl Fn(&Theme) -> Tokens + 'a) -> Self {
        self.style(move |theme: &Theme, status| kind.paint(&tokens(theme), status))
    }
}
