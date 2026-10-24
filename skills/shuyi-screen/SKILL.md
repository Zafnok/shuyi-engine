---
name: shuyi-screen
description: Add or change a screen of a game on Shuyi Engine — a screen is logic (update to a Transition), a view (plain data the tests read) and a skin (draw paints the view), with its text by key and its actions declared. Covers the Screen trait, the screen stack, overlays, a screen that answers the one below (PopWith), the engine's ready-made screens and widgets, and putting a screen on the game's list so `check --target` sees it. Use when adding a menu, a title, a question, a results screen or any other screen, or when a screen's text, look or controls change. Not for the keys a screen reacts to (shuyi-input), a turn-based grid stage (shuyi-stage-grid) or a screen's tests in depth (shuyi-test).
license: MIT OR Apache-2.0
metadata:
  version: "1.0"
---

# A screen: logic, a view, a skin

Every screen of a game on Shuyi Engine has the same shape. Its logic
reads the frame's actions and answers with a `Transition`. What it shows
is a **view**, plain data built from the game's state and text. Its
**skin** paints the view into the frame. Tests of what happened read the
view or the state; only tests of a look read cells and colours. So the
look can change, or be another device's, without touching the logic.

## When to use it

- A new screen: a menu, a question, a results or about screen, a pause
  over the stage.
- A screen's text, look, controls or flow changes.
- A screen needs to ask something and get the answer back.

## When not to

- Which keys and buttons give the actions: `shuyi-input`.
- A turn-based grid stage and its rules: `shuyi-stage-grid`.
- Writing the screen's tests in depth: `shuyi-test`. The checks before
  a commit: `shuyi-gates`.

## Steps

1. **Write the screen** in `crates/game/src/<name>.rs`: a type that
   implements `Screen<Data>` (`Data` is the game's data type in
   `lib.rs`), with a `view` function and its view type. The template's
   `crates/game/src/title.rs` is the model.
2. **Name it**: `pub mod <name>;` in `lib.rs`, and add it to `screens`
   there, the list of every screen the game can show. `cargo shuyi check
   --target` checks the controls of each screen on that list.
3. **Give it its text** in `files/en.ron`, by key. Never write player
   text in the code: `cargo shuyi check-text` and `lint` (`SH2102`) find
   it there, once `shuyi.ron` has a `check_text` part.
4. **Open it** from another screen: `Transition::Push(Box::new(MyScreen))`.
5. **Run it and look at it:**

   ```bash
   cargo shuyi run --script "Enter" --frames 1 --json
   cargo shuyi frame-png about.png --keys "Enter"
   ```

   `run` answers with the top screen, the screens on the stack and the
   frame as text. Look at the picture `frame-png` draws.
6. **Test it** through the Harness, reading its view, and run the gates
   (`shuyi-gates`).

## The Screen trait

```rust
impl Screen<Data> for About {
    // check-text: not player text, the screen's id
    fn name(&self) -> &'static str {
        "about"
    }

    /// Confirm and Cancel, which both go back.
    fn actions(&self) -> Option<Vec<UiAction>> {
        Some(vec![UiAction::Confirm, UiAction::Cancel])
    }

    fn update(&mut self, ctx: &mut Ctx<Data>, input: &FrameInput<UiAction>) -> Transition<Data> {
        if input.actions.contains(&UiAction::Cancel) {
            return Transition::Pop;
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx<Data>, buf: &mut GlyphBuffer) {
        let view = Self::view(ctx);
        // paint the view
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}
```

- `name`: a stable, unique `snake_case` id, for tests, `run`'s
  `top_screen` and debugging. It isn't player text: say so to
  `check-text` with the comment above it.
- `actions`: every action the screen reads (`shuyi-input`). The Harness
  hands a screen only these.
- `update`: called once a frame while the screen is on top. It answers
  with a `Transition`:

  | Transition | Does |
  | ---------- | ---- |
  | `None` | stays |
  | `Push(screen)` | puts a screen on top of this one |
  | `Pop` | closes this one: the one below is on top again |
  | `PopWith(result)`, `Transition::pop_with(value)` | closes it and hands `value` to the one below |
  | `Replace(screen)` | swaps this one for another |
  | `Quit` | leaves the game |

- `draw`: paints the screen. An opaque screen covers the whole buffer.
- `is_overlay`: `true` for a screen the one below shows through (a
  question, a menu over the stage): the stack draws the one below first.
- `as_any`: `Some(self)` lets a test find the screen on the stack
  (`h.screen::<About>()`).
- `covered`: called when a screen is pushed on top of this one.
- `uncovered(ctx, result)`: called when the one above pops; `result` is
  what it popped with, `None` after a plain `Pop`. The transition it
  returns is applied at once.

