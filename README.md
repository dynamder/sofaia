# sofaia

sofaia is a gui file manager built on iced, using vim-like motion to move through
folders quickly. It is also the living example for [`ericed`](crates/ericed), the
small extension crate that came out of writing it.

```
sofaia/
├── crates/
│   ├── ericed/          # ERgonomic ICED: the reusable half
│   │   ├── examples/
│   │   │   └── aimed.rs     # the three-step integration, runnable
│   │   └── src/
│   │       ├── aim.rs       # Aim, Aimed<T>, AimExt — one target, one gesture
│   │       ├── meaning.rs   # Meaning<I> + interpret — what pointing means
│   │       ├── opaque.rs    # OpaqueExt           — a layer the ones below cannot see
│   │       ├── wheel.rs     # WheelDirection, WheelArea — the sign convention, and who claims it
│   │       ├── list.rs      # Scroll, Slice, Slots, Fixed, Measured — a list longer than the window
│   │       ├── motion.rs    # Curve, Track, frames, transform — a value that moves, its clock, and where it goes
│   │       └── style/       # StyleExt + Tokens + the recipe traits
│   └── sofaia/          # the application: the half that only means something here
│       ├── src/
│       │   ├── intent.rs    # Motion / Row / Choice / Intent — what the user asked for
│       │   ├── keys.rs      # Chord  ──► Intent
│       │   ├── gesture.rs   # wheel ──► Intent (the ambient half)
│       │   ├── menu.rs      # a simulated context menu, in a Stack, with an entrance
│       │   ├── panes/       # the listing, and where targets come from
│       │   └── style/       # the visual language: kinds and what they look like
│       └── tests/
└── target/
```

LeftPane is for folder context, MiddlePane is the main managing zone, RightPane
is for the entry under the cursor.

## Keys

| key | intent |
| --- | ------ |
| `j` | move the cursor down |
| `k` | move the cursor up |
| `h` | go to the parent folder |
| `l` | enter the folder under the cursor |
| `Escape` | close the context menu |

## Mouse

| gesture | intent |
| --- | --- |
| click a row | `Focus` — point at it |
| double click a row | `Open` — enter the folder under the pointer |
| right click a row | `Menu` — open the context menu for it |
| click a menu item | `Choose` |
| click anywhere else while a menu is open | `Dismiss` |
| wheel | `Move` — one row per notch |

Three things are worth noticing in that table.

First, a click *points* while a double click *opens*: `mouse_area` publishes the
press before the double click, so the cursor is already on the row by the time
`Open` runs, which is why `Open` needs no target and `l` produces exactly the same
intent.

Second, the same `Press` means three different things — focus a row, choose a menu
line, dismiss the scrim — and no central table knows that. Each component's
target type says what pointing at it means, through `ericed::Meaning`:

| target | `Aim::Press` | `Aim::DoubleClick` | `Aim::RightPress` |
| --- | --- | --- | --- |
| `Row` (listing) | `Focus` | `Open` | `Menu` |
| `menu::Target::Item` | `Choose` | — | — |
| `menu::Target::Scrim` | `Dismiss` | — | — |

Adding a component adds a row to that table *in its own file*, and the compiler
forces the whole row to be filled: the match is exhaustive over `Aim`, so a new
gesture cannot silently fall into a `_` arm.

Third, the context menu closes through an intent (`Dismiss`), not through event
forwarding — the scrim is an aimed target like any other.

## What moves, and what a moving value is made of

Two things move on their own, and both are a `Track` in the model:

| what | the value | where it goes |
| --- | --- | --- |
| the listing, when the folder changes | 0 → 1 over `SLIDE` | `motion::translate`, so the pane slides without being re-laid-out |
| the menu panel, when it opens | 0 → 1 over `ENTRANCE` | `Tokens::fade` for the scrim and the card, `motion::scale` for the card |

Neither is a special case in the framework: `update` advances the track when a
frame message arrives, `view` reads `track.value()`, and painting is a plain
function of that number. `App::is_moving` is the only place that has to know a
third animation exists — it is what decides whether the frame subscription is
installed at all.

## How input becomes an intent

```
Event::Keyboard ──► Chord ──┐
                            ├──► Intent ──► App::dispatch ──► panes / menu / Task
Event::Mouse ──► Scroll ────┤
layout ──► Aimed<T> ──► Meaning<T> ──┘
```

The keyboard grammar lives in `keys.rs`, the ambient half of the pointer in
`gesture.rs`, and both return an `Intent`. `App::dispatch` is the only place that
changes state, and intents are terminal, so the message stream cannot loop.

The mouse adds one stage the keyboard does not have: a click has to know *what it
hit*, and only the widget tree knows that. `ericed::AimExt` performs that hit test
where the layout exists, so what travels through the app is `Row(3)` — never a
coordinate. The payload types are narrow on purpose: `Intent::Focus` only accepts
a `Row` and `Intent::Choose` only a `Choice`, so an illegal combination is not
something to check — it is something that cannot be written.

## Testing

```sh
cargo test --workspace
```

The grammars, the architecture rules and the palette are tested without a window;
`tests/interaction.rs` and `tests/render.rs` need `iced_test` and build a widget
tree (the render test writes PNGs to `target/`).
