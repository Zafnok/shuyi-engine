# ADR-0114: A game's release from a tag: one plan of its targets, one export workflow shared with the engine, and a package holding private files uploaded only for a release

- **Status:** Accepted
- **Date:** 2026-11-13
- **Related tickets:** 3050, 3004, 2011, 2513, 2590
- **Amends:** ADR-0111 §5 (no workflow of the template uploads an artifact
  that embeds the private files)

## Context

3004 built `shuyi export <target>` for `windows`, `linux` (in Valve's
sniper SDK, ADR-0058), `macos` and `web`, and the engine's CI exports
its example with a job of its own. A game had no way to turn a tag into
packages a player can download: Linux and macOS packages come from CI
only (Nick's machine is Windows with the GNU toolchain), and Bevy's game
template does it with a release workflow on a tag (3050's context).

Three things decide where each package builds: the exporters (each knows
its system), the game's `shuyi.ron` (which targets it ships on), and CI
(which runner, which image). Written three times, they drift.

ADR-0111 §5 says no workflow of the template uploads an artifact that
embeds the private files: anyone signed in to GitHub can download a public
repository's artifacts. A release must hand each package from the runner
that built it to the job that publishes it, and a package of a game with
private files holds them.

## Decision

### 1. One target list: `export --plan`

`shuyi export --plan [--tag <tag>]` is a mode of the `export` operation
(`contract::export::plan`) that builds nothing. It answers an
`ExportPlan`: the version the packages carry (`app.version`, else the
package's, as `export-report.json`), whether it is a prerelease, and one
`PlannedExport { exporter, profile, host, container, ci, why }` per
exporter of each target of `shuyi.ron`'s `targets`. A target with no
exporter is a limit, not an error. With `--tag` it fails (`SH0001`)
unless the tag is `v` and the version.

The `Exporter` trait says it: `host()` (`linux`, `linux-arm64`,
`windows`, `macos`), `container()` and `ci()` (`InCi::Build`, or
`InCi::CheckOnly(why)`), with no default, so a new exporter can't forget
them. The Linux image is one constant, `export::SNIPER`; a test holds the
engine's CI matrix to it. `playdate` is check-only: its SDK's license
doesn't let CI download it (3010).

The op's output schema is `Answer<ExportResult>`, an untagged union of
the package's report and the plan, so the MCP tool's `outputSchema`
covers both answers.

### 2. One export workflow, run by the engine's CI

The base's `.github/workflows/export.yml` is reusable (`workflow_call`):
one target on its runner, in its image, with the setup 3004's job used
(`setup-linux.sh`, Binaryen for the web, MSYS2's GCC for Windows, both
Mac targets), `cargo fetch --locked`, the export, a folder package zipped
with `ditto`, and the artifact `package-<target>`. The engine's
`.github/workflows/export.yml` is a byte-equal copy (a test), and the
engine's `export` job calls it for Hello, Grid with a `check` script
(`tools/export-check/ci.sh`: `check.sh`, then the same exposure report on
a second export). So every engine pull request runs the file a game's
release runs.

`release.yml` runs the gates (`ci.yml`, now also `workflow_call`), the
plan, `export.yml` over the plan's `build` entries, then on a tag only a
GitHub Release with every package, its debug file and `SHA256SUMS`,
attested with `actions/attest` on a public repository, and with
`ITCH_GAME` set, butler pushes to itch.io. Only the `release` job has
write permissions, and no job of a tag's run reads the Actions cache.

### 3. A package holding private files is uploaded only for a release

`export.yml` uploads a package of a game with private files (its action
`.github/actions/private-files` is there) only when its caller passes
`upload-private: true`. `release.yml` passes it on a pushed tag, whose
GitHub Release publishes the same files, and in a private repository,
whose artifacts are private. A dry run of a public repository builds
the packages and uploads none, and says so in the summary. This amends
ADR-0111 §5: a release's own packages are the one artifact that may hold
the private files, since the release makes them public anyway. Caches
stay as §5 says: no `target/` cache for such a game, and none at all on a
tag.

The private files' action gains an input `required` (`"true"` by
default): a release's dry run passes `false`, and without the key warns
and builds on the public files.

## Consequences

- A game names its targets once, in `shuyi.ron`; the plan, the release's
  matrix and the engine's checks follow it. 3071's `linux-arm64` and
  3012's `steam-deck` fill three methods and are released.
- The engine's PR CI exercises `export.yml`, not `release.yml`: the
  release first runs in a game's repository. actionlint and zizmor read
  it on every engine PR, and the `templates` job runs the plan.
- Adding required trait methods breaks any `Exporter` outside the engine
  (none is known).
- For seven days after a release of a public game with private files,
  its packages are artifacts too: the same bytes the release publishes.
- GitHub's Arm runner is in the private repositories' table too (its docs
  on 2026-11-13, billed as other runners there), so the plan's
  `linux-arm64` entries are kept for a private repository.

## Alternatives considered

- **Each export job uploads to a draft release.** No artifact holds the
  private files, but every export job would need `contents: write`.
- **The plan as an operation of its own.** It answers what `export`
  would build; a separate op splits one target list over two operations.
- **A static matrix in `release.yml`.** It drifts from `shuyi.ron` and
  from the exporters, which is what the plan prevents.
- **The engine's export job kept beside a call of `export.yml`.** Every
  pull request would export Hello, Grid twice; the `check` input keeps
  the engine's checks in one job.
