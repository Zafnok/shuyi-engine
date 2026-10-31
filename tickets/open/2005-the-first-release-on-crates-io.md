---
id: "2005"
title: "The first release: the engine's crates on crates.io, a version, a changelog, and the game on versions instead of a git revision"
type: infra
milestone: MVP
model: reasoning
effort: high
status: todo
blocked_by: ["2002", "1129", "1133", "1134", "1132", "2801", "2804", "2010", "2011", "2012", "2013", "2016", "2509", "2513", "2808", "2812"]
nick_input: sign-off
completed:
---

# 2005 — The first release on crates.io

## Context

Plan B ([`docs/decisions.md`](../../docs/decisions.md), 2026-10-14):
carve, reach parity with the game, and publish with the AI tooling
([`docs/ROADMAP.md`](../../docs/ROADMAP.md), phase 2). The code came here
with the game's ticket 1131 (game#1131), which left publishing out. Until
this ticket, every crate is `publish = false` at version 0.1.0, and games
(the game first) depend on a git revision. Game ADR-0063 §8 says the game
moves to crates.io versions "later": this is that ticket.

A crate name on crates.io is first come, first served, and `shuyi-*` are
the names Nick chose (`docs/decisions.md`, *The name*).

Nick, 2026-10-20 (`docs/decisions.md`, *a second comparison*): the first
release waits for the protections a release needs. Its `blocked_by`
holds the tickets that add them: 2010 (API breaks named on every PR),
2011 (Dependabot, and no publishing token in CI), 2012 (the pinned
Rust), 2013 (public types that can grow), 2016 (governance and the repository's settings), 2509
(problem codes), 2513 (zero setup on Linux) and 2808 (docs agents can
read).

## Nick input

**Sign-off:** before `cargo publish` runs, Nick says go. Publishing can't
be undone (a version can be yanked, never deleted), and it is the
engine's first public release. Nick's contribution policy is settled
first: 1132, in this ticket's `blocked_by`, asks him (the game's 0059,
*Still open* in `docs/decisions.md`). `CONTRIBUTING.md` says it is
decided "before the engine's first public release".

