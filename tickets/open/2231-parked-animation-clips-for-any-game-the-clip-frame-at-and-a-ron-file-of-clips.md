---
id: "2231"
title: "Parked: Animation clips for any game: AnimationClip, frame_at and a RON file of clips, split out of 1123"
type: feature
milestone: "Engine: real time"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: blocked
blocked_by: []
nick_input: none
completed:
---

# 2231 — Animation clips for any game

> **Parked** (Nick, 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): this is real-time work, so it waits until Visions of Shuyi launches, unless the game needs it first (a game ticket asks, through the game's `engine-change` skill, for animated terrain tiles or for unit idle and walk frames as data, which unparks this ticket and 2359 together; or the game's ticket 0060 says the army walks around freely, which unparks 1119–1125 and this with them). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engines GameMaker, Defold and Construct 3.
Each plays frame-by-frame sprite animation as a built-in:

- Godot's `AnimatedSprite2D` plays `SpriteFrames`: frames with a speed
  and a loop flag
  (https://docs.godotengine.org/en/stable/classes/class_animatedsprite2d.html).
- Unreal has Paper2D flipbooks. Unity has sprite animation clips.
- Defold's flipbooks play once or in a loop, forward, backward or
  ping-pong (https://defold.com/manuals/flipbook-animation/). Construct
  3's animations have a speed, a loop, a repeat count and ping-pong.
  GameMaker's sprites have frames and a speed.
- Bevy draws a sprite from a `TextureAtlas` index, and its sprite-sheet
  example times the frames with a `Timer` in the game's own code.

Shuyi today:

- ADR-0004 §3 defines the clip: frames (a rectangle and a duration in
  ms), a direction (forward, reverse, ping-pong) and a repeat rule
  (loop, once, n times). The frame shown is a pure function of the clip
  and the time since it started, as the cinematics already are (game
  ADR-0060, `crates/assets/src/cinematic.rs`).
- Nothing implements it: no crate names `AnimationClip`, `AnimationSet`
  or `frame_at`. 1123 planned the types, `frame_at`, a RON form and the
  Aseprite and TexturePacker importers, blocked by 1119 (the tick
  simulation). But `frame_at(clip, elapsed_ms)` needs no `Fx` and no
  step. A tick game only adds storing `(clip, start_step)` per thing,
  in whichever store it uses (the last **Out** line below). The block
  came from parking 1119–1125 together, not from the code.
- The only animation is hard-coded: `crates/ui/src/grid/walk.rs` has
  `IDLE_FRAMES`, `WALK_FRAMES`, `IDLE_STEP_MS` and `WALK_FRAME_S` for
  one sheet layout (three frames a row, a row per `Facing`).
- Other parked tickets need the clip, not the importers: particles
  (2103), state machines over clips (2104), a visual novel's mouths and
  blinks (2725), and the tile-layer painter (2360). 2359 puts clips in
  the grid kit.
- Every RON sidecar of the engine names the files it reads from its
  own folder: the font atlas reads the PNG beside it, and the audio
  manifest reads `sfx/a.wav` as `audio/sfx/a.wav` (`docs/formats.md`,
  *The built-in formats*). `shuyi import` reads a file's folder as its
  bundle (`folder_bundle` in `crates/cli/src/contract/import.rs`), so
  only a path from the file's folder is found there. A set of clips
  names its picture the same way.

This ticket takes the clip core out of 1123. 1123 keeps the Aseprite
and TexturePacker importers (into 2222's `Atlas`) and the `asefile`
measurement, and waits for this one.

Why it matters:

- **The game first.** Visions of Shuyi is turn-based, and its units step
  and walk as `walk.rs` hard-codes. Nothing in the game asks for clips
  today, so this stays parked with real time. It is the small piece the
  game can have the day it asks for moving water or unit frames as data,
  without the tick simulation.
- **Other 2D games.** Puzzles, card games and visual novels animate
  without a tick simulation: a coin that spins, a cursor that blinks, a
  character that blinks. Every benchmark engine gives them clips.
- **The end goal** (Nick, 2026-10-19): a game in few lines. A clip is a
  few lines of RON and one `frame_at` call.

The frame type is generic. It is a rectangle of a picture by default, a
tile id for a Tiled tileset (2360, over 1124's import), or a frame
number of a sheet for the grid kit's units (2359).

## Nick input

None.

## Scope

**In:**
- **`shuyi_assets::anim`** (`crates/assets/src/anim.rs`, tests in
  `crates/assets/src/anim/tests.rs` as `cinematic.rs` has them; `pub mod
  anim;` in `crates/assets/src/lib.rs`, named in the crate docs),
  `no_std` with `alloc` like the rest of the crate:
  - `ClipFrame<F> { pub show: F, pub ms: u32 }`;
  - `Direction` (`Forward`, the default; `Reverse`; `PingPong`);
  - `Repeat` (`Loop`, the default; `Once`; `Times(u16)`);
  - `AnimationClip<F = ImageRect> { pub frames: Vec<ClipFrame<F>>,
    pub direction: Direction, pub repeat: Repeat }`, deriving `Debug`,
    `Clone`, `PartialEq`, `Eq`, `Serialize` and `Deserialize`, with
    `#[serde(default)]` on `direction` and `repeat` so a file may leave
    them out. `Direction`, `Repeat` and `ClipFrame` derive the same,
    and `Direction` and `Repeat` derive `Copy` and `Default` too.
    `AnimationClip` and `ClipFrame` have `#[serde(deny_unknown_fields)]`,
    as the engine's other file types do (`crates/assets/src/audio.rs`,
    `credits.rs`), so a misspelt field is an error;
  - `AnimationClip::new(frames: Vec<ClipFrame<F>>) -> Self`: `Forward`
    and `Loop`, for code that builds a clip (2359's unit clips);
  - `AnimationClip::cycle_ms(&self) -> u64`: one pass of its order;
  - `AnimationClip::length_ms(&self) -> Option<u64>`: `None` for
    `Loop`, one cycle for `Once`, `n` cycles for `Times(n)`;
  - `AnimationClip::index_at(&self, elapsed_ms: u64) -> Option<usize>`:
    the index in `frames` shown, `None` only for a clip with no frame;
  - `AnimationClip::ended(&self, elapsed_ms: u64) -> bool`: true from
    `length_ms` on (never for `Loop`), for 2104's `ended`;
  - `pub fn frame_at<F>(clip: &AnimationClip<F>, elapsed_ms: u64) ->
    Option<&F>`, the name ADR-0004 §3 and 2104 use. `frame_at`'s docs say a tick
    game passes `shuyi_core::timer::ms_from_steps(step - start_step)`,
    the one rule from steps to ms.
- **The order**, written in the module docs with an example:
  - `Forward` shows frames `0..n`, `Reverse` shows `n - 1` down to `0`;
  - `PingPong` shows `0..n`, then `n - 2` down to `1`: a cycle of
    `2n - 2` frames (one frame when `n` is 1), so a loop never shows an
    end frame twice running;
  - after the end of `Once` or `Times(n)`: `Forward` holds the last
    frame, `Reverse` the first, `PingPong` the first (where its cycle
    comes back to);
  - a clip whose cycle is 0 ms (every frame 0 ms long, built in code;
    the format refuses it) shows the first frame of its order;
  - nothing panics or overflows for any `elapsed_ms` up to `u64::MAX`
    and any `ms` up to `u32::MAX`: sums are `u64`, saturating.
- **`AnimationSet`**, the asset kind (ADR-0019 §1):
  `AnimationSet { pub image: String, pub clips: BTreeMap<String,
  AnimationClip> }`, deriving `Debug`, `Clone`, `PartialEq`, `Eq`,
  `Default`, `Serialize` and `Deserialize`, with
  `#[serde(deny_unknown_fields)]` and `AssetKind::NAME = "animation
  set"` (no other kind has that name), and:
  - `AnimationSet::clip(&self, name: &str) -> Option<&AnimationClip>`;
  - `image` is the picture's path from the set file's folder, as
    written (`water.png` beside it, `sheets/water.png` below it), as the
    audio manifest names its sounds;
  - `AnimationSet::image_path(&self, dir: &str) -> String`:
    `format::join(dir, &self.image)`, the picture's bundle path, which
    `ImageTable::id` takes. `dir` is the set file's folder
    (`AssetPath::dir`): `"anim"` for `anim/water.ron`.
- **`AnimationSetRon`**, its format (ADR-0007, ADR-0019;
  `docs/formats.md` says how to write one), a unit struct deriving
  `Debug`, `Clone`, `Copy`, `Default`, `PartialEq` and `Eq`: id
  `animation-set`, extension `ron`, limit `TEXT_MAX_BYTES`, registered
  in `Formats::register_standard` (`crates/assets/src/format.rs`; the
  kind is new, so it clashes with no format). It reads the text with
  `format::text`, then with the crate's `ron::parse_ron`
  (`crates/assets/src/ron.rs`; syntax errors get a line and column),
  then checks:
  - at least one clip; no clip named `""`; at least one frame in each
    clip; every frame at least 1 ms; no `Times(0)`. Each message names
    the clip, in an error on the set file (`file.shown()`);
  - `image` is not empty, doesn't start with `/` and has no `..` part
    (an error on the set file);
  - the picture at `join(file.dir(), &image)` is in the bundle
    (`DecodeCx::bytes`). When it is missing the error is on the
    picture, `cx.shown(path)` with `format::NOT_FOUND`, as the font atlas
    reports its PNG (`crates/assets/src/font.rs`);
  - its size is read by `image::read_info` (`crates/assets/src/image.rs`;
    private today, make it `pub(crate)`), which reads a PNG's header or,
    when `Bitmap::is_bitmap` says a one-bit picture stands in the PNG's
    place, its source size. A bad picture is an error on the picture;
  - every frame's rectangle has `w` and `h` above 0 and lies inside the
    picture, with `x + w` and `y + h` summed as `u64`. The message names
    the clip and the frame's index.
  An example file, `anim/water.ron` beside `anim/water.png`:
  ```ron
  (
      image: "water.png",
      clips: {
          "waves": (frames: [
              (show: (x: 0, y: 0, w: 16, h: 16), ms: 250),
              (show: (x: 16, y: 0, w: 16, h: 16), ms: 250),
          ]),
          "splash": (frames: [
              (show: (x: 0, y: 16, w: 16, h: 16), ms: 100),
          ], repeat: Once),
      },
  )
  ```
- **`Encode` for `AnimationSetRon`**: the RON a person writes, through
  `ron`'s `to_string_pretty` with a `PrettyConfig` whose `new_line`
  is `"\n"`, as `crates/cli/src/font_atlas.rs` writes an atlas (its
  default is `"\r\n"` on Windows). It works without `std` (ron 0.12's
  writer needs only `alloc`). **Either order with 2218:** if 2218 is
  done, write it the way 2218's writers are written (the same
  `PrettyConfig`, and a shared helper if 2218 made one); if not, write
  it here, and 2218 may move it.
- **`ImageRect`** (`crates/assets/src/image.rs`) derives `Serialize` and
  `Deserialize` beside its `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`
  and `Hash`, with `#[serde(deny_unknown_fields)]`, so a frame is
  written `(x: 0, y: 0, w: 16, h: 16)`.
- **The pack** (ADR-0023): `AnimationSet` is bakeable with no code of
  its own (`pack::Bakeable` covers every serde kind). In
  `crates/assets/src/pack/tests.rs`:
  - `sources()` gains `anim/water.ron` (the fixture set) and
    `anim/water.png` (`png_header(32, 32)`). The picture is not under
    `images/`, which `every_kind()` bakes as `Image`, so it stays a raw
    file as the font atlas's PNG does;
  - `every_kind()` gains `.asset::<AnimationSet>("anim/water")`;
  - `the_binary_form_of_every_engine_kind_is_pinned_by_the_pack_version`
    gains `form!(AnimationSet, "anim/water.ron")`.
  The snapshot only gains the new kind's lines and no existing form
  changes, so `pack::VERSION` stays 2 (ADR-0023 §5 bumps it when a
  form changes); the PR says so.
- **`shuyi import`**: `Importers::standard()`
  (`crates/cli/src/contract/import.rs`) gains
  `kind_with_text::<AnimationSet>`, summed up with the file's `count`
  helper as `3 clips: splash, waves, wind`, with `--print` writing its
  RON through `Encode`.
- **Public types (ADR-0039):** the new types follow it, and the
  Scope's choices are written in the PR (whether `AnimationClip` and
  `AnimationSet` are `#[non_exhaustive]`, with `AnimationClip::new` as
  their constructor).
- **Docs** (`docs/formats.md`):
  - a row of *The built-in formats* table: `animation set`,
    `anim::AnimationSet`, `anim::AnimationSetRon`, `animation-set`,
    `ron`, none, 4 MiB;
  - a bullet beside the font atlas's: the animation set reads its
    picture from its own folder (`anim/water.ron` names `water.png`,
    the bundle's `anim/water.png`);
  - the table *The kinds of ADR-0007 §1*: `AnimationSet` is
    `anim::AnimationSet` (2231), `Atlas` is 2222's, and the importers
    are 1123's.
- **Either order with 1122:** the `movers` answer of the bodies concern
  in `docs/modules.ron` (2003 wrote it parked) is 1122's movers and
  these clips. If 1122 is done, set it to ready; if not, nothing to do
  (1122 sets it).
- **From 1134 (done):** its `shuyi-animate` skill
  (`skills/shuyi-animate/SKILL.md`) teaches clips: a set file beside
  its picture, `AnimationSet::clip` and `frame_at`, in place of its
  note that clips are to come. It drops `2231` from its `waits_for`
  (`check-skills` fails on an id not in `tickets/open/`) and gains its
  worked example, `skills/shuyi-animate/example.ron`, a script of
  contract operations that CI's `skills` job runs on a fresh `shuyi
  new` game as it runs the pack's others. Its section on particles
  stays as it is.
- **For the game:** no API break. Everything is new except two derives
  on `ImageRect` and a function made `pub(crate)`, and
  `Formats::standard()` gains a format of a new kind, which clashes
  with nothing. The PR says so.

**Out (do not do):**
- `Atlas` (2222), and the Aseprite and TexturePacker importers and
  `asefile` (1123);
- which clip plays when, direction sets and frame events (2104);
- animated tiles and unit frames in the grid kit (2359), and the
  tile-layer painter with Tiled's tile animations (2360, 1124);
- storing `(clip, start_step)` in a tick simulation: the game does, in
  whichever store it uses (the last line below);
- particles (2103); tweens and easing (the cinematics today, 2109);
  skeletal animation (2105);
- packing pictures into atlas pages (2222);
- the rest of the `shuyi-animate` skill: it is 1134's, and this
  ticket adds only its clips and its worked example (**In**);
- a shorthand for frames laid out in a grid of cells (a follow-up if
  files get long);
- a picture outside the set's folder (`..`): a follow-up if games need
  it;
- painting anything: a clip says which frame, and a skin paints it
  (`AGENTS.md` rule 9);
- code over a store: clips are plain data and `frame_at` is a pure
  function, naming neither `Arena` (1119) nor `shuyi-ecs`'s `World`
  (2904's ADR). A game keeps each thing's clip and start time in
  whichever store it uses.

## Implementation steps

1. Read ADR-0004 §2–§3, ADR-0019, ADR-0023 §5, `docs/formats.md`,
   `crates/assets/src/font.rs` (a sidecar that reads its picture),
   `crates/assets/src/audio/formats.rs` (`AudioManifestRon::decode`, a
   sidecar that joins names to its folder) and
   `crates/assets/src/format/builtin_tests.rs` (how the built-in formats
   are tested).
2. Write the types, the order, `new`, `cycle_ms`, `length_ms`,
   `index_at`, `ended` and `frame_at`, with the unit and property tests
   below.
3. Write `AnimationSet`, `image_path`, `AnimationSetRon` and its checks,
   and its `Encode`. Make `image::read_info` `pub(crate)`. Register the
   format. In `builtin_tests.rs`:
   - a `pub(crate) const ANIMATIONS: &str`: a set of three clips
     (`splash`, `waves`, `wind`) naming `water.png`;
   - `("anim/water.png", png_header(32, 32))` in `fixtures()`;
   - a bad set for each check;
   - the format in `every_built_in_has_its_id_extensions_and_limit`,
     `every_text_format_refuses_bytes_that_arent_utf8` and
     `no_bytes_panic_a_ron_sidecar` (decoded at `anim/water.ron`, with
     `edited(ANIMATIONS)`).
4. Add the kind to the pack's tests and to `Importers::standard()`, and
   add `"animation set"` to `the_standard_kinds_are_the_engines`
   (`crates/cli/src/contract/import/tests.rs`).
5. Point the tickets at this one, in this PR, if they still point at
   1123 for the clip: 1123's Scope drops `AnimationClip`, `AnimationSet`,
   `frame_at` and the clips' RON form, and its `blocked_by` gains
   `"2231"`; 2103, 2104 and 2725 name 2231 for `AnimationClip` and
   `frame_at` (their `blocked_by` swaps `"1123"` for `"2231"` where they
   need only the clip).
6. Write the docs, and run the gates.

## Acceptance criteria

- [ ] For random clips of 1 to 6 frames of 1 to 20 ms, in every
      direction and repeat, `frame_at` equals a reference in the test
      that lays the clip out one entry per millisecond, over three
      cycles and past the end (property test).
- [ ] A looping clip is periodic: `index_at(t) == index_at(t +
      cycle_ms)` (property test).
- [ ] A ping-pong loop of two frames or more never shows its first or
      last frame twice running: in the sequence of indices it shows, one
      entry per frame shown, no two neighbours are equal (property test).
- [ ] `Once` and `Times(n)` hold their end frame from `length_ms` on;
      `ended` is false before it and true from it.
- [ ] `index_at(u64::MAX)`, frames of 0 ms and of `u32::MAX` ms, and
      `Times(u16::MAX)` never panic.
- [ ] Each bad fixture gives its error: no clip, a clip named `""`, an
      empty clip, a 0 ms frame, `Times(0)` and a rectangle outside the
      picture each name the set file and the clip; an `image` with `..`
      names the set file; a missing picture names the picture. A RON
      syntax error and a misspelt field each give a line and column.
- [ ] `Formats::standard().load::<AnimationSet>(bundle, "anim/water")`
      reads the fixture, and `image_path("anim")` is `anim/water.png`.
- [ ] Random bytes and the fixture with a few characters changed never
      panic the format (`no_bytes_panic_a_ron_sidecar`).
- [ ] Decoding what `encode` writes gives the set encoded, for the
      fixture and for random valid sets over the fixture's picture
      (property test, decoded beside `fixtures()`).
- [ ] `every_built_in_has_its_id_extensions_and_limit` lists
      `animation-set`.
- [ ] The pack's pinned binary form includes `AnimationSet`, and
      `pack::VERSION` is unchanged.
- [ ] `shuyi import` of a set beside its picture in a scratch folder
      prints `animation set: 3 clips: splash, waves, wind`, and
      `--print` its RON with no "no writer" limit.
- [ ] CI's `no-std` job passes, including `cargo build -p shuyi-assets
      --no-default-features --target thumbv7em-none-eabihf --locked`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each direction and repeat on a three-frame clip at chosen
  times; a one-frame ping-pong; a clip with no frame; a 0 ms cycle;
  `length_ms` and `ended`; each format check; `AnimationSet::clip` and
  `image_path`.
- Property: the reference comparison, the period, ping-pong's ends,
  never panicking, and the encode round trip.
- Snapshot / integration: the pack's pinned form; `shuyi import` and
  `--print` on the fixture (`crates/cli/src/contract/import/tests.rs`,
  in a `scratch` folder).

## Completion notes

*(Filled in by the session that completes the ticket.)*
