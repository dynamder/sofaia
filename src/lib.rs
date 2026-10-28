use crate::panes::{left::LeftPane, middle::MiddlePane, right::RightPane};

pub mod panes;

pub struct App {
    left_pane: LeftPane,
    middle_pane: MiddlePane,
    right_pane: RightPane,
}
