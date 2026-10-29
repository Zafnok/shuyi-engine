---
id: "2366"
title: "Parked: Captions for sound effects and music, not speech: a caption key on each cue, a player setting, and a strip the engine paints over every screen"
type: feature
milestone: "Engine: accessibility"
model: sonnet-5
effort: high
status: blocked
blocked_by: []
nick_input: none
completed:
---

# 2366 — Captions for sound effects and music, not speech

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; on 2026-10-20 he kept parked the gaps the game doesn't need): the game doesn't ask for it, so this waits until after the MVP (`docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Unreal gives sounds subtitles, and its Subtitles and
Closed Captions plugin queues a caption when a sound plays. Bevy, Godot
and Unity have nothing built in. Microsoft's Xbox Accessibility
Guidelines (XAG 104, subtitles and captions) and the Game Accessibility
Guidelines ("ensure no essential information is conveyed by sounds
alone") both ask for captions of the sounds that matter.

What Shuyi has today (checked 2026-10-21):

- `SoundCue { files, volume, credit }` and `MusicCue { file, volume,
  looped, length_ms, credit }` (`crates/assets/src/audio.rs:61` and
  `:73`) carry no text. The manifest is read by `from_source`
  (`crates/assets/src/audio.rs:324`) with `parse_ron`, and its raw
  entries (`RawSound`, `RawMusic`) deny unknown fields. `Check::sound`
  and `Check::music` (`:430`, `:461`) build the cues.
- `MusicCue`s are built as struct literals in `crates/app/src/audio.rs`,
  `crates/assets/src/credits.rs`, `crates/assets/src/cinematic/tests.rs`,
  `crates/ui/src/audio.rs` (tests) and `crates/ui/src/toy.rs`.
  **From 2024 (done):** `SoundCue` is `#[non_exhaustive]`, and every
  caller outside `crates/assets` builds it with `SoundCue::new`; the
  only `SoundCue` literals left are `Check::sound`'s and the tests' in
  `crates/assets/src/audio.rs` and `crates/assets/src/credits.rs`.
- Screens ask for sounds through `AudioQueue` (`crates/ui/src/audio.rs`).
  `Game::collect_audio` (`crates/ui/src/game.rs:281`) takes the requests
  each frame and runs `MusicState`, which turns them into
  `MusicCommand`s. `Game::frame` (`crates/ui/src/game.rs:255`) calls it
  after `step`, which has already redrawn the buffer.
- The engine's own widgets play the `AudioConfig` cues (`ui_move`,
  `ui_select`, `ui_cancel`, `ui_cursor`).
- **Speech is already text.** A voice clip plays by dialogue line id, and
  the line shows in the text box. The visual novel backlog (2723) keeps
  speech history.
- **The word "caption" is taken twice.** The dialogue's `@caption` step
  is a location or time line, translated under
  `caption.<scene>_<hash>` (`caption_key`,
  `crates/assets/src/dialogue/line_id.rs:104`), and the debug camera
  view has a `caption` field (`crates/ui/src/debug/camera.rs:60`). So
  this ticket says **sound captions** in code and text, and its keys
  start with `cc.`.
- **Agents already see every cue.** `Harness::audio_log` and
  `Harness::sounds` (`crates/platform/src/harness.rs:345`, `:367`) and
  the `sounds` of `shuyi run` (`Sound`,
  `crates/cli/src/contract/session.rs:84`) list each one by cue id.
- `Settings` (`crates/ui/src/settings.rs`) has text speed, fullscreen,
  the theme, two volumes and the layout. Each game saves them in its own
  form (`GameSettings::to_ron`): the toy, hello-grid and both templates.
  `layout` is `pub(crate)`, so no game builds `Settings` as a struct
  literal.
- `GlyphBuffer::print` writes cells but leaves sprites and overlays over
  them. `GlyphBuffer::fill_rect` cuts the items over the cells it fills
  (`crates/ui/src/frame/glyph_buffer.rs:669`).
- Planned accessibility covers sight, motion and hands: 2322 (the
  accessibility tree and `Ctx::announce`), 2323 (text size), 2324
  (colour vision), 2404 (hold or toggle), 3035 (screen readers), all
  parked; 2340 (reduced motion and flashing) after the MVP. Nothing
  covers hearing.

Why it matters:

