---
id: "2236"
title: "Frozen format fixtures: each release keeps a copy of every engine format's fixtures, and CI reads the copies the version rule still covers"
type: infra
milestone: After MVP
model: implementation
effort: high
status: todo
blocked_by: ["2005"]
nick_input: none
completed:
---

# 2236 — Frozen format fixtures from each release

## Context

[2005](2005-the-first-release-on-crates-io.md) makes the first release
and writes the version policy as an ADR. The policy covers the
engine's text formats: every format `Formats::standard()` registers,
plus keymaps, language packs, cinematics, `.map`, `.dlg`, target
profiles and `shuyi.ron`. Its rule for them:

1. a field added with `#[serde(default)]` is not breaking;
2. a renamed or removed field or value is still read for one more
   minor release (`#[serde(alias)]`, or a field read and ignored);
3. a change of meaning always comes with a new name;
4. anything else is breaking, and its `Breaking:` line names the
   format and the field
   ([ADR-0038](../../docs/adr/0038-the-public-api-gate-a-listing-diff-and-semver-checks-on-every-pr-and-a-breaking-line-for-each-break.md) §6).

Nothing checks point 2. The fixtures change with the formats:
`crates/assets/src/format/builtin_tests.rs` holds them as string
constants (`PALETTE`, `MAP`, `SCRIPT`, `FILM`, `MANIFEST` and more), and
a PR that renames a field edits the constant in the same diff. So the
tests read today's form only. A rename without `#[serde(alias)]` passes
every gate, and every game's files stop parsing when it updates. Most
of the engine's file types have `#[serde(deny_unknown_fields)]` (33 in
`crates/assets/src`, and `Config` in `crates/cli/src/project.rs`), so
an old name is an error, not a value quietly dropped.

[2517](2517-format-upgrades-shuyi-upgrade-rewrites-a-games-files-and-shuyi-check-warns-on-old-names.md)
builds the tools the rule leans on: a format lists its old names
(`Format::changes`), `shuyi check` warns on them and `shuyi upgrade`
rewrites a game's files. It tests a change someone lists, with a
fixture of its old form. A change nobody lists isn't caught there
either. Its Out leaves "frozen copies of every format's fixtures from
each release, tested to still read" to a separate gate: this ticket.

The pack already pins its binary form: a snapshot test in
`crates/assets/src/pack/tests.rs` writes every engine kind's binary
form, so a change makes `pack::VERSION` the next number
([ADR-0023](../../docs/adr/0023-the-pack-one-indexed-file-of-baked-assets-proven-equal-to-its-sources.md)).
The text forms have no such pin.

What there is to freeze (read on 2026-10-24):

- `pack/tests.rs`'s `sources()`, "a game of one file of every kind the
  built-in formats read": the constants above, the keymap
  `crates/assets/src/keymap/sample.ron`, the engine's
  `targets/playdate.ron`, and binary files made by helpers
  (`png_header`, `wav`, `ogg_file`, `png_file`);
- its `formats()`: the standard formats, with the formats of a game's
  types registered over test kinds (`Dlg::<Cue>`, `map_text()`,
  `KeymapRon::<sample::Action>`, `CinematicRon::<Shot, ()>`,
  `lang_pack()`, `Ron::<Table>`);
- the snapshot test's `form!` list: each kind and the file it loads;
- the engine's target profiles, `targets/*.ron` at the root;
- `shuyi.ron`, read by `Config::parse` (`crates/cli/src/project.rs`),
  whose tests use small inline texts only.

Formats are plugins, and standard files always load
([ADR-0007](../../docs/adr/0007-formats-are-plugins-standard-files-always-optimized-ones-optional.md)).
This ticket makes "always" hold across releases.

## Nick input

None.

## Scope

**In:**

- **The fixtures as files,** in `crates/assets/tests/formats/`, at the
  paths `sources()` gives them: `audio/audio.ron`, `data/credits.ron`,
  `fonts/atlas.ron`, `data/palette.ron`, `data/palette@onebit.ron`,
  `data/lights.ron`, `data/names.ron`, `dialogue/gate.dlg`,
  `maps/field.map`, `films/title.ron`, `lang/en/ui.ron`,
  `lang/xx/lang.ron` and `lang/xx/ui.ron`:
  - each of those constants in `builtin_tests.rs` becomes an
    `include_str!` of its file, so every test reads it as before.
    `TABLE` (a test kind, not an engine format) and `BAD_CREDITS` (a
    broken file) stay constants;
  - each file ends with a newline (`.editorconfig`). The crate's tests
    pass as before, and the pack's snapshot doesn't change;
  - `sources()`'s list of files and `formats()` move from
    `pack/tests.rs` to `builtin_tests.rs`, as `pub(crate) fn
    source_files()` and `pub(crate) fn game_formats()` (names to
    taste), used by the pack's tests and the test below;
  - the folder has no `main.rs`, so cargo builds no test target from
    it.
