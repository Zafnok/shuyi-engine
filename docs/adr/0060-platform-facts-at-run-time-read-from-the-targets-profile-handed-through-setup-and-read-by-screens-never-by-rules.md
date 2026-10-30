# ADR-0060: Platform facts at run time: read from the target's profile, handed through `Setup`, and read by screens, never by rules

- **Status:** Accepted; extends ADR-0008 §1, ADR-0024 §2, §3 and §6, and
  ADR-0040 §7
- **Date:** 2026-10-30
- **Related tickets:** 3053 (this), 2778 (the template's title), 3012
  (the Steam Deck's build), 3010 (the Playdate's backend), 3007
  (per-target asset layers), 2349 (the system's languages), 2348
  (messages' `target`), 2518 (session records), 2371 (a default painter
  per target), 2407 (touch controls, parked)

## Context

- **Other engines let a running game ask where it runs.** Godot's
  feature tags (`OS.has_feature("web")`), Unity's
  `Application.platform`, Unreal's device profiles and GameMaker's
  `os_get_config()`. Bevy has none: its `bevy_new_2d` template leaves its
  Exit button out of the web build with `#[cfg(not(target_family =
  "wasm"))]`, which an agent must know to write and a test can't switch.
- **The profiles knew more than the running game.** `TargetProfile`
  (ADR-0008 §1) says each target's pointer, touch and keyboard, but only
  `shuyi check --target` read it. A running game got two facts, one at a
  time: `Setup::keyboard` (always `Present` unless a backend changed it
  with `Start::before_load`) and the key prompt.
- **Dead controls.** The web build ignores a quit (ADR-0024 §3), yet
  Hello, Grid's title on Pages offered Quit, and the template's title
  quit on Cancel. The options screen's Fullscreen row showed on devices
  that are always full screen.
- **Plans wait on the id:** 3007's per-target asset layers, 3012's Deck
  build, 2348's `target` argument, 2349's system languages.

## Decision

### 1. A fact is constant for a run

It comes from the target's profile or from what the backend reads of the
system, before the game loads, and never changes while it runs. What the
player changes (the Fullscreen setting) stays a setting.

### 2. Two fields in the profile format

`TargetProfile::quit: Quit` (`Game`, the default: the game's own Quit
closes it; `System`: the player leaves through the system, and a game
shows no Quit) and `Display::windowed: bool` (default `false`: always
full screen). Both have serde defaults, so a game's own profile still
loads. `desktop` and `web` are windowed; `web`, `playdate`, `switch`,
`android` and `ios` quit through the system; `steam-deck` keeps both
defaults.

### 3. `PlatformFacts`, in `shuyi-ui`

`shuyi_ui::PlatformFacts` (`no_std`, `#[non_exhaustive]`, public fields,
ADR-0039): `target` (the profile's id), `quit`, `windowed`, `keyboard`
(moved from `Ctx::keyboard`), `pointer` and `touch`.
`from_profile(&TargetProfile)` builds it, `can_quit()` is `quit ==
Quit::Game`, and `Default` is the `desktop` profile's facts (a test
checks), so every test written before runs as it did. The context holds
it as `Ctx::platform`; `Ctx::key_prompt` stays apart, since it is where
the game starts, not a fact of the target.

### 4. `Platform::facts` is required

`Platform::facts(&self) -> PlatformFacts` has no default body. A backend
always says which target it is: a default would hand a new console's
backend the desktop's facts, a Quit that does nothing and a Fullscreen
row, and nothing would fail. This is the exception ADR-0039 §7 names to
ADR-0040 §7's rule that every method added to `Platform` has a default;
`crates/platform/tests/it/bare.rs` now implements eight methods. `App`
asks once, as the game loads (ADR-0024 §6's frame 2), into
`Setup::platform`, which `Setup::ctx` puts into `Ctx::platform`.
`HeadlessPlatform::with_facts` sets the headless backend's, the
`desktop` target's until called.

### 5. How `shuyi-app` learns its id

`crates/app/src/facts.rs`: the id is `option_env!("SHUYI_TARGET")` when
the build sets it, else `web` on wasm32 and `desktop` elsewhere (a pure
`target_id(wasm, set)` holds the rule, so native tests cover the web's
case). One binary serves Linux desktops and the Deck, so only the build
can tell them apart: 3012's exporter sets `SHUYI_TARGET=steam-deck`. The
profiles of the targets `shuyi-app` builds (`desktop`, `web`,
`steam-deck`) are embedded with `include_str!` from `targets/` and read
with `TargetProfile::from_source`, so a fact is never typed twice. An id
with no embedded profile becomes the game's load error, shown in the
window as a broken asset is (`Show::Errors`); never a panic.

### 6. Screens read the facts; rules never do

The facts live in `shuyi-ui`, which `shuyi-core` and a game's rules crate
don't depend on (`cargo xtask check-layers` and the grid template's
`crates/rules/tests/layers.rs` hold that). A replay doesn't depend on
them: Hello, Grid's test plays the same keys as `desktop` and `web` and
gets the same stage state hash. The engine's own screens honour them:
`Game::toggle_fullscreen` does nothing where `windowed` is false, and
`OptionsScreen::rows` leaves out the Fullscreen row there. A test runs a
screen as a target with `Harness::as_target(&TargetProfile)`.

### 7. Per-target data is asset layers, not settings files

Per-target data is 3007's `assets@<target>/` layers, picked by this id.
There are no per-target settings files and no `family` (desktop,
handheld, console): a game branches on a fact, and the id says the rest.

## Consequences

- A game hides Quit on the web with one call (`ctx.platform.can_quit()`),
  and a Harness test runs a title "as the web".
- A new fact (the system's languages, 2349; reduced motion, 2377; HTTP,
  2601) is a field of `PlatformFacts`, filled by each backend's `facts`.
- The API breaks: `Ctx::keyboard` and `Setup::keyboard` become
  `ctx.platform.keyboard` and `setup.platform.keyboard`, and every
  `Platform` implements `facts`.
- `shuyi-app` embeds files from outside its crate, as `shuyi-cli` does;
  2005 (the first release) moves `targets/` into a package for both.
- `shuyi run --target` and `scene --target` don't exist yet: they need
  the contract's session to take the facts, and would call
  `Harness::as_target`.

## Alternatives considered

- **`cfg!(target_arch = "wasm32")` in games**, as Bevy's template does.
  A test can't switch it, and it can't tell the Deck from a Linux
  desktop.
- **A default body for `Platform::facts`** returning the desktop's. It
  keeps ADR-0040 §7's rule, but a backend that forgets gets the wrong
  facts silently.
- **The whole `TargetProfile` in `Ctx`.** It carries budgets and button
  tables a screen has no business reading, and `shuyi-ui` would hand
  them on as API.
- **A `family` enum (desktop, web, handheld, console).** A game would
  branch on "handheld" where it means "no keyboard"; the facts say that
  directly.
- **Facts typed into `shuyi-app` as constants.** They would drift from
  `targets/`, which `shuyi check --target` reads.
