//! The application: an Elm loop, two device grammars, and a menu.
//!
//! The layering, from the bottom:
//!
//! ```text
//! ericed        device facts      aimed mouse areas, the Meaning seam, typed styles
//! intent.rs     meaning           Motion / Row / Choice / Intent
//! keys.rs       keyboard grammar  Chord ──► Intent
//! gesture.rs    ambient grammar   WheelDirection ──► Intent
//! panes/, menu  components        Aimed<TheirTarget> ──► Intent (their own impl)
//! lib.rs        this file         AppMsg ──► update ──► dispatch ──► view
//! ```
//!
//! Only the grammars and the components decide meaning, and each does it in its
//! own file. There is no central table of "which gesture on which target": the
//! target's *type* answers that question, through [`ericed::Meaning`].
//!
//! # Note on `impl From<…>` and the orphan rule
//!
//! `ericed` deliberately ships **no** blanket `From` impls for
//! [`Aimed`](ericed::Aimed). If it did, the adapters below would be
//! impossible — a blanket `impl<T, M: From<T>> From<Aimed<T>> for M` in the
//! library would collide with every application's own. A library that wants to
//! be extended must leave the `From` space empty.

use std::path::{Path, PathBuf};

use iced::event::{self, Status};
use iced::keyboard;
use iced::mouse;
use iced::widget::{column, container, row, stack, text};
use iced::{Element, Event, Length, Subscription, Task, Theme, window};

use ericed::{Aimed, Meaning, WheelDirection, interpret};

use crate::fs::Entry;
use crate::intent::{Choice, Intent, Row};
use crate::keys::{Chord, ChordState};
use crate::menu::{Action, Menu};
use crate::panes::middle::{MiddlePane, MiddlePaneMsg};
use crate::panes::{left, right};
use crate::style::StyleExt;

pub mod fs;
pub mod gesture;
pub mod intent;
pub mod keys;
pub mod menu;
pub mod panes;
pub mod style;

pub struct App {
    middle: MiddlePane,
    menu: Menu,
    /// The keyboard grammar. Lives in the application state, so there is a
    /// single place where keys are interpreted.
    keys: ChordState,
    theme: Theme,
}

#[derive(Debug, Clone)]
pub enum AppMsg {
    /// A key press, already normalized at the edge of the program.
    ///
    /// A device-level message: it exists because a `Subscription` can only
    /// produce messages, never state.
    Chord(Chord),
    /// A wheel turn. Also device-level, and also from the subscription — the
    /// wheel is the one pointer gesture with no target, so no widget owns it.
    Scroll(WheelDirection),
    /// A gesture aimed at a row of the listing, relayed from the pane.
    ///
    /// The payload is a *fact* — which aim, which row — not a conclusion. The
    /// conclusion is [`Row`]'s `Meaning` impl.
    Listing(Aimed<Row>),
    /// A gesture aimed at a region of the menu.
    Menu(Aimed<menu::Target>),
    /// What one of the above meant.
    ///
    /// Always delivered through `Task::done`, so intents are visible in the
    /// message stream instead of being applied as a side effect.
    Intent(Intent),
    MiddlePane(MiddlePaneMsg),
}

impl From<MiddlePaneMsg> for AppMsg {
    fn from(message: MiddlePaneMsg) -> Self {
        match message {
            // A gesture aimed at a row is the application's to interpret, not
            // the pane's: the pane has no opinion, and `MiddlePane::update`
            // never sees this variant.
            MiddlePaneMsg::Aimed(aimed) => AppMsg::Listing(aimed),
            message => AppMsg::MiddlePane(message),
        }
    }
}

impl From<Aimed<menu::Target>> for AppMsg {
    fn from(aimed: Aimed<menu::Target>) -> Self {
        AppMsg::Menu(aimed)
    }
}

/// The one funnel every aimed gesture goes through, whatever component was hit.
///
/// One generic function instead of a match over target kinds: the meaning comes
/// from the component's own [`Meaning`] impl, and this function never has to
/// learn about a new component.
fn interpret_task<T: Meaning<Intent>>(aimed: Aimed<T>) -> Task<AppMsg> {
    match interpret(aimed) {
        Some(intent) => Task::done(AppMsg::Intent(intent)),
        None => Task::none(),
    }
}

impl App {
    pub fn new(start: PathBuf) -> (Self, Task<AppMsg>) {
        let (middle, task) = MiddlePane::new(start);

        (Self::with_panes(middle), task.map(AppMsg::from))
    }

    pub fn with_panes(middle: MiddlePane) -> Self {
        Self {
            middle,
            menu: Menu::new(),
            keys: ChordState,
            theme: Theme::CatppuccinMocha,
        }
    }

    pub fn update(&mut self, message: AppMsg) -> Task<AppMsg> {
        match message {
            AppMsg::Chord(chord) => match self.feed_chord(chord) {
                Some(intent) => Task::done(AppMsg::Intent(intent)),
                None => Task::none(),
            },
            AppMsg::Scroll(direction) => Task::done(AppMsg::Intent(gesture::scroll(direction))),
            AppMsg::Listing(aimed) => interpret_task(aimed),
            AppMsg::Menu(aimed) => interpret_task(aimed),
            AppMsg::Intent(intent) => self.dispatch(intent),
            AppMsg::MiddlePane(message) => self.middle.update(message).map(AppMsg::MiddlePane),
        }
    }

