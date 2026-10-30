//! The smallest possible consumer: two impls and three lines of view.
//!
//! This file exists to be *run and compiled*: `cargo run -p ericed --example
//! aimed` is the proof that a second crate can use the extension without copying
//! anything out of it. It also runs an assertion, so the example is a test of the
//! seam rather than a decoration.
//!
//! The steps, in order:
//!
//! 1. declare that your message type can be built from an aimed gesture;
//! 2. declare what pointing at *your* target means;
//! 3. bring the extension trait into scope, and aim at the container whose
//!    bounds are the row.

use ericed::{Aim, AimExt, Aimed, Meaning};
use iced_widget::{MouseArea, container, text};

/// A row of *this* application. `ericed` never sees this type, which is why a
/// second crate can keep its own vocabulary.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // the index is what a real application would act on
struct Entry(usize);

/// A consumer's message type.
#[derive(Debug, Clone)]
#[allow(dead_code)] // the payload is what the application would match on
enum Message {
    Entry(Aimed<Entry>),
}

impl From<Aimed<Entry>> for Message {
    fn from(aimed: Aimed<Entry>) -> Self {
        Message::Entry(aimed)
    }
}

/// What pointing at an [`Entry`] means.
///
/// The meaning type is `&'static str` here only to make the point: `I` is the
/// application's own vocabulary. A real application would return its `Intent`.
impl Meaning<&'static str> for Entry {
    fn meaning(self, aim: Aim) -> Option<&'static str> {
        match aim {
            Aim::Press => Some("focus"),
            Aim::DoubleClick => Some("open"),
            Aim::RightPress => Some("menu"),
        }
    }
}

fn row() -> MouseArea<'static, Message> {
    // The container, not the text: the hit box is the row's, not the glyphs'.
    container(text("README.md")).aimed(Entry(3))
}

fn main() {
    let _row = row();

    assert_eq!(
        ericed::interpret(Aimed::new(Aim::DoubleClick, Entry(3))),
        Some("open")
    );
}
