---
id: "2110"
title: "Save migrations: an opt-in chain of typed steps from older versions, golden fixtures of every version as the shape guard, and the engine's own records on it"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2511"]
nick_input: none
completed:
---

# 2110 — Save migrations and golden fixtures

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6 and
Unreal 5.8. The others read old saved data after an update:

- Unreal remaps renamed classes and properties as old assets load (Core
  Redirects,
  `dev.epicgames.com/documentation/en-us/unreal-engine/core-redirects-in-unreal-engine`)
  and versions each system's data with `FCustomVersion`;
- Unity keeps a renamed field's data with `FormerlySerializedAs`;
- Bevy's `bevy_save` steps saved data through each version with a
  `Migrate` trait (`docs.rs/bevy_save`);
- Godot's guide to saving games says nothing about versions.

What Shuyi has (read on 2026-10-20):

- `crates/core/src/save.rs`: `SaveFormat` (`NAME`, `VERSION`) and
  `SaveHeader<F>`, which reads a save's version whatever the rest holds.
  A save with no `version` field is an error. Its module doc says a save
  of another version is "refused, never migrated" (from game ADR-0039).
- `crates/ui/src/input/bindings.rs`: `PlayerKeys::from_ron` reads
  `PLAYER_KEYS_VERSION` (2) and, by hand, version 1
  (`OLDEST_READABLE_VERSION` and a private `VersionOnly`). Any other
  version gives the default keys.
- `crates/ui/src/profiles.rs`: `Profiles::from_ron` refuses any version
  but `PROFILES_VERSION` (1), and `profiles::load` then rebuilds the list
  and saves it over the file it could not read.
- `crates/ui/src/ctx.rs`, `Ctx::load_settings`: settings that can't be
  read become the defaults with a warning, and the player's next change
  writes over them. Their saved form is the game's (`GameSettings::to_ron`
  and `from_ron`, `crates/ui/src/settings.rs`). The templates' form
  (`templates/*/crates/game/src/settings.rs`, the same file in both) has
  no version. 2511 (MVP) moves the blank template's form into the engine
  as `impl GameSettings for ()`, still with no version.
- One golden file: `crates/ui/src/input/bindings/saved_before_1106.ron`,
  which must load and save back byte for byte.
- Plans: 1121 and 3017 (parked) and 3006 each plan "a migration" of
  their own. 2724 (parked) makes visual novel saves survive patches, for
  its own format only. Nothing gives the engine one way to do it.

Why it matters:

- Visions of Shuyi is paid and will be patched. Under game ADR-0039 any
  change to a saved type makes every player's saves "from an incompatible
  version". The game wrote its own golden-file guard (game#0821). Whether
  the game's saves read their older versions is the game's call; this
  ticket gives it the means.
- The engine's own records have the flaw for every game on it: a change
  to the saved key bindings or profile list resets them, and settings a
  build can't read are lost at the player's next change. A player who
  opens an older build (a beta branch, a Steam Deck not yet updated, a
  cloud save synced from a newer build, 3037) loses them too.
- The end goal (2026-10-19, `docs/decisions.md`) is games made by
  agents. A fixture test is how an agent learns that its edit to a struct
  broke saves: tests are the review.

**Why typed steps, not a value tree.** A step over a value tree such as
`ron::Value` would tie the chain to RON (`AGENTS.md` rule 10), and RON's
`deserialize_any` keeps no struct or variant names (ron 0.12.2,
`src/de/mod.rs`: an identifier goes to `handle_any_struct` and is
dropped), so a "rename this variant" step can't be written over it. A
step that reads the old version into its own frozen Rust type and
converts it works with any serde format, keeps `shuyi-core` `no_std` with
serde only, and the compiler checks it. serde's `default` and `alias`
already cover an added or renamed field.

## Nick input

None. The chain is opt-in, and the engine's own records are technical.
Whether Visions of Shuyi's saves read their older versions is the game's
choice (game ADR-0039); the game's ticket that adopts this asks Nick what
players keep after a patch.

## Scope

