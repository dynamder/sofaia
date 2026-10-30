use iced::widget::container::Style;
use iced::{Background, Border, Color, Shadow};

use ericed::style::{ContainerRecipe, Frame, Tokens};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct RowInput {
    pub hovered: bool,
    pub cursor: bool,
    pub selected: bool,
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
    /// A labelled value in the preview: the same surface as a row, but free to
    /// be as tall as its two lines of text.
    ///
    /// A row is a fixed box because the listing is windowed; a field is not, so
    /// it is a different kind rather than a flag on [`Kind::Row`].
    Field,
    /// The context menu's card.
    Menu,
    /// The layer that dims everything under a menu.
    Scrim,
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

pub fn field() -> Kind {
    Kind::Field
}

pub fn row(input: RowInput) -> Kind {
    Kind::Row(input)
}

pub fn menu() -> Kind {
    Kind::Menu
}

pub fn scrim() -> Kind {
    Kind::Scrim
}

pub fn gap() -> f32 {
    8.0
}

/// The height of a listing row, in pixels.
///
/// A row is a *fixed* box, because a windowed list does arithmetic on it: the
/// geometry that decides which rows to build multiplies this number by an
/// index, and the style lays the row out with it. Two constants would drift
/// apart, and the drift would show up as a scrollbar that is subtly wrong.
///
/// 28 = 13px body text at the default line height (≈15.6) plus the row's 6px
/// vertical padding, rounded up.
pub fn row_height() -> f32 {
    28.0
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
            Self::Field,
            Self::Menu,
            Self::Scrim,
        ]
    }
}

impl ContainerRecipe for Kind {
    fn frame(&self) -> Frame {
        match self {
            Kind::Shell => shell::frame(),
            Kind::Sidebar => card::frame(sidebar::width()),
            Kind::Content => card::frame(content::width()),
            Kind::Preview => card::frame(preview::width()),
            Kind::Header => header::frame(),
            Kind::StatusBar => status_bar::frame(),
            Kind::Row(_) => row::frame(),
            Kind::Field => row::field_frame(),
            Kind::Menu => menu::frame(),
            Kind::Scrim => scrim::frame(),
        }
    }

    fn paint(&self, tokens: &Tokens) -> Style {
        match self {
            Kind::Shell => shell::style(tokens),
            Kind::Sidebar | Kind::Content | Kind::Preview => card::style(tokens),
            Kind::Header => header::style(tokens),
            Kind::StatusBar => status_bar::style(tokens),
            Kind::Row(input) => row::style(tokens, *input),
            Kind::Field => row::style(tokens, RowInput::default()),
            Kind::Menu => menu::style(tokens),
            Kind::Scrim => scrim::style(tokens),
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

    use super::{Frame, Tokens, base};

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

    use super::{Frame, Tokens, base};

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

    use super::{Frame, Tokens, base};

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

    use super::{Frame, Tokens, base};

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

    use super::{Frame, RowInput, Tokens, base};

    pub(super) fn frame() -> Frame {
        Frame {
            padding: Padding::new(6.0).horizontal(10.0),
            width: Length::Fill,
            // A fixed height, not `Shrink`: the listing is windowed, so its
            // arithmetic needs every row to be the same box. See
            // `container::row_height`.
            height: Length::Fixed(super::row_height()),
            clip: true,
        }
    }

    /// A row's surface, sized by its contents.
    ///
    /// The preview's fields are two lines of text, so they cannot share the
    /// listing's fixed box.
    pub(super) fn field_frame() -> Frame {
        Frame {
            height: Length::Shrink,
            ..frame()
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

mod menu {
    use iced::widget::container::Style;
    use iced::{Color, Length, Padding, Shadow, Vector};

    use super::{Frame, Tokens};

    pub(super) fn frame() -> Frame {
        Frame {
            padding: Padding::new(6.0),
            // Fixed, not `Shrink`: the rows and the header inside are
            // `Length::Fill`, and a `Fill` child of a `Shrink` parent resolves
            // to zero width. The whole menu used to collapse into a one-pixel
            // vertical line because of it.
            width: Length::Fixed(width()),
            height: Length::Shrink,
            clip: true,
        }
    }

    fn width() -> f32 {
        240.0
    }

    pub(super) fn style(tokens: &Tokens) -> Style {
        Style {
            // A menu floats over everything, including the cards it sits on,
            // so it cannot be the same surface colour as a card.
            background: Some(iced::Background::Color(tokens.surface_cursor)),
            text_color: Some(tokens.text),
            border: iced::Border {
                color: tokens.border,
                width: 1.0,
                radius: 10.0.into(),
            },
            shadow: Shadow {
                color: Color {
                    a: 0.35,
                    ..Color::BLACK
                },
                offset: Vector::new(0.0, 4.0),
                blur_radius: 16.0,
            },
            snap: false,
        }
    }
}

mod scrim {
    use iced::widget::container::Style;
    use iced::{Border, Length, Padding};

    use super::{Frame, Tokens, base};

    pub(super) fn frame() -> Frame {
        Frame {
            padding: Padding::new(0.0),
            width: Length::Fill,
            height: Length::Fill,
            clip: false,
        }
    }

    pub(super) fn style(tokens: &Tokens) -> Style {
        base(Some(tokens.scrim), tokens.text, Border::default())
    }
}
