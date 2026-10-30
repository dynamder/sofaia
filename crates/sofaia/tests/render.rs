//! Screenshots, for the things arithmetic cannot check: spacing, clipping, and
//! whether the menu really sits over the app instead of inside it.
//!
//! The images are written to the workspace's `target/`, so this test is also a
//! manual review tool: run it, open the PNGs, look at them.

use std::path::PathBuf;
use std::time::Duration;

use iced::time::Instant;
use iced::{Element, Size, Task, Theme, window};

use sofaia::fs::{DataType, Entry, ItemType};
use sofaia::intent::{Intent, Row};
use sofaia::panes::middle::MiddlePane;
use sofaia::{App, AppMsg};

struct Harness {
    /// `None` leaves the menu closed; `Some(elapsed)` opens it and advances the
    /// clock by that much, so a screenshot can be taken *during* the entrance.
    menu: Option<Duration>,
    /// Navigate to the parent on boot, which starts the slide.
    navigate: bool,
}

impl iced::Program for Harness {
    type State = App;
    type Message = AppMsg;
    type Theme = Theme;
    type Renderer = iced::Renderer;
    type Executor = iced::executor::Default;

    fn name() -> &'static str {
        "sofaia"
    }

    fn settings(&self) -> iced::Settings {
        iced::Settings::default()
    }

    fn window(&self) -> Option<window::Settings> {
        None
    }

    fn boot(&self) -> (App, Task<AppMsg>) {
        let mut app = fixture();

        if let Some(elapsed) = self.menu {
            // The menu is opened through an intent, like everything else: a
            // screenshot needs no second way into the state.
            let _ = app.dispatch(Intent::Menu(Row(1)));

            // A screenshot is *one* frame, and the first frame of an entrance is
            // a menu that has not arrived yet — an invisible scrim over a card at
            // 96%. So the clock is set explicitly: one tick to start the motion,
            // one to place it `elapsed` later. `Duration::ZERO` would look like
            // no menu at all; a second is the settled picture.
            let now = Instant::now();
            let _ = app.update(AppMsg::Frame(now));
            let _ = app.update(AppMsg::Frame(now + elapsed));
        }

        if self.navigate {
            // Same trick for the slide: a folder change is an intent too. How
            // far the animation has got by the time the screenshot is taken is
            // whatever the emulator managed — which is itself worth looking at.
            let _ = app.dispatch(Intent::Parent);
        }

        (app, Task::none())
    }

    fn update(&self, state: &mut App, message: AppMsg) -> Task<AppMsg> {
        state.update(message)
    }

    fn view<'a>(&self, state: &'a App, _window: window::Id) -> Element<'a, AppMsg> {
        state.view()
    }

    fn theme(&self, state: &App, _window: window::Id) -> Option<Theme> {
        Some(state.theme())
    }
}

fn fixture() -> App {
    let root = PathBuf::from("/home/dynamder");
    let entries = vec![
        Entry::new(root.join("Projects"), ItemType::Folder),
        Entry::new(root.join("Documents"), ItemType::Folder),
        Entry::new(root.join("Downloads"), ItemType::Folder),
        Entry::new(root.join("Pictures"), ItemType::Folder),
        Entry::new(root.join("notes.md"), ItemType::File(DataType::Text)),
        Entry::new(root.join("cover.png"), ItemType::File(DataType::Image)),
        Entry::new(root.join("archive.zip"), ItemType::File(DataType::Archive)),
    ];

    App::with_panes(MiddlePane::with_entries(root, entries))
}

/// The workspace's `target/`, not the crate's: the app is not the workspace
/// root any more.
fn target_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("crate lives in <workspace>/crates/<crate>")
        .join("target")
}

#[test]
fn render_screenshots() {
    let cases = [
        ("dark", Theme::CatppuccinMocha, None, false),
        ("light", Theme::Light, None, false),
        (
            "menu",
            Theme::CatppuccinMocha,
            Some(Duration::from_secs(1)),
            false,
        ),
        // The same menu, part-way in: the scrim is dimmer than it will be and
        // the card is a little small. Two pictures of one value, which is the
        // whole claim the animation system makes.
        (
            "menu-arriving",
            Theme::CatppuccinMocha,
            Some(Duration::from_millis(60)),
            false,
        ),
        ("slide", Theme::CatppuccinMocha, None, true),
    ];

    for (name, theme, menu, navigate) in cases {
        let shot = iced_test::screenshot(
            &Harness { menu, navigate },
            &theme,
            Size::from((1200, 760)),
            1.0,
            Duration::from_millis(300),
        );

        let path = target_dir().join(format!("ui-{name}.png"));
        let file = std::fs::File::create(&path).unwrap();
        let mut encoder = png::Encoder::new(
            std::io::BufWriter::new(file),
            shot.size.width,
            shot.size.height,
        );

        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&shot.rgba)
            .unwrap();

        eprintln!("screenshot: {}", path.display());
    }
}
