---
id: "2223"
title: "Credits are the game's: license lists from its files with ADR-0002 §8 as the default, every third-party file covered by a credit in shuyi check, and the credits in every build's NOTICES.txt"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3039"]
nick_input: none
completed:
---

# 2223 — Credits are the game's

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Godot hands a game its engine's credits and license
texts as data (`Engine.get_license_info`, `get_copyright_info`) and
leaves the screen to the game
(<https://docs.godotengine.org/en/stable/about/complying_with_licenses.html>).
Asset stores attach a license to every listing. Bevy leaves credits to
each game. None of them checks that a game's own third-party files are
credited.

What Shuyi has (`crates/assets/src/credits.rs`, checked 2026-10-21):

- the credits file: `CreditsRon`, `CreditsFile`, `FileCredit` with
  `hidden` and `private`;
- `merge`, which joins it with the audio manifest's credits into the
  plain `Credits` a game's credits screen shows;
- `uncredited` (line 184), which finds the bought files of the private
  folder that no credit covers (game ADR-0051). `private-assets --pin`
  runs it (`crates/cli/src/private_assets.rs`) on the folders that
  `shuyi.ron`'s `private_assets.credits` part names (`Credited` in
  `crates/cli/src/project.rs`).

Three problems:

1. **The game's policy is hard-coded in the engine** (`AGENTS.md` rule
   4):
   - `credits::LICENSES` (line 84) is "game ADR-0013's code and font
     lists";
   - `audio::LICENSES` (`crates/assets/src/audio.rs`, line 31) cites
     game ADR-0013 and ADR-0027;
   - the errors cite them as if they were this repository's ADRs:
     `(ADR-0013: …)` in `credits.rs`, `(ADR-0027: …)` in `audio.rs`.
   Every game gets these lists and can't change them.
2. **The lists don't follow ADR-0002 §8.** The credits file takes the
   code-and-font list for every group. So an Art credit under
   `CC-BY-4.0`, which ADR-0002 §8 allows for art, is refused, and one
   under `MIT` passes.
3. **Only bought files are checked.** A CC0 or CC-BY pack an agent
   downloads into a game's `files/` needs no credit today. The templates
   (`templates/grid`, `templates/blank`) ship Terminus (OFL-1.1) with a
   README line and no credits file. `THIRD_PARTY_ASSETS.md` says a
   game's "own registry lists them", and no template has one.

3039 writes `NOTICES.txt` into every web build from the crates, the web
shell's scripts and the game's license texts, and 3004 writes it into
every package. This ticket adds the game's credits to that file, makes
the license lists the game's, and checks that every third-party file
has a credit.

