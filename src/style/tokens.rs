use iced::theme::palette;
use iced::{Color, Theme};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tokens {
    pub canvas: Color,
    pub surface: Color,
    pub surface_hover: Color,
    pub surface_cursor: Color,
    pub surface_selected: Color,
    pub border: Color,
    pub text: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub on_accent: Color,
    pub danger: Color,
    pub on_danger: Color,
    pub info: Color,
    pub warning: Color,
    pub danger_text: Color,
    pub info_text: Color,
    pub warning_text: Color,
}

impl Tokens {
    pub fn from_theme(theme: &Theme) -> Self {
        let extended = theme.extended_palette();
        let canvas = extended.background.base.color;
        let text = extended.background.base.text;

        Self {
            canvas,
            surface: extended.background.weak.color,
            surface_hover: extended.background.neutral.color,
            surface_cursor: extended.background.strong.color,
            surface_selected: extended.primary.weak.color,
            border: extended.background.stronger.color,
            text,
            text_muted: palette::deviate(text, 0.15),
            accent: extended.primary.base.color,
            on_accent: extended.primary.base.text,
            danger: extended.danger.base.color,
            on_danger: extended.danger.base.text,
            info: extended.secondary.base.color,
            warning: extended.warning.base.color,
            danger_text: palette::readable(canvas, extended.danger.base.color),
            info_text: palette::readable(canvas, extended.secondary.base.color),
            warning_text: palette::readable(canvas, extended.warning.base.color),
        }
    }
}
