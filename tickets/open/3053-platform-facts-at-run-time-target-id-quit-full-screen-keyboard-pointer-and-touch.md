---
id: "3053"
title: "Platform facts at run time: the target's id, whether a game can quit, whether its window can go full screen, and its keyboard, pointer and touch, read from the target profile and handed through Setup into Ctx"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3053 — Platform facts at run time

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6,
Unreal 5.8 and GameMaker. Each lets a running game ask where it runs:

- Godot: `OS.has_feature` with built-in feature tags (the platform,
  `web`, `mobile`, `pc`), and project settings overridden per tag
  (<https://docs.godotengine.org/en/stable/tutorials/export/feature_tags.html>).
- Unity: `Application.platform` at run time, and Build Profiles that
  override player settings per platform.
- Unreal: Device Profiles set console variables per device
  (<https://dev.epicgames.com/documentation/en-us/unreal-engine/setting-up-device-profiles-in-unreal-engine>).
- GameMaker: build configurations, read with `os_get_config()`
  (<https://manual.gamemaker.io/lts/en/Settings/Configurations.htm>).
- Bevy has no run-time fact. Its `bevy_new_2d` template leaves its Exit
  button out of the web build with `#[cfg(not(target_family = "wasm"))]`,
  which an agent must know to write and a test can't switch.

What Shuyi has (read 2026-10-21):

- **Two facts cross the boundary, one at a time.** `Setup`
  (`crates/platform/src/app.rs:117`) carries `keyboard: Keyboard` and
  `key_prompt: KeyPrompt` beside the display, the storage, the seed and
  the voice manifest. `Setup::ctx` (line 140) copies them into
  `Ctx::keyboard` and `Ctx::key_prompt` (`crates/ui/src/ctx.rs:306`
  and 301). `App::load` (`app.rs:475`) always sets `Keyboard::Present`
  and `KeyPrompt::Waiting`; a backend changes them with
  `Start::before_load`. Only the first-key check in `Game::step`
  (`crates/ui/src/game.rs:303-308`) and the key bindings screen read
  `keyboard`. That screen reads it at line 314 and keeps a copy in its
  own `keyboard` field (line 321).
- **`DisplayInfo`** (`crates/platform/src/platform.rs:55-69`) has the
  width, the height and the colour depth only.
- **The target profiles know more than the running game.**
  `TargetProfile` (`crates/assets/src/target.rs:54-75`) gives each
  target's `input.pointer`, `input.touch` and `input.keyboard`
  (`targets/*.ron`). Only `shuyi check --target` reads them (3001,
  3002); `shuyi-cli` embeds the files with `include_dir!`
  (`TARGETS`, `crates/cli/src/contract/target.rs:33`). Nothing hands a
  profile, or even a target's id, to a running game.
- **Quit on the web freezes the game.** `shuyi_app::run` ignores
  `FrameOutcome::Quit` on wasm32 (`crates/app/src/lib.rs:176-180`:
  "The web page can't be closed, so it keeps showing the last frame";
  ADR-0024 §3). Yet:
  - Hello, Grid's title offers Quit (`TITLE_ITEMS`,
    `examples/hello-grid/src/screens.rs:113-118`, chosen at line 186),
    and Pages publishes that build (`.github/workflows/pages.yml`);
  - the blank template's title only quits, on Cancel
    (`templates/blank/crates/game/src/title.rs:60-66`), and its help
    says "{Cancel} leave" (`templates/blank/crates/game/files/en.ron`);
  - the grid template's stage quits on Cancel
    (`templates/grid/crates/game/src/stage.rs:249`), and its help says
    "{Cancel} leave" (`stage.help` and `stage.help.over` in
    `templates/grid/crates/game/files/en.ron`).
- **The Fullscreen row shows everywhere.** `OptionRow::fullscreen`
  (`crates/ui/src/screens/options.rs:219`) is offered on a device that
  is always full screen (the Steam Deck, 3012; the Playdate, 3010).
  `Frame::fullscreen`'s docs say such a device ignores it, so the row
  does nothing there.
- **Plans that assume the target is known at run time:** 3007's
  `assets@<target>/` layers are used "when baking or running for that
  target"; 3012 makes the `shuyi-app` backend behave as a Deck; 2348
  reserves a `target` argument for this ticket; 2349 asks this ticket
  to fold in the system's languages; 2333 leaves a default per target
  for after it.

`Setup::key_prompt` stays where it is. It is not a fact of the target
but where the game starts: the wait that `Game` moves on to
`KeyPrompt::Pressed`. Every backend starts it at `Waiting` today.

Why it matters:

- **Visions of Shuyi** has a desktop and a web build, and plans the
  Playdate (3010) and the Steam Deck (3012). Its title offers Quit on
  every build, and its Playdate front end sets `Keyboard::Absent` by
  hand. With this, the backend says both, and the game's title can hide
  Quit with one call at its next pin bump, without a `cfg!`.
- **Other 2D games:** each port shows only what works there.
- **The end goal** (Nick, 2026-10-19, `docs/decisions.md`): an agent
  makes a game "with a web ver, a playdate ver, etc." in under an hour.
  A fact in `Ctx` is one field an agent reads, and a Harness test can
  run a title "as the web".

## Nick input

None.

## Scope

**In:**
- **Two fields in the profile format** (`crates/assets/src/target.rs`),
  each with a serde default, so every game's own profile still loads:
  - `quit: Quit` on `TargetProfile`, a new `Quit` enum beside `Color`:
    - `Game` (the default): the game's own Quit closes it;
    - `System`: the player leaves through the system (a browser tab,
      the Playdate's Home, a console's or a phone's home button), and a
      game shows no Quit.
  - `windowed: bool` on `Display` (default `false`): the game is in a
    window the player can switch to full screen and back.
  - The engine's files, each new value with a one-line comment:
    - `desktop.ron`: `windowed: true`;
    - `web.ron`: `quit: System`, `windowed: true` (the canvas goes full
      screen through macroquad's `set_fullscreen`, as 3052 describes);
    - `playdate.ron`, `switch.ron`, `android.ron`, `ios.ron`:
      `quit: System`;
    - `steam-deck.ron`: the defaults (the game quits itself, always
      full screen).
- **`shuyi_ui::PlatformFacts`** (`crates/ui/src/platform_facts.rs`,
  `no_std`, re-exported from the crate's root; `Debug, Clone,
  PartialEq, Eq`, `#[non_exhaustive]`, public fields):
  - `target: String`: the profile's id (`desktop`, `web`,
    `steam-deck`, `playdate`);
  - `quit: Quit` and `windowed: bool`, from the profile;
  - `keyboard: Keyboard`, moved here from `Ctx::keyboard`:
    `Present` where `input.keyboard` is true;
  - `pointer: bool` and `touch: bool`, copied from `input`. Nothing
    reads them in this ticket;
  - `PlatformFacts::from_profile(&TargetProfile) -> Self`;
  - `can_quit(&self) -> bool`: `quit == Quit::Game`;
  - `Default`: the `desktop` profile's facts, so every test written
    before runs as it did.
- **`Ctx::platform: PlatformFacts`** replaces `Ctx::keyboard`.
  `Game::step`'s first-key check and the key bindings screen read
  `ctx.platform.keyboard`. `Ctx::key_prompt` stays.
- **`Game::toggle_fullscreen`** (`crates/ui/src/game.rs:248`) does
  nothing where `ctx.platform.windowed` is false. So `App`'s
  `SystemEvent::ToggleFullscreen`, and any other caller, leaves the
  setting as it was on a device that is always full screen.
- **The boundary** (`crates/platform`):
  - `Platform::facts(&self) -> PlatformFacts`, a required method: a
    backend always says which target it is, so a console's backend
    can't inherit the desktop's facts by forgetting;
  - `Setup::platform: PlatformFacts` replaces `Setup::keyboard`;
    `App::load` fills it from `facts()`, and `Setup::ctx` puts it into
    `Ctx::platform`. `Setup::key_prompt` stays;
  - `HeadlessPlatform::with_facts(PlatformFacts)`, and
    `PlatformFacts::default()` until it is called.
- **`shuyi-app` knows its target** (`crates/app/src/facts.rs`, new):
  - the id is `option_env!("SHUYI_TARGET")` when a build sets it, else
    `web` on wasm32 and `desktop` elsewhere. One binary serves Linux
    desktops and the Deck, so only the build can tell them apart:
    3012's exporter sets `SHUYI_TARGET=steam-deck`. A pure
    `target_id(wasm: bool, set: Option<&'static str>) -> &'static str`
    holds the rule, so native tests cover the web's case;
  - the profiles of the targets `shuyi-app` builds (`desktop`, `web`,
    `steam-deck`) are embedded with `include_str!` from `targets/`
    (`include_str!("../../../targets/desktop.ron")`), as `shuyi-cli`
    embeds them with `include_dir!`, and read with
    `TargetProfile::from_source`. The facts come from the profile and
    are never typed twice;
  - `facts_for(id: &str) -> Result<PlatformFacts, String>` reads the
    profile of `id`. An id with no embedded profile is an error naming
    it and the ids it knows. Never a panic;
  - `run` calls it before the loop. On an error it swaps the `Start`'s
    `load` (a public field) for one that returns that error, so the
    window shows it (`Show::Errors`, as a broken asset does), and
    `Macroquad` keeps `PlatformFacts::default()`, which no game sees.
    Otherwise `Macroquad::facts` returns the facts read.
- **Engine screens honour the facts.** `OptionsScreen::rows`
  (`crates/ui/src/screens/options.rs:484`) leaves out the row with
  `OptionRow::fullscreen`'s key where `ctx.platform.windowed` is false,
  whatever the game's `Rows` function returns. Every other reader of the
  rows (`focus`, `focused_row`, the view) goes through `rows`.
- **The Harness runs as a target.** `Harness::as_target(self, profile:
  &TargetProfile) -> Self` (`#[must_use]`, like `at_prompt` at
  `crates/platform/src/harness.rs:112`) sets `Ctx::platform` from the
  profile and redraws. Call it before the screen under test opens:
  Hello, Grid's title and the options screen read `ctx` each frame, but
  the key bindings screen keeps the keyboard it opened with. A game's
  test reads the engine's profiles with
  `shuyi_cli::contract::target::engine_profiles()`, as the blank
  template's test already does (`tests/it/main.rs:61`). An engine
  crate's test reads one from `targets/` with `include_str!` and
  `TargetProfile::from_source`.
- **Hello, Grid and the templates use it** (the dead Quit on Pages
  goes):
  - Hello, Grid: `Title::menu` takes `ctx` and leaves out Quit where
    `!ctx.platform.can_quit()`. Quit is the last item, so the other
    items' focus indices don't move;
  - blank template: where `!can_quit()`, Cancel does nothing on the
    title and the help line is empty: the title has nothing to choose
    there;
  - grid template: where `!can_quit()`, Cancel does nothing on the
    stage, and the help uses two new keys in its `files/en.ron`:
    `stage.help.stay` ("{Cursor} step") and `stage.help.over.stay`
    ("{Confirm} play again");
  - both templates' `AGENTS.md`, under *The layers*: "Ask
    `ctx.platform`, never `cfg!`: offer Quit only where
    `ctx.platform.can_quit()`. Screens read the facts; the rules crate
    never does."
- **The facts never reach the rules.** They live in `shuyi-ui`, which
  `shuyi-core` and a game's rules crate don't depend on. **From 2015
  (done):** `cargo xtask check-layers` holds that for `shuyi-core`
  (`tools/layers.ron`), and the grid template's
  `crates/rules/tests/layers.rs` fails a rules crate that reaches
  `shuyi-ui`. A Hello, Grid test plays the same keys as `desktop` and
  as `web` and gets the same stage state hash
  (`shuyi_core::state_hash`).
- **An ADR** (the next free number in `docs/adr/`, listed in
  `docs/adr/README.md`), extending ADR-0008 §1 and ADR-0024 §2, §3 and
  §6:
  - a fact is constant for a run, and comes from the target's profile
    or from what the backend reads of the system;
  - the two profile fields, `PlatformFacts`, and `Platform::facts` as a
    required method, and why it has no default body;
  - how `shuyi-app` learns its id;
  - facts are read by screens, never by rules, and replays don't depend
    on them;
  - per-target data is 3007's `assets@<target>/` layers, picked by this
    id, not per-target settings files.
- **Docs:** `docs/platform-backends.md`: a `### facts(&self) ->
  PlatformFacts` section under *Each method*, `facts` in the load's
  frame 2 (*What one frame does*), and a checklist line.
- **Either order:**
  - **2349** (the system's languages): if it is done, its
    `Platform::languages` and `Setup::languages` fold into
    `PlatformFacts::languages: Vec<LangCode>`, filled by each backend's
    `facts()`; `Setup::ctx` still hands them to
    `Ctx::set_system_languages` before the settings load. If not, 2349
    adds them to `PlatformFacts` itself (its clause says so).
  - **2348** (messages): if it is done, its built-in `target` argument
    is `ctx.platform.target`. If not, 2348 fills it from
    `PlatformFacts` (its clause says so).
  - **2518** (session records): if it is done, `SessionRecord` keeps the
    facts `App::load` got, and playback hands them to
    `HeadlessPlatform::with_facts`, so a session plays back as the
    target it was recorded on. If not, 2518 does that.
  - **2803** (the tutorial): if it is done, its steps under
    `site/tutorial/` get the grid template's change above. If not, 2803
    starts from the changed template.
  - **3007** (per-target asset layers): if it is done, a running
    game's load also picks the `assets@<target>/` layer by
    `setup.platform.target`, so a run that isn't from a target's baked
    pack still uses its layer (3007 picked it when baking only). If
    not, nothing to do (3007 picks it at load, its clause says so).
  - **2005** (the first release): if it is done and has moved or copied
    `targets/` into a crate's package, embed the profiles from there.
    If not, embed them from `targets/`, and 2005 moves them for both
    `shuyi-cli` and `shuyi-app`.
  - **Public types (ADR-0039):** `PlatformFacts` and `Quit` follow it,
    and `Platform::facts` is a stated exception to its rule that a new
    trait method gets a default body: `Platform` is implemented by
    backends, not games, and a default would hand a new backend the
    desktop's facts. ADR-0039's rule on traits names this exception, and
    this ticket's ADR says why.
- **For the game:** the API breaks. `Ctx::keyboard` and
  `Setup::keyboard` become `ctx.platform.keyboard` and
  `setup.platform.keyboard`, and any `Platform` of the game's own
  implements `facts`. Say so in the PR, with 2010's `Breaking:` line.
  The game's PR that moves the pin follows it.
- **Size:** if the PR passes about 600 changed lines, the two
  templates' part (Cancel and the help where `!can_quit()`, the grid
  template's new keys, both `AGENTS.md` lines, and the 2803 clause) goes
  to a follow-up ticket (the `write-ticket` skill, After MVP, blocked by
  this one). Hello, Grid keeps its part here, so the dead Quit on Pages
  still goes.

**Out (do not do):**
- `shuyi run --target` and `scene --target`: they need
  `GameTools::session` to take the facts and a contract version bump.
  A ticket of their own when an agent needs them; they would call
  `Harness::as_target`.
- check-time capabilities and module needs (3017, parked). It reads the
  same profiles when it lands.
- a `family` (desktop, web, handheld, console): a game branches on a
  fact, never on a family, and the id says the rest.
- per-target settings files: 3007's `assets@<target>/` layers carry
  per-target data.
- reading `pointer` and `touch`: 2407 (parked) shows its on-screen
  controls from `touch` (its either-order clauses with this ticket).
  No ticket reads `pointer` yet: 2405 works the same with or without
  one, since a backend without a pointer sends no pointer events, and
  1121 (parked) checks pointers only in `shuyi check --target`.
- the game's identity at run time: 3048's `AppInfo`. These are the
  platform's facts, not the game's.
- other facts later tickets add: reduced motion read at start
  (2377), the system's languages (2349), whether HTTP is present
  (2601, parked), and a phone's browser as `web-touch` (2407, parked).
- a default scale mode per target: neither this ticket nor 2333 does
  it. It is a follow-up of its own, blocked by both, written when a
  game needs a different default on one target (2333's Out says the
  same).
- a default painter set per target (`glyph` or `onebit`): 2371, which
  waits for this ticket and adds it to the profile and to
  `PlatformFacts`.
- the Steam Deck's and the Playdate's facts in their backends, and
  starting `ctx.device` on their pad: 3012 and 3010, which wait for
  this ticket.
- embedding `android.ron` and `ios.ron` in `shuyi-app`: 3015 (parked).
- what a console's home button must do (certification rules).
- the game's own title: the game's PR.

## Implementation steps

1. Read `crates/platform/src/app.rs` (`Setup`, `Start`, `App::load`,
   `on_system_event`), `platform.rs`, `headless.rs` and `harness.rs`;
   `crates/ui/src/ctx.rs`, `game.rs`, `screens/options.rs` and
   `screens/key_bindings.rs`; `crates/assets/src/target.rs`;
   `crates/app/src/lib.rs` and `backend.rs`; ADR-0008 §1 and ADR-0024;
   `docs/platform-backends.md`.
2. Add `Quit` and `windowed` to the profile format with their tests, and
   the values to `targets/*.ron`.
3. Add `PlatformFacts`, move `Ctx::keyboard` into it, guard
   `Game::toggle_fullscreen`, and fix every reader and writer
   (`grep -rn "Keyboard::" crates examples templates` finds them, tests
   included).
4. Add `Platform::facts`, `Setup::platform` and
   `HeadlessPlatform::with_facts`. Implement `facts` on every
   `Platform`: `HeadlessPlatform`, `Macroquad`, and the test platforms
   `RecordLook` and `WithRefusing` in `crates/platform/src/app/tests.rs`.
5. Write `crates/app/src/facts.rs` (`target_id`, the embedded profiles,
   `facts_for`), and wire it into `Macroquad` and `run`.
6. Filter the Fullscreen row, and add `Harness::as_target`.
7. Change Hello, Grid and both templates, with their tests. The
   templates are outside `cargo fmt --all`: make a game from each
   (`cargo run -p shuyi-cli -- new grid <dir> --engine-path .`, then
   `blank`) and run CI's `templates` job's gates in it (`cargo fmt
   --all --check`, clippy, `cargo test --workspace`, `cargo shuyi check
   --target all`).
8. Write the ADR and the docs, and run the gates.

## Acceptance criteria

- [ ] Every file in `targets/` decodes with the `quit` and `windowed`
      values above (a test reads each), and a profile without the two
      fields loads with `Game` and `false`.
- [ ] `PlatformFacts::default()` equals `from_profile` of
      `targets/desktop.ron`.
- [ ] `target_id(false, None)` is `desktop`, `target_id(true, None)` is
      `web`, and `target_id(_, Some("steam-deck"))` is `steam-deck` on
      either; `facts_for` reads each embedded profile, and
      `facts_for("nowhere")` is an error naming `nowhere` and the three
      ids it knows.
- [ ] `App::load` on a headless platform with the `steam-deck` facts
      gives a context with `Keyboard::Absent` and `windowed` false.
- [ ] Hello, Grid's title as `web` has no Quit item and no input quits
      it; as `desktop` it has Quit, and its title snapshot is unchanged
      (Harness tests reading the view).
- [ ] The blank template as `web`: Cancel on the title doesn't quit and
      the help is empty; as `desktop`, Cancel quits.
- [ ] The grid template as `web`: Cancel on the stage doesn't quit and
      the help names no leaving; as `desktop`, it quits.
- [ ] The toy's options screen as `steam-deck` and as `playdate` has no
      Fullscreen row; as `desktop` and as `web` it has one.
- [ ] `ToggleFullscreen` where `windowed` is false leaves the setting as
      it was; where it is true, it flips it as before.
- [ ] The same keys in Hello, Grid as `desktop` and as `web` give the
      same stage state hash.
- [ ] `shuyi-ui` and `shuyi-platform` still build for
      `thumbv7em-none-eabihf` (CI's no-std job).
- [ ] A game made from each template passes CI's `templates` job's
      gates.
- [ ] The ADR is written and listed in `docs/adr/README.md`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the profile fields (decoding, defaults, each engine file);
  `from_profile` on each engine profile; `can_quit`; `target_id`;
  `facts_for` (each embedded profile, an unknown id); the options rows'
  filter; `App::load` handing `facts()` into the context; the
  `toggle_fullscreen` guard.
- Integration: the Harness tests above, in Hello, Grid
  (`tests/it/playthrough.rs`, `tests/it/determinism.rs`), in each
  template's `crates/game/tests/it/main.rs`, and the toy's options
  screen in `crates/ui/tests/it/screens.rs`.
- Snapshot: Hello, Grid's title as `web` in `tests/it/snapshot.rs`,
  beside `tests/it/snapshots/it__snapshot__title.snap`.

## Completion notes

*(Filled in by the session that completes the ticket.)*