The sign-off also:
- asks how often the engine releases: on a fixed rhythm (Bevy ships
  every three to four months) or when the game needs a release. His
  answer goes in `docs/decisions.md` (the `ask-nick` skill), and the
  ADR follows it. It tells him that fixes go to the latest release only
  (the ADR's rule, below), so he can say otherwise;
- asks again who the backup is, if Nick chose nobody in 2016 (its
  option C);
- holds the release's setup, which is Nick's own clicks. The crates
  exist already (the 0.0.0 placeholders, below), so no token is made
  for this release: Nick adds this repository's `release.yml` and the
  `release` environment as the trusted publisher on each crate's
  settings page on crates.io, and turns on the setting that refuses API
  tokens for the crate if crates.io offers it. The session writes the
  clicks here, checked against crates.io on the day, before it asks.

## Scope

**In:**
- Each crate's metadata for crates.io: `description`, `keywords`,
  `categories`, `readme`, `documentation`, and the two license files in
  each crate's package (`cargo package --list` shows them).
- **A version policy as an ADR:** semver, 0.x while the API moves, and
  one version for `shuyi` and all `shuyi-*` crates. It says:
  - **What a breaking change needs:** a `Breaking:` line and a
    `migration` block under it in the ticket's completion notes (2804),
    a version bump after the first release (`work-ticket` §3), and a
    pin bump in the game. It cites ADR-0038 (2010) for how a break is
    found (CI's `api` job: cargo-semver-checks against each PR's base) and
    recorded (a `Breaking:` line in the ticket's completion notes)
    instead of restating it, and it says how the `Breaking:` lines since
    the last tag decide the next 0.x version. It cites 2013's ADR on the
    shape of public types (`#[non_exhaustive]`, constructors) and
    doesn't restate it.
  - **Deprecate where a forwarder is cheap.** This covers a rename, a
    moved path (`pub use`), and a new parameter with a default (the old
    function kept, calling the new one). The old name stays behind
    ``#[deprecated(since = "X.Y.Z", note = "use `New`; see ticket NNNN's migration entry")]``
    for one minor release, then goes. Any other break goes straight in,
    with its migration entry. No window is required for every change
    while the engine is 0.x.
  - **Fixes go to the latest release only:** no backports, no LTS.
    `SECURITY.md` (1132) says so in one line.
  - **The `since` of a `migration` block** is the workspace version
    after the PR that breaks. 2523's `shuyi migrations` relies on this.
  - **Golden values.** It counts as breaking a changed or removed value
    in a golden table (2116's `shuyi_assets::golden`, checked by CI's
    `determinism` job): in an engine crate's table, or in an example's
    table when that example's own `src/`, `files/`, `tests/records/` or
    `Cargo.toml` didn't change in the same PR. A replay that plays
    differently is a break even when no API changed. CI asks for its
    `Breaking:` line once 2035 (`cargo xtask golden --breaks`) is
    done.
  - **Problem codes** (2509) and the exit codes 0 to 3 are public API:
    a code is never reused or renumbered, a retired code keeps its page,
    and changing what an exit code means is a breaking change.
  - **What players keep:** the engine's stored records (settings, key
    bindings, the profile list) and saved forms are part of the
    compatibility promise. A change to one raises its version and, once
    2110 is done (after the MVP), reads the old one through a step
    instead of resetting it. Until then a version they can't read is
    refused, as `PlayerKeys` and `Profiles` do today.
  - **A *Rust version* section.** Before the first release,
    `rust-version` equals the pinned Rust's `X.Y`, which `cargo xtask
    check-toolchain` (2012) enforces. After it, the section says what
    `rust-version` promises to games: the pin, or a stated lag behind
    it. It says what raising it needs: on 0.x, a minor version and a
    changelog line, like any breaking change. And it says that the PR
    which first sets `rust-version` below the pin adds an `msrv` CI job,
    either `cargo check --workspace --locked` on exactly that Rust (as
    Bevy's `msrv` job does) or `cargo hack check --rust-version`. That
    same PR changes check-toolchain's rule from equal to at-or-below.
    The `bump-rust` skill (2012) then points at this section.
- **The policy covers the engine's text formats too.** These are every
  format `Formats::standard()` registers, plus keymaps, language packs,
  cinematics, `.map`, `.dlg`, target profiles and `shuyi.ron`. They are
  public interfaces like the API:
  1. a field added with `#[serde(default)]` is not breaking;
  2. a renamed or removed field or value is still read for one more
     minor release (`#[serde(alias)]`, or a field read and ignored, even
     where the type has `deny_unknown_fields`), and its format lists the
     change so `shuyi check` warns on it and `shuyi upgrade` rewrites it
     (2517, after this release; until then the `Breaking:` line names
     the old and the new name);
  3. a change of meaning always comes with a new name, so the old form
     can be found;
  4. anything else is breaking, and its `Breaking:` line names the
     format and the field (2804 lists it first).

  Packs stay refused, never migrated (ADR-0023 §5). The `work-ticket`
  skill's *Public API* bullet says the formats count too.
- `CHANGELOG.md`, starting at the first release, written by 2804's
  `cargo xtask changelog --format keep-a-changelog` in the release
  workflow (it reads the tags and the whole history, so the checkout is
  full: `fetch-depth: 0`). **From 2804 (done):** the tool writes the
  whole file, not one release's notes, so the GitHub release's body is
  the section of `CHANGELOG.md` under the tag's `## [X.Y.Z]` heading,
  up to the next `## [`; the release's News post is made by `pages.yml`
  (`cargo xtask changelog --site site/content`), and 2804 left no step
  for it here.
- **Files outside the crates' folders.** `cargo package` leaves out what
  is outside a crate's folder:
  - `shuyi new`'s templates (`templates/` at the root, embedded by
    `crates/cli` with `include_dir`, ADR-0022 §7) are outside
    `shuyi-cli`'s folder, so its package lacks them: move or copy them
    into the crate (or a build step that writes them in), and check
    `cargo package --list` names them, including each template's
    `.devcontainer/devcontainer.json` and `scripts/setup-linux.sh`
    (2513). Hidden folders (`.cargo/`, `.github/`, `.devcontainer/`) are
    easy to drop from a package, so the check names one file from each.
  - If the templates move into `crates/cli`, what reads them by path
    moves with them, in this PR: `cargo xtask check-toolchain`'s walk
    and the `rust-toolchain` update's `directories` in
    `.github/dependabot.yml` (both from 2012), and the github-actions
    update's `directories` there and `cargo xtask check-pins`' reading
    of the templates' `Cargo.toml` files (both from 2011). 2011's
    `check-pins` fails until the Dependabot entry follows.
    The `dependency-upkeep` skill's step 3 (`templates/*/Cargo.toml`)
    and the docs map's pair rows (`docs/README.md`) name the new
    folders too; no gate checks them.
  - `targets/` at the root is outside the crates that embed it too:
    `shuyi-cli` (`TARGETS`, `crates/cli/src/contract/target.rs:33`, with
    `include_dir`) and `shuyi-app` (`crates/app/src/facts.rs`, with
    `include_str!`; from 3053, done). Move or copy the
    profiles into one package (`shuyi-assets`, which owns
    `TargetProfile`, is the natural one), embed them there, and have
    each crate read them from it rather than by a path into its folder.
    Check `cargo package --list` names them.
  - `crates/cli/errors/` (2509's pages, embedded with `include_str!`)
    is inside the crate already: check that `cargo package --list` for
    `shuyi-cli` names it.
  - `skills/` at the root (1134), which `shuyi new` copies into new
    games, is outside `shuyi-cli`'s folder too. Copy it into the
    package rather than move it (the plugin's marketplace entry and
    the skill installers read it at the root), and check `cargo
    package --list` names one skill's `SKILL.md`.
  - **From 2003 (done):** `shuyi modules` and `shuyi add` read the
    catalog of the engine the game builds with. `catalog::engine_root`
    takes the folder two above the `shuyi` package the game resolves,
    and `Catalog::read` reads `docs/modules.ron` and `crates/*` there
    (ADR-0041 §4). That works for a path or a git checkout, not for a
    crates.io package, whose folder holds that crate alone. Once new
    games take the released version, both would fail on every new
    game, and CI wouldn't see it (its tests use path engines). So in
    this PR: ship `docs/modules.ron` in a published package (`shuyi`,
    the front door, fits); find the modules, with their README and
    skill, from what cargo resolved for the game rather than under
    `crates/`, including the ones the game hasn't turned on; record it
    in an ADR that amends ADR-0041 §4; and test both operations on a
    game whose `shuyi` comes from a registry (a local one in a
    temporary folder), not a checkout.
- New games then depend on the released version, not the repository's
  `main`. `Engine`'s `#[default]` (`crates/cli/src/contract/new.rs:79`,
  `Main` today) becomes the released version, and `main` stays only
  when asked for. `{{engine.docs}}` (2812, `fill_for` in
  `crates/cli/src/contract/new.rs`) names, for a game on a released
  version, `AGENTS.md` at the tag and
  `<ENGINE_SITE>/v<x.y>/llms-full.txt`.
