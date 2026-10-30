use std::path::PathBuf;
use std::time::Duration;

use iced::{Element, Size, Task, Theme, window};
use sofaia::fs::{DataType, Entry, ItemType};
use sofaia::panes::middle::MiddlePane;
use sofaia::{App, AppMsg};

struct Harness;

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
        (fixture(), Task::none())
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

#[test]
fn render_screenshots() {
    for (name, theme) in [("dark", Theme::CatppuccinMocha), ("light", Theme::Light)] {
        let shot = iced_test::screenshot(
            &Harness,
            &theme,
            Size::from((1200, 760)),
            1.0,
            Duration::from_millis(300),
        );

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!("ui-{name}.png"));
        let file = std::fs::File::create(&path).unwrap();
        let mut encoder =
            png::Encoder::new(std::io::BufWriter::new(file), shot.size.width, shot.size.height);

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
