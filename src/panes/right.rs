use iced::widget::{column, container, text};
use iced::Element;

use crate::panes::middle::FileItem;
use crate::style::container::RowInput;
use crate::style::{self, StyleExt};

pub fn view<'a, M: 'a>(item: Option<&'a FileItem>) -> Element<'a, M> {
    let mut list = column![
        container(text("PREVIEW").styled(style::text::caption()))
            .styled(style::container::header()),
        style::rule::horizontal(),
    ];

    if let Some(item) = item {
        list = list.push(entry("NAME", item.name()));
        list = list.push(entry("TYPE", item.item_type().label().to_owned()));
        list = list.push(entry("PATH", item.path().display().to_string()));
    }

    list.into()
}

fn entry<'a, M: 'a>(label: &'a str, value: String) -> Element<'a, M> {
    let content = column![
        text(label).styled(style::text::caption()),
        text(value).styled(style::text::body()),
    ];

    container(content)
        .styled(style::container::row(RowInput::default()))
        .into()
}
