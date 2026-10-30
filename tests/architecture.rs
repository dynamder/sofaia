use std::path::{Path, PathBuf};

const FORBIDDEN: &[&str] = &[
    ".style(",
    "Color::",
    "Border",
    "Padding",
    "Shadow",
    "iced::widget::button::Style",
    "iced::widget::container::Style",
];

#[test]
fn views_do_not_contain_raw_styling() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/panes");
    let files = rust_files(&root);

    assert!(!files.is_empty(), "no view files found");

    let mut violations = Vec::new();

    for file in files {
        let source = std::fs::read_to_string(&file).expect("failed to read view file");

        for (line_index, line) in source.lines().enumerate() {
            let code = line.split("//").next().unwrap_or_default();

            for pattern in FORBIDDEN {
                if code.contains(pattern) {
                    violations.push(format!(
                        "{}:{}: `{pattern}` is forbidden in views",
                        file.display(),
                        line_index + 1,
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "raw styling found in views:\n{}",
        violations.join("\n")
    );
}

#[test]
fn style_ext_is_implemented_for_iced_widgets() {
    fn implemented<W: sofaia::style::StyleExt>() {}

    implemented::<iced::widget::Container<'static, ()>>();
    implemented::<iced::widget::Text<'static>>();
    implemented::<iced::widget::Rule<'static>>();
    implemented::<iced::widget::Button<'static, ()>>();
}

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    collect(dir, &mut found);
    found.sort();
    found
}

fn collect(dir: &Path, found: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            collect(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
}