- **The game first.** No game ticket asks for sound captions, so this is
  parked. The game's PR that moves the pin needs only the small changes
  in *What changes for the game* below.
- **Other 2D games.** An alarm, a door opening behind the player, a foe
  off screen: a deaf or hard-of-hearing player misses whatever only a
  sound says. With text keys, language packs and `lang-status` already
  in place, a caption is one line on a cue.

## Nick input

None.

## Scope

**In:**
- **A caption key on a cue** (`crates/assets/src/audio.rs`):
  - `SoundCue::caption: Option<String>` and `MusicCue::caption:
    Option<String>`: the text key of the cue's sound caption, in the
    game's screen text (`en.ron`, a pack's `ui.ron`). `None` by default.
  - `RawSound` and `RawMusic` get `#[serde(default)] caption:
    Option<String>`, so old manifests read as none. `Check::sound` and
    `Check::music` copy it into the cue.
  - `from_source` reads the file with `parse_ron_implicit_some`
    (`crates/assets/src/ron.rs:96`, as `target.rs` does), so a cue writes
    `caption: "cc.door_opens"`. The manifest has no other `Option`
    field, so nothing else reads differently.
  - The manifest check rejects a caption key that doesn't start with
    `cc.` or has nothing after it, at the cue's line (`Check::err`).
  - Every struct literal of the two cues listed above gets
    `caption: None` (`grep -rn "SoundCue {\|MusicCue {"` finds them).
  - The pack's binary form of `AudioManifest` changes (postcard), so
    `pack::VERSION` (`crates/assets/src/pack.rs:62`) goes up by one and
    the snapshot of
    `the_binary_form_of_every_engine_kind_is_pinned_by_the_pack_version`
    (`crates/assets/src/pack/tests.rs:579`) is accepted again.
  - A line for the `caption` field in `docs/formats.md`'s audio manifest
    notes (line 119).
- **`shuyi_ui::sound_captions`** (`crates/ui/src/sound_captions.rs`,
  `no_std` with `alloc`, like the rest of `shuyi-ui`):
  - `SoundCaptions`, the log: the lines showing, each with its text key,
    a count and the seconds left.
    - `HOLD_SECS: f32 = 3.0` (*tunable*): how long a line stays.
      `SoundCue` has no length, so every line holds the same time.
    - `MAX_LINES: usize = 3` (*tunable*): a new line past it drops the
      oldest.
    - The same key while its line shows adds one to the count (capped
      at 99) and restarts the hold; the line keeps its place.
    - `feed<V>(&mut self, requests: &[AudioRequest<V>], music:
      &[MusicCommand], manifest: &AudioManifest, config: AudioConfig)`:
      a `PlaySound` of a cue with a caption adds its line, whatever its
      volume and the player's sound volume (a player who turned the
      sound down still sees it). A `MusicCommand::Start` of a cue with
      a caption adds its line, so a music caption shows when the track
      starts, not when it is asked for. Voice requests, `StopMusic` and
      `config`'s cues (`AudioConfig::cue` and `cursor_cue`) add nothing.
    - `advance(&mut self, dt: f32)` counts the holds down with the
      frame's time, as `MusicState` counts its fade, and drops a line
      whose time is up. `clear()` empties it.
    - `view(&self) -> SoundCaptionsView`.
  - `SoundCaptionsView { lines: Vec<SoundCaptionLine> }` and
    `SoundCaptionLine { text: Label, count: u16 }`, plain data. The text
    is a `Label::Key`, which the painter translates with `Ctx::label`
    (ADR-0020).
  - `paint<G: GameData>(ctx: &Ctx<G>, view: &SoundCaptionsView, buf:
    &mut GlyphBuffer)`, the engine's glyph painter:
    - one row per line, centred, on the rows just above the last row
      (the last row is where the engine's screens put their help line),
      newest at the bottom;
    - each line's cells, its text and one panel cell on each side, are
      first filled with a blank `EngineColor::Panel` cell through
      `fill_rect`, so a sprite or overlay there is cut away; then the
      text is printed in `EngineColor::Text` on `EngineColor::Panel`;
    - a count over 1 shows through the text key
      `sound_captions.repeat` (`"{text} ×{count}"`, `Ctx::text_with`);
    - a line wider than the buffer is cut at its edge;
    - it reads `buf.width()` and `buf.height()`, so it follows 2323's
      text size when that is done.
- **`Game`** (`crates/ui/src/game.rs`):
  - owns a `SoundCaptions`;
  - `step` no longer calls `redraw`. `frame` calls `step`, then
    `collect_audio`, then `redraw`, so a sound's caption shows on the
    frame its request goes out. `act` still redraws itself, and its
    sounds still go out with the next frame;
  - at the end of `collect_audio`, while
    `self.ctx.settings().sound_captions` is on, it advances the log by
    `dt`, then feeds it this frame's `audio_out` and `music_out` with
    `self.ctx.audio.config()`. While it is off, it clears the log and
    feeds nothing;
  - `redraw` paints the strip last, over the whole stack, through
    `G::paint_sound_captions`, when the log has lines;
  - `Game::sound_captions(&self) -> &SoundCaptions`, for tests.
- **The painter is the game's to replace:** `GameData::
  paint_sound_captions(ctx: &Ctx<Self>, view: &SoundCaptionsView, buf:
  &mut GlyphBuffer)` (`crates/ui/src/ctx.rs`), with no `self` (the game
  is `ctx.game`), defaulted to `sound_captions::paint`. A game whose text
  box sits on those rows paints the strip elsewhere.
- **The setting:** `Settings::sound_captions: bool`, off by default,
  restored by `Settings::restored`. `OptionRow::sound_captions()`, a
  `RowValue::Toggle` like `OptionRow::fullscreen()`
  (`crates/ui/src/screens/options.rs:219`), labelled
  `options.row.sound_captions`.
- **Saved forms:** every `GameSettings` in the repository saves the
  field, and a saved form without it reads off (`#[serde(default)]`, as
  today): the toy's (`crates/ui/src/toy.rs`), hello-grid's
  (`examples/hello-grid/src/settings.rs`), the grid template's
  (`templates/grid/crates/game/src/settings.rs`), and the engine's
  `impl GameSettings for ()` (`crates/ui/src/settings.rs`, from 2511),
  which the blank template uses. `cargo fmt --all`
  doesn't format `templates/`; CI's templates job does.