Why it matters: Visions of Shuyi keeps its credits file as it is (its
lists are ADR-0002 §8's), and can turn the check on for its own folders.
Any game made from a template is credited and checked from its first
commit. For the end goal (2026-10-19), an agent that pulls an art pack
into a new game is stopped by `shuyi check` until it credits the pack.

## Nick input

None. ADR-0002 §8 stays the default every game gets. A game that sets a
wider list decides that for itself, and the licenses the engine itself
ships (rule 7, `deny.toml`) don't change.

## Scope

**In:**
- **License lists by group, ADR-0002 §8 by default**
  (`crates/assets/src/credits.rs`, `crates/assets/src/audio.rs`):
  - `credits::CODE_LICENSES` (the 14 code ids; if 2526 added it first,
    keep it), `credits::ART_LICENSES`
    (`CC0-1.0`, `CC-BY-3.0`, `CC-BY-4.0`), and
    `credits::default_licenses(group) -> &'static [&'static str]`. Art,
    Music and SoundEffects take the art list, Fonts the code list and
    `OFL-1.1`, Software the code list.
  - `credits::LICENSES` stays, documented as ADR-0002 §8's font list
    (the code list and `OFL-1.1`). `audio::LICENSES` stays, documented
    as its art and audio list with `Own`. The game re-exports both, so
    nothing it uses breaks.
  - A credits file may say `licenses: { Art: ["CC0-1.0"] }`: a group
    named there takes that list instead of its default. `CreditsFile`
    gains `licenses: BTreeMap<CreditGroup, Vec<String>>` (empty by
    default, and `#[serde(default)]` in the file's `RawFile`) and
    `CreditsFile::allowed(group)`: the file's list for the group, or its
    default. An empty list is an error, and so is a list for Music or
    SoundEffects (their credits are the audio manifest's).
  - The audio manifest may say `licenses: [...]` the same way, kept as
    `AudioManifest::licenses: Vec<String>` (empty by default). It
    replaces the art list for the manifest's credits. `Own` is always
    allowed there.
  - Errors name the group and where its list came from: `license
    "GPL-3.0" is not allowed for Software (ADR-0002 §8: MIT, …)`, or
    `(this file's licenses for Art: …)`. No game ADR is cited any more.
  - CI builds `shuyi-assets` without its `std` feature for
    `thumbv7em-none-eabihf`, so this code uses `alloc` only. Walking
    folders stays in `shuyi-cli`.
- **The pack's version.** `CreditsFile`, `FileCredit` and
  `AudioManifest` gain fields, so their binary form changes. Make
  `pack::VERSION` (`crates/assets/src/pack.rs`) the next number. The
  `CREDITS` and `MANIFEST` sources of `crates/assets/src/pack/tests.rs`
  gain a `licenses` and a `files` entry, and the new snapshot of
  `the_binary_form_of_every_engine_kind_is_pinned_by_the_pack_version`
  is accepted.
- **A credit covers files anywhere:**
  - `FileCredit.files: Vec<String>` (`#[serde(default)]`): paths from
    the project's root, `/`-separated, each a file, or a folder ending
    in `/`;
  - `FileCredit::covers_file(path)`: `path` is one of them, or is inside
    one of the folders;
  - `credits::uncovered<'a>(file: &CreditsFile, own: &[&str], paths:
    &[&'a str]) -> Vec<&'a str>`: the paths that no credit's `files`
    and no `own` entry covers. `own` entries take the same form as
    `files`;
  - an empty path, one starting with `/`, one holding `\`, or one with
    a `..` part is an error;
  - `uncredited`, `covers` and `private` don't change.
- **`check` runs a credits pass when `shuyi.ron` has a `credits` part**
  (a new `CreditsCheck` part of `Config` in
  `crates/cli/src/project.rs`, `deny_unknown_fields` like the others):
  ```ron
  credits: (
      file: "crates/game/files/credits.ron",
      folders: ["crates/game/files"],
      own: ["crates/game/files/en.ron", "crates/game/files/keymap.ron"],
      audio_manifest: None,
  ),
  ```
  - `audio_manifest` is the manifest's path from the project's root, if
    the game has a manifest file. The credits file's errors name it
    (`CreditsRon::new`'s first argument), or "the audio manifest"
    without one. The private folder in its errors is
    `private_assets.credits.private_root` when that part is set, or
    `private`.
  - The part has nothing to do with `private_assets.credits`, which
    stays as it is for `--pin`. A folder that `private_assets` copies
    bought files into is left out of `folders`: `--pin` checks those.
  - A new module, `crates/cli/src/credits.rs`, holds the pass:
    `pass(project) -> Result<Vec<ContentError>, String>`. It reads the
    credits file, walks every file below each of `folders` (recursive,
    `/`-separated paths from the project's root on every OS, sorted),
    and reports:
    - the credits file's own errors;
    - each path that `uncovered` returns, for example
      `crates/game/files/x.png: no credit covers it: add one to
      crates/game/files/credits.ron, or list it in shuyi.ron's
      credits.own`. The credits file itself never needs one;
    - each credit's `files` entry and each `own` entry that names no
      file on disk (a folder, for an entry ending in `/`): a stale
      entry. One in the credits file is reported there, at its line;
      one in `own` is reported in `shuyi.ron`.
  - `contract::check::check` (`crates/cli/src/contract/check.rs`) takes
    the project too, `check(project, game, input)`, and adds the pass's
    errors to its `errors`. Its one caller is `contract::cli::with_game`.
    Without the part, `check` is unchanged. The answer's shape doesn't
    change: these are `Problem`s like the others.
- **The credits in `NOTICES.txt`:** when the `credits` part is there,
  3039's `notices` module gets a section "Works the game credits"
  before the crates: each credit with its group, title, author, license
  and source. `works(project) -> Result<Vec<notices::Work>, String>`
  in `crates/cli/src/credits.rs` builds it, and `web` (in
  `crates/cli/src/lib.rs`, which has the project) hands it to
  `web::run`.
  - Unlike the credits screen (`merge`), hidden credits are listed:
    vendored software the player never sees still owes its notice.
  - When `audio_manifest` names one, every third-party credit of the
    audio manifest is listed too. A new `audio::credits_of(file,
    source) -> Result<BTreeMap<String, Credit>, Vec<ContentError>>`
    reads the manifest and checks only its credits, so it needs no
    sound files.
  - A bought work is listed when one of its `private` files is in the
    build, as `merge` does. The build's bought files are the ones
    `private_assets.rs` walks for `--pin`: each of
    `private_assets.credits.folders` in the checkout. With no checkout
    or no such part, there are none.
  - A game that also credits the web shell's scripts (as Visions of
    Shuyi does) sees them twice in the file. That is harmless: don't
    try to merge them.
  - The `wasm` job's check of `dist/web/NOTICES.txt`
    (`.github/workflows/ci.yml`, 3039) also looks for the heading
    `Works the game credits`.
- **Templates and Hello, Grid:**
  - `templates/grid/crates/game/files/credits.ron` and
    `templates/blank/crates/game/files/credits.ron` hold one credit:
    ```ron
    (credits: [
        (id: "terminus", group: Fonts, title: "Terminus Font",
         author: "Dimitar Toshkov Zhekov",
         source: "https://terminus-font.sourceforge.net/", license: "OFL-1.1",
         files: ["crates/game/files/font/"]),
    ])
    ```
  - their `shuyi.ron` gets the `credits` part, with `folders:
    ["crates/game/files"]` and the template's other files in `own`,
    each as `crates/game/files/<name>`: `en.ron`, `keymap.ron` and
    `palette.ron`, and in `grid` also `stage.map`;
  - `examples/hello-grid/files/credits.ron` the same, its credit
    covering `files/font/`, `files/tiles.png` (whose glyphs are stamped
    from the font, `THIRD_PARTY_ASSETS.md`) and
    `files-src/ter-u16n.bdf`. Its `shuyi.ron` gets the part, with
    `folders: ["files"]` and, each as `files/<name>`, `en.ron`,
    `intro.dlg`, `keymap.ron`, `palette.ron` and `stage.map` in `own`;
  - the READMEs' "Art and licenses" sections say that a third-party
    file needs a credit or `check` fails. In `THIRD_PARTY_ASSETS.md`,
    the line 3039 rewrote about what a game ships adds that a game's
    own third-party files are credited in its credits file, which
    `check` checks when `shuyi.ron` has a `credits` part.
- **Docs:** `docs/contract.md`'s `check` row says it runs the credits
  pass when the project has a `credits` part. `docs/formats.md`'s line
  on `CreditsRon::new` says where the license lists come from.
- **An ADR** (the next free number, the `write-adr` skill): a game's
  license lists and credits are its own; ADR-0002 §8 is the default;
  the pass is turned on by the `credits` part; the engine's own lists
  (rule 7) don't change.

**Either order:**
- **From 3004 (done):** `shuyi export` takes the game's works in
  `ExportCx::of` (`crates/cli/src/export.rs`) with
  `notices::game_texts`, and every exporter writes them into its
  package's `NOTICES.txt`; `shuyi web` takes them the same way in
  `lib.rs`'s `web`. Add `works(project)` to the `notices` module and
  call it from both, so every package gets the credits. CI's `export`
  job exports `examples/hello-grid` and checks each package with
  `tools/export-check/check.sh`: give hello-grid a `credits` part, and
  check that the `web` and `linux` packages' `NOTICES.txt` show its
  section "Works the game credits" (in `check.sh`, or in a test in
  `crates/cli/src/export/tests.rs`).
- **2218** (writers of the text formats): if it is done, the writers of
  `CreditsRon` and `AudioManifestRon` write `licenses` and `files`, and
  their round trips cover them. If not, 2218 writes the files as they
  are then.
- **2501** (templates as data, done): the credits file and the
  `credits` part go in the `base` part of `templates/shuyi-default/`.
  The grid stage part's `stage.map` joins the `credits` part's `own`
  list through that part's own `shuyi.ron` piece, which `shuyi new`
  merges into the base's (ADR-0042).
- **2505** (`shuyi template check`): if it is done, it runs the credits
  pass on each game it makes. If not, 2505 adds it (its scope says so).
- **From 1134 (done)** (the skill pack): `shuyi-ship`
  (`skills/shuyi-ship/SKILL.md`) teaches the credits file, the
  `credits` part and the pass, and that an agent never widens a game's
  `licenses` to make a check pass; `check-skills` passes.
- **2352** (parked: engine screens machine-translated): if it is done,
  the credits section of `NOTICES.txt` says which languages' engine
  screens a game shows machine-translated. If not, 2352 adds that, as
  its scope says.
- **Either order with 2032:** if 2032 is done, extend its test
  `licenses_are_deny_tomls_allow_list` (`crates/assets/src/credits.rs`'s
  tests) so `CODE_LICENSES` plus `OFL-1.1` is also checked against
  `deny.toml`'s allow list, and keep its sentence naming `deny.toml` in
  `LICENSES`'s new doc comment; if not, nothing to do (2032 checks
  `CODE_LICENSES` too).

**Out (do not do):**
- An engine screen of credits or notices. Godot leaves it to the game,
  `merge` already hands a game the data, and only consoles and stores
  with no folder a player can browse need one (3014–3016, parked).
  Write it when one of them, or a game, asks.
- The crates' and the web shell's notices (3039), and the engine's own
  non-crate files in native packages (3004).
- Audio files in the pass: each cue already names its credit in the
  audio manifest, so a game leaves its audio folders out of `folders`.
- Reporting shipped files no load reads (2516).
- Scanning where a file came from, or reading a license out of a file.
- Legal advice, and any change to the licenses the engine itself ships
  (rule 7, `deny.toml`).

## Implementation steps

1. Write the ADR.
2. In `credits.rs`: the lists, `licenses`, `allowed`, `files`,
   `covers_file`, `uncovered` and the new error texts. In `audio.rs`:
   `licenses` and `credits_of`. Move the existing tests' expected texts
   (`credits.rs` tests, and the `(ADR-0027: …)` one in `audio.rs`).
3. Make `pack::VERSION` the next number, add the new fields to the pack
   test's `CREDITS` and `MANIFEST`, and accept the new snapshot. Add
   them to the `BAD_CREDITS` and `MANIFEST` seeds of
   `crates/assets/src/format/builtin_tests.rs`.
4. Add the `CreditsCheck` part to `Config`, write
   `crates/cli/src/credits.rs` (`pass`, `works`), and pass the project
   to `check::check`.
5. Add the section to 3039's `notices` module, wire it through `web`,
   and extend the `wasm` job's check.
6. Write the credits files and parts of the templates and Hello, Grid.
   Update the READMEs, `THIRD_PARTY_ASSETS.md`, `docs/contract.md` and
   `docs/formats.md`.
7. Say in the PR that the game's credits file loads unchanged, that the
   error texts changed, and that the pack version moved (rebake any
   pack). The game's PR that moves the pin follows it. The completion
   notes carry `Breaking:` lines (2010): `CreditsFile`,
   `FileCredit` and `AudioManifest` gain public fields, the pack
   version, and `contract::check::check`'s new argument.
8. Run the gates.

**Size:** about 700 to 800 changed lines with tests. If the diff runs
long, the credits in `NOTICES.txt` (step 5: `works`,
`audio::credits_of`, the section and the `wasm` job's check) move to a
follow-up ticket of the same milestone (the `write-ticket` skill,
blocked by this one), and the completion notes say so.

## Acceptance criteria

- [ ] An Art credit under `CC-BY-4.0` loads. One under `MIT` fails,
      naming ADR-0002 §8's art list.
- [ ] A file with `licenses: { Art: ["CC0-1.0"] }` refuses a
      `CC-BY-4.0` Art credit, naming the file's list.
- [ ] A fixture in the shape of a real game's credits file, with
      made-up names (two bought Art works, an `OFL-1.1` font, `MIT`,
      `0BSD` and `Zlib` software), loads with no `licenses` set.
- [ ] In a fixture project with the `credits` part, adding
      `files/pack.png` makes `check` fail, naming it; a credit with
      `files: ["files/pack.png"]` makes it pass.
- [ ] A credit's `files` entry that names no file fails `check` as
      stale.
- [ ] The `templates` CI job's `cargo shuyi check` passes on both
      templates with their `credits` parts.
- [ ] Hello, Grid's `dist/web/NOTICES.txt` lists Terminus Font under
      "Works the game credits", and the `wasm` CI job checks it. A
      hidden credit in a fixture is listed there too.
- [ ] `pack::VERSION` is one higher, and the binary-form snapshot holds
      the new fields.
- [ ] `check` without a `credits` part answers as before (the existing
      tests pass unchanged).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit:
  - the default lists by group, and a file's `licenses` replacing one
    group's;
  - bad `licenses` (empty, an audio group) and bad `files` paths;
  - `covers_file`, and `uncovered` with `own`;
  - `credits_of` on a manifest with `Own` and CC-BY credits, and its
    `licenses`;
  - parsing the `credits` part;
  - the pass on a temporary folder: an uncovered file, the credits file
    left alone, stale `files` and `own` entries;
  - the notices section with hidden and bought credits.
- Property: the no-panic test of the RON formats
  (`no_bytes_panic_a_ron_sidecar` in
  `crates/assets/src/format/builtin_tests.rs`) runs on credits files
  and manifests with `licenses` and `files`.
- Snapshot: the pack's binary forms.
- Integration: `check`'s answer on a fixture project with an uncovered
  file and a stale entry (snapshots of the text and the JSON).

## Completion notes

*(Filled in by the session that completes the ticket.)*
