---
id: "2377"
title: "The system's reduced-motion preference at first start: a platform fact each backend reads, and the default of the player's motion setting"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: ["2340", "3053"]
nick_input: none
completed:
---

# 2377 — The system's reduced-motion preference at first start

## Context

2340 gives the player a motion setting, `Settings::motion` (`Full` or
`Reduced`), kept per profile. A new player starts at `Full`, even one
who has told their system they want less motion. 2340's Out leaves
reading that preference to this ticket, through 3053's platform facts,
and 3053's Out lists it among the facts later tickets add.

Each system keeps the preference in its own place:

- the web: the `prefers-reduced-motion` media query
  (<https://www.w3.org/TR/mediaqueries-5/#prefers-reduced-motion>),
  read with `window.matchMedia`;
- Windows: Accessibility > Visual effects > Animation effects, read
  with `SystemParametersInfoW` and `SPI_GETCLIENTAREAANIMATION`
  (<https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-systemparametersinfow>);
- macOS: Accessibility > Display > Reduce motion, saved as
  `reduceMotion` in the `com.apple.universalaccess` defaults;
- GNOME: `enable-animations` in `org.gnome.desktop.interface`, read
  with `gsettings`.

WCAG 2.3.3 asks that motion started by an interaction can be turned
off
(<https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions.html>).
2340 gives the switch; this ticket sets it from the start for a player
who has already asked their system.

What Shuyi will have once the blockers are done:

- **3053's facts.** `PlatformFacts` (`crates/ui/src/platform_facts.rs`)
  on `Ctx::platform`: constant for a run, from the target's profile or
  from what the backend reads of the system. `Platform::facts` is a
  required method, `HeadlessPlatform::with_facts` sets them in tests,
  and `shuyi-app`'s `run` reads the profile's facts with `facts_for`
  (`crates/app/src/facts.rs`) before the loop.
- **The order at load.** `Setup::ctx` (`crates/platform/src/app.rs`)
  sets the context's fields before `with_storage` loads the settings
  (it sets `ctx.keyboard` there today; 3053 puts `ctx.platform` in its
  place).
- **The defaults.** `Ctx::load_settings` (`crates/ui/src/ctx.rs`) uses
  `Settings::default()` when the profile has no settings saved
  (`Ok(None)`) and when they can't be read. It runs at start and, from
  `load_profile`, on a profile switch, so a new profile starts there
  too.
- **Restore defaults.** `OptionRow::restore_defaults`
  (`crates/ui/src/screens/options.rs`) sets `s.restored()`: every
  setting at its default but the layout.
- **The browser.** The web build has no `wasm-bindgen` (game
  ADR-0034), so `shuyi-app` asks the browser through a miniquad JS
  plugin of the engine's own, as `crates/cli/web/gamepad.js` and its
  Rust side `crates/app/src/pads/web.rs` do.

Why it matters:

- **The game first.** No game ticket asks. Once the game offers 2340's
  motion row, its players get this default at the next pin bump, with
  no code of its own.
- **Other 2D games:** a player who turned animations off in their
  system gets a game that moves less from its first frame, before they
  find the options screen.

Graphics are a skin
([ADR-0002](../../docs/adr/0002-inherited-rules-layers-determinism-skins-actions-tests-licenses.md)
§4, `AGENTS.md` rule 9). The fact sets a setting's default, the setting
changes only the look, and no rule reads either, so replays don't
depend on them.

## Nick input

None.

## Scope

**In:**
- **The fact.** `PlatformFacts::reduced_motion: bool` (3053's type):
  whether the system asks for less motion. `false` from
  `PlatformFacts::from_profile`, and so in `Default`: a profile can't
  know it, the backend reads it. Its doc says it is read once at start
  and sets only the default of the player's motion setting.
  `PlatformFacts` is `#[non_exhaustive]` (ADR-0039), so the field is
  additive.
- **The default motion**, on `Ctx` (`crates/ui/src/ctx.rs`):
  - `Ctx::default_motion(&self) -> Motion`: `Reduced` where
    `self.platform.reduced_motion`, else `Full`;
  - `Ctx::default_settings(&self) -> Settings<G::Settings>`:
    `Settings::default()` with `motion` at `default_motion()`;
  - `load_settings` uses `default_settings()` in place of
    `Settings::default()` in both places: no settings saved for the
    profile, and settings that can't be read. So a first start, and a
    new profile, start at `Reduced` when the system asks;
  - a saved form is read as saved: a player who picked `Full` keeps
    `Full`, whatever the system says.
- **Restore defaults** (`OptionRow::restore_defaults`): the restored
  settings take `ctx.default_motion()`, so restoring doesn't hand
  `Full` back to a player whose system asks for less. The rest is as
  `Settings::restored` does it.
- **The order at load.** Check that `Setup::ctx` puts `Ctx::platform`
  in place before `with_storage` (3053 writes it so). If not, move it,
  with a comment saying why.
- **`shuyi-app` reads the system** (`crates/app/src/motion.rs`, new,
  split by target as `pads.rs` is):
  - `pub(crate) fn system_prefers_reduced() -> bool`, called once by
    `run` after `facts_for` returns the profile's facts, which it sets
    `reduced_motion` on. Where it can't tell, `false`.
  - **The web** (`motion/web.rs`, wasm32): a new plugin of the
    engine's own, `crates/cli/web/motion.js`, named `shuyi_motion`,
    with one function, `shuyi_reduced_motion()`: 1 where
    `window.matchMedia("(prefers-reduced-motion: reduce)").matches`,
    else 0 (0 also where `matchMedia` is missing or throws). The Rust
    side imports exactly that as a `safe fn`, with its own
    `PLUGIN_VERSION` and a `shuyi_motion_crate_version` export, in the
    manner of `pads/web.rs`.
  - **Windows** (`motion/windows.rs`, `cfg(windows)`):
    `SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, &mut value,
    0)`, with `SPI_GETCLIENTAREAANIMATION` as `0x1042` and `value` an
    `i32` (a `BOOL`). It is declared in an `unsafe extern "system"`
    block with `#[link(name = "user32")]`, which mingw-w64 provides.
    Less motion where the call returns nonzero and `value` is 0.
  - **macOS** (`cfg(target_os = "macos")`): `/usr/bin/defaults read
    com.apple.universalaccess reduceMotion`, through
    `std::process::Command`. Less motion where it succeeds and prints
    `1`. A key never set makes it fail: `false`.
  - **Linux** (`cfg(target_os = "linux")`): `gsettings get
    org.gnome.desktop.interface enable-animations`, found on `PATH`
    (distributions put it in different places). Less motion where it
    succeeds and prints `false`. Where `gsettings` or GNOME's key is
    missing (KDE, say), the answer is `false`.
  - Every other target answers `false`.
  - The rules on what the system answered are pure functions in
    `motion.rs`, built on every target so each CI runner tests all
    three. `None` is a command that didn't run or failed, and the text
    is trimmed:
    - `windows_answer(called: bool, animation: i32) -> bool`;
    - `macos_answer(stdout: Option<&str>) -> bool`;
    - `gnome_answer(stdout: Option<&str>) -> bool`.
  - No new crate: `windows-sys` isn't needed for one call, and neither
    `defaults` nor `gsettings` needs a binding. `cargo deny` sees no
    change.
  - `unsafe` stays in the platform backend. The comment on
    `unsafe_code` in `crates/app/Cargo.toml` names every file with an
    `#[allow(unsafe_code)]`, now with `motion/web.rs` and
    `motion/windows.rs`. Each `unsafe` block has a `// SAFETY:`
    comment.
  - If Sonar flags a command found on `PATH`, argue it in
    `tools/sonar-triage/policy.jsonc` (the `sonar-triage` skill): the
    arguments are fixed and only one word of the output is read.
- **`cargo shuyi web`** (`crates/cli/src/web.rs`): `SHELL` gains
  `motion.js`, and `index.html` loads it after `gamepad.js`, between
  the bundle and `load("%WASM%")`. Three tests, as gamepad's are:
  - `motion_js_provides_exactly_the_functions_the_app_imports`;
  - `motion_js_version_matches_the_app`;
  - `index_html_loads_motion_js_between_the_bundle_and_the_game`.

  `crates/cli/web/README.md` gets a section like *The engine's own
  `gamepad.js`*, and its list of files a line for `motion.js`. Other
  tickets add files to `SHELL` too; whichever lands later sets its
  length.
- **Either order with 2349** (the system's languages): if it is done,
  `motion.js` and `motion/web.rs` follow its `languages.js` and the web
  half of its `languages.rs`, `index.html` loads `motion.js` beside
  `languages.js`, and the comment on `unsafe_code` names its files and
  this ticket's. If not, follow `gamepad.js` and `pads/web.rs`, and
  2349 adds `languages.js` beside `motion.js` (its clause says so).
- **Either order with 3039** (the web build's notices): if it is done,
  `motion.js` gets a row in its table of `SHELL`'s files, as the
  engine's own beside `gamepad.js` (its test fails until it does); if
  not, 3039 lists it (its clause says so).
- **Either order with 3052** (the web shell's progress and errors): if
  it is done, `SHELL` holds its `shell.js` and the page ends with
  `if (shuyiShell.start(…)) load("%WASM%");`, and `motion.js` loads
  between the bundle and that line. If not, nothing to do (3052 keeps
  `motion.js` when it changes the page).
- **Docs:** 3053's `### facts(&self) -> PlatformFacts` section in
  `docs/platform-backends.md` says what `reduced_motion` is, that a
  backend reads it once at start, and that it answers `false` where it
  can't tell. The docs of `Settings::motion`, `Ctx::default_motion`,
  `Ctx::default_settings` and `PlatformFacts::reduced_motion` say the
  default follows the system. No ADR: 3053's ADR already says a fact
  may come from what the backend reads of the system.
- **What changes for the game:** additive. `PlatformFacts` gains a
  field and `Ctx` two methods. A game's own `Platform` builds its facts
  with `from_profile` or `Default`, so it answers `false` and nothing
  changes. If CI's `api` job finds a break all the same, the completion
  notes carry its `Breaking:` line (ADR-0038). The game's PR that moves
  the pin follows this one and needs no code; say so in the PR.

**Out (do not do):**
- following a change of the system's setting: the fact is read once a
  run, and watching it (`matchMedia`'s change event,
  `WM_SETTINGCHANGE`) would break "constant for a run" (3053's ADR).
  Once a profile's settings are saved (a change to any setting saves
  them all), its motion is what was saved. A later change in the
  system shows in a new profile, or after Restore defaults;
- a profile whose settings were saved before 2340: it has no `motion`
  and reads `Full`, 2340's serde default, as any saved form does;
- the system's other preferences (less flashing, more contrast, larger
  text): not facts here;
- KDE's animation speed and other desktops' settings: they answer
  `false`;
- a screen that asks a new player about motion: the game's, built on
  2340's row;
- what `Reduced` does to shakes, flashes, transitions and idle motion:
  2340's.

## Implementation steps

1. Read 2340's and 3053's results: `Motion` and `Settings::motion`
   (`crates/ui/src/settings.rs`), `PlatformFacts`, `Setup::ctx`,
   `HeadlessPlatform::with_facts`, `crates/app/src/facts.rs` and `run`
   in `crates/app/src/lib.rs`. Then `load_settings` and `load_profile`
   in `crates/ui/src/ctx.rs`, `OptionRow::restore_defaults`, and
   `crates/app/src/pads.rs`, `pads/web.rs` and
   `crates/cli/web/gamepad.js` with their tests in
   `crates/cli/src/web.rs`. Check whether 2349, 3039 and 3052 are done,
   for their clauses.
2. Add `PlatformFacts::reduced_motion`, `Ctx::default_motion` and
   `Ctx::default_settings`. Use them in `load_settings` and in Restore
   defaults, and check the order in `Setup::ctx`. With their tests.
3. Write `crates/app/src/motion.rs`: the three answer functions and
   their tests first, then the macOS and Linux reads, then
   `motion/windows.rs`. Set the fact in `run`.
4. Write `motion.js` and `motion/web.rs`. Add the file to `SHELL` and
   `index.html`, with the three tests and the README's section.
5. Update the comment on `unsafe_code` in `crates/app/Cargo.toml`, and
   write the docs.
6. Run the gates, the web build included.

## Acceptance criteria

- [ ] A `Ctx` whose `platform.reduced_motion` is true, given an empty
      storage, has `settings().motion == Motion::Reduced`. With it
      false, `Full` (unit tests).
- [ ] With the fact true, a profile whose saved settings say `Full`
      keeps `Full`, and settings that can't be read load as `Reduced`
      (unit tests).
- [ ] With the fact true, a new profile, once switched to, starts at
      `Reduced` (unit test).
- [ ] `App::load` on `HeadlessPlatform::with_facts` with
      `reduced_motion` true and no saved settings gives a game whose
      settings are `Reduced` (test in
      `crates/platform/src/app/tests.rs`).
- [ ] Restore defaults on the toy's options screen gives `Reduced`
      with the fact true and `Full` with it false, and keeps the layout
      (tests in `crates/ui/src/screens/options/tests.rs`).
- [ ] `windows_answer`, `macos_answer` and `gnome_answer` give the
      answers above, and `false` for a failed call, no output, or
      output they don't know (unit tests, run on every CI runner).
- [ ] `system_prefers_reduced()` returns without a panic on the
      Windows, Linux and macOS runners of CI's `test` job (a test that
      calls it and ignores the answer).
- [ ] `motion.js` provides exactly the functions `motion/web.rs`
      imports, its version equals the app's, and `index.html` loads it
      between the bundle and the game (tests in
      `crates/cli/src/web.rs`); the web build passes.
- [ ] `Cargo.lock` gains no crate.
- [ ] Every existing test passes unchanged: the default facts say
      `false`, so every test starts at `Full` as before.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `default_motion` both ways; `load_settings` with no saved
  settings, saved `Full`, saved `Reduced` and unreadable settings,
  each with the fact on and off; a new profile; Restore defaults; the
  three answer functions on true, false, failed and unknown answers;
  the system read on each CI runner.
- Unit (`crates/cli/src/web.rs`): `motion.js` against the app's
  imports, its version, and its place in `index.html`.
- Integration: `App::load` on the headless backend with the fact on.
- Snapshot: none. The fact sets a setting, and 2340's tests cover what
  `Reduced` draws.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
