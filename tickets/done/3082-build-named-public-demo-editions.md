---
id: "3082"
title: "Build named demo editions from separate code and content roots"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["3081", "3075"]
nick_input: none
completed: 2026-11-10
---

# 3082 — Build named public-demo editions

## Context

A runtime lock does not hide content already packaged. The default template embeds its whole files directory with include_dir!, while Cargo features are additive. A game needs a reproducible smaller program and content root for a public demo. The final-artifact gate (3081) measures the result; export feature support (3075) supplies the build seam.

**From 3081 (done):** every export already runs the gate, `export::disclosure::gate`, after its exporter, against `shuyi.ron`'s `disclosure` part, so nothing here invokes it again: an edition needs its own part (or its own lists in one), since the part is one per project today. `exposure-report.json`'s `inputs` holds the package, program, cargo arguments, default-features flag and resolved features, read from `Exporter::cargo_args`, so a build with an edition's flags shows there if `cargo_args` returns them; add `edition` and `content root` to `BuildInputs`, and raise the contract's version. Compressed bytes a game makes itself aren't searched: the fixture's canaries stay plain.

## Nick input

None.

## Scope

**In:**
- Named distribution editions in shuyi.ron, independent of hardware targets.
- An edition selects a game package or binary, explicit Cargo default-feature behavior and features, content root and allowed loose files before compilation and baking.
- The exporter records the resolved edition and invokes the exposure gate on its final artifact.
- **From 3085 (done):** put the edition's selected package and final
  default-feature/feature flags in the exporter plan
  (`Exporter::cargo_args`). The common export path resolves every build
  there with Cargo (`export::live_link::refuse`) before preparation or
  compilation, so edition builds are checked with no separate edition
  bypass. A build flag the guard doesn't know (`live_link::tree_args`)
  refuses the export: teach it the flag in the same PR.
- **From 2586 (done):** the dialogue preview a game hands a writer is in
  the book's chapter 6, section *A preview for your writer*
  (`site/content/learn/book/games-and-screens.md`). Add a sentence there:
  a preview program is built as an edition that leaves the game's own
  scripts out, so it holds only the scenes in its folder. Check that an
  edition can do that (a content root without the scripts folder) and
  name it in the sentence.

**Out (do not do):**
- Automatic guesses about which mechanic, story or art a game considers secret.
- Runtime menu locking as a substitute for omission.
- A full-game API change without a Breaking: completion note.

## Implementation steps

1. Add edition parsing and validation to the CLI config and export contract.
2. Build the selected package with resolved Cargo flags and selected input tree; make the template show separate public-demo and full roots.
3. Audit the final archive and teach the book and shuyi-ship skill the disclosure boundary.

## Acceptance criteria

- [x] A fixture game's public-demo and full editions both play on desktop and web.
- [x] Future maps, dialogue and code canaries appear only in full, even after pack extraction and binary string inspection.
- [x] Export reports name edition, package, features and content root.
- [x] The book's chapter is updated (2802), or the completion notes have a Book: line saying why not.
- [x] All gates in the run-gates skill pass.

## Tests required

- Integration: build both editions and inspect the final files, packs and binary/Wasm strings.
- Regression: default Cargo features do not silently turn full-only code on in public-demo.

## Completion notes

`shuyi.ron` has `editions`: each names its `package`, `bin`,
`default_features` (required), `features`, `never_features`, `content` (the
folder its program embeds), `beside` and `disclosure`, each in place of what
the target would build. `shuyi export <target> --edition <name>` builds one on
any target, into `dist/export/<edition>/<target>`, named
`<program>-<version>-<edition>-<target>`.
[ADR-0100](../../docs/adr/0100-editions-named-builds-in-shuyi-ron-a-content-root-handed-to-the-build-and-default-features-always-said.md)
has the choices.

- **Before compilation:** `default_features: false` puts
  `--no-default-features` in every build the exporter plans
  (`Exporter::cargo_args`), so 3085's resolved-build guard checks the same
  flags; `live_link::tree_args` already knew the flag. The same resolution
  refuses an edition's `never_features` (`live_link::unwanted`), naming how
  Cargo reached each. The content root reaches the desktop and web builds as
  `SHUYI_CONTENT`, and a build with none has the variable taken away.
- **The template** reads it: `build.rs` hands `SHUYI_FILES` to
  `include_dir!`, `files/` unless an edition says; its `shuyi.ron` shows a
  public-demo and a full edition, commented out. The build script now says
  `rerun-if-changed=build.rs`, so it no longer reruns on every change in the
  package.
- **Reports:** `export-report.json` has `edition`; `exposure-report.json`'s
  `inputs` has `edition` and `content_root` beside the package, flags and
  resolved features. The gate holds the package to the edition's
  `disclosure` part if it has one, else the project's. Contract 43.
