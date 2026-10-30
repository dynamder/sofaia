use std::path::PathBuf;

use sofaia::App;

fn main() -> iced::Result {
    let start = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("/"));

    iced::application(move || App::new(start.clone()), App::update, App::view)
        .subscription(App::subscription)
        .theme(App::theme)
        .centered()
        .run()
}