- A release workflow (`.github/workflows/release.yml`): on a tag
  `vX.Y.Z`, a check that the tag is `v` and the workspace's `version`,
  the gates, then `cargo publish` of each crate in dependency order
  (`shuyi-core`, `shuyi-assets`, `shuyi-ui`, `shuyi-platform`,
  `shuyi-app`, `shuyi`, `shuyi-cli`; `xtask` is never published). No
  crates.io token is stored anywhere (2011's ADR). The publish job runs
  in a `release` environment with `permissions: id-token: write`. It
  gets a short-lived token through crates.io trusted publishing
  (`rust-lang/crates-io-auth-action`, pinned by SHA). crates.io lets a
  crate trust a workflow only once the crate exists. The seven crates
  exist since the 0.0.0 placeholders (below), so the first real version
  goes out through the workflow too, with no publish by hand. Check
  this against crates.io's docs on the day. No job of the tag's run
  reads the Actions cache.
- **Owners and the credential** (`docs/continuity.md`, 2016): if Nick
  chose a backup with crates.io rights in 2016 (option A), after the
  release, Nick invites them to `shuyi` and every `shuyi-*` crate
  (`cargo owner --add <login> <crate>`, with a token that has the
  `change-owners` scope, or the crate's settings page), and they accept
  on crates.io. `docs/continuity.md`'s crates.io row names the owners,
  the trusted publisher, and the placeholders' two tokens
  (`docs/decisions.md`, 2026-10-21 for the six, 2026-10-22 for
  `shuyi`): each one's scope, its crates and that it was revoked, never
  its value.
- **Docs at the release's version** (2808): the release workflow runs
  `cargo xtask llms --full-only --out <dir>` at the tag and attaches
  `llms-full.txt` to the GitHub release. `pages.yml` downloads, for each
  minor version, the newest release's `llms-full.txt` (`gh release
  download <tag> --pattern llms-full.txt`, with `GH_TOKEN: ${{
  github.token }}`) into `public/v<x.y>/` before `cargo xtask llms`
  runs, and `cargo xtask llms` lists it under *Released versions*. A
  release without the file is skipped with a notice.
- **The book after the release (2802, done):** the quick start's
  install line (`site/content/learn/book/quick-start.md`) becomes `cargo
  install --locked shuyi-cli` from crates.io (the `--git` form goes),
  and Learn adds each crate's docs.rs link beside its `/api/` link
  (`site/templates/learn.html`, over `site/data/api.toml`).
- **Either order with 3050** (release a game from a tag): if 3050 is
  done, the tag workflow also exports Hello, Grid through the engine's
  `.github/workflows/export.yml` (3050) for each `build` entry of
  `cargo shuyi export --plan`, made without `--tag` (the workflow has
  already checked the tag). It attaches the packages, a `SHA256SUMS`
  and attestations (`actions/attest`; the repository is public) to the
  engine's GitHub Release. If not, 3050 adds this job when it lands.
- **Either order with 2544** (`bots` folded into `playtest`, which
  removes the `bots` operation and `GameTools::bots`): if 2544 is done,
  nothing to do: its break came before the first release. If not,
  nothing to do here: 2544 takes the version bump the policy asks for.
- **Either order with 2906** (research on asset memory): if 2906 is
  done, nothing to do: a build ticket of its ADR that changes the
  `&'static` reads (`Bundle`, `DecodeCx`, `ImageId`) carries its own
  `Breaking:` line. If not, nothing to do either: 2906 counts a change
  of lifetimes after this release as a 0.x break, as this policy says.
- The names are held already: on 2026-10-21 Nick had the six crates
  published as empty 0.0.0 placeholders, and `shuyi` on 2026-10-22,
  all owned by `Zafnok` (`docs/decisions.md`). The first release is the
  next version of each and replaces the placeholder's README and
  description. Nick wants no real version until the engine is stable,
  so the placeholders don't hurry this ticket; only a threat to the
  names would, and he is asked first (`docs/decisions.md`, 2026-10-21).
- **From 2003 (done): the front door `shuyi`** (`crates/shuyi`), the
  crate every new game names first (`ENGINE_CRATES`, `{{engine.shuyi}}`
  in the templates). It is released here with the six: its crates.io
  metadata like theirs, the one version, Nick's trusted-publisher and
  owner clicks, and a place in the publish order after `shuyi-app`
  (it re-exports `shuyi-core` to `shuyi-app`). `docs/decisions.md`
  (2026-10-22) had 2003 publish its first real version, when 2003 was
  to land after this release; it landed first, with no publishing set
  up, so this ticket does it, through trusted publishing as that entry
  asks. The sign-off says so to Nick.
- In the game repository: a ticket that moves the game from the git
  revision to the released versions (`shuyi-engine.rev` and `cargo xtask
  engine-pin` retire, or stay for the local checkout only).
  That ticket also keeps the game's pin moves working on versions
  (its watcher on the engine's `main`, game ADR-0096), since they are
  how the game finds what an engine change breaks (2014, a build of the
  game here, was dropped on 2026-10-26: `docs/decisions.md`).

**Out (do not do):**
- The MVP tickets themselves.
- `docs.rs` beyond what `cargo publish` gives.
- `shuyi upgrade` and the warnings in `shuyi check` that rest on the
  formats' rule (2517, after this release).
- Checking a release against the last version on crates.io (`cargo
  semver-checks --baseline-version`): 2028 does it for each later
  release, since this one has nothing to compare with. 2010 checks each
  PR against its base.

## Implementation steps

1. Write the ADR and the changelog.
2. Fill in the metadata; `cargo package` for each crate must pass.
3. Write the release workflow with trusted publishing; dry-run it with
   `cargo publish --dry-run`.
4. Run `cargo xtask repo-settings` (2016); every check must be `ok`.
5. Ask Nick for the sign-off, then tag and publish.
6. Write the game's ticket for moving to versions.

If the PR grows well past 600 changed lines (the moved templates
aside), move the release workflow to a follow-up ticket that the
game's ticket for the move waits for.

## Acceptance criteria

- [ ] `shuyi` and every `shuyi-*` crate are on crates.io at the first
      version, with their README, licenses and docs.
- [ ] A tag publishes a release through CI.
- [ ] The game's follow-up ticket exists.
- [ ] `docs/continuity.md`'s crates.io row lists the owners and the
      release credential (no value), and `cargo xtask repo-settings`
      passed on the day of the release.
- [ ] After the release, `/v<x.y>/llms-full.txt` is live and
      `/llms.txt` links it.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `cargo package` and `cargo publish --dry-run` for each crate, in CI.
- Integration (2003's operations): `shuyi modules` and `shuyi add` on
  a game whose `shuyi` comes from a registry, not a checkout.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
