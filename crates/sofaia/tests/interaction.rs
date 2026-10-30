//! What the widget tree actually reports, tested with `iced_test`.
//!
//! Everything above this file — the grammars, the meanings, the intents — is
//! tested without a window. This file answers the two questions only a real
//! layout can answer:
//!
//! 1. does the wiring in a view produce the message the component expects?
//! 2. does a `Stack` hand the press to the *top* layer, so a menu click never
//!    reaches the listing underneath it?

use std::path::PathBuf;
use std::time::Duration;

use iced::Point;
use iced::time::Instant;
use iced_test::simulator;
use iced_test::simulator::click;

use ericed::{Aim, Aimed};

use sofaia::fs::{DataType, Entry, ItemType};
use sofaia::intent::{Intent, Row};
use sofaia::menu::Target as Region;
use sofaia::panes::middle::{MiddlePane, MiddlePaneMsg};
use sofaia::{App, AppMsg};

fn pane() -> MiddlePane {
    let root = PathBuf::from("/tmp/sofaia");
    let entries = vec![
        Entry::new(root.join("src"), ItemType::Folder),
        Entry::new(root.join("README.md"), ItemType::File(DataType::Text)),
    ];

    MiddlePane::with_entries(root, entries)
}

fn app() -> App {
    let root = PathBuf::from("/home/user");
    let entries = vec![
        Entry::new(root.join("src"), ItemType::Folder),
        Entry::new(root.join("assets"), ItemType::Folder),
        Entry::new(root.join("notes.md"), ItemType::File(DataType::Text)),
    ];

    App::with_panes(MiddlePane::with_entries(root, entries))
}

/// Lets an entrance finish.
///
/// A click lands on the picture it can see, and a menu that has just opened is
/// still scaling up — the card is drawn at 96%, so every coordinate in it is
/// four percent off until the entrance ends. These tests are about *meaning*
/// (which press reaches which target), not about the entrance, so they start
/// from the settled picture: two frames, one to start the motion and one to
/// finish it, and the instant is chosen here rather than waited for.
fn settle(app: &mut App) {
    let start = Instant::now();

    let _ = app.update(AppMsg::Frame(start));
    let _ = app.update(AppMsg::Frame(start + Duration::from_secs(1)));
}

#[test]
fn clicking_a_row_emits_an_aimed_press() {
    let pane = pane();

    for (label, expected) in [("src/", 0), ("README.md", 1)] {
        let mut ui = simulator(pane.view());

        ui.click(label).unwrap();

        let messages: Vec<_> = ui.into_messages().collect();

        assert!(
            messages.iter().any(|message| matches!(
                message,
                MiddlePaneMsg::Aimed(Aimed {
                    aim: Aim::Press,
                    target: Row(index),
                }) if *index == expected
            )),
            "{label}: {messages:?}"
        );
    }
}

#[test]
fn a_single_click_is_only_a_press() {
    let pane = pane();
    let mut ui = simulator(pane.view());

    ui.click("README.md").unwrap();

    let messages: Vec<_> = ui.into_messages().collect();

    // `mouse_area` publishes `on_press` and then, on a second click inside the
    // platform's window, `on_double_click`. A single simulated click must not be
    // upgraded to an activation — that decision belongs to the platform's click
    // counting, not to us.
    assert!(
        !messages.iter().any(|message| matches!(
            message,
            MiddlePaneMsg::Aimed(Aimed {
                aim: Aim::DoubleClick,
                ..
            })
        )),
        "{messages:?}"
    );
}

#[test]
fn a_click_on_the_menu_stays_on_the_menu() {
    let mut app = app();
    let _ = app.dispatch(Intent::Menu(Row(1)));
    settle(&mut app);

    let mut ui = simulator(app.view());

    ui.click("Open").unwrap();

    let messages: Vec<_> = ui.into_messages().collect();

    // The listing is under the menu and also contains text near the panel. If
    // the `Stack` were a filter, one of these would be a `Listing` gesture.
    assert!(
        messages.iter().any(|message| matches!(
            message,
            AppMsg::Menu(Aimed {
                aim: Aim::Press,
                target: Region::Item(0),
            })
        )),
        "{messages:?}"
    );
    assert!(
        !messages
            .iter()
            .any(|message| matches!(message, AppMsg::Listing(_))),
        "a click on the panel reached the listing: {messages:?}"
    );
}

#[test]
fn a_click_outside_the_menu_hits_the_scrim() {
    let mut app = app();
    let _ = app.dispatch(Intent::Menu(Row(1)));
    settle(&mut app);

    let mut ui = simulator(app.view());

    // Any corner is outside the centred card and inside the scrim, which covers
    // the whole window. `point_at` + `click` is a raw click at a coordinate,
    // because no widget selector describes "somewhere empty".
    ui.point_at(Point::new(4.0, 4.0));
    ui.simulate(click());

    let messages: Vec<_> = ui.into_messages().collect();

    assert!(
        messages.iter().any(|message| matches!(
            message,
            AppMsg::Menu(Aimed {
                aim: Aim::Press,
                target: Region::Scrim,
            })
        )),
        "{messages:?}"
    );
}

#[test]
fn without_a_menu_nothing_sits_above_the_listing() {
    let app = app();

    let mut ui = simulator(app.view());

    ui.click("assets/").unwrap();

    let messages: Vec<_> = ui.into_messages().collect();

    assert!(
        messages.iter().any(|message| matches!(
            message,
            AppMsg::Listing(Aimed {
                aim: Aim::Press,
                target: Row(1),
            })
        )),
        "{messages:?}"
    );
}
