//! The wiring from a kind to a widget.
//!
//! [`StyleExt`] is generic over the kind rather than carrying an associated
//! type, because an associated type fixes *one* kind family per widget type
//! forever. Generic means a container can be painted by any kind that answers
//! [`ContainerRecipe`], including one defined in a crate this one has never
//! heard of.

use iced_core::Theme;
use iced_widget::{Button, Container, Rule, Text};

use crate::style::Tokens;
use crate::style::recipe::{ButtonRecipe, ContainerRecipe, RuleRecipe, TextRecipe};

/// Paints a widget from a kind.
pub trait StyleExt<K> {
    fn styled(self, kind: K) -> Self;
}

impl<'a, M: 'a, K: ContainerRecipe + Copy + 'a> StyleExt<K> for Container<'a, M> {
    fn styled(self, kind: K) -> Self {
        let frame = kind.frame();

        self.padding(frame.padding)
            .width(frame.width)
            .height(frame.height)
            .clip(frame.clip)
            .style(move |theme: &Theme| kind.paint(&Tokens::from_theme(theme)))
    }
}

impl<'a, K: TextRecipe + Copy + 'a> StyleExt<K> for Text<'a> {
    fn styled(self, kind: K) -> Self {
        self.size(kind.size())
            .style(move |theme: &Theme| kind.paint(&Tokens::from_theme(theme)))
    }
}

impl<'a, K: RuleRecipe + Copy + 'a> StyleExt<K> for Rule<'a> {
    fn styled(self, kind: K) -> Self {
        self.style(move |theme: &Theme| kind.paint(&Tokens::from_theme(theme)))
    }
}

impl<'a, M: 'a, K: ButtonRecipe + Copy + 'a> StyleExt<K> for Button<'a, M> {
    fn styled(self, kind: K) -> Self {
        self.style(move |theme: &Theme, status| kind.paint(&Tokens::from_theme(theme), status))
    }
}
