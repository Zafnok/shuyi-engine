---
id: "3050"
title: "Release a game from a tag: every target's package built in CI, a GitHub Release with checksums, and an optional itch.io upload with butler"
type: infra
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: ["3004", "2011", "2513"]
nick_input: none
completed:
---

# 3050 — Release a game from a tag

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Bevy's CI template for games
(<https://github.com/bevyengine/bevy_github_ci_template>, its
`.github/workflows/release.yaml`) does this when a tag is pushed:

- it builds Windows, Linux, macOS and web packages;
- it attaches them to the tag's GitHub Release;
- when an `itch_target` is set, it pushes each package to itch.io with
  butler.

Godot's `--export-release` and Unreal's BuildCookRun package a game
from the command line, which is what 3004 does. Their release pipelines
come from community templates or paid services. So what Shuyi lacks is
a workflow in the templates, not a publisher inside the engine.

What Shuyi has today:

- **The templates' CI.** `templates/blank/.github/workflows/ci.yml` and
  `templates/grid/.github/workflows/ci.yml` are the same file. It runs
  the gates on pull requests and on `main`. It says "Commit Cargo.lock
  and add `--locked` to the cargo commands once the game has one", and
  no ticket picks that up. A game's `.gitignore` leaves `Cargo.lock`
  out of its list, so the file can be committed.
- **The engine's CI.** The `wasm` job in `.github/workflows/ci.yml`
  builds Hello, Grid for the web as a PR artifact, and `pages.yml`
  publishes it. Nothing else is published.
- **Exporters.** 3004 builds `shuyi export <target>` for `windows`,
  `linux`, `linux-arm64` (on GitHub's `ubuntu-24.04-arm` runner),
  `macos` and `web`. It exports the examples as 7-day PR artifacts,
  and builds Linux and macOS in CI only. Its Out list has signing and
  store uploads.
- **More packages.** 3012 adds a `steam-deck` package built in the
  Steam Linux Runtime container. 3010 adds a Playdate `.pdx`, built in
  CI only if the SDK's license allows.
- **CI per target.** 2504 gives a game made by the wizard a CI job per
  picked target, on PRs. It is blocked by 2502, the wizard.
- **Releases.** 2005 publishes the engine's crates to crates.io from a
  `vX.Y.Z` tag.
- **Workflow checks (MVP).** 2011 runs zizmor over the root and both
  templates, and `cargo xtask check-pins` holds every `uses:` to one
  commit SHA. 2513 replaces the system-library line with
  `scripts/setup-linux.sh` in the engine and the templates, and runs
  the `templates` job in a bare `ubuntu:24.04` container with no `git`.
- **Skills.** 1134 (done) wrote the `shuyi-ship` skill
  (`skills/shuyi-ship/SKILL.md`: builds, licenses, credits, store
  pages).
- **Debug files.** 2520's Out leaves keeping a release's debug file to
  3004 and to this ticket.

Nothing turns a tag into packages a player can download.

Why it matters:

- **The game.** Visions of Shuyi doesn't need this. Steam (the partner
  account, Steamworks, depots, the store page) stays the game's ticket
  0903 (`docs/decisions.md`, 2026-10-15), so SteamPipe stays out. That
  is why this ticket is After MVP and not MVP.
- **Other 2D games.** A jam game or a small game ships as downloads on
  GitHub and itch.io. Nick's machine is Windows with the GNU toolchain,
  and Linux and macOS packages come from CI only (3004). So a game made
  on a machine like his needs CI to release on three systems.
- **The end goal** (2026-10-19). An agent makes a game "with a web
  build, a Playdate build and the other targets, in under an hour".
  Pushing one tag is the last step it takes.

## Nick input

None. The itch.io job stays off until a game's own repository sets a
variable and a secret, so it brings Nick no account and no agreement.
The engine's own examples are never pushed to itch.io.

## Scope

Each `templates/<name>/` path here means the `base` part of the
engine's template, `templates/shuyi-default/base` (2501), or its place
if 2005 has moved it.

