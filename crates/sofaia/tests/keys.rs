//! The keymap, tested without a window.
//!
//! A key press has two halves, and they are tested separately on purpose:
//!
//! - [`App::feed_chord`] says *what a key means* (pure);
//! - [`App::dispatch`] applies that meaning (state + tasks).
//!
//! The line in between is `Task::done(AppMsg::Intent(..))`, which delays the
//! effect by one `update`. That is deliberate, so it gets its own test below
//! instead of being silently assumed.
//!
//! `dispatch` returns a `Task`; the tests drop it, because a pane moves its
//! cursor or changes its path synchronously and only the directory read is
//! asynchronous.

use std::path::{Path, PathBuf};

use iced::keyboard::key::Named;
use iced::keyboard::{Key, Modifiers};

use sofaia::fs::{DataType, Entry, ItemType};
use sofaia::intent::{Intent, Motion};
use sofaia::keys::Chord;
use sofaia::panes::middle::MiddlePane;
use sofaia::{App, AppMsg};

fn app() -> App {
    let root = PathBuf::from("/home/user");
    let entries = vec![
        Entry::new(root.join("src"), ItemType::Folder),
        Entry::new(root.join("README.md"), ItemType::File(DataType::Text)),
    ];

    App::with_panes(MiddlePane::with_entries(root, entries))
}

fn down() -> Intent {
    Intent::Move {
        motion: Motion::Next,
    }
}

fn up() -> Intent {
    Intent::Move {
        motion: Motion::Previous,
    }
}

#[test]
fn hjkl_map_to_intents() {
    let mut app = app();

    assert_eq!(app.feed_chord(Chord::Char('j')), Some(down()));
    assert_eq!(app.feed_chord(Chord::Char('k')), Some(up()));
    assert_eq!(app.feed_chord(Chord::Char('h')), Some(Intent::Parent));
    assert_eq!(app.feed_chord(Chord::Char('l')), Some(Intent::Open));
}

#[test]
fn unbound_keys_do_nothing() {
    let mut app = app();

    assert_eq!(app.feed_chord(Chord::Char('x')), None);
    assert_eq!(app.feed_chord(Chord::Named(Named::Enter)), None);
}

#[test]
fn escape_dismisses() {
    let mut app = app();

    // The one bound named key. It is how the keyboard closes what the mouse
    // opened, and it produces the same `Intent` as a press on the background.
    assert_eq!(
        app.feed_chord(Chord::Named(Named::Escape)),
        Some(Intent::Dismiss)
    );
}

#[test]
fn j_and_k_move_the_cursor_and_stop_at_the_edges() {
    let mut app = app();
    assert_eq!(app.cursor(), 0);

    let _ = app.dispatch(down());
    assert_eq!(app.cursor(), 1);

    // Already on the last entry.
    let _ = app.dispatch(down());
    assert_eq!(app.cursor(), 1);

    let _ = app.dispatch(up());
    assert_eq!(app.cursor(), 0);

    // Already on the first entry.
    let _ = app.dispatch(up());
    assert_eq!(app.cursor(), 0);
}

#[test]
fn l_enters_the_folder_under_the_cursor() {
    let mut app = app();

    let _ = app.dispatch(Intent::Open);

    assert_eq!(app.path(), Path::new("/home/user/src"));
    assert_eq!(app.cursor(), 0);
}

#[test]
fn l_on_a_file_does_nothing() {
    let mut app = app();

    let _ = app.dispatch(down());
    let _ = app.dispatch(Intent::Open);

    assert_eq!(app.path(), Path::new("/home/user"));
    assert_eq!(app.cursor(), 1);
}

#[test]
fn h_walks_up_and_stops_at_the_root() {
    let mut app = app();

    let _ = app.dispatch(Intent::Parent);
    assert_eq!(app.path(), Path::new("/home"));

    let _ = app.dispatch(Intent::Parent);
    assert_eq!(app.path(), Path::new("/"));

    // The root has no parent; the intent must be a no-op rather than an error.
    let _ = app.dispatch(Intent::Parent);
    assert_eq!(app.path(), Path::new("/"));
}

#[test]
fn a_chord_is_forwarded_through_a_task() {
    let mut app = app();

    let _task = app.update(AppMsg::Chord(Chord::Char('j')));

    // The intent travels as a message (`Task::done`), so it lands on a later
    // `update`; interpreting the key must not move the cursor by itself.
    assert_eq!(app.cursor(), 0);
}

#[test]
fn chord_read_applies_shift_and_ctrl() {
    // `modified_key` carries Shift, so the grammar sees an uppercase character.
    let shifted: Key = Key::Character("G".into());
    assert_eq!(
        Chord::read(&shifted, Modifiers::SHIFT),
        Some(Chord::Char('G'))
    );

    // Ctrl is *not* applied to `modified_key`, so it is read from the modifiers.
    let plain: Key = Key::Character("d".into());
    assert_eq!(Chord::read(&plain, Modifiers::CTRL), Some(Chord::Ctrl('d')));

    assert_eq!(Chord::read(&Key::Unidentified, Modifiers::default()), None);
}
