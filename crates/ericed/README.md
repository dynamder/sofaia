# ericed

**ERgonomic ICED.** The parts every iced application writes again: typed styles,
aimed mouse areas, per-component gesture meanings, opaque layers, and the wheel's
sign convention.

The rule this crate lives by: **nothing here knows anything about the
application using it.** No target type, no intent type, no pane names, no widths.

## Using it

```toml
[dependencies]
ericed = { path = "../ericed" }          # or git = "…"
```

```rust
use ericed::{Aim, AimExt, Aimed, Meaning};

// 1. Your target type says what pointing at it means.
impl Meaning<MyIntent> for MyTarget {
    fn meaning(self, aim: Aim) -> Option<MyIntent> {
        match aim {
            Aim::Press => Some(MyIntent::Focus(self)),
            Aim::DoubleClick => Some(MyIntent::Open),
            Aim::RightPress => Some(MyIntent::Menu(self)),
        }
    }
}

// 2. Your message type says it can carry an aimed gesture.
impl From<Aimed<MyTarget>> for MyMessage {
    fn from(aimed: Aimed<MyTarget>) -> Self { MyMessage::Aimed(aimed) }
}

// 3. Bring the trait into scope, and aim at the container that *is* the row.
let row = container(text("README.md")).aimed(MyTarget::Entry(3));
```

Both types are yours. Leaving `Aimed` and `Meaning` generic is what keeps the
crate application-agnostic, and it is why the orphan rule lets you write those
impls (your types, our traits).

`ericed::interpret(aimed)` is the only place that switches on the gesture, and it
does so generically — add a component, and no central table has to learn about
it.

A runnable example lives in [`examples/aimed.rs`](examples/aimed.rs).

## What it gives you

| item | the problem it solves |
| --- | --- |
| `StyleExt::styled(kind)` | `text(..).style(\|theme\| …)` says nothing about what the text *is* |
| `AimExt::aimed(target)` | `on_press` and `on_double_click` must name the same target, or a double click opens the row next to the one it focused |
| `Meaning<I>` | a gesture means different things on different components; the rule belongs on the target's type, not in one table |
| `OpaqueExt::opaque()` | a panel made of containers is a *filter*: `Interaction::None` does not block the layers below |
| `WheelDirection::read(delta)` | positive `y` means *up*, and nobody remembers |

## What it deliberately does not do

- **Policy.** Which gesture means which action, what `on_release` implies, how a
  row looks: those belong to the application.
- **Blanket `From` impls.** A blanket `impl<T, M: From<T>> From<Aimed<T>> for M`
  would make every application's own adapter impossible. The `From` space is
  left empty on purpose.

## Versions

`ericed` is coupled to iced's minor version: `ericed 0.1` ⇄ `iced 0.14`. Mixing
minors gives you two `iced_widget` copies, and then `ericed`'s `Container` is a
*different type* from yours.

## Before publishing

`Cargo.toml` still needs `license`, `repository`, and `publish` flipped on.
