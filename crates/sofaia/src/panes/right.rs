use iced::Element;
use iced::widget::{column, container, text};

use crate::fs::Entry;
use crate::style::container::RowInput;
use crate::style::{self, StyleExt};

pub fn view<'a, M: 'a>(entry: Option<&'a Entry>) -> Element<'a, M> {
    let mut list = column![
        container(text("PREVIEW").styled(style::text::caption()))
            .styled(style::container::header()),
        style::rule::horizontal(),
    ];

    if let Some(entry) = entry {
        list = list.push(field("NAME", entry.name()));
        list = list.push(field("TYPE", entry.item_type().label().to_owned()));
        list = list.push(field("PATH", entry.path().display().to_string()));
    }

    list.into()
}

fn field<'a, M: 'a>(label: &'a str, value: String) -> Element<'a, M> {
    let content = column![
        text(label).styled(style::text::caption()),
        text(value).styled(style::text::body()),
    ];

    container(content)
        .styled(style::container::row(RowInput::default()))
        .into()
}
