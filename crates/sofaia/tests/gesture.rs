//! The pointer grammar, tested without a window.
//!
//! After the refactor there is no single "pointer grammar" any more, and that is
//! the point: an aimed gesture means whatever the *component that owns the
//! target* says it means. These tests are therefore split the way the code is —
//! one block per component — plus the wheel, which is the only part still
//! interpreted in one place.
//!
//! The keyboard's tests are in `keys.rs` and follow the same shape: ask what an
//! input means (`interpret`), then apply it (`dispatch`).

use std::path::{Path, PathBuf};

use ericed::{Aim, Aimed, Meaning, WheelDirection, interpret};

use sofaia::fs::{DataType, Entry, ItemType};
use sofaia::gesture;
use sofaia::intent::{Choice, Intent, Motion, Row};
use sofaia::menu::Target as Region;
use sofaia::panes::middle::{MiddlePane, MiddlePaneMsg};
use sofaia::{App, AppMsg};

fn app() -> App {
    let root = PathBuf::from("/home/user");
    let entries = vec![
        Entry::new(root.join("src"), ItemType::Folder),
        Entry::new(root.join("assets"), ItemType::Folder),
        Entry::new(root.join("README.md"), ItemType::File(DataType::Text)),
    ];

    App::with_panes(MiddlePane::with_entries(root, entries))
}

/// Feeds a gesture the way `App::update` does: interpret, then dispatch.
fn apply<T: Meaning<Intent>>(app: &mut App, aimed: Aimed<T>) {
    if let Some(intent) = interpret(aimed) {
        let _ = app.dispatch(intent);
    }
}

// --- the listing -----------------------------------------------------------

#[test]
fn a_press_points_at_a_row() {
    let mut app = app();

    assert_eq!(
        interpret(Aimed::new(Aim::Press, Row(2))),
        Some(Intent::Focus(Row(2)))
    );

    apply(&mut app, Aimed::new(Aim::Press, Row(2)));

    assert_eq!(app.cursor(), 2);
}

#[test]
fn a_double_click_opens_the_folder_under_the_pointer() {
    let mut app = app();

    // `mouse_area` publishes press → release → press → double click, so the
    // press always arrives first. That is what moves the cursor, and it is why
    // `Intent::Open` can stay target-less: it acts on the cursor, exactly like
    // `l` does.
    apply(&mut app, Aimed::new(Aim::Press, Row(1)));
    apply(&mut app, Aimed::new(Aim::DoubleClick, Row(1)));

    assert_eq!(app.path(), Path::new("/home/user/assets"));
}

#[test]
fn a_double_click_on_a_file_does_not_navigate() {
    let mut app = app();

    apply(&mut app, Aimed::new(Aim::Press, Row(2)));
    apply(&mut app, Aimed::new(Aim::DoubleClick, Row(2)));

    assert_eq!(app.path(), Path::new("/home/user"));
    assert_eq!(app.cursor(), 2);
}

#[test]
fn a_stale_row_is_a_no_op() {
    let mut app = app();

    // A target comes from a layout that may be a frame old: an asynchronous
    // read can have replaced the listing underneath it. Out of range is a
    // no-op, never a panic.
    let _ = app.dispatch(Intent::Focus(Row(99)));

    assert_eq!(app.cursor(), 0);
}

// --- the meaning is the component's, not a table's -------------------------

#[test]
fn the_same_gesture_means_what_each_component_says() {
    // One `Aim::Press`, three different meanings, and no central table that
    // knows the three components exist.
    assert_eq!(
        interpret(Aimed::new(Aim::Press, Row(0))),
        Some(Intent::Focus(Row(0)))
    );
    assert_eq!(
        interpret(Aimed::new(Aim::Press, Region::Item(1))),
        Some(Intent::Choose(Choice(1)))
    );
    assert_eq!(
        interpret(Aimed::new(Aim::Press, Region::Scrim)),
        Some(Intent::Dismiss)
    );
}

#[test]
fn gestures_a_component_does_not_use_have_no_meaning() {
    // The menu is pressed, never double-clicked, and has no menu of its own.
    assert_eq!(
        interpret(Aimed::new(Aim::DoubleClick, Region::Item(0))),
        None
    );
    assert_eq!(
        interpret(Aimed::new(Aim::RightPress, Region::Item(0))),
        None
    );
    assert_eq!(interpret(Aimed::new(Aim::DoubleClick, Region::Scrim)), None);
    assert_eq!(interpret(Aimed::new(Aim::RightPress, Region::Scrim)), None);
}

