use std::path::{Path, PathBuf};

use anyhow::Result;
use iced::futures::StreamExt;
use iced::widget::{self, Rule, column, container, rule, text};
use iced::{Element, Task};
use tokio_stream::wrappers::ReadDirStream;

pub struct MiddlePane {
    files: Vec<FileItem>,
    current_path: PathBuf,
    dir_stream: Option<ReadDirStream>,
    errors: Vec<anyhow::Error>,
}

pub enum ItemType {
    File,
    Folder,
    SymLink,
}

pub struct FileItem {
    path: PathBuf,
    item_type: ItemType,
    selected: bool,
}

pub enum MiddlePaneMsg {
    RequestStartReadDir,
    ReceivedStartedReadDirStream(Result<ReadDirStream>),
    RequestFindNextFileItem,
    ReceivedFileItem(Option<Result<FileItem>>), //Same behavior as the rust stream, None for the stream end.
}

impl MiddlePane {
    pub fn update(&mut self, message: MiddlePaneMsg) -> Task<MiddlePaneMsg> {
        match message {
            MiddlePaneMsg::RequestStartReadDir => {
                let path = self.current_path.clone();
                Task::perform(
                    start_read_dir_stream(path),
                    MiddlePaneMsg::ReceivedStartedReadDirStream,
                )
            }
            MiddlePaneMsg::ReceivedStartedReadDirStream(dir_stream) => {
                match dir_stream {
                    Ok(stream) => self.dir_stream = Some(stream),
                    Err(error) => self.errors.push(error),
                };
                Task::none()
            }
            MiddlePaneMsg::RequestFindNextFileItem => {
                if let Some(dir_stream) = &self.dir_stream {
                    Task::perform(dir_stream.next(), MiddlePaneMsg::ReceivedFileItem)
                } else {
                    self.errors
                        .push(anyhow::anyhow!("The ReadDirStream not exist."));
                    Task::none()
                }
            }
            MiddlePaneMsg::ReceivedFileItem(file_tiem) => {
                if let Some(item) = file_tiem {
                    match item {
                        Ok(item) => {
                            self.files.push(item);
                        }
                        Err(error) => {
                            self.errors.push(error);
                        }
                    }
                    Task::done(MiddlePaneMsg::RequestFindNextFileItem)
                } else {
                    Task::none()
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, MiddlePaneMsg> {
        let mut column = column![
            text(self.current_path.to_str().unwrap()),
            rule::horizontal(2),
        ];

        for item in &self.files {
            let path = item.path.to_str();
            if let Some(path) = path {
                let visual_item = container(text(path));
                column = column.push(visual_item);
            }
        }

        column.into()
    }
}

async fn start_read_dir_stream(dir: PathBuf) -> Result<ReadDirStream> {
    tokio::fs::read_dir(dir)
        .await
        .map(ReadDirStream::new)
        .map_err(anyhow::Error::from)
}

async fn next_file_item(stream: &ReadDirStream) -> Option<Result<FileItem>> {
    let dir_entry = stream.next().await
    .map(|entry| {
            entry.map(|ok_entry| {
                let path = ok_entry.path();
            })
        })
}
