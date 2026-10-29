---
id: "2018"
title: "The real window checked in CI: a capture mode in shuyi-app, a render card, and a job under Xvfb and llvmpipe that compares the window with the raster"
type: infra
milestone: After MVP
model: opus-5.5
effort: high
status: done
blocked_by: ["2332"]
nick_input: none
completed: 2026-10-29
---

# 2018 — The real window checked in CI

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Bevy's `example-run.yml` runs its examples on Linux
under `xvfb-run` with Mesa, saves screenshots at set frames
(`CI_TESTING_CONFIG`) and compares them with `main`'s through Pixel
Eagle
(https://github.com/bevyengine/bevy/blob/main/.github/workflows/example-run.yml).
Unreal compares screenshots with ground truth within a tolerance.

Shuyi today:

- `shuyi-app`'s renderer (`crates/app/src/render.rs`) is what players
  see. No test draws with it: `shuyi-app`'s tests cover its audio,
  keys, pads, storage and window settings, and the `coverage` job
  leaves it out (`--exclude shuyi-app` in `.github/workflows/ci.yml`).
- CI installs `libgl1-mesa-dev` only to build. No job opens a window.
- The raster (`shuyi_ui::frame::raster`, 2332) is what `frame-png`,
  the tool contract and MCP (1133) show. `frame_png.rs` calls
  `frame-png` "never a gate".
- Tickets promise the same pixels. 2703 and 2323 (parked) are blocked
  by this one. 1120 (parked), 2222, 2301, 2335, 2336 and 2337 ask this
  ticket to compare their scenes if they land first.

A renderer bug, a macroquad upgrade or a driver change can make the
window differ from the raster while every gate stays green. Agents
would then judge a game by a picture players never see.

ADR-0011 §5 already says GPU output is compared with the software
reference "only by eye or with a tolerance", and the exact gates stay
on the software picture and the snapshot text. This ticket automates
the tolerance check for 2D. These are tests of a look, which may read
pixels (`AGENTS.md` rule 9).

Why it matters:

- **The game first.** Visions of Shuyi ships the desktop build, and its
  tests and the agents working on it read the raster.
- **Other 2D games** on `shuyi-app` get the same net. The parked
  rewrites need it before they start: 3034 (replacing macroquad), 3104
  (the GPU 3D painter), 2326 (effects on wgpu).
- **The end goal** (2026-10-19): what an agent sees of its game must
  be what players see.

## Nick input

None.

## Scope

**In:**
- **An ADR** (the `write-adr` skill):
  - the raster (2332) is the reference picture of a frame; every
    painter (`shuyi-app` now; wgpu, 3D and a console's later) is
    compared with it;
  - real backends run in CI on software GL: Mesa's llvmpipe under Xvfb
    for the desktop, headless Chromium for the web (2019); no GPU
    runners;
  - the comparison is `raster::compare` with a `Tolerance` the plan
    names; the default and the numbers measured on llvmpipe are
    recorded in the ADR;
  - the capture mode is a cargo feature of a backend, never in a
    shipped build. A plan is a test input of the backend, not an asset
    kind (ADR-0007 is about what a game loads);
  - `frame-png` stays a tool, never a gate; the exact gates stay on
    snapshot text and the raster's own pictures (ADR-0011 §5 holds for
    2D too);
  - a new frame item or paint goes on the render card (below) in the PR
    that adds it.
- **Plans, in `shuyi-platform`** (`crates/platform/src/capture.rs`):
  - behind a new `capture` feature that turns on `shuyi-ui/raster` and
    optional `serde` and `ron` (workspace dependencies). It is
    `no_std`, and CI's `no-std` job builds it so (`cargo build -p
    shuyi-platform --no-default-features --features capture --target
    thumbv7em-none-eabihf --locked`). The crate's tests turn it on
    through its dev-dependency on itself, as they do `harness`, so
    coverage measures it. `proptest` joins its dev-dependencies;
  - `Plan { steps: Vec<Step>, tolerance: Tolerance }`, read from RON by
    `Plan::parse(&str) -> Result<Plan, String>`; `tolerance` defaults
    to `Tolerance::default()`;
  - `Step` has the names and meaning of `frame-png`'s `Step`
    (`crates/cli/src/frame_png.rs`, `rename_all = "kebab-case"`) and
    one more: `keys("Enter Up")`, `pad("South+East")`, `wait(0.5)`,
    `capture("title")`. `parse` checks key and button names with
    `Chord::parse` and `Button::parse`, so a typo is an error before
    the first frame runs. A plan that starts with `capture` (or with
    only waits of 0 before its first `capture`), repeats a
    capture's name, or names one with anything but `a`-`z`, `0`-`9`,
    `-` and `_` (names become file names) doesn't parse either. Every
    error names the step by its number and its text;
  - `capture::FRAME_DT` (1/60 s), equal to the Harness's `FRAME_DT` (a
    test keeps the two equal; the Harness is test-only);
  - `Plan::frames(&self) -> Vec<PlannedFrame>`, with `PlannedFrame {
    events: Vec<RawInputEvent>, capture: Option<String> }`: every frame
    of the run, each `FRAME_DT` long:
    - `keys`: per chord, a frame with `RawInputEvent::Down(chord)`,
      then one with `Up(chord.key)`, as `Harness::keys` does;
    - `pad`: buttons joined by `+` go down together in one frame
      (`PadDown(button, PadKind::default())`) and up together in the
      next (`PadUp(button)`), as `Harness::pad` does;
    - `wait`: whole frames with no events, the seconds at 60 a second
      rounded up, so a wait above 0 is at least one frame and a wait of
      0 is none (a product within a thousandth of a whole number counts
      as that number: 0.5 s is 30 frames). It is the one rule 2566
      writes as `script::wait_frames`, for the link and the Harness's
      scripts too;
    - `capture`: names the frame before it, the one on the screen.
- **The capture mode in `shuyi-app`** (`crates/app/src/capture.rs`,
  public behind a new `capture` feature: `shuyi-platform/capture`,
  `shuyi-ui/raster`, and an optional `image` dependency):
  - `run` (`crates/app/src/lib.rs`) reads `SHUYI_CAPTURE` on native:
    the path of a plan, read with `macroquad::file::load_string`. Unset,
    nothing changes. Set, the run follows the plan, then exits. Without
    the feature the variable is never read;
  - **exact frames:** `App::frame_after(&mut platform,
    capture::FRAME_DT)` from the first frame, never `App::frame`. The
    time, the fixed steps and the game's seed (`Setup::seed`, from the
    clock `App` keeps) are the same on every run;
  - **only the plan plays:** `poll_input` gives the plan's events. The
    keyboard, controllers (no `PadInput`, so no gilrs), the window's
    shortcuts and focus are not read; the close button still quits;
  - **nothing outside:** storage is a `shuyi_ui::storage::MemoryStorage`
    (no saves read or written), audio is `shuyi_platform::Silent` (no
    quad-snd), no voice manifest is read, and `Macroquad::start`
    doesn't run;
  - **a capture:** on a frame the plan names, `present` paints the
    raster from the same `buffer`, `background` and `look` it draws
    (`Show::Game`), with a `raster::Painter` made from the look and
    that `background`. It is made again when the background changes,
    since `Renderer::set_clear` follows it each frame. Its pictures are
    decoded as `render::upload` decodes textures
    (`Image::from_file_with_format`), so both start from the same
    pixels. The scale is the one `console::layout` gives the window.
    After the draw, `get_screen_data()` reads the window. macroquad
    returns rows bottom up (its `export_png` flips them,
    `macroquad-0.4.16/src/texture.rs`), so flip them, cut out the
    console at `layout`'s offset, and compare with `raster::compare`
    and the plan's tolerance;
  - **one path for the run and the example:** reading the window
    (flipped and cut out, through a pure helper the tests call) and
    writing a capture's files and its line of `capture.txt` are public
    functions of `shuyi_app::capture`, so `render_card` uses them too;
  - **what it writes**, under `SHUYI_CAPTURE_OUT` (default
    `target/capture`): `<name>.window.png` and `<name>.raster.png` for
    every capture, `<name>.diff.png` when one fails, and `capture.txt`
    with one line per capture: name, size, scale, pixels over the
    tolerance, the worst difference and the first such pixel. The
    pictures are encoded with the `image` crate macroquad already uses
    (0.24 with `png`, `default-features = false`; MIT OR Apache-2.0,
    allowed by `deny.toml`; no new crate in `Cargo.lock`);
  - **exit codes:** 0 when every capture passes; 1 when one fails; 2
    when the plan can't run (it doesn't parse, the game fails to load,
    or a capture falls on a frame that isn't the game's);
  - on wasm32 the feature builds and does nothing yet (2019 does the
    web).
- **The render card** (`shuyi_ui::frame::raster::card`, behind
  `raster`): `card::build() -> RenderCard`, with `RenderCard { buffer:
  GlyphBuffer, atlas: FontAtlasDef, atlas_image: Image, table:
  ImageTable, images: BTreeMap<ImageId, Image>, clear: Rgb }` and
  `RenderCard::painter(&self) -> raster::Painter`. One fixed frame of
  at most 320 × 240 pixels with every kind of thing the frame draws,
  and no game behind it:
  - cells: backgrounds in several colours, see-through cells, glyphs in
    several colours, and a glyph the atlas lacks (the fallback, in
    `MISSING_COLOR`);
  - `Item::Rect` and `Item::Sprite` in both `Layer`s; sprites in every
    `Paint` (`Image`, `Solid`, `Dimmed`, `Shaded`), at opacity 255 and
    128, at brightness 255 and 102, with `flip_x`, cut by their `clip`,
    and one whose image is missing (in `table`, not in `images`);
  - a backdrop at zoom 2, its scene offset by half a scene pixel;
  - its pictures made in code: the test card (`raster::test_card`,
    2332) and a small font atlas of boxes and bars (a `FontAtlasDef`
    and its `Image`). Only an `ImageTable` makes an `ImageId`, so
    `table` comes from `ImageTable::from_files` over a 24-byte PNG
    header per picture (the signature, `IHDR`, the width and the
    height: all that `png_size` reads). No PNG is encoded in
    `shuyi-ui`;
  - a test with a `match` on `Item`, `Paint` and `Layer` with no `_`
    arm, so a new variant doesn't compile until the card shows it;
  - if 2335, 2336 or 2337 is done, its items and paints are on the
    card; if 2301 is done, a line of wide and narrow glyphs is; if 2222
    is done, an atlas sprite and an inserted one are; if 1120 is done,
    its test scene's world layers are (two parallax layers of y-sorted
    sprites, and an inset viewport); if 2334 is done, a text run with
    an outline and a shadow is; if 2360 is done, a small tile-layer
    item is. If one isn't done, it adds its own when it lands. Their
    own clauses say the same.
- **`crates/app/examples/render_card.rs`** (`required-features =
  ["capture"]`): opens a window three times the card's size, so
  `layout` gives scale 3 (an odd scale). It draws the card with
  `render::Renderer` (made from `atlas`, the atlas picture, `clear`
  and `table`), captures it as above with the default tolerance (or the
  one step 7 sets), and exits with the same codes. The renderer reads
  files through a `fn`, so the card's pictures are encoded to PNG once
  with `image` and kept in a `std::sync::OnceLock`.
- **Hello, Grid:** a `capture` feature in
  `examples/hello-grid/Cargo.toml` (`capture = ["shuyi-app/capture"]`,
  not a default) and `examples/hello-grid/capture/frames.ron`, which
  waits for the load, then captures:
  - the title;
  - the stage in the glyph look, the piece picked up;
  - the stage in the Tiles look, switched on the options screen as a
    player does.
  Keys are the example's defaults (the `input-actions` skill: test
  scripts may press them).
- **The CI job `render`** in `ci.yml` (`ubuntu-latest`, `needs:
  changes`, on code changes only, with the other jobs' checkout
  (`persist-credentials: false`), `rustup show` and cache steps):
  - runs `bash scripts/setup-linux.sh` as the other Linux jobs do
    (from 2513), then installs
    `xvfb`, `libgl1-mesa-dri` and `mesa-utils` with `apt-get`; sets
    `LIBGL_ALWAYS_SOFTWARE=1`; prints `glxinfo -B` under `xvfb-run`, so
    the log names the renderer and Mesa's version;
  - runs `cargo test -p shuyi-app -p shuyi-platform --features
    shuyi-app/capture --locked`;
  - under `xvfb-run -a -s "-screen 0 1280x1024x24"`: runs the card
    example with `SHUYI_CAPTURE_OUT=target/capture/card`; runs
    hello-grid (`--features capture`) with
    `SHUYI_CAPTURE=examples/hello-grid/capture/frames.ron` twice, into
    `target/capture/hello-grid` and `target/capture/hello-grid-again`,
    and checks with `cmp` that the two runs' `.window.png` and
    `.raster.png` files are equal; writes a plan with an unknown key
    name to `$RUNNER_TEMP` and checks that hello-grid exits 2 on it;
  - writes each `capture.txt` into the step summary, and uploads
    `target/capture/` as an artifact with `if: always()`
    (`actions/upload-artifact`, pinned by SHA as the `wasm` job's is).
- **The `clippy` job** also lints with `--features
  shuyi-app/capture,hello-grid/capture`, so the capture code and the
  example are linted.
- **Joining `ci-result`:** before merging, re-run `render` three times
  on the PR. If every run passes with the same `capture.txt`, `render`
  joins `ci-result`'s `needs` in this PR. If not, it stays out, the PR
  says why, and a follow-up ticket makes it stable. A gate never starts
  flaky (`AGENTS.md` rule 6).
- **The docs:**
  - `docs/platform-backends.md`'s checklist gains a line: its pixels
    match the raster (a capture mode like `shuyi-app`'s, the render card
    and a game's plans, within the plan's tolerance), in CI where the
    device can run there;
  - `crates/app/src/lib.rs`'s module docs say how to run a plan on any
    desktop (`SHUYI_CAPTURE=… cargo run -p hello-grid --features
    capture`);
  - the `run-gates` skill says `render` runs in CI only (it needs Xvfb)
    and how to run it on Linux, and gets the `no-std` job's new line.
- **Either order:**
  - with 2333 (scale modes): if 2333 is done, the plans run at a whole
    scale (`ScaleMode::Integer`), and one more capture uses `Fit`: it
    reads the render target 2333 draws into (its texture's pixels,
    before they are stretched) and compares them with the raster at
    `draw_scale`; the stretched window is saved for a look only. If
    not, 2333 adds that (its own clause says so);
  - with 3042 (focus) and 3044 (the window remembers its size and mode,
    and the OS cursor hides when idle): if they are done, a capture run
    sends no focus events, neither reads nor writes the remembered
    window (3044's `window_conf` reads it before `run`, so it checks
    `SHUYI_CAPTURE` too) and leaves the cursor alone; if not, they keep
    the capture mode so;
  - with 3046 (crash reports): if it is done, the `render` job also
    runs hello-grid once without a plan, with `SHUYI_PANIC_AT_FRAME=3`
    and `XDG_DATA_HOME` in a temporary folder, under `xvfb-run`, and
    checks that one `crash-*.ron` appeared under it with 3046's fixed
    message and `frame: 3`, and that the exit code is 101; if not, 3046
    adds that step (its own clause says so);
  - **from 2521 (done)** (the live link): `crates/app/src/readback.rs`
    reads the window (`window_png`: `get_screen_data()`, its rows turned
    upright and made opaque by `upright`, PNG by `png` with `image`).
    The capture mode reuses that helper instead of writing it again, and
    builds its frames of raw input the way `shuyi_platform::link` plays
    key, button and wait steps (its private `plan`, `press_keys` and
    `press_pad`): move them where both use one copy (the module is
    behind its `link` feature today; `readback` behind `shuyi-app`'s
    `link`, so give it a cfg that both features turn on). With 2566
    done first, that copy is already `script::frames` (the clause
    below);
  - **Either order with 2566:** if 2566 is done, a plan's steps are
    `capture::PlanStep::Input(shuyi_platform::script::Step)` or
    `PlanStep::Capture(String)` (`#[non_exhaustive]`), with no
    `capture::Step` of their own. A capture is a mark between input
    steps, not one more `script::Step` the Harness, the link and the
    contract would each have to refuse, so this is the smaller change.
    The RON stays as above, with `act("Confirm")` too (how `PlanStep`
    reads it is the worker's choice; a test reads each kind);
    `Plan::parse` checks each input step with `Step::check`;
    `Plan::frames(&self, keymap) -> Result<Vec<PlannedFrame>, String>`
    builds on `script::frames`, a `Planned::Idle(n)` as `n` frames with
    no events. Nothing moves out of `link`. If not, build `capture::Step`
    and `Plan::frames` as written above, with the wait rule above, and
    2566 folds them into `script::Step` (its own clause says so);
  - the new public types follow ADR-0039;
  - 2703 and 2323 (blocked by this one) say in their own clauses how
    their scenes join the card or a plan; 1120, 2301 and 2222 are in
    the card's list above.

**Out (do not do):**
- the web build (2019);
- macOS and Windows runners with real GPUs;
- a hosted comparison service such as Pixel Eagle;
- wgpu (3034, 2326, 2331) and 3D (3104): they reuse this later;
- coverage of `shuyi-app` from the Xvfb job (a follow-up if wanted);
- launching 3004's packaged exports;
- the templates: a game opts in with its own `capture` feature and
  plans;
- `frame-png`'s `Step` and `Script`: 2566 moves them into
  `shuyi_platform::script` (the clause with 2566 above);
- reading a running game's window on request (2521, the live link);
- plans that play a session record (2029, which waits for this ticket,
  2518 and 2519).

## Implementation steps

1. Read 2332's `raster` module; `crates/app/src/lib.rs`, `backend.rs`
   and `render.rs`; `crates/platform/src/app.rs`, `harness.rs` and
   `headless.rs`; `frame_png.rs`'s `Step` (or `shuyi_platform::script`,
   from 2566); ADR-0011 §5, ADR-0024 and `docs/platform-backends.md`.
   Check which of 1120, 2222, 2301, 2333, 2335, 2336, 2337, 2513, 2566,
   3042, 3044 and 3046 are done.
2. Write the ADR.
3. Write `shuyi_platform::capture` and its tests, and add the `no-std`
   job's line.
4. Write the render card and its variant test in `shuyi-ui`.
5. Write the capture mode in `shuyi-app` and the example, and run them
   on your own desktop.
6. Add hello-grid's feature and plan.
7. Add the `render` job and the `clippy` change. Measure on the runner.
   If the window differs from the raster, the plans and the card name
   the smallest tolerance that passes: never above 3104's for 3D (2 %
   of a capture's pixels over 24 in a channel), and never so loose that
   the one-pixel break in the acceptance criteria passes. Record the
   numbers in the ADR.
8. Show the one-pixel break with a temporary commit, then revert it.
9. Re-run `render` three times and decide on `ci-result`.
10. Update the docs and run the gates.

Size: if the diff runs well past 600 changed lines, move the render
card (`raster::card`, its variant test and
`crates/app/examples/render_card.rs`) to a follow-up ticket, After MVP
and blocked by this one, and keep the capture, the capture mode,
hello-grid's plan and the `render` job here. The completion notes then
say that the clauses naming 2018's render card mean that ticket.

## Acceptance criteria

- [x] Under Xvfb on the CI runner, hello-grid with
      `SHUYI_CAPTURE=examples/hello-grid/capture/frames.ron` writes its
      three captures and `capture.txt`, and exits 0.
- [x] ~~The render card example exits 0 under Xvfb, and its `capture.txt`
      is in the step summary.~~ (not done: the card moved to 2392 by the
      Size step)
- [x] A broken window is caught: shown once in the PR with a temporary
      commit that draws `Item::Rect` one pixel to the right in
      `draw_items` (`crates/app/src/render.rs`), then reverted. The job
      fails and uploads a diff.
- [x] Two runs of the same plan write byte-identical `.window.png` and
      `.raster.png` files (`cmp` in the job).
- [x] `Plan::parse` of a plan with an unknown key name fails and names
      the step, and hello-grid exits 2 on that plan in the job.
- [x] Without the `capture` feature `shuyi-app` compiles no capture code
      (`mod capture` and the read of `SHUYI_CAPTURE` are behind
      `#[cfg(feature = "capture")]`), and hello-grid's default features
      don't include `capture`.
- [x] `cargo build -p shuyi-platform --no-default-features --features
      capture --target thumbv7em-none-eabihf --locked` passes, in CI's
      `no-std` job.
- [x] ~~The card holds every `Item`, `Paint` and `Layer` variant: the
      match with no `_` arm compiles, and a test counts them.~~ (not
      done: the card moved to 2392 by the Size step)
- [x] The window is read through one helper the link uses too
      (`crates/app/src/readback.rs`). If 3046 is done, the job's crash
      step passes.
- [x] The ADR records the tolerance and the measured numbers; `render`
      is in `ci-result`, or the PR says why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Plan::parse` (each step, the default tolerance, errors: an
  unknown key or button, a `capture` first, a repeated or bad name);
  `Plan::frames` (the events per step and their order, frame counts,
  waits of 0, 1/60, 0.5 and 0.51 s giving 0, 1, 30 and 31 frames, the
  capture mark, `capture::FRAME_DT` equal to the Harness's); the
  card's variant coverage; flipping rows and cutting out the console on
  a hand-made 4 × 3 picture (the pure helper).
- Property: for random plans, `Plan::frames` has as many frames as its
  steps add up to.
- Integration (CI): the `render` job.

## Completion notes

**Done (2026-10-29), PR Zafnok/shuyi-engine#171.**

- **ADR-0052** records the decision: the raster is the reference
  picture; real backends run in CI on software GL; the capture mode is a
  backend's cargo feature; a plan is a test input, not an asset kind;
  the tolerance and the measured numbers; a new item goes on the render
  card.
- **Plans** (`shuyi_platform::capture`, the `capture` feature, `no_std`,
  built so in CI's `no-std` job): `Plan::parse`, `Plan::steps`,
  `Plan::tolerance`, `Plan::frames`, `PlannedFrame`, `Step` (`keys`,
  `pad`, `wait`, `capture`), `FRAME_DT`, `MAX_WAIT` and `wait_frames`.
  `Plan`'s fields are private, so only `parse` makes one. `parse` also
  refuses two captures with no frame between them (they would name one
  frame), a case the ticket didn't list.
- **One copy of the presses** (from 2521): the link's private
  `press_keys`/`press_pad` now wrap `crates/platform/src/press.rs`
  (`press::keys`, `press::pad`, `press::buttons`, crate-private, behind
  `link` or `capture`), which `Plan::frames` uses too. 2566 wasn't done,
  so this is the "if not" form; 2566 folds them (its clause says so).
- **The capture mode** (`shuyi_app::capture`, the `capture` feature,
  native only; on wasm32 it builds and does nothing): it wraps the
  `Macroquad` backend in a `Platform` of its own (`Capturing`) that gives
  the plan's input, the close button alone, `MemoryStorage` and
  `Silent`, and captures in `present`. The window is read by
  `readback::window`, which the link's `window_png` now uses too, then
  turned upright by the link's `upright` and cut out (`cut_out`, the
  pure helper the tests call). Public pieces for the card: `window_layout`,
  `read_window`, `cut_out`, `Report`, `PASSED`, `FAILED`, `CANNOT_RUN`,
  `PLAN_VAR`, `OUT_VAR`, `DEFAULT_OUT`.
- **Hello, Grid**: a `capture` feature (`["window", "shuyi-app/capture"]`:
  the binary needs `window`, as `link` does) and
  `examples/hello-grid/capture/frames.ron` (title, stage in glyphs with
  the piece picked up, stage in tiles switched on the options screen).
  The Tiles stage draws `Item::Rect`s (the path and the cursor), so the
  one-pixel break shows on Hello, Grid alone.
- **CI**: the `render` job, and `clippy` with
  `--features shuyi-app/capture,hello-grid/capture`. The root
  `shuyi.ron`'s key lint lists `crates/platform/src/capture.rs` and
  `press.rs` among the files that may name keys (scripts of keys, like
  the link's).
- **Measured**: 0 pixels over the default tolerance in every capture, on
  llvmpipe 21.1.8 (Mesa 26.0.8) and on a Windows desktop GPU; the worst
  channel difference is 1 on llvmpipe, 0 on the desktop. No plan names a
  tolerance of its own.
- **The one-pixel break**: shown with a temporary commit and reverted
  (the PR says which runs).
- **Docs**: ADR-0052, `crates/app/src/lib.rs`'s module docs, the
  backend checklist in `docs/platform-backends.md`, the `run-gates`
  skill, and the book's *Testing* chapter (a game's own plan).

**Deviation: the render card moved to 2392.** The diff ran well past 600
changed lines without it (about 1 300), so as the Size step says,
`raster::card`, its variant test and `crates/app/examples/render_card.rs`
are ticket 2392 (After MVP, blocked by this one). The clauses that named
2018's render card now name 2392's: 1120, 2222, 2301, 2334, 2335, 2336,
2337, 2360 and 2391 have an "Either order with 2392" clause, and 2019,
2323, 2386 and 2703 are blocked by 2392. The acceptance criteria on the
card (its example exits 0, every variant on it) are 2392's.

**Either-order clauses taken:** 2333, 3042, 3044, 3046 and 2566 weren't
done, so each "if not" branch holds: their tickets now say "From 2018
(done)" with what they add (2333 the `Fit` capture, 3042 and 3044 keep
the capture mode free of focus, the remembered window and the cursor,
3046 its crash step in `render`, 2566 the fold into `script::Step`).

No break: every public item is new (`shuyi_platform::capture`,
`shuyi_app::capture`), behind features that are off by default.
