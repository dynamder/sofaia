use iced::widget::rule::{self, Style};
use iced::widget::Rule;
use iced::Theme;

use crate::style::ext::StyleExt;
use crate::style::tokens::Tokens;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Divider,
}

pub fn horizontal<'a>() -> Rule<'a> {
    rule::horizontal(thickness()).styled(divider())
}

pub fn divider() -> Kind {
    Kind::Divider
}

fn thickness() -> f32 {
    1.0
}

impl Kind {
    pub fn all() -> &'static [Self] {
        &[Self::Divider]
    }

    pub fn paint(self, tokens: &Tokens) -> Style {
        Style {
            color: tokens.border,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        }
    }
}

impl<'a> StyleExt for Rule<'a> {
    type Kind = Kind;

    fn styled(self, kind: Kind) -> Self {
        self.style(move |theme: &Theme| kind.paint(&Tokens::from_theme(theme)))
    }
}
