use iced::widget::text::Style;
use iced::widget::Text;
use iced::Theme;

use crate::style::ext::StyleExt;
use crate::style::tokens::Tokens;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Normal,
    Muted,
    Info,
    Warning,
    Error,
}

pub fn normal() -> Kind {
    Kind::Normal
}

pub fn muted() -> Kind {
    Kind::Muted
}

pub fn info() -> Kind {
    Kind::Info
}

pub fn warning() -> Kind {
    Kind::Warning
}

pub fn error() -> Kind {
    Kind::Error
}

impl Kind {
    pub fn all() -> &'static [Self] {
        &[
            Self::Normal,
            Self::Muted,
            Self::Info,
            Self::Warning,
            Self::Error,
        ]
    }

    pub fn paint(self, tokens: &Tokens) -> Style {
        let color = match self {
            Kind::Normal => tokens.text,
            Kind::Muted => tokens.text_muted,
            Kind::Info => tokens.info_text,
            Kind::Warning => tokens.warning_text,
            Kind::Error => tokens.danger_text,
        };

        Style { color: Some(color) }
    }
}

impl<'a> StyleExt for Text<'a> {
    type Kind = Kind;

    fn styled(self, kind: Kind) -> Self {
        self.style(move |theme: &Theme| kind.paint(&Tokens::from_theme(theme)))
    }
}
