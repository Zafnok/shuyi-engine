# ADR-0060: A game's icon: one square picture downscaled in integers by the tool, and the window's icon in `AppInfo`

- **Status:** Accepted; extends ADR-0059
- **Date:** 2026-10-30
- **Related tickets:** 3072 (this), 3048 (the identity), 3049 (the
  identity in every package)

## Context

- **ADR-0059** put who a game is in `shuyi.ron`'s `app` and left the
  icon to this ADR: one source picture, downscaled by the tool, and the
  window's icons in `AppInfo`.
- **The sizes a game needs** differ by target: the window's 16, 32 and
  64 pixel pictures (macroquad's `Icon`, miniquad's `conf::Icon`), the
  Windows executable's icon, macOS's `.icns` up to 1024 pixels, the
  Linux `.desktop` icon, the web's favicon and app icons (3049). A
  game drawing each one by hand is work no small game wants, and a
  hand-kept set drifts from the source.
- **Other engines** take one big picture and scale it at export: Godot
  (`application/config/icon`, scaled for each export), Unity (the
  Player settings' default icon, scaled for each size it lacks), Defold
  (one icon per platform field, from which the bundler makes the rest).
- **Where it is scaled.** At run time the binary would carry the big
  picture and a decoder and scaler, on every target, for pictures that
  never change. In a build script it would rerun in every game's build
  (ADR-0059's reason against build scripts, 2009's budget). The tool
  already writes `code` and checks it with `--check`.
- **Floats** give different bytes on different machines once a
  filter's sums round differently, and a generated file must be the
  same everywhere, or `--check` fails on one runner and not another.
- **`shuyi_app::Icon`** held the window's three PNG files, but
  `AppInfo` is in `shuyi-platform`, which `shuyi-app` depends on, not
  the other way round, and a playdate or console backend has no
  macroquad.

## Decision

1. **One picture.** `shuyi.ron`'s `app.icon` names one square PNG,
   512 pixels a side or more. `shuyi_cli::app_info::problems` checks
   that it reads, decodes as PNG, is square and is big enough (each
   problem `app.icon`, an error), so `check --target` reports it on
   every target. Every size a target asks is made from it; none is
   ever scaled up. Pictures drawn per size wait for a target that
   needs them.
2. **Downscaled in integers by the tool, never at run time.**
   `app_info::downscale(width, height, rgba, size)` makes a `size` ×
   `size` picture: each pixel out is the mean of the pixels of its
   square in, from `x * width / size` to `(x + 1) * width / size` (the
   same down), in premultiplied alpha, so a transparent pixel adds
   nothing to the color beside it, each mean rounded half up, all in
   `u64`. The same picture gives the same bytes on every machine.
   `codec::encode_png` writes the result. 3049 uses `downscale` for
   every package's size.
3. **The window's icons are generated files.** `app.window_icons`
   names a folder; with an icon, `app-info` writes `16.png`, `32.png`
   and `64.png` there, and `files` lists them with `code`, so `stale`,
   `--check` and `check --target` cover them (a stale PNG file is a
   problem `app.window_icons`, compared byte for byte, never by line
   endings). The generated Rust gets `.with_window_icon(Some(
   WindowIcon::new(include_bytes!(..), ..)))` last, its paths relative
   to the Rust file, with `/`. An icon and `code` without
   `window_icons` is an error (`app.window_icons`), since the Rust
   would have nothing to include. A new game's `window_icons` is
   `crates/game/icon`, outside `files/`, so the pictures aren't
   embedded twice; its `icon` is commented out, since an icon is art
   the game's author makes.
4. **`WindowIcon` is `shuyi-platform`'s.** `shuyi_platform::WindowIcon
   { small, medium, big: &'static [u8] }`, `#[non_exhaustive]` with a
   `const fn new` (ADR-0039), replaces `shuyi_app::Icon`, which stays
   as `pub use shuyi_platform::WindowIcon as Icon`, so `Window::icon`
   and `Window::with_icon` don't change. `AppInfo` gains `window_icon:
   Option<WindowIcon>` (`None` in `new`) and `const fn
   with_window_icon`, and `run_game`'s window takes it.

## Consequences

- A game draws one picture; its window shows it on every desktop, and
  3049's packages follow from the same picture.
- The window's three PNG files are in the game's repository, beside
  `app_info.rs`, and change only when the icon does; `check --target`
  fails until `app-info` is run after a change to the icon.
- The box filter is plain: a picture with thin lines or text loses them
  at 16 pixels. A game that minds draws its icon for that, or waits for
  pictures drawn per size.
- `AppInfo` and `shuyi_app::Icon` are now one type: a game that wrote
  `shuyi_app::Icon` keeps compiling, and a backend other than
  macroquad reads the same icon.

## Alternatives considered

- **Scaling at run time** — every target carries the big picture, a PNG
  decoder and a scaler for pictures that never change.
- **A build script** — reruns in every build (ADR-0059, 2009), and
  needs the CLI's PNG code in every game.
- **Floats, or a library filter (Lanczos, the `image` crate's
  `resize`)** — sharper, but their bytes may differ between machines,
  which a checked generated file can't allow, and `image` is a large
  dependency for one function.
- **Straight alpha** — a transparent edge pixel's color (often black)
  darkens the edge's mean.
- **`WindowIcon` staying in `shuyi-app`** — `AppInfo` couldn't carry
  it, and the generated file would name a backend's type.
