use iced::widget::{column, container, row, text};
use iced::{Element, Length, Task, Theme};

use crate::panes::{
    left,
    middle::{MiddlePane, MiddlePaneMsg},
    right,
};

pub mod panes;
pub mod style;

use style::StyleExt;

pub struct App {
    middle_pane: MiddlePane,
    theme: Theme,
}

pub enum AppMsg {
    LeftPane,
    MiddlePane(MiddlePaneMsg),
    RightPane,
}

impl From<MiddlePaneMsg> for AppMsg {
    fn from(value: MiddlePaneMsg) -> Self {
        AppMsg::MiddlePane(value)
    }
}

impl App {
    pub fn new() -> (Self, Task<AppMsg>) {
        let init_dir = std::env::current_dir().unwrap();
        let middle_pane = MiddlePane::new(init_dir);
        let init_task = Task::done(MiddlePaneMsg::RequestStartReadDir.into());
        let app = App {
            middle_pane,
            theme: Theme::CatppuccinMocha,
        };
        (app, init_task)
    }

    pub fn update(&mut self, message: AppMsg) -> Task<AppMsg> {
        match message {
            AppMsg::LeftPane => Task::none(),
            AppMsg::MiddlePane(middle_pane_msg) => {
                self.middle_pane.update(middle_pane_msg).map(AppMsg::from)
            }
            AppMsg::RightPane => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, AppMsg> {
        let panes = row![
            container(left::view(self.middle_pane.current_path()))
                .styled(style::container::sidebar()),
            container(self.middle_pane.view().map(AppMsg::from))
                .styled(style::container::content()),
            container(right::view(self.middle_pane.cursor_item()))
                .styled(style::container::preview()),
        ]
        .spacing(style::container::gap())
        .height(Length::Fill);

        let status = container(
            text(format!("{} items", self.middle_pane.len())).styled(style::text::caption()),
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
