use iced::widget::container::Style;
use iced::widget::Container;
use iced::{Background, Border, Color, Length, Padding, Shadow, Theme};

use crate::style::ext::StyleExt;
use crate::style::tokens::Tokens;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct RowInput {
    pub hovered: bool,
    pub cursor: bool,
    pub selected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub padding: Padding,
    pub width: Length,
    pub height: Length,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Pane,
    PaneHeader,
    StatusBar,
    ListRow(RowInput),
}

pub fn pane() -> Kind {
    Kind::Pane
}

pub fn pane_header() -> Kind {
    Kind::PaneHeader
}

pub fn status_bar() -> Kind {
    Kind::StatusBar
}

pub fn list_row(input: RowInput) -> Kind {
    Kind::ListRow(input)
}

impl Kind {
    pub fn all() -> &'static [Self] {
        &[
            Self::Pane,
            Self::PaneHeader,
            Self::StatusBar,
            Self::ListRow(RowInput {
                hovered: false,
                cursor: false,
                selected: false,
            }),
        ]
    }

    pub fn frame(self) -> Frame {
        Frame {
            padding: self.padding(),
            width: Length::Fill,
            height: match self {
                Kind::Pane => Length::Fill,
                _ => Length::Shrink,
            },
        }
    }

    pub fn paint(self, tokens: &Tokens) -> Style {
        match self {
            Kind::Pane => pane::style(tokens),
            Kind::PaneHeader => bar::style(tokens),
            Kind::StatusBar => bar::muted_style(tokens),
            Kind::ListRow(input) => row::style(tokens, input),
        }
    }

    fn padding(self) -> Padding {
        match self {
            Kind::Pane => pane::padding(),
            Kind::PaneHeader | Kind::StatusBar => bar::padding(),
            Kind::ListRow(_) => row::padding(),
        }
    }
}

fn base(fill: Option<Color>, text: Color, border: Border) -> Style {
    Style {
        text_color: Some(text),
        background: fill.map(Background::Color),
        border,
        shadow: Shadow::default(),
        snap: false,
    }
}

mod pane {
    use iced::widget::container::Style;
    use iced::{Border, Padding};

    use super::{base, Tokens};

    pub(super) fn padding() -> Padding {
        Padding::new(8.0)
    }

    pub(super) fn style(tokens: &Tokens) -> Style {
        base(Some(tokens.canvas), tokens.text, Border::default())
    }
}

mod bar {
    use iced::widget::container::Style;
    use iced::{Border, Padding};

    use super::{base, Tokens};

    pub(super) fn padding() -> Padding {
        Padding::new(4.0).horizontal(8.0)
    }

    pub(super) fn style(tokens: &Tokens) -> Style {
        base(Some(tokens.surface), tokens.text, Border::default())
    }

    pub(super) fn muted_style(tokens: &Tokens) -> Style {
        base(Some(tokens.surface), tokens.text_muted, Border::default())
    }
}

mod row {
    use iced::widget::container::Style;
    use iced::{Border, Color, Padding};

    use super::{base, RowInput, Tokens};

    pub(super) fn padding() -> Padding {
        Padding::new(3.0).horizontal(8.0)
    }

    pub(super) fn style(tokens: &Tokens, input: RowInput) -> Style {
        let fill = if input.selected {
            Some(tokens.surface_selected)
        } else if input.hovered {
            Some(tokens.surface_hover)
        } else {
            None
        };

        let border = Border {
            color: if input.cursor {
                tokens.accent
            } else {
                Color::TRANSPARENT
            },
            width: if input.cursor { outline_width() } else { 0.0 },
            radius: radius().into(),
        };

        base(fill, tokens.text, border)
    }

    fn radius() -> f32 {
        4.0
    }

    fn outline_width() -> f32 {
        1.0
    }
}

impl<'a, M: 'a> StyleExt for Container<'a, M> {
    type Kind = Kind;

    fn styled(self, kind: Kind) -> Self {
        let frame = kind.frame();

        self.padding(frame.padding)
            .width(frame.width)
            .height(frame.height)
            .style(move |theme: &Theme| kind.paint(&Tokens::from_theme(theme)))
    }
}
