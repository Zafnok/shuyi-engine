# ADR-0096: An export asks Cargo what each build turns on, and refuses the live link before anything is built

- **Status:** Accepted; amends ADR-0058 §2
- **Date:** 2026-11-08
- **Related tickets:** 3085 (this), 3004 and 3075 (the exporters and
  their features), 3082 (editions, which go through the same check)

## Context

ADR-0047 makes the live link a debug build's alone, and ADR-0058 §2
said an export never turns it on: `export::no_live_link` refuses a
build whose arguments name `link`. A review of PRs 183 and 212 found that
the arguments aren't the only way. A game package with
`default = ["link"]`, a `devtools = ["link"]` that `export.features`
lists, or a dependency whose own features ask for `shuyi-app/link` (under
any name it gives the engine) all turn the link on with no `link` in the
arguments.

What a build turns on is Cargo's to say. Its feature resolver unifies
features over the packages of one build, honours `default`, aliases and
`dep/feature` forwarding, follows renamed dependencies, leaves out
dependencies for other targets (by `cfg`, which Rust flags can set), and,
with resolver 2, leaves out dev-dependencies when no test is built. That
last one matters here: `shuyi-app` turns its own `link` on in its
dev-dependencies to test it, and an ordinary export must still pass.

## Decision

1. **Resolve each planned build with Cargo.** `export::live_link::refuse`
   runs over every build `Exporter::cargo_args` returns: the desktop
   slices, the web build, the Playdate's device and Simulator libraries.
   Each is turned into `cargo tree` arguments by `live_link::tree_args`
   from the build's own arguments: `-p`, `--target`, `--features`/`-F`,
   `--no-default-features` and `--all-features` kept as they are; the
   subcommand (`build` or `rustc`), `--bin`, `--lib`, `--crate-type` and
   `--release` left out, since they change no feature. A flag it doesn't
   know refuses the export rather than being guessed at. The tree runs
   with `-e normal,build` (what the program is made of), `--prefix depth`
   and the format `|{f}|{p}`, in the project's folder.
2. **The same Rust flags.** A new defaulted method,
   `Exporter::rustflags(triple)`, gives the flags an exporter adds to the
   machine's; the Playdate's device and Simulator flags (its
   `--cfg=shuyi_playdate_sdk` among them) come from it for both the build
   and the check. The check hands them to Cargo in
   `CARGO_ENCODED_RUSTFLAGS` exactly when the build does: a release build,
   or an exporter that adds flags.
3. **What is refused.** The package built with its own `link` on, and
   `shuyi-app` or `shuyi-platform` with `link` on, anywhere in the tree
   (`live_link::ENGINE`). Another package's feature called `link`, and a
   feature such as `linked`, are not. The refusal (`SH1002`) names each
   package's `link` once: the package's own with its features as built,
   the engine's with the packages it was reached through.
4. **Before anything.** The common export path
   (`contract::export::export`) runs the check after `ExportCx::of` and
   before `Exporter::prepare` (the Playdate's baking) and
   `Exporter::export`: a refused export prepares, bakes, builds, packages
   and runs nothing. `no_live_link` stays first, for its quick answer, and
   in the exporters.
5. **Failure is refusal.** `cargo` not starting, `cargo tree` failing
   (an unknown feature, a broken manifest), or an answer it can't read
   refuse the export with the cause; none means the build is safe.

## Consequences

- Every way Cargo has of turning the link on is caught, by Cargo's own
  resolver rather than one of ours, with no new dependency.
- Each export runs one `cargo tree` per build first: about a second each
  on the engine's workspace, more when Cargo has to fetch a dependency
  the build would fetch anyway.
- A new build flag in an exporter's plan (an edition's, 3082) has to be
  taught to `tree_args`, or the export is refused; the refusal says
  which flag.
- `--without` can't hide the link: a game whose `default` turns it on
  is refused until the default changes.
- The check reads `cargo tree`'s text output, which is stable but not
  JSON; a line it can't read refuses the export rather than passing it.

## Alternatives considered

- **Walk the package's feature map ourselves** (as
  `disclosure::resolved_features` does for the report) — catches
  defaults and aliases, but not dependency forwarding, renaming, targets
  or dev-dependency separation without rewriting Cargo's resolver.
- **`cargo build --unit-graph`** — the exact units, but unstable
  (nightly only); the engine pins stable Rust.
- **`cargo metadata` with features** — its `resolve` unifies across
  every target and dev-dependency, so `shuyi-app`'s own tests would
  refuse every export.
- **Read the built program for the listener** — after the build, too
  late to build nothing, and a symbol search is weaker than the
  resolver's answer.
