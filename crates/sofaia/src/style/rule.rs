use iced::widget::Rule;
use iced::widget::rule::{self, Style};

use crate::style::StyleExt;
use ericed::style::{RuleRecipe, Tokens};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Divider,
}

pub fn horizontal<'a>() -> Rule<'a> {
    rule::horizontal(thickness()).styled(divider())
}

pub fn divider() -> Kind {
    Kind::Divider
}

fn thickness() -> f32 {
    1.0
}

impl Kind {
    pub fn all() -> &'static [Self] {
        &[Self::Divider]
    }
}

impl RuleRecipe for Kind {
    fn paint(&self, tokens: &Tokens) -> Style {
        Style {
            color: tokens.border,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        }
    }
}