- **The project file's fixture,** `crates/cli/tests/formats/shuyi.ron`:
  one `shuyi.ron` that sets every part of `Config` (`command`, `game`,
  `check_keys`, `check_text`, `rules`, `font_atlas`, `web`,
  `glyph_tiles`, `private_assets`, `targets`, `target_profiles`), each
  part with all its fields. A test in `project.rs` parses it and takes
  the `Config` apart with no `..`, checking that each part is set. A
  new part then fails to compile until the fixture has it.
- **The frozen copies,** `crates/assets/tests/frozen/v<X.Y.Z>/`, one
  folder per release, never edited once merged:
  - `assets/`: the files of `crates/assets/tests/formats/`, the keymap
    `crates/assets/src/keymap/sample.ron` as `data/keymap.ron`, and
    every `targets/*.ron` under `targets/`;
  - `project/shuyi.ron`: the project file's fixture.
- **The breaks,** `crates/assets/tests/frozen/breaks.ron`: the files
  that stop reading on purpose, a list of `(before: "X.Y.Z", file:
  "<path in the copy>", ticket: "<id>")`, empty at first.
  - An entry skips that file in every copy of a release before
    `before`.
  - It is for the breaks point 4 allows, and the ticket's `Breaking:`
    line names the format and the field.
- **The releases the rule covers,** in
  `crates/assets/src/format/frozen_covered.rs`. It is test code,
  declared under `#[cfg(test)]` in `format.rs` and included in the
  cli's test with a `#[path]` module, so the rule has one home:
  - a release folder is named `v<major>.<minor>.<patch>`. Any other
    entry in `frozen/` but `breaks.ron` fails;
  - the version now is the crate's `CARGO_PKG_VERSION` (both crates
    have the workspace's one version). A copy of a later release fails;
  - covered: the copies of the version now's `major.minor`, and those
    of the newest earlier `major.minor` that has copies. By point 2, a
    file of the last minor release reads in this one, so older copies
    aren't read;
  - examples: at 0.3.2, with copies 0.1.0, 0.1.2, 0.2.0 and 0.3.1, the
    covered ones are 0.2.0 and 0.3.1. At 0.3.0 (raised by a break, not
    yet released), with copies 0.1.0, 0.2.0 and 0.2.1, they are 0.2.0
    and 0.2.1;
  - it reads `breaks.ron`. An entry whose file no copy before `before`
    holds fails, so a mistyped entry is found.