**In:**
- **The export plan: one target list.** `shuyi export --plan [--tag
  <tag>] [--json]` is a mode of 3004's `export` operation, and it builds
  nothing. It reads the `targets` table of `shuyi.ron` (the
  `BTreeMap<String, TargetSetting>` `targets` field of `Config` in
  `crates/cli/src/project.rs`) and answers an `ExportPlan`:
  - `version`: the version the packages carry, the one 3004's
    `export-report.json` writes (if 3048 is done, that is `shuyi.ron`'s
    `app.version` when the part sets one; if not, 3048 makes it so);
  - `prerelease`: whether the version has a pre-release part
    (`1.2.0-beta.1`);
  - `exports`: one `PlannedExport { exporter, profile, host, container,
    ci, why }` per exporter of each target. Targets come in the map's
    order (sorted by id), and a profile's exporters in the order they
    are registered. The `desktop` profile (`targets/desktop.ron`) gives
    `windows`, `linux`, `linux-arm64` and `macos`, and `web` gives
    `web`. If 3004's step 8 moved `linux-arm64` to a follow-up ticket
    that isn't done, that ticket fills its `host()`, `container()` and
    `ci()` when it lands;
  - `host` is `linux`, `linux-arm64`, `windows` or `macos`, the runner
    the exporter builds on in CI. `linux-arm64` builds on Arm, as
    3004's CI does. `container` is an image to build in, or none. `ci`
    is `build` or `check-only`, and `why` says why for `check-only`;
  - a target with no registered exporter (a `Planned` profile such as
    `switch`, or `steam-deck` before 3012) is a `limits` entry that
    names the target and says it is checked but not packaged. It is not
    an error;
  - with `--tag`, the answer fails unless the tag is `v` followed by
    `version` exactly, and its error names both.

  The `Exporter` trait (3004) gains `host()`, `container()` and `ci()`,
  and every registered exporter fills them. The plan needs the profile
  each exporter checks (3004's edit: `windows`, `linux`, `linux-arm64`
  and `macos` check `desktop`). If 3004's trait has no method that says
  so, add `profile()` beside the three.
- **The per-target job.** Each template gets
  `.github/workflows/export.yml`, a reusable workflow
  (`on: workflow_call`, as the engine's `changes.yml` is). Its inputs
  are `target`, `host`, `container` and `dir` (default `.`):
  - it runs on the runner for `host` (`ubuntu-latest`,
    `ubuntu-24.04-arm`, `windows-latest`, `macos-latest`), in
    `container` when it isn't empty, with the setup 3004's CI job uses
    for that exporter: on Linux `bash scripts/setup-linux.sh` (2513),
    and on Windows the MSYS2 mingw-w64 linker the template's
    `.cargo/config.toml` names;
  - it runs `cargo fetch --locked`, then `cargo shuyi export <target>
    --release` in `dir`;
  - a package that is a folder is made into one file on its own runner
    before the upload, because `actions/upload-artifact` keeps no file
    permissions. The web package uses the zip 3004 writes (3052's edit
    to 3004), or a zip of the folder if there is none. The macOS
    package is zipped with `ditto -c -k --keepParent`, which keeps the
    executable bit and the ad-hoc signature;
  - it uploads the package as the artifact `package-<target>`, kept 7
    days, with the `.debug` file beside it if 3004 writes one (2520's
    edit to 3004).
- **The release.** Each template gets `.github/workflows/release.yml`.
  It runs on a pushed tag `v*`, and on `workflow_dispatch` as a dry run
  that skips the last two jobs:
  1. `gates`: the template's `ci.yml`, called as a reusable workflow
     (its `on:` gains `workflow_call:`);
  2. `plan`: on Linux after the same setup as `ci.yml`, `cargo shuyi
     export --plan --tag "$TAG" --json` (no `--tag` on a dry run). The
     `build` entries, read with `jq`, are the job's `matrix` output. In
     a private repository `jq` also drops `linux-arm64`, since 3004's
     `ubuntu-24.04-arm` runner is free for public repositories (check
     on the day whether a private one gets it free too, and keep the
     entry if so). The job fails with a plain message when no entry is
     left. The `check-only` entries, a dropped entry and the limits go
     in the step summary;
  3. `export`: the matrix (`fromJSON` of that output) over
     `export.yml`, after `gates` and `plan`;
  4. `release`, on tags only. It needs no checkout: `gh` works on
     `GH_REPO: ${{ github.repository }}`.
     - each package is named `<crate>-<version>-<exporter>.<ext>`;
     - each `.debug` file, if any, is named
       `<crate>-<version>-<exporter>.debug`. A private repository's
       release is private, and a public one's source is public
       already, so the file shows nothing the repository doesn't;
     - `SHA256SUMS` (the output of `sha256sum`) lists every file
       attached;
     - when the repository is public (`if: ${{
       !github.event.repository.private }}`), artifact attestations are
       made with `actions/attest` (MIT), `subject-checksums:
       SHA256SUMS`. With no predicate it makes SLSA build provenance.
       When the repository is private they are skipped, with a line in
       the summary: private repositories need GitHub Enterprise Cloud
       for them;
     - the notes are the annotated tag's message, read through the API
       (`gh api repos/$GITHUB_REPOSITORY/git/ref/tags/$TAG`, then the
       tag object's `message`) and written to a file. A tag made without
       `-a` has no message, and its notes are `<crate> <version>`.
       `actions/checkout` can fetch an annotated tag as a plain one, so
       the notes don't come from a checkout;
     - then `gh release create "$TAG" --verify-tag --notes-file
       <file>` runs, with `--prerelease` when the plan says so, and the
       files and `SHA256SUMS` attached;
  5. `itch`, on tags only, never for a prerelease:
     - it runs when the repository variable `ITCH_GAME` (`user/game`)
       is set;
     - it reads `BUTLER_API_KEY`, a repository secret, and fails with a
       plain message if it is missing. A repository secret works on
       every plan; environment secrets in a private repository need
       GitHub Pro or Team. itch.io has no trusted publishing (OIDC), so
       2011's ADR rule (a release publishes through OIDC where the
       registry has it) allows the key;
     - it installs butler (MIT, `github.com/itchio/butler/releases`,
       15.32.0 on 2026-10-20, `butler-linux-amd64.zip`) at a pinned
       version. The release publishes no checksum file, so the zip's
       SHA-256 is written in the workflow and checked;
     - for each package but `steam-deck`, and never a `.debug` file, it
       unpacks the archive (zip or tar.gz) into a folder and runs
       `butler push <folder> "$ITCH_GAME:<exporter>" --userversion
       "$VERSION" --if-changed`. itch.io tags a channel by its name
       (`win`, `linux`, `mac`), so the exporter ids work as channel
       names;
     - the key is only an environment variable of that step, and
       nothing echoes it.
- **The rules every one of these workflows keeps:**
  - `contents: read` at the top. Only `release` gets `contents: write`,
    `id-token: write`, `attestations: write` and
    `artifact-metadata: write`;
  - every action is pinned by its commit SHA with the version in a
    comment, the same SHA the engine's workflows use (2011's
    `check-pins`), and every checkout has `persist-credentials: false`;
  - values from the event reach scripts through `env:`, never written
    into a `run:` with `${{ }}`;
  - no job of a tag's run reads the Actions cache. The
    `Swatinem/rust-cache` steps in `ci.yml` and `export.yml` get
    `if: ${{ !startsWith(github.ref, 'refs/tags/') }}`, one of the fixes
    zizmor's `cache-poisoning` audit lists. A called workflow sees the
    caller's `github.ref`, so this holds through `workflow_call`.
