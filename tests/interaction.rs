use std::path::PathBuf;

use iced_test::simulator;
use sofaia::fs::{DataType, Entry, ItemType};
use sofaia::panes::middle::{MiddlePane, MiddlePaneMsg};

fn pane() -> MiddlePane {
    let root = PathBuf::from("/tmp/sofaia");
    let entries = vec![
        Entry::new(root.join("src"), ItemType::Folder),
        Entry::new(root.join("README.md"), ItemType::File(DataType::Text)),
    ];

    MiddlePane::with_entries(root, entries)
}

#[test]
fn clicking_rows_emits_entry_activated() {
    let pane = pane();

    for (label, expected) in [("src/", 0), ("README.md", 1)] {
        let mut ui = simulator(pane.view());

        ui.click(label).unwrap();

        let messages: Vec<_> = ui.into_messages().collect();

        assert!(
            messages
                .iter()
                .any(|message| matches!(message, MiddlePaneMsg::EntryActivated(index) if *index == expected)),
            "{label}: {messages:?}"
        );
    }
}
