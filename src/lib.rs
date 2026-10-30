use std::path::PathBuf;

use iced::widget::{column, container, row, text};
use iced::{Element, Length, Task, Theme};

use crate::panes::middle::{MiddlePane, MiddlePaneMsg};
use crate::panes::{left, right};
use crate::style::StyleExt;

pub mod fs;
pub mod panes;
pub mod style;

pub struct App {
    middle: MiddlePane,
    theme: Theme,
}

#[derive(Debug, Clone)]
pub enum AppMsg {
    MiddlePane(MiddlePaneMsg),
}

impl From<MiddlePaneMsg> for AppMsg {
    fn from(message: MiddlePaneMsg) -> Self {
        AppMsg::MiddlePane(message)
    }
}

impl App {
    pub fn new(start: PathBuf) -> (Self, Task<AppMsg>) {
        let (middle, task) = MiddlePane::new(start);

        (Self::with_panes(middle), task.map(AppMsg::from))
    }

    pub fn with_panes(middle: MiddlePane) -> Self {
        Self {
            middle,
            theme: Theme::CatppuccinMocha,
        }
    }

    pub fn update(&mut self, message: AppMsg) -> Task<AppMsg> {
        match message {
            AppMsg::MiddlePane(message) => self.middle.update(message).map(AppMsg::from),
        }
    }

    pub fn view(&self) -> Element<'_, AppMsg> {
        let panes = row![
            container(left::view(self.middle.path())).styled(style::container::sidebar()),
            container(self.middle.view().map(AppMsg::from))
                .styled(style::container::content()),
            container(right::view(self.middle.cursor_entry()))
                .styled(style::container::preview()),
        ]
        .spacing(style::container::gap())
        .height(Length::Fill);

        let status = container(
            text(format!("{} items", self.middle.len())).styled(style::text::caption()),
        )
        .styled(style::container::status_bar());

        container(column![panes, status].spacing(style::container::gap()))
            .styled(style::container::shell())
            .into()
    }

    pub fn theme(&self) -> Theme {
        self.theme.clone()
    }
}
