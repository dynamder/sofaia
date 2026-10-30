use std::path::{Path, PathBuf};

use iced::widget::{column, container, mouse_area, text};
use iced::{Element, Task};

use crate::fs::{self, Entry};
use crate::style::container::RowInput;
use crate::style::{self, StyleExt};

pub struct MiddlePane {
    path: PathBuf,
    entries: Vec<Entry>,
    cursor: usize,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum MiddlePaneMsg {
    EntryActivated(usize),
    EntriesLoaded(Result<Vec<Entry>, String>),
}

impl MiddlePane {
    pub fn new(path: PathBuf) -> (Self, Task<MiddlePaneMsg>) {
        let pane = Self::with_entries(path.clone(), Vec::new());

        (pane, load(path))
    }

    pub fn with_entries(path: PathBuf, entries: Vec<Entry>) -> Self {
        Self {
            path,
            entries,
            cursor: 0,
            error: None,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn cursor_entry(&self) -> Option<&Entry> {
        self.entries.get(self.cursor)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn update(&mut self, message: MiddlePaneMsg) -> Task<MiddlePaneMsg> {
        match message {
            MiddlePaneMsg::EntryActivated(index) => {
                let Some(entry) = self.entries.get(index) else {
                    return Task::none();
                };

                if !entry.item_type().is_folder() {
                    self.cursor = index;

                    return Task::none();
                }

                let path = entry.path().to_path_buf();
                self.path = path.clone();
                self.entries.clear();
                self.cursor = 0;

                load(path)
            }
            MiddlePaneMsg::EntriesLoaded(Ok(entries)) => {
                self.entries = entries;
                self.cursor = 0;
                self.error = None;

                Task::none()
            }
            MiddlePaneMsg::EntriesLoaded(Err(error)) => {
                self.error = Some(error);

                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, MiddlePaneMsg> {
        let mut list = column![
            container(text(self.path.display().to_string()).styled(style::text::caption()))
                .styled(style::container::header()),
            style::rule::horizontal(),
        ];

        if let Some(error) = &self.error {
            list = list.push(
                container(text(error.clone()).styled(style::text::error()))
                    .styled(style::container::row(RowInput::default())),
            );
        }

        for (index, entry) in self.entries.iter().enumerate() {
            let input = RowInput {
                hovered: false,
                cursor: index == self.cursor,
                selected: false,
            };

            list = list.push(
                mouse_area(
                    container(text(entry.label()).styled(style::text::body()))
                        .styled(style::container::row(input)),
                )
                .on_press(MiddlePaneMsg::EntryActivated(index)),
            );
        }

        list.into()
    }
}

fn load(path: PathBuf) -> Task<MiddlePaneMsg> {
    Task::perform(fs::read_dir(path), |result| {
        MiddlePaneMsg::EntriesLoaded(result.map_err(|error| error.to_string()))
    })
}