- **Cargo.lock is committed.** In both templates' `ci.yml`:
  - a first step fails with "commit Cargo.lock: run `cargo
    generate-lockfile` (or any build) and commit it" when the file is
    missing;
  - `--locked` goes on `clippy` and `test`, and the old comment goes.

  `new`'s limit "the game was made, not built: run `cargo test` in …"
  (`make` in `crates/cli/src/contract/new.rs`) adds "and commit the
  Cargo.lock it writes".
- **GitHub expressions in templates.** `fill` in
  `crates/cli/src/contract/new.rs` fails on any `{{` left after the
  placeholders are replaced, and the module's doc says "No other `{{`
  may be in a template's text". So today a template file can't hold a
  GitHub Actions expression such as `${{ matrix.target }}`. `fill` lets
  `${{` through, and the doc says so. Any other unknown `{{` stays an
  error with its file and line.
- **The engine checks the templates' workflows.**
  - The `templates` job in `.github/workflows/ci.yml` runs `cargo
    generate-lockfile` in the made game, as its author's first build
    would, and adds `--locked` to the gates it runs there.
  - The same job runs `actionlint` on the made game. actionlint finds a
    project only in a folder with both `.git` and `.github/workflows`,
    and it reads `./.github/workflows/…` calls from that project. The
    job's container has no `git` (2513), so it installs it
    (`apt-get install -y --no-install-recommends git`; none is needed
    if `git` is a row of 2513's table by then, since the script installs
    it) and runs `git init -q` in the made game first.
  - zizmor needs nothing new: 2011's job reads `.`, and zizmor takes in
    every `.github/workflows` folder under it, the root's and the
    templates' wherever they live. Don't add the templates as inputs
    (2011 found that reports each finding twice). Fix what it reports
    on the new files.
  - It runs `cargo shuyi export --plan --tag v0.1.0 --json`, which must
    pass (the templates' version is 0.1.0), and `--tag v9.9.9`, which
    must fail.
  - `actionlint` (MIT) comes from its GitHub release at a pinned
    version (1.7.12 on 2026-10-20), checked against the release's
    `actionlint_<version>_checksums.txt`. It runs in CI only and ships
    in nothing.
  - The engine's own `.github/workflows/export.yml` is a byte-equal copy
    of the templates' `export.yml`. The job 3004 added to export the
    examples on every PR calls it with `dir: examples/hello-grid`. So
    every engine PR runs the file a game's release runs.
- **Docs:**
  - each template's `README.md` gets a "Releasing" section:
    - set the version (`[workspace.package]` in `Cargo.toml`, or
      `app.version` in `shuyi.ron` when the game sets it, if 3048 is
      done; if not, 3048 adds that when it lands), commit, run
      `git tag -a vX.Y.Z -m "<notes>"`, and push the tag;
    - a dry run from the Actions tab;
    - the itch.io variable and secret, and that the web channel is set
      to "playable in the browser" once, on the itch.io page;
    - the macOS `.app` has no Developer ID signature (3004 signs it ad
      hoc; 3051, parked, is the paid kind), so macOS warns, and the
      player opens it from System Settings, Privacy & Security;
    - checking a package with `gh attestation verify <file> --repo
      <owner>/<repo>`, on a public repository;
  - each template's `AGENTS.md` gets the same in a few lines, for
    agents;
  - `docs/contract.md` gets `export --plan`, and its section "The
    templates" names `release.yml` and `export.yml`.
- **Either order:**
  - **2501** (templates as data, done): the two workflows, the
    `ci.yml` changes and the docs go in the `base` part of
    `templates/shuyi-default/`, whose composer already lets `${{`
    through (`shuyi_cli::template::fill`). The test that keeps the
    engine's own `export.yml` equal to the template's reads the base
    part's copy.
  - **2504** (CI per target). If it is done, its per-target PR job
    becomes calls to `export.yml` with the matrix from `export --plan`,
    so a game names its targets once, in `shuyi.ron`. If not, 2504 does
    that (its scope says so).
  - **2005** (the engine's first release). If it is done, its tag
    workflow gets one more job. Hello, Grid is exported through the
    engine's `export.yml` for each `build` entry of its plan, made
    without `--tag` (2005 checks the engine's tag against the crates'
    version). Its packages go on the engine's GitHub Release with
    `SHA256SUMS` and attestations (the engine's repository is public).
    If not, 2005 adds it (its scope says so).
  - **3010 and 3012.** If either is done, this ticket fills its
    exporter's `host()`, `container()` and `ci()`:
    - `steam-deck` builds on `linux` in its Steam Linux Runtime
      container;
    - `playdate` is `check-only`, with the SDK's license as `why`,
      unless 3010 found that CI may download the SDK.

    If not, each fills them (their scopes say so).
  - **3051** (parked: signing). If it is done, its opt-in signing step
    (`SHUYI_SIGN`) goes into `release.yml` and `export.yml`, and the
    README's and `AGENTS.md`'s "Releasing" sections say signing is
    opt-in and link its `docs/signing.md`. If not, 3051 adds both, as
    its scope says.
  - **From 1134 (done)** (the skill pack): `shuyi-ship`
    (`skills/shuyi-ship/SKILL.md`) teaches the release: `cargo shuyi
    export --plan`, `git tag -a vX.Y.Z -m <notes>`, the dry run from
    the Actions tab, and turning on itch.io with the `ITCH_GAME`
    variable and the `BUTLER_API_KEY` secret; `check-skills` passes.
  - **2802** (the book). If it is done, chapter 9 ("Targets and
    export") gets a "Releasing" section with the README's text. If not,
    2802 adds it (its scope says so).
  - **Either order with 2033:** if 2033 is done, run `cargo test -p
    shuyi-cli --lib template` after adding `--locked`. Its test of the
    gate lists drops `--locked` and reads the template's `ci.yml` gates
    from `cargo fmt --all --check` on, so keep the Cargo.lock step
    before `cargo fmt`, and leave `--locked` out of the template's
    `AGENTS.md` list; if not, nothing to do (2033's test is written that
    way).

**Out (do not do):**
- the exporters and their packages (3004, 3010, 3012). Which glibc the
  Linux package needs and which architectures the `.app` holds are
  3004's choices; the plan only reports where each exporter builds;
- SteamPipe, and anything Steam (game#0903; `docs/decisions.md`,
  2026-10-15);
- signing and notarizing on macOS and Windows (3051, parked; it costs
  money);
- the game's identity in each package: icons, version resources,
  `Info.plist` (3048, 3049);
- crates.io, and the engine's own release workflow beyond the job above
  (2005);
- zizmor, the pins check and Dependabot over the templates' workflows
  (2011), and the Linux setup script (2513);
- release notes written by a tool: a game's notes are its tag's
  message, and 2804's changelog is the engine's;
- an updater inside the game (the itch.io app updates games pushed
  with butler);
- pushing the engine's examples to itch.io, which needs an account of
  Nick's;
- the web shell's progress bar and notes for portals (3052).

## Implementation steps

1. Read 3004's `Exporter` trait, its `export` operation and its CI job,
   2520's `.debug` file if 3004 writes it, 2011's zizmor job and
   `check-pins`, 2513's `templates` job, and 2005's workflow if it is
   done. Check where the templates live now.
2. Add `host()`, `container()` and `ci()` to `Exporter` (and
   `profile()` if it is missing), and `--plan` and `--tag` to `export`.
   Raise the contract's `VERSION` (`crates/cli/src/contract.rs`), and
   regenerate the schemas in `crates/cli/schemas/`, `versions.json`
   included.
3. Let `${{` through `fill`, update the module's doc, and add `new`'s
   Cargo.lock limit. The test `every_template_fills_every_placeholder`
   in `crates/cli/src/contract/new/tests.rs` asserts no `{{` in any
   made file: make it allow `${{` and nothing else.
4. Write `export.yml`, `release.yml` and the `ci.yml` changes in
   `templates/grid/.github/workflows/`. Copy them to `templates/blank/`,
   and `export.yml` to the engine's `.github/workflows/`.
5. Point 3004's example export job at `export.yml`, and extend the
   `templates` job.
6. Write the docs, then the either-order parts that apply.
7. Run the gates and `cargo xtask check-pins`. Put the made game's plan
   answer and its matrix in the PR description, and say there that
   `release.yml` itself first runs in a game's repository (this
   repository's CI runs `export.yml` and actionlint, not the release).

Size: about 500 changed lines, plus the byte-equal copies (the second
template, the engine's `export.yml`) and the regenerated schemas. If it
runs well past 600 without those, move the `itch` job to a follow-up
ticket (the `write-ticket` skill) and say so in the completion notes.

## Acceptance criteria

- [ ] `cargo shuyi export --plan --json` on a game whose `shuyi.ron`
      lists `desktop` and `web` lists `windows`, `linux`, `linux-arm64`
      (once registered) and `macos`, in the order they are registered,
      then `web`, each with its host and `ci`.
- [ ] `--tag v0.1.0` passes on a game at version 0.1.0. `--tag v0.1.1`
      and `--tag 0.1.0` fail, each naming the tag and the version.
- [ ] A target with no exporter is a `limits` entry naming the target,
      and the answer still passes.
- [ ] A template file holding `${{ github.ref }}` fills. One holding
      `{{other}}` still fails with its file and line.
- [ ] The `templates` job makes a game from each template, writes its
      `Cargo.lock`, runs its gates with `--locked`, and runs
      `actionlint` on its workflows with nothing reported.
- [ ] 2011's zizmor job and `cargo xtask check-pins` pass with the new
      workflows in place.
- [ ] The two templates' `.github/workflows/` files are byte-equal, and
      the engine's `export.yml` equals theirs (a test).
- [ ] A made game's `ci.yml` fails its first step when `Cargo.lock` is
      missing.
- [ ] In `release.yml`, `ci.yml` and `export.yml`, every
      `Swatinem/rust-cache` step is off on a tag, and only the
      `release` job has write permissions.
- [ ] The engine's PR CI exports Hello, Grid through `export.yml`.
- [ ] Each template's `README.md` and `AGENTS.md` say how to release,
      that the macOS app has no Developer ID signature, and how to turn
      on itch.io.
- [ ] `docs/contract.md` describes `export --plan`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit:
  - the plan, from fake exporters and a `shuyi.ron` with `desktop` and
    `web`, with a `Planned` profile, and with `steam-deck` and no
    exporter for it;
  - the tag check: equal, another version, no `v`, a prerelease;
  - `fill` with `${{` and with an unknown `{{x}}`, and
    `every_template_fills_every_placeholder` allowing `${{` only;
  - the byte-equal workflow files, in `crates/cli/src/contract/new/tests.rs`;
  - the made `ci.yml` and `release.yml`: no `rust-cache` step without
    the tag `if:`, and write permissions in the `release` job only
    (read as text, as the copy tests read files).
- Schema: the regenerated `export` schemas, and the contract's
  `VERSION` raised.
- Integration (CI):
  - the `templates` job: the lock file, the `--locked` gates,
    `actionlint`, and the plan with a right and a wrong tag;
  - 2011's zizmor job over the new files;
  - the engine's PR export of Hello, Grid through `export.yml`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