**In:**
- **`shuyi_core::save`** (`no_std`, serde only, no new dependency):
  - `SaveFormat` gains `const UNVERSIONED: Option<u32>`, defaulted to
    `None`: the version a save with no `version` field is read as, for a
    form saved before it had one (2511's `()` settings, or a game's own).
    `SaveHeader` reads a missing `version` as it. `None` keeps today's
    "missing field" error.
  - `trait Migrate: SaveFormat + DeserializeOwned`:
    - `const OLDEST: u32`, defaulted to `Self::VERSION`: a format that
      sets nothing reads only its own version, as today;
    - `fn upgrade<'de, D: Deserializer<'de>>(version: u32, from: D) ->
      Result<Self, D::Error>`: reads a save of an older `version` (from
      `OLDEST` to `VERSION - 1`) into that version's own frozen type
      and converts it, step by step, to this one. The value it returns
      is of this version: a `version` field in it says `VERSION`. The
      default is never called.
  - `enum SaveError { Newer { found, newest }, TooOld { found, oldest },
    Unreadable(String) }`, whose `Display` names both versions ("version
    3 is newer than 2, the newest this build reads").
  - `fn read_version<'de, F: Migrate, D: Deserializer<'de>>(version:
    u32, from: D) -> Result<F, SaveError>`: this version as it is, an
    older one through `upgrade`, any other refused.
  - The module docs: raise `VERSION` for any change to the saved form;
    keep each old type frozen in a `v<N>` module with its own
    `#[serde(rename)]`; an added `#[serde(default)]` field still gets a
    new version, read by `Self::deserialize`; `UNVERSIONED` is for a
    form that had no version. A doc test shows a chain of two steps. The
    "refused, never migrated" sentence goes.
- **`shuyi_assets::ron`** (beside `parse_ron`):
  - `fn read_save<F: Migrate>(text: &str) -> Result<F, SaveError>`:
    `SaveHeader<F>` first, then `read_version` over
    `ron::Deserializer::from_str`. Another text format adds its own
    `read_save`; the chain doesn't change.
  - **The guard:** `fn check_saved_forms<F: Migrate + PartialEq + Debug>(
    fixtures: &[(&str, &str)], write: impl Fn(&F) -> String) ->
    Result<(), String>`, over (file name, text) pairs named
    `v<N>_<what>.ron`:
    1. each name has that form, and the text's version (`SaveHeader`)
       is its `N`;
    2. there is a fixture of `F::VERSION`;
    3. each fixture of `VERSION` reads, and `write` gives its text back
       byte for byte, CRLF compared as LF (`ron`'s pretty writer ends
       lines as the platform does, as
       `a_file_saved_before_input_was_generic_round_trips_byte_for_byte`
       in `crates/ui/src/input/bindings/tests.rs` handles);
    4. each fixture from `OLDEST` to `VERSION - 1` reads; when a
       fixture of `VERSION` has the same `<what>`, the upgraded save
       equals it;
    5. each fixture below `OLDEST` is refused as `TooOld`, so dropping a
       version is a choice a test shows.
    A failure lists every problem, one per line, and ends with what to
    do: "a saved form changed: raise `VERSION`, keep the old fixtures
    (they must still read, through `Migrate::upgrade`), and add
    `v<N+1>_*.ron` written by this build", followed by the text this
    build writes, ready to copy. This is the shape guard: a change to the
    saved form without a new version fails rule 3. A fixture should hold
    every variant of the save's enums, since the guard can't see a shape
    no fixture holds; the docs say so.
- **The engine's own records on it** (`shuyi-ui`):
  - `PlayerKeysFile` implements `SaveFormat` (`"PlayerKeys"`, 2) and
    `Migrate` (`OLDEST` 1: version 1 reads as the current type, whose
    `pad` is `#[serde(default)]`, with its `version` then set to 2).
    `PlayerKeys::from_ron` reads through `read_save`;
    `OLDEST_READABLE_VERSION` and `VersionOnly` go. Its repairs and
    their warnings stay. A new `PlayerKeys::read(text, def) ->
    Result<(Self, Vec<String>), String>`, which `from_ron` wraps, tells a
    whole refusal (`Err`) from a repair.
  - `ProfilesFile` implements `SaveFormat` (`"Profiles"`, 1) and
    `Migrate`; `Profiles::from_ron` reads through `read_save`, and its
    error names both versions. (`profiles::migrate`, which moves keys
    from before profiles, is another thing and stays.)
  - **The engine's settings** (`impl GameSettings for ()` in
    `crates/ui/src/settings.rs`, from 2511): its saved form gets
    `version: u32` and implements `SaveFormat` (`"Settings"`, 1,
    `UNVERSIONED` 0) and `Migrate` (`OLDEST` 0: version 0 is 2511's
    form, frozen in a `v0` module and converted). So a record a game
    wrote before this ticket still reads, and 2511's test of the blank
    template's record still passes.
  - **A record refused whole is kept aside.** When the settings, the key
    bindings or the profile list can't be read (unreadable, newer, or
    older than the oldest), `Ctx::load_settings`, `Ctx::load_player_keys`
    and `profiles::load` copy the text to `<key>-refused`
    (`settings-refused`, `keybindings-refused`, `profiles-refused`)
    before anything writes the key, with a warning naming the copy. A
    later refusal replaces the copy. The keys are valid `Storage` keys,
    and the two kept per profile pass `fits_every_profile`. The copy is
    a plain `Storage::write`, so 3045's flushed writes cover it in
    either order.
  - **Fixtures of every version:** `crates/ui/tests/saves/keybindings/`
    (`v1_keys_only.ron`; `v2_before_1106.ron`, moved from
    `crates/ui/src/input/bindings/saved_before_1106.ron`, with its
    round-trip test pointed there; a v2 with buttons),
    `crates/ui/tests/saves/profiles/` (`v1_two_profiles.ron`) and
    `crates/ui/tests/saves/settings/` (`v0_changed.ron`, as 2511's form
    writes it, with every field changed and a layout picked;
    `v1_changed.ron`, the same settings; `v1_default.ron`). The unit
    tests of the modules that own the file types run `check_saved_forms`
    over each folder. They read it with `std::fs::read_dir` under
    `CARGO_MANIFEST_DIR`, so a new file is picked up, behind
    `cfg(feature = "std")` as `cargo xtask check-no-std` asks. The file
    types derive `PartialEq` and `Debug` for the guard. A fixture stays
    while its version is read.