- **Text:** `options.row.sound_captions` ("Sound captions") and
  `sound_captions.repeat` go in `crates/ui/src/toy/en.ron` and
  `examples/hello-grid/files/en.ron`. **Either order with 2351:** if it
  is done, both go in the engine's English (`crates/ui/lang/en/ui.ron`)
  instead, and hello-grid needs no copy. The `cc.` keys are screen
  text, so `lang-status` already lists them when a pack lacks them.
- **The toy** (`crates/ui/src/toy.rs`):
  - its options screen (`option_rows`) shows the row after
    `OptionRow::sound_volume()`;
  - its `click` cue gets `cc.click` ("[Click]") and its `theme` track
    `cc.theme` ("[Music]"), both in the toy's `ENGLISH` array. Its menu
    cues are copies of `click` made before the caption is set, so they
    stay uncaptioned and pass the check.
- **`check`** (`sound_captions::check(file: &str, manifest:
  &AudioManifest, lang: &Lang, config: AudioConfig, cols: u16) ->
  Vec<ContentError>`), for a game's loader and `GameTools::check`. Each
  error names the cue. It reports:
  - a caption key the English text lacks (`Lang::has`,
    `crates/assets/src/lang.rs:201`);
  - a caption on one of `config`'s cues. Captioning menu sounds is
    noise the guidelines advise against;
  - a caption whose English, with ` ×99` after it and a panel cell on
    each side, is wider than `cols` (`text_width`,
    `crates/assets/src/lang.rs:56`).

  Its messages are for the game's developer, not the player: mark them
  `// check-text: not player text`, as the engine's other checks do.
- **hello-grid** (`examples/hello-grid/`):
  - one gameplay cue, `foe_step`, added to `sound::manifest()`: if
    Hello, Grid's sounds are already `.sfx.ron` files (2225), a new
    `files/sfx/foe_step.sfx.ron` made with `shuyi sfx` from a preset;
    if not, a new blip in `BLIPS` (`src/sound.rs`). It plays when the
    foe steps (`foe_moves`, `src/screens/stage.rs:214`; it and
    `walked`, which calls it, then take the `ctx`). It is captioned
    `cc.foe_step` ("[Footsteps]") in `files/en.ron`. Its menu sounds
    stay uncaptioned;
  - its options rows (`option_rows`, `src/screens.rs:56`) get
    `OptionRow::sound_captions()`;
  - `Hello::load_from` (`src/lib.rs:180`) runs `sound_captions::check`
    on the `Lang` and manifest it built, with `CONSOLE.cols`, and
    returns its errors, so `cargo shuyi check` reports them.