    /// Feeds a key to the keyboard grammar.
    ///
    /// Separated from [`Self::update`] and [`Self::dispatch`] so a keymap can be
    /// tested by asking what a key means, without a window or a runtime.
    pub fn feed_chord(&mut self, chord: Chord) -> Option<Intent> {
        self.keys.feed(chord)
    }

    /// Applies an intent.
    ///
    /// Intents are terminal: they change state or start a `Task`, and never
    /// produce another intent.
    pub fn dispatch(&mut self, intent: Intent) -> Task<AppMsg> {
        match intent {
            Intent::Move { motion } => {
                self.middle.move_cursor(motion);

                Task::none()
            }
            Intent::Focus(row) => {
                self.middle.focus(row);

                Task::none()
            }
            Intent::Open => self.open_cursor(),
            Intent::Parent => self.middle.parent().map(AppMsg::MiddlePane),
            Intent::Menu(row) => {
                // Right-clicking a row both points at it and opens a menu about
                // it, so the highlight follows the menu. The menu keeps the row
                // as a snapshot: this focus is for the eye, not for the action.
                self.middle.focus(row);

                let title = self
                    .middle
                    .cursor_entry()
                    .map(Entry::name)
                    .unwrap_or_default();

                self.menu.open(row, title);

                Task::none()
            }
            Intent::Choose(choice) => self.choose(choice),
            Intent::Dismiss => {
                self.menu.close();

                Task::none()
            }
        }
    }

    fn open_cursor(&mut self) -> Task<AppMsg> {
        self.menu.close();

        self.middle.open_cursor().map(AppMsg::MiddlePane)
    }

    /// Applies a menu choice.
    ///
    /// The action is resolved from the menu's own list, and it acts on the row
    /// the menu is *about* rather than on the cursor: the cursor may have moved
    /// since the menu opened, and the menu is a snapshot of a target.
    fn choose(&mut self, choice: Choice) -> Task<AppMsg> {
        let Some(action) = self.menu.action(choice.0) else {
            return Task::none();
        };

        match action {
            Action::Open => {
                if let Some(row) = self.menu.row() {
                    self.middle.focus(row);
                }

                self.open_cursor()
            }
            Action::Dismiss => {
                self.menu.close();

                Task::none()
            }
        }
    }

    /// The event subscription.
    ///
    /// One stream, two devices. `listen_with` is used instead of the typed
    /// helpers because it hands us the [`Status`] of every event, and that is
    /// the rule that keeps a widget's input out of the grammars: a key that a
    /// focused widget captured is not ours, and neither is a press that a
    /// `mouse_area` already turned into a message.
    pub fn subscription(&self) -> Subscription<AppMsg> {
        event::listen_with(on_event)
    }

    pub fn path(&self) -> &Path {
        self.middle.path()
    }

    pub fn cursor(&self) -> usize {
        self.middle.cursor()
    }

    pub fn menu_open(&self) -> bool {
        self.menu.is_open()
    }

    pub fn view(&self) -> Element<'_, AppMsg> {
        let panes = row![
            container(left::view(self.middle.path())).styled(style::container::sidebar()),
            container(self.middle.view().map(AppMsg::from)).styled(style::container::content()),
            container(right::view(self.middle.cursor_entry())).styled(style::container::preview()),
        ]
        .spacing(style::container::gap())
        .height(Length::Fill);

        let status =
            container(text(format!("{} items", self.middle.len())).styled(style::text::caption()))
                .styled(style::container::status_bar());

        let base = container(column![panes, status].spacing(style::container::gap()))
            .styled(style::container::shell());

        if self.menu.is_open() {
            // The order *is* the contract: the panel is asked first, then the
            // scrim, then the app. Each layer is opaque to the ones below it,
            // so no event has to be forwarded and none leaks.
            stack![base, self.menu.scrim(), self.menu.panel()].into()
        } else {
            base.into()
        }
    }

    pub fn theme(&self) -> Theme {
        self.theme.clone()
    }
}

/// The single place where iced events enter the application.
///
/// `listen_with` takes a plain `fn`, so this cannot read or write state; all it
/// does is normalize a device event and drop everything else.
fn on_event(event: Event, status: Status, _window: window::Id) -> Option<AppMsg> {
    // A widget that captured the event owns it; reading it here as well would
    // handle the same gesture twice. The rule is the same for both devices —
    // and for the mouse it is *required*, because `on_press` captures.
    if status == Status::Captured {
        return None;
    }

    match event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key,
            modified_key,
            modifiers,
            ..
        }) => {
            let chord =
                Chord::read(&modified_key, modifiers).or_else(|| Chord::read(&key, modifiers))?;

            Some(AppMsg::Chord(chord))
        }
        // The wheel is the one pointer gesture that still arrives as device
        // data, because it has no target and no widget owns it. If a
        // `scrollable` is ever placed under the pointer it captures the wheel
        // itself and this arm goes quiet — the same ownership rule as for keys.
        Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
            WheelDirection::read(delta).map(AppMsg::Scroll)
        }
        _ => None,
    }
}
