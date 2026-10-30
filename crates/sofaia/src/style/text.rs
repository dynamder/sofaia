use iced::widget::text::Style;

use ericed::style::{TextRecipe, Tokens};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Size {
    Title,
    Body,
    Caption,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tone {
    Normal,
    Muted,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Kind {
    size: Size,
    tone: Tone,
}

pub fn title() -> Kind {
    Kind {
        size: Size::Title,
        tone: Tone::Normal,
    }
}

pub fn body() -> Kind {
    Kind {
        size: Size::Body,
        tone: Tone::Normal,
    }
}

pub fn muted() -> Kind {
    Kind {
        size: Size::Body,
        tone: Tone::Muted,
    }
}

pub fn caption() -> Kind {
    Kind {
        size: Size::Caption,
        tone: Tone::Muted,
    }
}

pub fn info() -> Kind {
    Kind {
        size: Size::Body,
        tone: Tone::Info,
    }
}

pub fn warning() -> Kind {
    Kind {
        size: Size::Body,
        tone: Tone::Warning,
    }
}

pub fn error() -> Kind {
    Kind {
        size: Size::Body,
        tone: Tone::Error,
    }
}

impl Kind {
    pub fn all() -> [Self; 7] {
        [
            title(),
            body(),
            muted(),
            caption(),
            info(),
            warning(),
            error(),
        ]
    }
}

impl TextRecipe for Kind {
    fn size(&self) -> f32 {
        match self.size {
            Size::Title => 16.0,
            Size::Body => 13.0,
            Size::Caption => 11.0,
        }
    }

    fn paint(&self, tokens: &Tokens) -> Style {
        let color = match self.tone {
            Tone::Normal => tokens.text,
            Tone::Muted => tokens.text_muted,
            Tone::Info => tokens.info_text,
            Tone::Warning => tokens.warning_text,
            Tone::Error => tokens.danger_text,
        };

        Style { color: Some(color) }
    }
}
