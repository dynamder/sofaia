//! Screenshots, for the things arithmetic cannot check: spacing, clipping, and
//! whether the menu really sits over the app instead of inside it.
//!
//! The images are written to the workspace's `target/`, so this test is also a
//! manual review tool: run it, open the PNGs, look at them.

use std::path::PathBuf;
use std::time::Duration;

use iced::{Element, Size, Task, Theme, window};

use sofaia::fs::{DataType, Entry, ItemType};
use sofaia::intent::{Intent, Row};
use sofaia::panes::middle::MiddlePane;
use sofaia::{App, AppMsg};

struct Harness {
    open_menu: bool,
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

        if self.open_menu {
            // The menu is opened through an intent, like everything else: a
            // screenshot needs no second way into the state.
            let _ = app.dispatch(Intent::Menu(Row(1)));
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
        ("dark", Theme::CatppuccinMocha, false, false),
        ("light", Theme::Light, false, false),
        ("menu", Theme::CatppuccinMocha, true, false),
        ("slide", Theme::CatppuccinMocha, false, true),
    ];

    for (name, theme, open_menu, navigate) in cases {
        let shot = iced_test::screenshot(
            &Harness {
                open_menu,
                navigate,
            },
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
