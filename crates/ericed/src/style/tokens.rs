//! The palette every recipe reads.
//!
//! Recipes never touch raw colours: they ask for a *role* (`text_muted`,
//! `surface_hover`, `danger_text`) and get a value that already agrees with the
//! rest of the application in every theme. Adding a role here is cheap; adding
//! a `Color::from_rgb(..)` in a recipe is how themes start to rot.
//!
//! # A palette is a point, not a bag of colours
//!
//! Because every recipe reads the same struct, the whole visual language can be
//! moved at once between two palettes — a hover cross-fade, a theme transition —
//! and that is what [`Interpolable`] is for. It is iced's trait, not one of this
//! crate's: it is what `Animation::interpolate_with` asks for, so a `Tokens` can
//! be driven by iced's animation as readily as by
//! [`motion::Track`](crate::motion::Track), and the straight line between two
//! palettes is a line both of them walk.

use iced_core::animation::Interpolable;
use iced_core::theme::palette;
use iced_core::{Color, Theme};

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
    pub scrim: Color,
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
            // A scrim is the only role that is deliberately translucent: it *is*
            // a dimming layer, so it has to be.
            scrim: Color {
                a: 0.45,
                ..Color::BLACK
            },
        }
    }

    /// The palette `ratio` of the way from `self` to `other`.
    ///
    /// The same line as [`Interpolable::interpolated`], spelled as a free
    /// function of the two ends — which is how `iced::theme::palette::mix`
    /// spells it for a single colour, and the reason the two read alike.
    pub fn mix(from: Self, to: Self, ratio: f32) -> Self {
        from.interpolated(to, ratio)
    }

    /// The same palette, `factor` times as opaque.
    ///
    /// This is what a fade is, because it is the only thing a fade *can* be:
    /// iced draws no subtree at reduced opacity (`Renderer` has layers and
    /// transformations, not alpha), so a fading panel has to be a panel whose
    /// colours are fading. Every role is scaled, so a role that is already
    /// translucent — the scrim — fades proportionally instead of jumping to
    /// full opacity.
    pub fn fade(self, factor: f32) -> Self {
        let faded = |color: Color| color.scale_alpha(factor);

        Self {
            canvas: faded(self.canvas),
            surface: faded(self.surface),
            surface_hover: faded(self.surface_hover),
            surface_cursor: faded(self.surface_cursor),
            surface_selected: faded(self.surface_selected),
            border: faded(self.border),
            text: faded(self.text),
            text_muted: faded(self.text_muted),
            accent: faded(self.accent),
            on_accent: faded(self.on_accent),
            danger: faded(self.danger),
            on_danger: faded(self.on_danger),
            info: faded(self.info),
            warning: faded(self.warning),
            danger_text: faded(self.danger_text),
            info_text: faded(self.info_text),
            warning_text: faded(self.warning_text),
            scrim: faded(self.scrim),
        }
    }
}

impl Interpolable for Tokens {
    /// Linear in light, not in sRGB.
    ///
    /// Each role goes through `palette::mix`, which converts to linear RGB
    /// before averaging: half-way between black and white is *half the light*,
    /// which is not half of the sRGB numbers. Alpha is linear already and is
    /// interpolated as it stands.
    fn interpolated(&self, other: Self, ratio: f32) -> Self {
        // Exact at both ends. `palette::mix` average in linear RGB and converts
        // back, and that round trip can move the last bit — so a palette that
        // has *arrived* has to be the palette, not a rounding error away from
        // it, or a finished animation would keep differing from its target.
        if ratio <= 0.0 {
            return *self;
        }

        if ratio >= 1.0 {
            return other;
        }

        let mix = |a, b| palette::mix(a, b, ratio);

        Self {
            canvas: mix(self.canvas, other.canvas),
            surface: mix(self.surface, other.surface),
            surface_hover: mix(self.surface_hover, other.surface_hover),
            surface_cursor: mix(self.surface_cursor, other.surface_cursor),
            surface_selected: mix(self.surface_selected, other.surface_selected),
            border: mix(self.border, other.border),
            text: mix(self.text, other.text),
            text_muted: mix(self.text_muted, other.text_muted),
            accent: mix(self.accent, other.accent),
            on_accent: mix(self.on_accent, other.on_accent),
            danger: mix(self.danger, other.danger),
            on_danger: mix(self.on_danger, other.on_danger),
            info: mix(self.info, other.info),
            warning: mix(self.warning, other.warning),
            danger_text: mix(self.danger_text, other.danger_text),
            info_text: mix(self.info_text, other.info_text),
            warning_text: mix(self.warning_text, other.warning_text),
            scrim: mix(self.scrim, other.scrim),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_palette_walk_fixes_its_ends_and_moves_every_role() {
        let dark = Tokens::from_theme(&Theme::Dark);
        let light = Tokens::from_theme(&Theme::Light);

        assert_eq!(Tokens::mix(dark, light, 0.0), dark);
        assert_eq!(Tokens::mix(dark, light, 1.0), light);

        let middle = Tokens::mix(dark, light, 0.5);

        assert_ne!(middle, dark);
        assert_ne!(middle, light);
        assert_eq!(middle.canvas, palette::mix(dark.canvas, light.canvas, 0.5));
        assert_eq!(middle.text, palette::mix(dark.text, light.text, 0.5));
    }

    #[test]
    fn a_mid_point_is_a_mid_point_of_light() {
        // The whole reason this is not a field-wise average of the numbers as
        // written: sRGB is not linear, so the arithmetic mean of black and white
        // is four fifths of the light, not half of it.
        let middle = palette::mix(Color::BLACK, Color::WHITE, 0.5);

        assert!((middle.r - 0.5).abs() > 0.1, "got {}", middle.r);
        assert!((middle.r - 0.735).abs() < 0.01, "got {}", middle.r);
    }

    #[test]
    fn alpha_stays_linear() {
        // Alpha is opacity, not light: a scrim fading in must be half as opaque
        // at the half-way point, with no transfer function in between.
        let a = Color {
            a: 0.0,
            ..Color::BLACK
        };
        let b = Color {
            a: 1.0,
            ..Color::BLACK
        };

        assert_eq!(palette::mix(a, b, 0.5).a, 0.5);
    }

    #[test]
    fn a_fade_scales_every_role_and_nothing_else() {
        let tokens = Tokens::from_theme(&Theme::Dark);

        let gone = tokens.fade(0.0);
        let half = tokens.fade(0.5);

        assert_eq!(gone.canvas.a, 0.0);
        assert_eq!(gone.text.a, 0.0);
        assert_eq!(
            half.scrim.a,
            tokens.scrim.a * 0.5,
            "a scrim fades, it does not jump"
        );

        // A fade is an alpha, not a tint: the colours themselves are untouched.
        assert_eq!(half.accent.r, tokens.accent.r);
        assert_eq!(half.text.g, tokens.text.g);
    }
}
