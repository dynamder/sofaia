use std::path::Path;

use iced::Element;
use iced::widget::{column, container, text};

use crate::style::container::RowInput;
use crate::style::{self, StyleExt};

pub fn view<'a, M: 'a>(path: &'a Path) -> Element<'a, M> {
    let mut list = column![
        container(text("PARENTS").styled(style::text::caption()))
            .styled(style::container::header()),
        style::rule::horizontal(),
    ];

    for ancestor in path.ancestors().skip(1) {
        let label = ancestor.file_name().map_or_else(
            || "/".to_owned(),
            |name| name.to_string_lossy().into_owned(),
        );

        list = list.push(
            container(text(label).styled(style::text::body()))
                .styled(style::container::row(RowInput::default())),
        );
    }

    list.into()
}
