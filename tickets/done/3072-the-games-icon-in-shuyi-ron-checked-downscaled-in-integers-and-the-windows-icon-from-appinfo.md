---
id: "3072"
title: "The game's icon in shuyi.ron: one square PNG checked by check --target, downscaled in integers by app-info, and the window's icon from AppInfo"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: done
blocked_by: ["3048"]
nick_input: none
completed: 2026-10-30
---

# 3072 — The game's icon in shuyi.ron, and the window's icon from AppInfo

## Context

3048 put who a game is in `shuyi.ron`'s `app` part and left out the
icon, as its own *Size* clause said to when its PR passed about 600
lines. This ticket is that part, as 3048's scope wrote it.

What 3048 left (read on 2026-10-30):

- `AppPart` (`crates/cli/src/project.rs`): `name`, `id`, `version`,
  `build`, `publisher`, `storage` and `code`, with `deny_unknown_fields`,
  `#[non_exhaustive]`. No `icon`, no `window_icons`.
- `crates/cli/src/app_info.rs`: the rules (`problems`, `AppProblem`),
  `rust(part, command)` (the generated file: `AppInfo::new(name, id,
  version)` then a setter for each field off its default, in the order
  storage, build, publisher), `files`, `stale`, `run` and `parse_args`.
  `check --target` reports the problems as `TargetCheck::App`.
- `shuyi_platform::AppInfo` (`crates/platform/src/app_info.rs`):
  `#[non_exhaustive]`, `name`, `id`, `version`, `publisher`, `storage`,
  `build`, a `const fn new` and three `const fn` setters. No window
  icon.
- `shuyi_app::Icon` (`crates/app/src/lib.rs`): `#[non_exhaustive]`,
  `Icon::new(small, medium, big)`, PNG files of 16, 32 and 64 px.
  `Window::with_icon(self, icon: Option<Icon>)`.
- `run_game` (`crates/app/src/lib.rs`): `basic_window` titles the window
  with `game.name()` and gives it no icon; `basic_config` names the
  storage `game.storage_name()`.
- The template (`templates/shuyi-default/base/shuyi.ron`) and hello-grid
  have an `app` part with no icon.

3049 (the identity in every package) uses `downscale` for every
package's size, so it waits for this ticket.

## Nick input

None.

## Scope

**In:**
- **`AppPart`** gains:
  - `icon: Option<String>`: one square PNG, at least 512 px a side;
  - `window_icons: Option<String>`: the folder where `app-info` writes
    the window's 16, 32 and 64 px PNG files.
- **The rules** (`app_info::problems`):
  - `icon`: the file exists, decodes as PNG (`codec::decode_png`, or
    `shuyi_assets::bitmap::convert::png_pixels` if 2565 is done), is
    square, and is at least 512 px a side (error, `app.icon`);
  - `icon` and `code` set without `window_icons` (error).
- **`app_info::downscale(w, h, rgba, size) -> Vec<u8>`**, in integers:
  - each output pixel averages the source pixels of its square, whose
    bounds are `x * w / size` to `(x + 1) * w / size` (the same for `y`);
  - in premultiplied alpha, rounded half up, so a transparent edge
    doesn't darken the colour beside it;
  - the same input gives the same bytes everywhere; it runs only in the
    tool, never at run time.
- **`app-info`** with `icon`: writes `<window_icons>/16.png`, `32.png`
  and `64.png`, downscaled, with `codec::encode_png`; `files` lists them
  so `stale` and `--check` cover them; the generated Rust gets
  `.with_window_icon(Some(WindowIcon::new(include_bytes!(..), ..)))`
  last, its paths relative to the Rust file, with `/`.
- **`shuyi_platform::WindowIcon { small, medium, big: &'static [u8] }`**
  moves here from `shuyi_app::Icon`: `#[non_exhaustive]`, `pub const fn
  new(small, medium, big)`. `shuyi-app` keeps `pub use
  shuyi_platform::WindowIcon as Icon;`, so `shuyi_app::Icon`,
  `Icon::new`, `Window::icon` and `Window::with_icon` don't change.
- **`AppInfo`** gains `window_icon: Option<WindowIcon>` (`None` in
  `new`) and `#[must_use] pub const fn with_window_icon(self, icon:
  Option<WindowIcon>)`.
- **`run_game`**'s window has `game.app().window_icon` (`with_icon`), with
  a test.
- **The template's `shuyi.ron`**: `window_icons: "crates/game/icon"`, and
  the icon commented out (`// icon: "art/icon.png", // square, 512 px or
  more`). `crates/game/icon` is outside `files/`, so the icons aren't
  embedded twice.
- **An ADR** that extends ADR-0059 (accepted, so not edited): one
  source picture, downscaled in integers by the tool, never at run
  time, and `WindowIcon`'s move into `shuyi-platform`.
- **Docs:** `docs/contract.md`'s `check --target` row and the book's
  chapter 9 (*Who the game is*).

**Out (do not do):**
- the icon in each package (the exe, `.icns`, `.desktop`, web icons):
  3049;
- an icon for hello-grid or the template: an icon is art;
- icons drawn per size: added when a target needs them.

## Implementation steps

1. Read `crates/cli/src/app_info.rs`, `codec.rs`, `crates/app/src/lib.rs`
   and `crates/platform/src/app_info.rs` as 3048 left them, and ADR-0059.
2. Write `downscale` and its tests.
3. Add the fields, their rules and the files `app-info` writes.
4. Move `Icon` into `shuyi-platform` as `WindowIcon`, add it to
   `AppInfo`, and give `run_game`'s window the icon. Run the no-std
   build of `shuyi-platform`.
