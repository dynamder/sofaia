use iced::{Element, Task, Theme};

use crate::panes::{
    left::LeftPane,
    middle::{MiddlePane, MiddlePaneMsg},
    right::RightPane,
};

pub mod panes;
pub mod style;

pub struct App {
    left_pane: LeftPane,
    middle_pane: MiddlePane,
    right_pane: RightPane,
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
            left_pane: LeftPane::default(),
            middle_pane,
            right_pane: RightPane::default(),
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
        self.middle_pane.view().map(AppMsg::from)
    }

    pub fn theme(&self) -> Theme {
        self.theme.clone()
    }
}