- **The grid template** (`templates/grid/crates/game/src/settings.rs`;
  after 2511 the blank template has no settings file):
  - the settings' `File` gets `version: u32`, derives `PartialEq` and
    `Debug`, and implements `SaveFormat` (`"Settings"`, 1) and
    `Migrate`; `Prefs::from_ron` reads through `read_save`. A new game
    has no older saves, so it sets no `UNVERSIONED`;
  - a unit test in `settings.rs` runs `check_saved_forms` over
    `crates/game/tests/saves/settings/` (`v1_default.ron`,
    `v1_changed.ron`);
  - each template's `AGENTS.md` gets a short "Saves" section. The grid
    template's says what the test checks and what to do when it fails.
    The blank template's says the engine checks its settings, and that a
    saved form of the game's own gets a `VERSION`, fixtures and a test
    like the grid template's.
  - **Either order with 2501:** if the templates are made of parts, the
    change goes in the part that holds the grid template's settings
    file. If 2501 moved the grid stage onto the `()` settings, no
    template has a settings file left, and the engine's settings above
    cover it.
- **An ADR** (the next free number in `docs/adr/`): saved forms read
  their older versions through typed steps, opt-in per format; the
  fixture guard; records refused whole are kept aside. It replaces game
  ADR-0039's "refused, never migrated" as the engine's rule for saves.
  Packs stay refused (ADR-0023 §5).
- **Either order with 2802:** if the book is written, its chapter "Saves
  and settings" gets a section on versions, the steps and the fixtures.
  If not, 2802 writes it from the ADR.