- **The guide:** a *Sound captions* section in `docs/accessibility.md`
  (created here if none of 2322, 2323 and 2324 is done): caption what a
  player needs to know (an alarm, a foe off screen, a change of music),
  never menu sounds or speech; keep captions short; write them in
  brackets in the text, as `[Door opens]`.
- **Either order:** where a ticket below is still open, this PR adds
  its part to that ticket's file.
  - **2322 (parked):** if it is done, `Game` also announces each new
    line (not a repeat) `Polite` through `Ctx::announce` while sound
    captions are on, so 3035 reads it to a screen reader. If not, 2322
    does it.
  - **2340, and 2404 (parked):** each adds settings to `Settings` and
    the saved forms. Whichever lands second follows the first's shape.
  - **2345 and 2378:** their cue rules run in `App`, after
    `Game::collect_audio`, so a play they drop (a gap, a cap, every
    voice busy) still shows its caption: the thing happened. Done or
    not, nothing more to do here.
  - **2342 and 2345** each add a field to `SoundCue`, and 2342
    and 2365 each add one to `MusicCue`. The later to land rebases onto
    the other's cue and bumps `pack::VERSION` again. **From 2344
    (done):** `SoundCue` has `looped`, and `pack::VERSION` is 5.
  - **From 2344 (done):** `AudioRequest::StartSound` starts a sound
    with a handle, looped if its cue says so. `feed` treats a
    `StartSound` of a captioned cue as a `PlaySound`: its line is added
    once when it starts, never again per loop, with a test.
  - **Public types (ADR-0039):** `SoundCue` is under it (2024), and its
    `new` leaves `caption` at `None`. `MusicCue` moves under it in this
    PR if it isn't already, as its *When* rule says of a type that next
    gains a field: `#[non_exhaustive]`, with a `new` that leaves
    `caption` at `None`. **Either order with 2365 (parked, beats):** if
    2365 is done, it moved `MusicCue` already, so only add `caption` to
    its `new`; if not, 2365 adds its `tempo` to the `new` written here.
  - **2218:** if it is done, its `AudioManifestRon` writer writes
    `caption` only when it is `Some`, in the implicit-`Some` form, and
    its round trip covers a captioned cue. If not, 2218 does it.
- **What changes for the game:**
  - `SoundCue` and `MusicCue` gain a public field. `SoundCue`'s breaks
    no game: it is `#[non_exhaustive]` since 2024, and the game moves
    its `SoundCue {` literals to `SoundCue::new` when its pin passes
    2024. `MusicCue` becoming `#[non_exhaustive]` here breaks a game
    that builds it by literal: the PR says so in a `Breaking:` line
    (2010).
  - `Settings` gains a field (additive); `GameData` gains a defaulted
    method (additive); old manifests read as before. The game's
    `GameSettings` saves the field if it wants the setting kept.
  - `pack::VERSION` goes up, so a baked pack is baked again.
  - The game's PR that moves the pin follows this one. Whether the game
    captions any cue is the game's choice.

**Out (do not do):**
- speech: voice lines are already text in the text box, and the visual
  novel backlog is 2723; speaker labels;
- which way a sound came from (an arrow or a side): it needs positional
  sound (2363, parked), so 2381 does it, blocked by 2363 and
  this ticket;
- audio description, a mono downmix, and visual cues other than text
  (a flash on a sound is 2340's and the game's);
- a new event stream for captions in `shuyi run` or the Harness: they
  already list every cue by id, the manifest maps a cue to its key, and
  the frame text shows the strip when the setting is on;
- caption timing from each sound's real length;
- captions for a cinematic's own cues beyond what `AudioQueue` plays
  (2364 leaves captions here: its cues play through `AudioQueue`, so
  they get captions with no extra work);
- the templates' option rows and captions: they have no options screen,
  and their only sounds are menu sounds (2225), which stay uncaptioned
  like every menu sound. Only their saved forms change;
- the game's own cues and screens.

## Implementation steps