- **The assets test,** `crates/assets/src/format/frozen_tests.rs`,
  `#[cfg(test)]` beside `builtin_tests` (which has the test kinds):
  - its work is a function over a frozen root, so a unit test can run
    it on a scratch root; the real test runs it on
    `crates/assets/tests/frozen/` (from `CARGO_MANIFEST_DIR`, with
    `std::fs`);
  - for each covered copy, a bundle of its `assets/` files, plus each
    file of `source_files()` the copy doesn't hold. Those are the
    binary files the text ones name: PNG, WAV and Ogg Vorbis are
    outside standards, which the rule doesn't cover;
  - a table from a file's path in the copy to how it is read, taken
    from the `form!` list: `audio/audio.ron` loads `AudioManifest`,
    `maps/field.map` loads `GridMap<u8, NoFeature>`, any
    `targets/*.ron` loads `TargetProfile`, and so on. A file that
    another one names is listed as read through it (`lang/en/ui.ron`
    and `lang/xx/ui.ron` through `lang/xx/lang.ron`). A file of a copy
    that the table doesn't know fails, so the table grows with the
    fixtures;
  - each file loads with `game_formats()`, today's formats. Every
    error is collected, and the test fails once, listing each with the
    release, the file and the error (`ContentError` shows
    `file:line:column: message`, and serde's message names the field);
  - the message says what to do: read the old name too
    (`#[serde(alias = "old")]`, or a field read and ignored), as 2005's
    rule says; or, for a break the policy allows, list it in
    `breaks.ron` with the ticket whose `Breaking:` line names it. Never
    edit a copy.
- **The project file's test,** in `crates/cli/src/project.rs`'s tests
  (or a `project/frozen_tests.rs` beside it): each covered copy's
  `project/shuyi.ron` parses with `Config::parse`, with the same rule
  and `breaks.ron`, read from `../assets/tests/frozen/`. Its failure
  names the release and the parse error, and says what to do as above.
- **`cargo xtask freeze-formats [--release X.Y.Z] [--check]`**
  (`crates/xtask/src/freeze.rs`, with its tests; its lines in
  `main.rs`'s `USAGE`, `match` and module doc, and in
  `crates/xtask/Cargo.toml`'s `description`):
  - the version is `--release`, or `version` in the root
    `Cargo.toml`'s `[workspace.package]`, read with `toolchain.rs`'s
    `value_of` (made `pub(crate)`). It must be a plain `X.Y.Z`;
  - its sources are a table in the code, from each file or folder to
    its place in the copy, as listed above;
  - it writes `crates/assets/tests/frozen/v<X.Y.Z>/` byte for byte,
    and refuses a folder that exists: a copy is never written over;
  - `--check` writes nothing. It fails when the folder is missing, or
    when a file is changed, missing or extra against what freezing
    would write now, naming each.
- **The release** (2005's):
  - `.github/workflows/release.yml` runs `cargo xtask freeze-formats
    --check` in a job that every `cargo publish` job needs, so a tag
    without its copy publishes nothing;
  - where 2005 wrote how a release is made (find it with `grep -rn
    "cargo publish" docs .claude/skills`): in the PR that sets the
    release's version, run `cargo xtask freeze-formats` and commit the
    copy, then tag.
- **The gates leave the copies alone:** `_typos.toml`'s
  `extend-exclude` gets `crates/assets/tests/frozen/`, since a copy is
  never edited, not even for a typo.
- **The package:** `crates/assets/Cargo.toml` gets `exclude =
  ["tests/frozen/"]`, so the published crate doesn't grow with each
  release.
- **An ADR** (the next free number; the `write-adr` skill), "Frozen
  copies of the formats' fixtures from each release":
  - what is frozen, and from where;
  - each release freezes in the PR before its tag, and the release
    workflow checks it;
  - the covered releases, from point 2 of 2005's rule;
  - `breaks.ron` for point 4, and a copy is never edited;
  - the test kinds the copies are read with (`Shot`, `Cue`,
    `sample::Action`, the map's terrains) only grow.

  It extends 2005's version-policy ADR and ADR-0007, and doesn't
  restate them. Its row goes in `docs/adr/README.md`.
- **Docs,** in `docs/formats.md`:
  - a section *Frozen copies*, after *The built-in formats*: what is
    frozen, which copies are read, what a failure means and its fixes;
  - *5. Testing it* gets a line: an engine format's fixture is a file
    in `crates/assets/tests/formats/` that uses every field, so each
    release's copy holds every name.
- **Either order with 2517:** if 2517 is done, both tests' failure
  messages also say to list a rename as a `Change` (in the format's
  `changes()`, or in `project::CHANGES` for `shuyi.ron`), so `shuyi
  check` warns on it and `shuyi upgrade` rewrites a game's files, and
  *Frozen copies* links 2517's *Changing a format* in
  `docs/formats.md`; if not, nothing to do (2517 adds both when it
  lands).
- **For the game:** nothing changes. This is test files, `xtask`, a
  workflow step, an ADR and docs. No engine crate's public API moves
  and no public type is added (ADR-0039), so there is no `Breaking:`
  line.

**Out (do not do):**

- Reading copies older than the covered releases: the rule no longer
  promises them. 2517's `shuyi upgrade` carries a game across several
  releases with its change lists.
- The binary formats: packs (refused, never migrated, ADR-0023 §5),
  one-bit pictures (`bitmap::VERSION`), and the outside standards
  (PNG, WAV, Ogg Vorbis).
- Filling in the unused fields of today's fixtures: a field no fixture
  uses isn't checked until a ticket that touches its format adds it
  (the docs line above asks for that).
- A game's own kinds and files: the `files/` of Hello, Grid and the
  templates (their keymaps and maps read the game's types), and their
  `shuyi.ron` files (the fixture covers `Config`).
- `template.ron` (2501 and 2505 version it), saves and stored records
  (2110), and golden tables of outputs (2116).
- Finding or rewriting old names in a game's files (2517).
- A release workflow that commits the copy: a tag's run can't push to
  `main`, so the copy comes in the PR before the tag.

## Implementation steps

1. Read 2005's version-policy ADR and its `release.yml`, ADR-0007,
   ADR-0023, `crates/assets/src/format/builtin_tests.rs`,
   `crates/assets/src/pack/tests.rs`, `crates/cli/src/project.rs`,
   and `crates/xtask/src/main.rs` and `toolchain.rs`.
2. Before moving anything, note for each release tag (`git tag --list
   'v*'`) which sources changed since it: `git diff <tag> --
   crates/assets/src/format/builtin_tests.rs
   crates/assets/src/keymap/sample.ron targets/
   crates/cli/src/project.rs`.
3. Move the fixtures into `crates/assets/tests/formats/`, and
   `source_files()` and `game_formats()` into `builtin_tests.rs`. Run
   `cargo test -p shuyi-assets`: everything passes, and the pack's
   snapshot doesn't change.
4. Write the project file's fixture and its every-part test.
5. Write `freeze-formats` with its tests.
6. Write the copy of each release tagged so far, with `--release`:
   - where a fixture changed since that tag (step 2), write that file
     in the copy from the tag's text instead (`git show <tag>:<path>`;
     for a constant, its text at the tag);
   - the project file's fixture is new: a copy of an earlier release
     keeps `project/shuyi.ron` only if `Config` and its parts didn't
     change since that tag, and drops it otherwise;
   - write `breaks.ron` as `[]`.
7. Write `frozen_covered.rs`, then the assets test and the project
   file's test.
8. Add the step to `release.yml` and the line to the release steps,
   the `_typos.toml` entry and the package's `exclude`.
9. Write the ADR, its row and the docs. Settle the *Either order*
   clause. Run the gates.

## Acceptance criteria

- [ ] Every release tagged before this PR has its folder in
      `crates/assets/tests/frozen/`, and `cargo test -p shuyi-assets`
      and `cargo test -p shuyi-cli` read the covered ones (the PR names
      them).
- [ ] On a scratch root, a copy whose `audio/audio.ron` writes
      `music_fade_ms` as `fade_ms` fails the assets test, and the
      message names the release, the file and `fade_ms`.
- [ ] The covered releases match the Scope's examples (unit tests),
      and a folder of another name, or of a later release, fails.
- [ ] A file listed in `breaks.ron` is skipped in copies before its
      `before` and read in later ones; an entry naming a file that no
      such copy holds fails.
- [ ] A file in a copy that the table doesn't know fails the test.
- [ ] The project file's fixture sets every part of `Config`, and a
      scratch copy whose `project/shuyi.ron` has an unknown field
      fails, naming the release.
- [ ] `cargo xtask freeze-formats` writes the version's folder byte
      for byte and refuses one that exists; `--check` passes on an
      equal folder and fails on a changed, missing or extra file.
- [ ] `release.yml` runs `cargo xtask freeze-formats --check` before
      `cargo publish`, and the release steps say to freeze before the
      tag.
- [ ] The crate's tests pass with the fixtures read from files, and
      the pack's snapshot is unchanged.
- [ ] `cargo package --list -p shuyi-assets` lists nothing under
      `tests/frozen/`.
- [ ] The ADR exists with its row in `docs/adr/README.md`, and
      `docs/formats.md` has *Frozen copies*.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, `frozen_covered.rs`: the covered releases for the Scope's
  examples, for one copy only, and for none (nothing to read); a folder
  that isn't `v<major>.<minor>.<patch>`; a copy of a later release;
  `breaks.ron` skipping a file before `before` and not after, and an
  entry that matches no copy.
- Unit, `frozen_tests.rs`, on scratch roots under
  `std::env::temp_dir()`: today's fixtures frozen as a copy read with
  no error; the `fade_ms` copy fails with the release, the file and the
  field; a file the table doesn't know fails; a listed break is
  skipped.
- Unit, `project.rs`: the fixture sets every part; a scratch copy with
  an unknown field fails, naming the release.
- Unit, `freeze.rs`, on a scratch repository (as `toolchain.rs`'s tests
  make one): the version from a made-up `Cargo.toml` and from
  `--release`; a version that isn't `X.Y.Z`; the folder written byte
  for byte; an existing folder refused; `--check` on an equal folder,
  and on a changed, a missing and an extra file.
- Unit: `release.yml`, read with `serde_norway`, runs `cargo xtask
  freeze-formats --check` in a job its publish job needs (as
  `ci_installs_the_pinned_tools_and_nightly` reads `ci.yml`).
- Integration: the two tests on the real `crates/assets/tests/frozen/`
  pass.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