## The view and the skin

The view is plain data, built from the state and the text, and nothing
else:

```rust
/// What the about screen shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AboutView {
    pub heading: String,
    pub help: String,
}

impl About {
    pub fn view(ctx: &Ctx<Data>) -> AboutView {
        AboutView {
            heading: ctx.text("about.heading").to_owned(),
            help: ctx.text_with("about.help", &[]),
        }
    }
}
```

`ctx.text(key)` is the text of a key in the player's language;
`ctx.text_with(key, args)` fills its placeholders: each `{name}` that
`args` names (`&[("turn", &turn)]`), and each action's name (`{Cancel}`)
with the player's own key for it.

The skin paints the view into the `GlyphBuffer`, a console of glyph
cells: `fill_rect`, `draw_box`, `print` (it returns how many cells it
used), `set`, and the prelude's `print_centred`. Colours come from the
palette by role, never as numbers: `ctx.palette.engine(EngineColor::Text)`
(`Background`, `Panel`, `Border`, `Text`, `TextDim`, `Highlight`,
`Focus`, `Warning`, `Accent`, `Bright`), or `ctx.palette.lookup(name)`
for a name of the game's own `files/palette.ron`. A larger game puts its
painters in a module of their own (a `skin.rs`), each taking a view.

## A screen that answers the one below

Push the question, pop with the answer, and read it in `uncovered`:

```rust
// The question: Confirm says yes, Cancel no.
fn update(&mut self, _: &mut Ctx<Data>, input: &FrameInput<UiAction>) -> Transition<Data> {
    for &action in &input.actions {
        match action {
            UiAction::Confirm => return Transition::pop_with(true),
            UiAction::Cancel => return Transition::pop_with(false),
            _ => {}
        }
    }
    Transition::None
}

// The screen below it.
fn uncovered(&mut self, _: &mut Ctx<Data>, result: Option<ScreenResult>) -> Transition<Data> {
    if result.and_then(ScreenResult::take::<bool>) == Some(true) {
        Transition::Pop
    } else {
        Transition::None
    }
}
```

Any `'static` type will do as the answer: the two screens agree on it.

## The engine's own screens and widgets

Before writing one, look for it in the engine:

- **Screens** (`shuyi::ui::screens`): `OptionsScreen` (the player's
  settings, one per row), `KeyBindingsScreen` (rebinding),
  `LayoutPickerScreen` (the first-launch layout choice) and
  `ProfilesScreen` (the people who share a computer). Each takes the
  game's rows or legend.
- **Widgets** (`shuyi::ui::widgets`): `Menu` and its `MenuItem`s,
  `ListCursor` (a list's focus), `Ask` (a question over the screen),
  `NameEntry` (a name typed or spelled on a letter grid) and
  `word_wrap`. Each says what it shows as a view, and a skin paints it.
- The engine's screens react to `UiAction`s. A game with its own actions
  declares them for an engine screen with `ui_actions`
  (`shuyi::ui::input`).

## Tests

A screen's tests drive the game in the Harness and read the view:

```rust
let mut h = game::game().harness();
h.keys("Enter");
assert_eq!(h.screens(), ["title", "about"]);
let view = About::view(h.ctx());
assert!(h.snapshot().contains(&view.heading));
```

`h.snapshot()` is the frame as text; `h.top_screen()` the top screen's
name. When a change moves the flow, the tests that read the old flow
change with it.

## When it fails

Branch on the `code`, never the message (`cargo shuyi explain <code>`):

- `SH2102`: a screen's text is written in its code, or a screen reads a
  data name itself. Put the text in `files/en.ron` and ask for it by key.
- `SH2101`: the screen names a key (`shuyi-input`).
- `SH1002` from `check-text`: `shuyi.ron` has no `check_text` part. Add
  one: `dir`, `max_literals: 0`, `skip` (the binary's `main.rs`), `hint`
  and `name_hint`.
- `SH3005`: the game's tool program didn't build: fix the compiler's
  error the answer ends with.
- A screen not on the `screens` list isn't checked by `check --target`:
  its controls may not fit a target. A key the English text lacks panics
  in a debug build, naming the key.

## Worked example

[`example.ron`](example.ron) runs on a new `blank` game: it writes an
about screen with its view and its skin
([`example/about.rs`](example/about.rs)), names it in `lib.rs` and on the
list of every screen, has the title's Confirm push it, gives it its text
in `files/en.ron`, and turns on `check-text`; `run` then shows it on top
of the title with its text and the player's key, Cancel pops it, a
picture is drawn, its test reads the view
([`example/about_test.rs`](example/about_test.rs)), and `check --target
all`, `check-text`, `lint`, fmt, clippy and the tests pass.
