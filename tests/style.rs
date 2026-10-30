use iced::widget::button::Status;

use sofaia::style::container::{self, RowInput};
use sofaia::style::tokens::Tokens;
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
            let minimum = match kind {
                text::Kind::Muted => 3.0,
                _ => 4.5,
            };

            assert!(
                contrast >= minimum,
                "{theme:?} text::{kind:?} contrast {contrast:.2} is below {minimum}"
            );
        }
    }
}

#[test]
fn text_kinds_are_mutually_distinguishable() {
    for theme in iced::Theme::ALL {
        let tokens = Tokens::from_theme(theme);

        for (index, left) in text::Kind::all().iter().enumerate() {
            for right in &text::Kind::all()[index + 1..] {
                assert_ne!(
                    left.paint(&tokens),
                    right.paint(&tokens),
                    "{theme:?} text::{left:?} and text::{right:?} render identically"
                );
            }
        }
    }
}

#[test]
fn text_kinds_are_opaque() {
    for theme in iced::Theme::ALL {
        let tokens = Tokens::from_theme(theme);

        for kind in text::Kind::all() {
            let color = kind.paint(&tokens).color.unwrap();

            assert_eq!(color.a, 1.0, "{theme:?} text::{kind:?} must stay opaque");
        }
    }
}

#[test]
fn row_states_are_distinguishable() {
    for theme in iced::Theme::ALL {
        let tokens = Tokens::from_theme(theme);

        let normal = container::list_row(RowInput::default()).paint(&tokens);
        let hovered = container::list_row(RowInput {
            hovered: true,
            ..RowInput::default()
        })
        .paint(&tokens);
        let cursor = container::list_row(RowInput {
            cursor: true,
            ..RowInput::default()
        })
        .paint(&tokens);
        let selected = container::list_row(RowInput {
            selected: true,
            ..RowInput::default()
        })
        .paint(&tokens);
        let both = container::list_row(RowInput {
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
