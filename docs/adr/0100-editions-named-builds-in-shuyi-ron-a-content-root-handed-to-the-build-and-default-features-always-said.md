# ADR-0100: Editions: named builds in shuyi.ron, a content root handed to the build, and default features always said

- **Status:** Accepted
- **Date:** 2026-11-10
- **Related tickets:** 3082, 3081, 3075, 3085

## Context

A game wants a public demo that holds none of the rest of the game. Hiding
the rest behind a menu doesn't do it: the files are still in the package
(ADR-0070), and a new game embeds its whole `files/` folder with
`include_dir!` (`templates/shuyi-default/base/crates/game/src/lib.rs`). Code
can't be taken out by a flag at run time either. Cargo's features are
additive: a package's `default` list, a feature that lists another, or a
dependency can turn on the feature that holds the full game's code, so a demo
build that only "doesn't ask for it" may still have it.

What exports already have: one `export` part (3075) with features and files
beside the program, per-target overrides (`export.targets`), the resolved-build
guard that refuses debug features by Cargo's own resolution of each planned
build (`live_link::refuse`, ADR-0096), and the final-package gate that reads the
package back against one `disclosure` part per project (ADR-0070, 3081).

## Decision

1. **Editions in `shuyi.ron`.** `editions` is a map from a name
   (`public-demo`, `full`) to an `Edition` (`shuyi_cli::project::Edition`):
   `package`, `bin`, `default_features` (required, no default), `features`,
   `never_features`, `content`, `beside` and `disclosure`. They are
   independent of targets: `export <target> --edition <name>` builds one on
   any target. Without `--edition` an export builds what it did before.
2. **What an edition replaces.** Its `package` replaces the target's (the
   web build's too), its `bin` the program the package would ship, its
   `features` the `export` part's and the target's list (`--without` leaves
   out one of those, `--with` adds), its `beside` the part's, and its
   `disclosure` the project's part, which the gate then holds the package to.
   `ExportCx` carries what is left in `ExportCx::edition`
   (`export::EditionBuild`: name, default features, never features, content
   root).
3. **Default features are always said.** `default_features: false` adds
   `--no-default-features` to every build the exporter plans
   (`Exporter::cargo_args`), so the resolved-build guard and the disclosure
   report see the same flags the compiler does.
4. **`never_features` are refused by Cargo's resolution.** The same
   `cargo tree` resolution that refuses debug features checks each build for
   the edition's `never_features` (a bare name is the built package's,
   `package/feature` a dependency's), before anything is prepared or built,
   naming how the feature was reached (`live_link::unwanted`). Naming one in
   `features` too is refused at once.
5. **The content root reaches the build as `SHUYI_CONTENT`.** An edition's
   `content` is a folder of the project (checked: relative, with `/`, no
   `..`, a folder). The exporter sets `SHUYI_CONTENT` to its absolute path for
   the desktop and web builds, and takes the variable away from a build with
   no content root, so the shell can't choose what ships. The engine's
   template reads it in `build.rs` (`rerun-if-env-changed`) and hands
   `SHUYI_FILES` to `include_dir!`, falling back to `files/`; `cargo run`
   is unchanged. A game that embeds its files another way reads the same
   variable.
6. **Reports.** The package is named `<program>-<version>-<edition>-<target>`
   and goes in `dist/export/<edition>/<target>` by default, so two editions
   never sit beside each other (which the gate would refuse as files this
   export didn't make). `export-report.json` gains `edition`;
   `exposure-report.json`'s `inputs` gains `edition` and `content_root`
   beside the package, flags and features it already had. Contract 43.
7. **The Playdate.** Its library is always built without default features,
   and its pack is baked by the game's tools from the files they embed, so a
   Playdate export refuses an edition with `default_features: true` or a
   `content` root, in `prepare`, before anything is built.

## Consequences

- A demo is made of less, not hidden: what its folder doesn't hold and its
  features don't compile is not in the program, and the gate (with the
  edition's canaries) refuses a demo built from the wrong folder.
- The build cache flips between editions: the content root changes the
  build script's output and the features change the build, so exporting one
  edition after another rebuilds the game's crate.
- A game's build script must read `SHUYI_CONTENT` for `content` to have an
  effect. One that doesn't embeds what it names itself; only the gate's
  canaries would notice. The template does, and the book and the `shuyi-ship`
  skill say so.
- `NOTICES.txt` is still made from `cargo metadata` with the features, not
  `--no-default-features`, so a demo's notices can name a crate only the full
  game uses (a superset, as before).
- Playdate editions can't choose their files until the bake can (a follow-up
  ticket).
- Public API: `ExportCx`, `ExportOutput`, `BuildInputs` and `web::Options`
  gain fields, a break for a literal of them.

## Alternatives considered

- **A content root chosen by a feature in the game's code** (`#[cfg(feature
  = "full")] include_dir!(…)`): no engine change, but `content` in
  `shuyi.ron` would only describe, not choose, and the report couldn't say
  what was embedded. Games can still do this; the edition's `features` and
  `never_features` cover it.
- **Layered roots or a `leave_out` filter staged into the target folder:**
  saves copying shared files into a demo folder, but adds a staging step and
  hides what ships behind a computed folder. Separate roots keep "the demo
  is exactly this folder"; a filter can come later if games ask.
- **Editions per target (`export.targets.<id>.edition`):** mixes what is
  distributed with the hardware it runs on; an edition is the same on every
  target.
- **A default for `default_features`:** either value silently does the wrong
  thing for some edition; writing it is one line.
- **One `disclosure` part with per-edition lists inside it:** the edition's
  own part, replacing the project's, keeps the part's shape and its checks
  (ADR-0070) unchanged.