- **Tests** (`crates/cli/src/export/tests/editions.rs`): parsing, the choice
  of package, program, features, files and folder, every refusal, and a
  stand-in game (no dependency; its build script embeds its content root
  as `include_dir!` does, and a default `full` feature holds a line of code)
  whose public-demo and full editions are built by the real Cargo and
  exported for the web and this system's desktop: map, dialogue, code and
  packed canaries are only in full's final archive, the demo's pack lists
  only its own entry, the demo's own `disclosure` passes, and the program
  runs and lists its files. A demo pointed at the full folder is refused
  (`SH3012`) and leaves only the report. The regression: the demo with its
  defaults kept, or a feature it turns on that turns `full` on, is refused
  before building.
- **By hand**, a new grid game from the template with a `files-demo` folder,
  future files in `files/` and its code canary behind a default `full`
  feature: both editions exported for Windows and the web; the demo's
  packages hold none of the three canaries and full's hold all three (in the
  `.exe` and the `.wasm`); both programs opened their window and ran; both
  web builds played to the first stage in the browser with no console error.
  `linux` and `macos` editions weren't run here; the stand-in test runs
  `linux` in CI's `test` job.

Deviations and limits:

- The Playdate's pack is baked by the game's tools from the files they
  embed, so a Playdate export refuses an edition with a `content` root or
  `default_features: true`. Follow-up: 3304 (the bake takes an edition's
  content root), which 3300's packed exports will use too.
- `NOTICES.txt` still comes from `cargo metadata` without
  `--no-default-features`, so a demo's notices can name a crate only the
  full game uses. Follow-up: 3305.
- A game whose build script doesn't read `SHUYI_CONTENT` embeds what it names
  itself; only the edition's canaries would catch it. The book says so.
- `content` is one folder, as the ticket's title says; a demo folder copies
  the shared files (the font, the keymap). A filter over one folder was
  considered and left out (ADR-0100, *Alternatives*).
- 2586 landed first: *A preview for your writer* (chapter 6) now says to
  build the preview as an edition whose `bin` is the preview and whose
  `content` is a folder without the game's scenes, linking *A public demo*.
  An edition can: the stand-in's `writer-preview` edition ships its second
  program from the demo's folder, and holds no canary
  (`an_edition_ships_another_program_of_the_package_with_its_own_files`).
  Hello, Grid's own `hello-grid-preview` embeds `files/` by its manifest's
  path, not `SHUYI_CONTENT`, so it wasn't built as an edition. 2917's
  first plain statement is now in the book's *A public demo*.

Book: `targets-and-export.md`, *A public demo*, and `games-and-screens.md`,
*A preview for your writer*; the `shuyi-ship` skill,
`docs/contract.md` and `docs/targets/playdate.md` too.

Breaking: shuyi-cli: export::ExportCx: a new public field, `edition`; a literal of it adds it.

```migration
since: 0.2.0
what: ExportCx has a new field, the edition it builds.
old: ExportCx { .. }
new: ExportCx { edition, .. }
before: |
  ExportCx { wasm_opt, .. }
after: |
  ExportCx { wasm_opt, edition: None, .. }
find: ["ExportCx { wasm_opt, .. }"]
```

Breaking: shuyi-cli: contract::export::ExportOutput: a new public field, `edition`; a literal of it adds it.

```migration
since: 0.2.0
what: ExportOutput, export-report.json, names the edition it built.
old: ExportOutput { .. }
new: ExportOutput { edition, .. }
before: |
  ExportOutput { debug, .. }
after: |
  ExportOutput { debug, edition: None, .. }
find: ["ExportOutput { debug, .. }"]
```

Breaking: shuyi-cli: export::disclosure::BuildInputs: two new public fields, `edition` and `content_root`; a literal of it adds them.

```migration
since: 0.2.0
what: BuildInputs, the exposure report's inputs, names the edition and its content root.
old: BuildInputs { .. }
new: BuildInputs { edition, content_root, .. }
before: |
  BuildInputs { version, .. }
after: |
  BuildInputs { version, edition: None, content_root: None, .. }
find: ["BuildInputs { version, .. }"]
```

Breaking: shuyi-cli: web::Options: two new public fields, `no_default_features` and `content`; a literal of it adds them, or ends with `..Options::default()`.

```migration
since: 0.2.0
what: The web build's Options can leave the default features off and name a content root.
old: Options { release, features, require_wasm_opt }
new: Options { release, features, require_wasm_opt, no_default_features, content }
before: |
  Options { release: true, features: Vec::new(), require_wasm_opt: false }
after: |
  Options { release: true, ..Options::default() }
find: ["require_wasm_opt: false }"]
```

The game needs nothing to move its pin: it builds no literal of these, and a
game with no `editions` exports as before.