5. Change the template, the ADR and the docs, and run the gates.

## Acceptance criteria

- [x] `cargo shuyi app-info` on a fixture with a 512 px icon writes the
      Rust file and three PNG files of 16, 32 and 64 px. Running it twice
      gives the same bytes; `--check` then exits 0, and after a change
      to the icon it exits 1, naming the file.
- [x] `downscale` of a picture of one colour gives that colour; of a
      2 × 2 checkerboard to 1 × 1, the rounded mean; a fully transparent
      pixel adds nothing to its neighbours' colour.
- [x] Each icon rule has a test that breaks it and gets its error,
      naming `app.icon` or `app.window_icons`.
- [x] `WindowIcon` is `#[non_exhaustive]` with a `const fn new`;
      `shuyi_app::Icon` is the same type.
- [x] `run_game`'s window has `APP.window_icon`.
- [x] A game made by `shuyi new` still passes `app-info --check` and
      `check --target all`.
- [x] `shuyi-platform` still builds for `thumbv7em-none-eabihf`.
- [x] The book's chapter is updated, or the completion notes have a
      `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `downscale`; each rule; the generated text with an icon (a
  snapshot); `--check` on the icons; `AppInfo::with_window_icon`;
  `run_game`'s window with an icon.
- Integration: `new` from both short names, then `app-info --check`.

## Completion notes

**Done.**

- **`AppPart`** has `icon` and `window_icons`.
- **The rules** (`app_info::problems`, so `check --target`):
  - `icon` reads, decodes as PNG (`codec::decode_png`; 2565 isn't done),
    is square and is 512 px a side or more (`app.icon`);
  - `icon` and `code` set without `window_icons` is an error
    (`app.window_icons`);
  - a stale window icon is a problem `app.window_icons`, and a stale
    Rust file stays `app.code`.
- **New items in `app_info`:** `app_info::icon(part, root)` reads and
  checks the icon (`Icon { side, rgba }`), for 3049 too.
  `ICON_SIDE` and `WINDOW_ICON_SIDES` are the sizes.
- **`downscale`** is as the scope says: in `u64`, each output square
  never empty (`to.max(from + 1)`), premultiplied, rounded half up.
- **`app-info`:** `files(part, root, command)` lists the Rust file, then
  `<window_icons>/16.png`, `32.png` and `64.png`, so `stale` and
  `--check` cover them. A PNG file is compared byte for byte; only the
  Rust file ignores line endings. When the icon doesn't load, `stale`
  compares the Rust file alone and the icon's rule says why. With an
  icon and `window_icons` but no `code`, it still writes the PNG files.
  The generated Rust gets `.with_window_icon(Some(WindowIcon::new(
  include_bytes!(..), ..)))` last, with its paths relative to the Rust
  file (snapshot `rust_window_icon`).
- **`shuyi_platform::WindowIcon`** (re-exported at the crate's root) and
  `AppInfo::window_icon` / `with_window_icon`. `shuyi_app::Icon` is
  `pub use shuyi_platform::WindowIcon as Icon`. `run_game`'s window
  takes `game.app().window_icon`, with a test.
- **The template:** `window_icons: "crates/game/icon"` and the icon
  commented out, with a line in its `AGENTS.md`. The `new` test makes a
  game from both short names, then runs `app-info --check`; it then
  adds an icon and writes and checks the window's icons.
- **ADR-0061.**
- **Docs:** `docs/contract.md` (the `check --target` row and the
  template's paragraph), the book's chapter 9 and its snippet
  `project/shuyi.ron`, and the `shuyi-ship` skill.

**Deviations:**

- Hello, Grid's own `WINDOW` (its long form, not `run_game`) also takes
  `.with_icon(APP.window_icon)`, so 3044, which rewrites that window,
  keeps it. Hello, Grid has no icon, so it is `None`.
- The book's snippet sets `icon` and `window_icons`, so its test expects
  `app.icon` (the snippets have no picture) beside `app.id` and
  `app.code`.

**Follow-ups:** none new. 2565's clause now says `app_info::icon` moves
onto `png_pixels`, and 3049 and 3044 have *From 3072 (done)* lines.

Breaking: shuyi-app: Icon is now shuyi_platform::WindowIcon, re-exported under the same name: nothing changes in a game, since `shuyi_app::Icon`, `Icon::new`, `Window::icon` and `Window::with_icon` are the same

```migration
since: 0.1.0
what: shuyi_app::Icon is now a re-export of shuyi_platform::WindowIcon, the window's icon that AppInfo carries; its fields and constructor are the same.
old: "shuyi_app::Icon (a struct of shuyi-app)"
new: "shuyi_app::Icon = shuyi_platform::WindowIcon"
before: |
  const WINDOW: Window = Window::new("A game", 640, 480).with_icon(Some(Icon::new(SMALL, MEDIUM, BIG)));
after: |
  const WINDOW: Window = Window::new("A game", 640, 480).with_icon(Some(Icon::new(SMALL, MEDIUM, BIG)));
```

Breaking: shuyi-cli: app_info::files takes the project's root: a caller passes it, and gets a Result, since the window's icons are read from the icon

```migration
since: 0.1.0
what: shuyi_cli::app_info::files reads the icon from the project's root, and fails when it doesn't load.
old: "app_info::files(part: &AppPart, command: &str) -> Vec<(String, Vec<u8>)>"
new: "app_info::files(part: &AppPart, root: &Path, command: &str) -> Result<Vec<(String, Vec<u8>)>, String>"
before: |
  let files = app_info::files(&part, "shuyi");
after: |
  let files = app_info::files(&part, &root, "shuyi")?;
find: ["app_info::files(&part, \"shuyi\")"]
```