- **Either order with 1134:** if the skill pack is done, its
  `shuyi-test` skill gets a paragraph on saved forms (`VERSION`, the
  steps, the fixtures and what the guard's failure asks); if not, 1134
  writes it.
- **2111** leaves `SaveHeader` as it is (its Out): a game reads its own
  save's `made` with the rest. So `UNVERSIONED` is the only change to
  `SaveHeader`'s visitor, in either order.
- **Either order with the settings fields of 2333, 2340, 2342, 2349 and
  3043:** one that lands first is in version 1 of each form here. One
  that lands after raises the `VERSION` of each versioned form it
  changes and adds a fixture; the guard's failure says how.
- **For the game:** nothing breaks. `SaveFormat` gains a const with a
  default, so a game's impl compiles unchanged; `shuyi_core::save` and
  `shuyi_assets::ron` gain items; `PlayerKeys::from_ron` and
  `Profiles::from_ron` keep their signatures. The engine writes new
  `-refused` keys in a player's storage. Say so in the PR.

**Out (do not do):**
- Packs: ADR-0023 §5 keeps them refused; bake again.
- Visions of Shuyi's own saves: the game adopts this in its own ticket,
  after Nick's answer.
- Asset files in older forms: 3006's palettes and 3017's target profiles
  may read an old form through `Migrate`; rewriting a game's files to
  the current form is 2517.
- A serde-traced shape hash (`serde-reflection`, `schemars`): the
  fixtures catch changes to the shapes they hold, with no dependency.
  ADR-0023's Alternatives keep it as an idea if fixtures prove too easy
  to get around.
- A value tree with rename, default and remove helpers (see Context).
- `TipsSeen` (`crates/ui/src/tips.rs`): one id per line, no shape to
  change, and an id the game drops is ignored. It gets no version.
- Versions for the settings forms of the toy (`crates/ui/src/toy.rs`)
  and hello-grid (`examples/hello-grid/src/settings.rs`): test and
  example code that no player's save holds.
- The stamp of the engine, game and content that made a save or a
  record (2111: a game puts its `Made` in its own save, and
  `SaveHeader` doesn't read it).
- Keeping the last good copy on every write, and flushed writes (3045).
- Binary saves; cloud sync (3037).

## Implementation steps

1. Read `crates/core/src/save.rs`, `crates/assets/src/ron.rs`,
   `PlayerKeys::from_ron` and its tests, `Profiles::from_ron` and
   `profiles::load`, `Ctx::load_settings` and `Ctx::load_player_keys`,
   `impl GameSettings for ()` in `crates/ui/src/settings.rs` (2511), the
   grid template's `settings.rs`, and game#0821 (the game's guard, whose
   rules this generalizes).
2. Write `UNVERSIONED`, `Migrate`, `SaveError` and `read_version` in
   `shuyi-core`, with a toy format of three versions (a renamed field, a
   field split in two, a renamed enum variant) and a second toy whose
   first form had no version.
3. Write `read_save` and `check_saved_forms` in `shuyi-assets`, tested on
   the same toys.
4. Move `PlayerKeysFile`, `ProfilesFile` and the `()` settings onto it;
   add the keep-aside; add the fixtures.
5. Change the grid template's settings and both templates' `AGENTS.md`.
   Templates are outside `cargo fmt --all`: format them as CI's
   templates job checks them.
6. Write the ADR, settle the *Either order* clauses, and run the gates.
   If the diff passes about 600 lines without the fixtures, move the
   grid template's part into a follow-up ticket (the `write-ticket`
   skill) and say so in the PR.

## Acceptance criteria

- [ ] `shuyi-core` gains no dependency and still builds for
      `thumbv7em-none-eabihf` (CI's `no-std` job).
- [ ] The toy format reads its v1 and v2 fixtures as v3, and refuses a
      v4 as `Newer` and a v0 as `TooOld`, each message naming both
      versions.
- [ ] A format that sets no `OLDEST` refuses every other version, and
      one that sets no `UNVERSIONED` refuses a save with no `version`,
      as today.
- [ ] Adding a `#[serde(default)]` field to the toy's current type
      without raising `VERSION` makes `check_saved_forms` fail with the
      message above (a test does it with a second type).
- [ ] The saved key bindings, profile list and `()` settings load from
      every fixture with no warning, and `v2_before_1106.ron` still
      round-trips byte for byte.
- [ ] 2511's test of a settings record written by the blank template
      before 2511 still passes: the record reads as version 0.
- [ ] Settings, key bindings and a profile list of a newer version are
      copied to their `-refused` key before anything writes over them (a
      test with `MemoryStorage`: load, change a setting, both texts
      kept).
- [ ] A new game from the grid template passes its saves test, and one
      from the blank template passes its tests (CI's templates job).
- [ ] The ADR is written and listed in `docs/adr/README.md`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `read_version` for a current, older, newer, too old and
  unreadable save; `SaveHeader` on a save with no `version`, with and
  without `UNVERSIONED`; each rule of the guard, passing and failing;
  the keep-aside for each of the three records.
- Property: a toy save of random values written at v1 reads at v3 as
  the v1-to-v3 conversion of those values.
- Fixtures: the engine's three folders; the grid template's folder, in
  CI's templates job.

## Completion notes

*(Filled in by the session that completes the ticket.)*
