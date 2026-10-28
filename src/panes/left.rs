use std::path::PathBuf;

use crate::panes::middle::FileItem;

pub struct LeftPane;

pub enum LeftPaneVariant {
    Parent { path: PathBuf, items: FileItem },
    Marks,
}