1. Read `crates/assets/src/audio.rs`, `crates/ui/src/audio.rs`,
   `crates/ui/src/game.rs`, `crates/ui/src/ctx.rs`, `settings.rs`,
   `screens/options.rs`, `toy.rs`, `crates/ui/tests/it/audio.rs` (the
   toy's `Beeper` Harness), and hello-grid's `sound.rs`, `screens.rs`,
   `screens/stage.rs`, `settings.rs` and `lib.rs`. Read 2218,
   2322, 2323, 2340, 2342, 2344, 2345, 2351 and 2225, and note which are
   done.
2. Add the `caption` field, the implicit-`Some` read and the `cc.` rule
   to the manifest, with tests. Give every cue literal `caption: None`.
   Raise `pack::VERSION`, accept the binary-form snapshot, and write the
   `docs/formats.md` line.
3. Write `sound_captions.rs`: the log, the view and the painter, with
   unit and property tests.
4. Wire it into `Game` (the new order in `frame`), add
   `GameData::paint_sound_captions`, the setting, the option row and
   every saved form.
5. Update the toy: its row, its two captions and their English, and a
   Harness test in `crates/ui/tests/it/` built like `beeper()`.
6. Write `check`, with a test for each problem.
7. Update hello-grid: the `foe_step` cue, its caption, the row, the
   saved form, the check in `load_from`, and its tests.
8. Write the guide section, and settle each *Either order* item.
9. Run the gates, including the `no_std` build of `shuyi-ui`.

If the diff is well past 600 lines (snapshots aside) after step 5, ship
steps 1 to 5 with step 9, and write steps 6 to 8 as one follow-up
ticket (the `write-ticket` skill, this ticket's milestone), blocked by
this one. It carries their criteria below.

## Acceptance criteria

- [ ] An audio manifest with no `caption` reads as before; one with
      `caption: "cc.x"` reads `Some("cc.x")`; `caption: "door"` and
      `caption: "cc."` fail at the cue's line.
- [ ] `pack::VERSION` is one higher and the binary-form snapshot is
      accepted for it.
- [ ] With sound captions off (the default), every other existing
      snapshot is unchanged. The tests that list the toy's and
      hello-grid's option rows gain the new row.
- [ ] The toy with sound captions on, after its `click` cue plays, shows
      the strip; an `insta` glyph snapshot of it is committed.
- [ ] Playing `click` twice within `HOLD_SECS` shows one line with
      count 2; after `HOLD_SECS` with no sound the strip is gone.
- [ ] A music caption appears on the frame with the track's
      `MusicCommand::Start`, not on the frame it was asked for during a
      fade.
- [ ] Menu sounds (`AudioConfig` cues) and voice lines never add a line.
- [ ] A sprite over the strip's cells is cut there (a painter unit
      test).
- [ ] Each saved form keeps the setting and reads it back; a saved form
      without it reads off.
- [ ] `check` reports a caption key the English lacks, a menu cue with
      a caption, and a caption too wide for the console.
- [ ] hello-grid's foe step shows `[Footsteps]` with the setting on, and
      `cargo shuyi check` passes on hello-grid.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] Each *Either order* item is settled: done here, or added to the
      open ticket's file.
- [ ] If steps 6 to 8 were split off, their follow-up ticket is in
      `tickets/open/`, blocked by 2366, with their criteria.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit:
  - the manifest's `caption` field, the implicit `Some` and the `cc.`
    rule;
  - the log: feeding sounds at any volume, music starts, voice and menu
    requests; merging a repeat; dropping the oldest past `MAX_LINES`;
    the hold counting down; `clear`;
  - the painter: rows, centring, the repeat text, the cut at the edge,
    and a sprite under the strip cut away;
  - `Game` clearing the log when the setting goes off;
  - each problem `check` reports;
  - the option row toggling; each saved form, with and without the
    field.
- Property: for random requests and frame times, never more than
  `MAX_LINES` lines, no hold above `HOLD_SECS`, no `AudioConfig` cue
  ever shown.
- Snapshot / integration:
  - a Harness test on the toy with sound captions on (through
    `ctx_mut().change_settings`) that plays `click` and starts `theme`,
    reads `Game::sound_captions()` (what happened) and then the strip's
    cells (the look, `AGENTS.md` rule 9);
  - the updated pack binary-form snapshot;
  - hello-grid's foe step with captions on, read from `h.snapshot()`.

## Completion notes

*(Filled in by the session that completes the ticket.)*
