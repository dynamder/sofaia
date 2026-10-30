use std::fs::FileType;
use std::path::PathBuf;

use anyhow::Result;
use iced::futures::StreamExt;
use iced::widget::{column, container, text};
use iced::{Element, Task};
use tokio_stream::wrappers::ReadDirStream;

use crate::style::container::RowInput;
use crate::style::{self, StyleExt};

pub struct MiddlePane {
    files: Vec<FileItem>,
    current_path: PathBuf,
    dir_stream: Option<ReadDirStream>,
    errors: Vec<anyhow::Error>,
    cursor: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemType {
    File,
    Folder,
    SymLink,
    Unknown,
}

pub struct FileItem {
    path: PathBuf,
    item_type: ItemType,
    selected: bool,
}

impl FileItem {
    pub fn label(&self) -> String {
        let name = self.path.file_name().map_or_else(
            || self.path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );

        match self.item_type {
            ItemType::Folder => format!("{name}/"),
            ItemType::SymLink => format!("{name}@"),
            _ => name,
        }
    }
}

pub enum MiddlePaneMsg {
    RequestStartReadDir,
    ReceivedStartedReadDirStream(Result<ReadDirStream>),
    RequestFindNextFileItem,
    ReceivedFileItem(Option<(Result<FileItem>, ReadDirStream)>), //Same behavior as the rust stream, None for the stream end.
}

impl MiddlePane {
    pub fn new(current_path: PathBuf) -> Self {
        MiddlePane {
            files: Vec::new(),
            current_path,
            dir_stream: None,
            errors: Vec::new(),
            cursor: 0,
        }
    }
    pub fn update(&mut self, message: MiddlePaneMsg) -> Task<MiddlePaneMsg> {
        match message {
            MiddlePaneMsg::RequestStartReadDir => {
                let path = self.current_path.clone();
                Task::perform(
                    start_read_dir_stream(path),
                    MiddlePaneMsg::ReceivedStartedReadDirStream,
                )
            }
            MiddlePaneMsg::ReceivedStartedReadDirStream(dir_stream) => match dir_stream {
                Ok(stream) => {
                    self.dir_stream = Some(stream);
                    Task::done(MiddlePaneMsg::RequestFindNextFileItem)
                }
                Err(error) => {
                    self.errors.push(error);
                    Task::none()
                }
            },
            MiddlePaneMsg::RequestFindNextFileItem => {
                if let Some(dir_stream) = self.dir_stream.take() {
                    Task::perform(next_file_item(dir_stream), MiddlePaneMsg::ReceivedFileItem)
                } else {
                    self.errors
                        .push(anyhow::anyhow!("The ReadDirStream not exist."));
                    Task::none()
                }
            }
            MiddlePaneMsg::ReceivedFileItem(file_tiem) => {
                if let Some((item, stream)) = file_tiem {
                    match item {
                        Ok(item) => {
                            self.files.push(item);
                        }
                        Err(error) => {
                            self.errors.push(error);
                        }
                    }
                    self.dir_stream = Some(stream);
                    Task::done(MiddlePaneMsg::RequestFindNextFileItem)
                } else {
                    Task::none()
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, MiddlePaneMsg> {
        let mut list = column![
            container(text(self.current_path.display().to_string()))
                .styled(style::container::pane_header()),
            style::rule::horizontal(),
        ];

        for (index, item) in self.files.iter().enumerate() {
            let input = RowInput {
                hovered: false,
                cursor: index == self.cursor,
                selected: item.selected,
            };

            list = list.push(
                container(text(item.label())).styled(style::container::list_row(input)),
            );
        }

        container(list)
            .styled(style::container::pane())
            .into()
    }
}

async fn start_read_dir_stream(dir: PathBuf) -> Result<ReadDirStream> {
    tokio::fs::read_dir(dir)
        .await
        .map(ReadDirStream::new)
        .map_err(anyhow::Error::from)
}

async fn next_file_item(mut stream: ReadDirStream) -> Option<(Result<FileItem>, ReadDirStream)> {
    let dir_entry = stream.next().await?;
    match dir_entry {
        Ok(entry) => {
            let path = entry.path();
            let file_type = entry.file_type().await;
            let item_type = if let Ok(file_type) = file_type {
                map_item_type(file_type)
            } else {
                ItemType::Unknown
            };
            Some((
                Ok(FileItem {
                    path,
                    item_type,
                    selected: false,
                }),
                stream,
            ))
        }
        Err(err) => Some((Err(anyhow::Error::from(err)), stream)),
    }
}
fn map_item_type(file_type: FileType) -> ItemType {
    if file_type.is_dir() {
        ItemType::Folder
    } else if file_type.is_file() {
        ItemType::File
    } else if file_type.is_symlink() {
        ItemType::SymLink
    } else {
        ItemType::Unknown
    }
}
