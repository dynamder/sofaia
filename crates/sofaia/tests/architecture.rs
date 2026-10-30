//! The rules that are cheaper to check by reading the source than by hoping.
//!
//! Every rule here exists because it was, at some point, the thing that broke:
//! a raw colour in a view, a `mouse_area` wired by hand, a `.styled()` call on
//! a widget the library does not paint.

use std::path::{Path, PathBuf};

/// Inside `src/panes`: how a widget is wired must go through the library, not
/// through iced's builder.
///
/// `.on_press(` and friends are forbidden because `AimExt::aimed` exists: wired
/// by hand, `on_press` and `on_double_click` can name different targets, and
/// that bug is invisible until a double click opens the wrong row.
///
/// `mouse_area` itself is *not* forbidden: the listing's hover boundary
/// (`mouse_area(rows).on_exit(..)`) is a bare area on purpose — it has no
/// target, and `ericed` has no opinion about it.
const FORBIDDEN: &[&str] = &[
    // raw styling
    ".style(",
    "Color::",
    "Border",
    "Padding",
    "Shadow",
    "iced::widget::button::Style",
    "iced::widget::container::Style",
    // raw pointer wiring
    ".on_press(",
    ".on_double_click(",
    ".on_right_press(",
];

#[test]
fn views_do_not_contain_raw_styling_or_raw_pointer_wiring() {
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
        "raw styling or pointer wiring found in views:\n{}",
        violations.join("\n")
    );
}

#[test]
fn style_ext_is_implemented_for_every_widget_the_library_paints() {
    use ericed::style::StyleExt;
    use sofaia::style::{button, container, rule, text};

    // `'a` is the widget's own lifetime, which is also the lifetime of a
    // `styled_with` closure: a palette derived per frame may borrow from the
    // view that is being built.
    fn implemented<'a, W: StyleExt<'a, K>, K>() {}

    implemented::<iced::widget::Container<'static, ()>, container::Kind>();
    implemented::<iced::widget::Text<'static>, text::Kind>();
    implemented::<iced::widget::Rule<'static>, rule::Kind>();
    implemented::<iced::widget::Button<'static, ()>, button::Kind>();
}

#[test]
fn the_library_does_not_depend_on_the_application() {
    // The rule that keeps `ericed` reusable: it may not mention the app or the
    // app's vocabulary. (`sofaia` is checked from this side only — a dependency
    // in the other direction would be a cycle anyway.)
    let ericed = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates directory")
        .join("ericed/src");

    let mut violations = Vec::new();

    for file in rust_files(&ericed) {
        let source = std::fs::read_to_string(&file).expect("failed to read library file");

        for (line_index, line) in source.lines().enumerate() {
            let code = line.split("//").next().unwrap_or_default();

            // Code only: the module docs deliberately *name* what the library
            // leaves out, and that prose is the point.
            for pattern in ["sofaia", "crate::intent", "crate::gesture", "crate::panes"] {
                if code.contains(pattern) {
                    violations.push(format!(
                        "{}:{}: `{pattern}` is application vocabulary",
                        file.display(),
                        line_index + 1,
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "the library knows about the application:\n{}",
        violations.join("\n")
    );
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