// --- the menu --------------------------------------------------------------

#[test]
fn a_right_press_opens_the_menu_on_that_row() {
    let mut app = app();

    apply(&mut app, Aimed::new(Aim::RightPress, Row(1)));

    assert!(app.menu_open());
    assert_eq!(app.cursor(), 1);
}

#[test]
fn a_menu_line_is_chosen_with_one_press() {
    let mut app = app();

    apply(&mut app, Aimed::new(Aim::RightPress, Row(1)));
    assert!(app.menu_open());

    assert_eq!(
        interpret(Aimed::new(Aim::Press, Region::Item(0))),
        Some(Intent::Choose(Choice(0)))
    );

    apply(&mut app, Aimed::new(Aim::Press, Region::Item(0)));

    assert_eq!(app.path(), Path::new("/home/user/assets"));
    assert!(!app.menu_open());
}

#[test]
fn a_choice_with_nothing_to_do_only_closes() {
    let mut app = app();

    apply(&mut app, Aimed::new(Aim::RightPress, Row(0)));

    // Line 9 is `Dismiss`: a real action, unlike the placeholders in between.
    apply(&mut app, Aimed::new(Aim::Press, Region::Item(9)));

    assert_eq!(app.path(), Path::new("/home/user"));
    assert!(!app.menu_open());
}

#[test]
fn a_placeholder_line_has_nothing_to_do() {
    let mut app = app();

    apply(&mut app, Aimed::new(Aim::RightPress, Row(0)));

    // Placeholders are not targets, so no `Choice` for one can be produced by
    // the view. A stale one still must not do anything.
    let _ = app.dispatch(Intent::Choose(Choice(1)));

    assert!(app.menu_open());
    assert_eq!(app.path(), Path::new("/home/user"));
}

#[test]
fn a_press_on_the_scrim_dismisses() {
    let mut app = app();

    apply(&mut app, Aimed::new(Aim::RightPress, Row(0)));
    assert!(app.menu_open());

    apply(&mut app, Aimed::new(Aim::Press, Region::Scrim));

    assert!(!app.menu_open());
    // A dismissal changes nothing else.
    assert_eq!(app.cursor(), 0);
}

// --- the ambient half ------------------------------------------------------

#[test]
fn the_wheel_is_a_relative_motion() {
    assert_eq!(
        gesture::scroll(WheelDirection::Down),
        Intent::Move {
            motion: Motion::Next
        }
    );
    assert_eq!(
        gesture::scroll(WheelDirection::Up),
        Intent::Move {
            motion: Motion::Previous
        }
    );
}

#[test]
fn the_wheel_moves_the_cursor_and_stops_at_the_edges() {
    let mut app = app();

    let _ = app.dispatch(gesture::scroll(WheelDirection::Down));
    assert_eq!(app.cursor(), 1);

    let _ = app.dispatch(gesture::scroll(WheelDirection::Up));
    assert_eq!(app.cursor(), 0);

    // The same clamping as `j`/`k`: a scroll is a motion, so the pane owns the
    // edges.
    let _ = app.dispatch(gesture::scroll(WheelDirection::Up));
    assert_eq!(app.cursor(), 0);
}

#[test]
fn hover_is_not_a_gesture() {
    let mut pane = MiddlePane::with_entries(
        PathBuf::from("/home/user"),
        vec![Entry::new(
            PathBuf::from("/home/user/src"),
            ItemType::Folder,
        )],
    );

    let _ = pane.update(MiddlePaneMsg::Hover(Some(0)));

    // Appearance only: the row lights up, the cursor does not move, and no
    // intent is produced anywhere.
    assert_eq!(pane.hovered(), Some(0));
    assert_eq!(pane.cursor(), 0);

    let _ = pane.update(MiddlePaneMsg::Hover(None));
    assert_eq!(pane.hovered(), None);
}

#[test]
fn a_gesture_is_forwarded_through_a_task() {
    let mut app = app();

    let _task = app.update(AppMsg::Listing(Aimed::new(Aim::Press, Row(2))));

    // Same rule as the keymap: interpreting a gesture must not apply it.
    assert_eq!(app.cursor(), 0);
}
