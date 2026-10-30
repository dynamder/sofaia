use iced::theme::palette;
use iced::widget::button::{self, Style};
use iced::widget::Button;
use iced::{Background, Border, Shadow, Theme};

use crate::style::ext::StyleExt;
use crate::style::tokens::Tokens;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Primary,
    Secondary,
    Danger,
}

pub fn primary() -> Kind {
    Kind::Primary
}

pub fn secondary() -> Kind {
    Kind::Secondary
}

pub fn danger() -> Kind {
    Kind::Danger
}

impl Kind {
    pub fn all() -> &'static [Self] {
        &[Self::Primary, Self::Secondary, Self::Danger]
    }

    pub fn paint(self, tokens: &Tokens, status: button::Status) -> Style {
        let (fill, text) = match self {
            Kind::Primary => (tokens.accent, tokens.on_accent),
            Kind::Secondary => (tokens.surface, tokens.text),
            Kind::Danger => (tokens.danger, tokens.on_danger),
        };

        let (fill, text) = match status {
            button::Status::Active => (fill, text),
            button::Status::Hovered => (palette::mix(fill, text, 0.12), text),
            button::Status::Pressed => (palette::mix(fill, text, 0.24), text),
            button::Status::Disabled => {
                (palette::mix(fill, tokens.canvas, 0.6), tokens.text_muted)
            }
        };

        Style {
            background: Some(Background::Color(fill)),
            text_color: text,
            border: Border::default(),
            shadow: Shadow::default(),
            snap: false,
        }
    }
}

impl<'a, M: 'a> StyleExt for Button<'a, M> {
    type Kind = Kind;

    fn styled(self, kind: Kind) -> Self {
        self.style(move |theme: &Theme, status| {
            kind.paint(&Tokens::from_theme(theme), status)
        })
    }
}
