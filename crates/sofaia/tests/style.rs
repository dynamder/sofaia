//! The visual language, checked as arithmetic rather than as screenshots.
//!
//! These tests belong to the application, not to `ericed`: they assert things
//! about *this* palette and *these* kinds. The library only guarantees that a
//! recipe is asked in every theme and in every status — which is why
//! `every_kind_paints_in_every_theme` is the only structural one.

use iced::widget::button::Status;

use ericed::style::{ButtonRecipe, ContainerRecipe, RuleRecipe, TextRecipe, Tokens};

use sofaia::style::container::{self, RowInput};
use sofaia::style::{button, rule, text};

const ALL_STATUSES: &[Status] = &[
    Status::Active,
    Status::Hovered,
    Status::Pressed,
    Status::Disabled,
];

#[test]
fn text_kinds_stay_legible_on_the_canvas() {
    for theme in iced::Theme::ALL {
        let tokens = Tokens::from_theme(theme);

        for kind in text::Kind::all() {
            let color = kind.paint(&tokens).color.unwrap();
            let contrast = color.relative_contrast(tokens.canvas);
            let minimum = if color == tokens.text_muted { 3.0 } else { 4.5 };

            assert!(
                contrast >= minimum,
                "{theme:?} {kind:?} contrast {contrast:.2} is below {minimum}"
            );
        }
    }
}

#[test]
fn text_kinds_are_opaque() {
    for theme in iced::Theme::ALL {
        let tokens = Tokens::from_theme(theme);

        for kind in text::Kind::all() {
            let color = kind.paint(&tokens).color.unwrap();

            assert_eq!(color.a, 1.0, "{theme:?} {kind:?} must stay opaque");
        }
    }
}

#[test]
fn text_kinds_are_mutually_distinguishable() {
    for theme in iced::Theme::ALL {
        let tokens = Tokens::from_theme(theme);
        let presets = text::Kind::all();

        for (index, left) in presets.iter().enumerate() {
            for right in &presets[index + 1..] {
                let left_key = (left.size(), left.paint(&tokens));
                let right_key = (right.size(), right.paint(&tokens));

                assert_ne!(left_key, right_key, "{theme:?} {left:?} vs {right:?}");
            }
        }
    }
}

#[test]
fn row_states_are_distinguishable() {
    for theme in iced::Theme::ALL {
        let tokens = Tokens::from_theme(theme);

        let normal = container::row(RowInput::default()).paint(&tokens);
        let hovered = container::row(RowInput {
            hovered: true,
            ..RowInput::default()
        })
        .paint(&tokens);
        let cursor = container::row(RowInput {
            cursor: true,
            ..RowInput::default()
        })
        .paint(&tokens);
        let selected = container::row(RowInput {
            selected: true,
            ..RowInput::default()
        })
        .paint(&tokens);
        let both = container::row(RowInput {
            cursor: true,
            selected: true,
            ..RowInput::default()
        })
        .paint(&tokens);

        assert_ne!(normal, hovered, "{theme:?}");
        assert_ne!(hovered, cursor, "{theme:?}");
        assert_ne!(cursor, selected, "{theme:?}");
        assert_ne!(selected, both, "{theme:?}");

        assert_eq!(normal.border.width, 0.0, "{theme:?}");
        assert_eq!(hovered.border.width, 0.0, "{theme:?}");
        assert_eq!(selected.border.width, 0.0, "{theme:?}");
        assert!(cursor.border.width > 0.0, "{theme:?}");
        assert!(both.border.width > 0.0, "{theme:?}");
    }
}

#[test]
fn the_menu_covers_what_it_sits_on() {
    for theme in iced::Theme::ALL {
        let tokens = Tokens::from_theme(theme);

        let scrim = container::scrim().paint(&tokens);
        let menu = container::menu().paint(&tokens);
        let card = container::content().paint(&tokens);

        // A menu is drawn over cards, so it must not look like one.
        assert_ne!(menu, card, "{theme:?}");
        // A scrim has to be translucent: that is the whole point of it.
        let scrim_fill = match scrim.background.expect("scrim has a fill") {
            iced::Background::Color(fill) => fill,
            iced::Background::Gradient(_) => panic!("a scrim is a flat colour"),
        };
        assert!(scrim_fill.a < 1.0, "{theme:?}");
    }
}

#[test]
fn every_kind_paints_in_every_theme() {
    for theme in iced::Theme::ALL {
        let tokens = Tokens::from_theme(theme);

        for kind in container::Kind::all() {
            let style = kind.paint(&tokens);

            assert!(style.text_color.is_some(), "{theme:?} {kind:?}");
        }

        for kind in text::Kind::all() {
            let style = kind.paint(&tokens);

            assert!(style.color.is_some(), "{theme:?} {kind:?}");
        }

        for kind in rule::Kind::all() {
            let _ = kind.paint(&tokens);
        }

        for kind in button::Kind::all() {
            for status in ALL_STATUSES {
                let style = kind.paint(&tokens, *status);

                assert_eq!(style.text_color.a, 1.0, "{theme:?} {kind:?} {status:?}");
            }
        }
    }
}
