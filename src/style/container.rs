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
    pub clip: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Shell,
    Sidebar,
    Content,
    Preview,
    Header,
    StatusBar,
    Row(RowInput),
}

pub fn shell() -> Kind {
    Kind::Shell
}

pub fn sidebar() -> Kind {
    Kind::Sidebar
}

pub fn content() -> Kind {
    Kind::Content
}

pub fn preview() -> Kind {
    Kind::Preview
}

pub fn header() -> Kind {
    Kind::Header
}

pub fn status_bar() -> Kind {
    Kind::StatusBar
}

pub fn row(input: RowInput) -> Kind {
    Kind::Row(input)
}

pub fn gap() -> f32 {
    8.0
}

impl Kind {
    pub fn all() -> &'static [Self] {
        &[
            Self::Shell,
            Self::Sidebar,
            Self::Content,
            Self::Preview,
            Self::Header,
            Self::StatusBar,
            Self::Row(RowInput {
                hovered: false,
                cursor: false,
                selected: false,
            }),
        ]
    }

    pub fn frame(self) -> Frame {
        match self {
            Kind::Shell => shell::frame(),
            Kind::Sidebar => card::frame(sidebar::width()),
            Kind::Content => card::frame(content::width()),
            Kind::Preview => card::frame(preview::width()),
            Kind::Header => header::frame(),
            Kind::StatusBar => status_bar::frame(),
            Kind::Row(_) => row::frame(),
        }
    }

    pub fn paint(self, tokens: &Tokens) -> Style {
        match self {
            Kind::Shell => shell::style(tokens),
            Kind::Sidebar | Kind::Content | Kind::Preview => card::style(tokens),
            Kind::Header => header::style(tokens),
            Kind::StatusBar => status_bar::style(tokens),
            Kind::Row(input) => row::style(tokens, input),
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

fn rounded(radius: f32) -> Border {
    Border {
        color: Color::TRANSPARENT,
        width: 0.0,
        radius: radius.into(),
    }
}

mod shell {
    use iced::widget::container::Style;
    use iced::{Border, Length, Padding};

    use super::{base, Frame, Tokens};

    pub(super) fn frame() -> Frame {
        Frame {
            padding: Padding::new(8.0),
            width: Length::Fill,
            height: Length::Fill,
            clip: false,
        }
    }

    pub(super) fn style(tokens: &Tokens) -> Style {
        base(Some(tokens.canvas), tokens.text, Border::default())
    }
}

mod card {
    use iced::widget::container::Style;
    use iced::{Border, Length, Padding};

    use super::{base, Frame, Tokens};

    pub(super) fn frame(width: Length) -> Frame {
        Frame {
            padding: Padding::new(6.0),
            width,
            height: Length::Fill,
            clip: true,
        }
    }

    pub(super) fn style(tokens: &Tokens) -> Style {
        base(Some(tokens.surface), tokens.text, radius())
    }

    fn radius() -> Border {
        super::rounded(10.0)
    }
}

mod sidebar {
    use iced::Length;

    pub(super) fn width() -> Length {
        Length::Fixed(200.0)
    }
}

mod content {
    use iced::Length;

    pub(super) fn width() -> Length {
        Length::Fill
    }
}

mod preview {
    use iced::Length;

    pub(super) fn width() -> Length {
        Length::Fixed(240.0)
    }
}

mod header {
    use iced::widget::container::Style;
    use iced::{Border, Length, Padding};

    use super::{base, Frame, Tokens};

    pub(super) fn frame() -> Frame {
        Frame {
            padding: Padding::new(2.0).horizontal(8.0),
            width: Length::Fill,
            height: Length::Shrink,
            clip: true,
        }
    }

    pub(super) fn style(tokens: &Tokens) -> Style {
        base(None, tokens.text_muted, Border::default())
    }
}

mod status_bar {
    use iced::widget::container::Style;
    use iced::{Border, Length, Padding};

    use super::{base, Frame, Tokens};

    pub(super) fn frame() -> Frame {
        Frame {
            padding: Padding::new(6.0).horizontal(10.0),
            width: Length::Fill,
            height: Length::Shrink,
            clip: false,
        }
    }

    pub(super) fn style(tokens: &Tokens) -> Style {
        base(None, tokens.text_muted, Border::default())
    }
}

mod row {
    use iced::widget::container::Style;
    use iced::{Border, Color, Length, Padding};

    use super::{base, Frame, RowInput, Tokens};

    pub(super) fn frame() -> Frame {
        Frame {
            padding: Padding::new(6.0).horizontal(10.0),
            width: Length::Fill,
            height: Length::Shrink,
            clip: true,
        }
    }

    pub(super) fn style(tokens: &Tokens, input: RowInput) -> Style {
        let fill = if input.selected {
            Some(tokens.surface_selected)
        } else if input.hovered {
            Some(tokens.surface_hover)
        } else if input.cursor {
            Some(tokens.surface_cursor)
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
        6.0
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
            .clip(frame.clip)
            .style(move |theme: &Theme| kind.paint(&Tokens::from_theme(theme)))
    }
}
