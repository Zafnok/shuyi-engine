---
id: "2005"
title: "The first release: the engine's crates on crates.io, a version, a changelog, and the game on versions instead of a git revision"
type: infra
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2002", "2919", "1129", "1133", "1134", "2801", "2804", "2010", "2011", "2012", "2013", "2509", "2513", "2808", "2812", "2041", "3085", "2577", "2250", "2251", "2823", "2824", "2825", "2826", "2827", "3069", "2814", "2820", "2828", "2817", "2822", "2777", "2576", "2544", "2391", "2388", "2389", "2082", "2083", "2084", "2085", "2086", "2087", "2088", "2089", "2412", "2906", "2347", "2348", "3218", "2405", "2409", "2133", "2136", "2239", "3041", "3044", "2262", "2090", "2095", "2096", "3503", "3537", "2098", "3406", "3407", "3504", "3510"]
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
Rust), 2013 (public types that can grow), 2509
(problem codes), 2513 (zero setup on Linux) and 2808 (docs agents can
read). On 2026-11-01 Nick parked 2016 (governance) and 1132 (community
files), and kept three small parts here: `main`'s required checks,
private vulnerability reporting and a short `SECURITY.md`.

Nick, 2026-11-05 (`docs/decisions.md`, *the first release waits for
everything before going public*): the list above held the protections
only, so 2005 read as ready while the MVP was still open. Its
`blocked_by` now holds every ticket the release waits for, in three
groups:

- **The MVP's open tickets:** 2041 (the API gate sees every feature),
  3085 (an export never ships the live link), 2577 (a gate that holds on
  a busy Windows machine), 2250 and 2251 (the voice cast and plan the
  game moves onto).
- **Everything before going public.** The release is the public launch:
  crates.io, docs.rs and `cargo install shuyi-cli` in the quick start.
  It also freezes a copy of the site and `llms-full.txt` at its version,
  which 2829 compares later ones with, so the book has its final shape
  first. That is the one book (2823–2828), the playground (3069, 2814,
  2820), the tutorial's acceptance and tries (2817, 2822), new games on
  the web (2777) and 2576. Not the blog (2805) or sponsors (2806), which
  the release doesn't touch.
- **The breaks already planned,** cheaper before anyone installs: 2544
  (done: the `bots` operation and MCP tool are gone, folded into
  `playtest`), 2391 (one picture type), and 2389 with 2388 before it
  (the inks go).

Nick, 2026-11-14 (`docs/decisions.md`, *no run of breaking versions
after the MVP*): every planned break a release can't soften comes before
it, so later features ship as compatible releases. An audit of every
open ticket that day added a fourth group:

- **Types that can grow, and shapes that must be right the first time:**
  - 2082–2086 move every exhaustive type an open ticket plans to grow
    under ADR-0039 (the frame and widgets; input and controllers; sound,
    music and cinematics; the assets' and platform's data; the
    contract);
  - 2412 gives each pad event its pad;
  - 2087 takes outside crates' types out of a game's way in;
  - 2089 checks, on each PR, that the public API names no outside crate
    ADR-0119 doesn't list;
  - 2088 decides what the contract's version promises;
  - 2262 (done) handed out the bundle's reads as `Cow` bytes, not
    `&'static` (ADR-0128 §1, decided by 2919 from what 2906 measured),
    before the bundle's API is published. The other builds of ADR-0128 only add,
    so they come after the release.
- **Breaks no attribute softens,** done before the release: 2347
  (styled text) and 3218 (done, typed label arguments), 2405 and 2409 (mouse
  and touch on the engine's widgets), 2133 (done, the bots' settings files),
  2136 (motions moved onto `ease`), 2239 (a check on translated lines),
  3041 (macroquad's `log-rs`) and 3044 (the window kept as it was left).

The retro of 2026-11-17 (the last 100 PRs) found what the list missed:

- **An MVP ticket outside it:** 2090 (the outside-crate check reads
  the crates a dependency's types come from), which 2089 wrote. The
  rule of 2026-11-05 puts every open MVP ticket here.
- **A blocker's own regression:** 3218 put the game's Playdate frame
  over its budget. Its fix, 3219, was filed after the MVP but done the
  same day (#485), so the release need not wait for it.
- **Types the sweep of 2082–2086 missed:** 2095 (`FontAtlasDef`, the
  `Sound` kind and `DoctorInput` under ADR-0039), and 2096 (`cargo
  xtask api` classifies every new public type, so the next one isn't
  missed), and 3503 and 3537 (no constructor takes two neighbouring
  numbers, as constructors freeze at 0.1).
- **Public API with no second user,** hidden before 0.1 (2098), since
  hiding it later is a break.
- **What a new game's templates publish.** `shuyi-cli`'s package
  carries the templates, so every game made from the release starts
  with their workflows: 3406 (a public game's tag release publishes
  only what Nick decided, and never a debug file), 3407 (a job that
  checks out private files caches no build and uploads none) and 3504
  (a game with private files exports for Linux).

Nick, 2026-11-25 (`docs/decisions.md`, *the publish list (3510) joins
the MVP*): 3510 joined this list. Its publish list, exact pins and
runbook are what this ticket's release workflow runs on, and without
them `cargo package` refuses every crate that depends on another.

The release ships with quad-snd as the default sound backend. While
quad-snd is in the graph, the in-house backend needs the root
`Cargo.toml`'s `[patch.crates-io]` of `alsa-sys` (ADR-0075), which a
published crate can't carry. Making the in-house backend the default
later (3084) breaks no game's code (2084, ADR-0068).

## Nick input

**Sign-off:** before `cargo publish` runs, Nick says go. Publishing can't
be undone (a version can be yanked, never deleted), and it is the
engine's first public release. Outside pull requests stay closed
(`CONTRIBUTING.md`) until Nick unparks 1132.

The sign-off also:
- asks how often the engine releases: on a fixed rhythm (Bevy ships
  every three to four months) or when the game needs a release. His
  answer goes in `docs/decisions.md` (the `ask-nick` skill), and the
  ADR follows it. It tells him that fixes go to the latest release only
  (the ADR's rule, below), so he can say otherwise;
- holds the release's setup, which is Nick's own clicks. The crates
  exist already (the 0.0.0 placeholders, below), so no token is made
  for this release: Nick adds this repository's `release.yml` and the
  `release` environment as the trusted publisher on the settings page
  of each of the fourteen crates he holds on crates.io, and turns on the setting that refuses API
  tokens for the crate if crates.io offers it. The session writes the
  clicks here, checked against crates.io on the day, before it asks.
  The setup also has two clicks on GitHub, which Nick may do at any
  time before the release:
  - **The gates bind on `main`.** At
    `https://github.com/Zafnok/shuyi-engine/settings/rules`, in the
    ruleset "protect main": keep "Restrict deletions" and "Block
    force pushes"; turn on "Require a pull request before merging"
    with 0 required approvals; turn on "Require status checks to
    pass" with `ci-result` and `mutants (diff)` from GitHub Actions,
    and "Require branches to be up to date before merging"; remove
    every entry from "Bypass list", his own account included, since
    agents push with his credentials.
  - **Private vulnerability reporting.** At
    `https://github.com/Zafnok/shuyi-engine/settings/security_analysis`,
    turn it on.

  Private vulnerability reporting was already on on 2026-11-23 (`gh api
  repos/Zafnok/shuyi-engine/private-vulnerability-reporting`), so check
  it only. If 2849 is done and Nick bound the checks there, check the
  ruleset read-only and skip that click.

## Scope

**In:**
- Each crate's metadata for crates.io: `description`, `keywords`,
  `categories`, `readme`, `documentation`, and the two license files in
  each crate's package (`cargo package --list` shows them).
- **A version policy as an ADR:** semver, 0.x while the API moves, and
  one version for `shuyi` and all `shuyi-*` crates. It says:
  - **What a breaking change needs:** a `Breaking:` line and a
    `migration` block under it in the ticket's completion notes (2804),
    the version raised at release time from the `Breaking:` lines since
    the last tag (2028), never in the PR (3510 changed `work-ticket`
    §3), and a pin bump in the game. It cites 3510's publishing ADR
    (exact `=` pins between the crates, the publish list from the
    workspace) and doesn't restate it. It cites ADR-0038 (2010) for how a break is
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
    `SECURITY.md` says so in one line.
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
  - **A *Rust version* section.** **Either order with 3511:** if 3511
    is done, the section links its ADR and states the policy it set
    (the root `rust-version` trails the pin, CI's `msrv` job builds on
    it, check-toolchain accepts at-or-below for the root and equal for
    the templates, a raise never goes in a patch and carries a
    `Behaviour:` line); if not, the section keeps today's rule
    (`rust-version` equals the pin's `X.Y`, enforced by `cargo xtask
    check-toolchain`, 2012) and says 3511 decouples them, and nothing
    else is needed here (3511 amends this section when it lands).
  - **Either order with 3518:** if 3518 is done, the version-policy ADR
    cites 3518's ADR on how the extension traits are versioned and
    follows its choice (a separately versioned crate of extension
    traits, or one version with each outside module stating the engine
    versions it supports); if not, nothing to do (3518's ADR amends this
    one).
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
  the gates, then `cargo publish --workspace`, which publishes the
  fourteen engine crates (3510's `cargo xtask check-publish --order`,
  `shuyi-embed`, `shuyi-embed-macros`, `shuyi-script`, `shuyi-sound`
  and `shuyi-sound-out` among them) in dependency order; every other
  member is `publish = false` (3510). A run that stops part way follows
  3510's runbook. No crates.io token is stored anywhere (2011's ADR). The publish job runs
  in a `release` environment with `permissions: id-token: write`. It
  gets a short-lived token through crates.io trusted publishing
  (`rust-lang/crates-io-auth-action`, pinned by SHA). crates.io lets a
  crate trust a workflow only once the crate exists. The fourteen crates
  exist since the 0.0.0 placeholders (the last five held on 2026-11-24;
  below), so the first real version goes out through the workflow too,
  with no publish by hand. Check this against crates.io's docs on the
  day. No job of the tag's run reads the Actions cache.
  **Either order with 3517:** if 3517 is done, `shuyi-contract` is one
  more engine crate: check that `cargo publish --workspace` publishes
  it before `shuyi-cli`, and give it a 0.0.0 placeholder with the other
  crates and a trusted-publisher click; if not, nothing to do (3517
  gives `shuyi-contract` its crates.io metadata and its place in the
  publish order).
- **`SECURITY.md`** at the root, short: *Supported versions* (the latest
  0.x release only, no backports or LTS); how to report (the Security
  tab's "Report a vulnerability", never a public issue); and the
  response time, best effort with no number. **Either order with 2020:**
  if 2020 is done, it links `docs/formats.md`, *Untrusted files*; if not,
  2020 adds the link. **Either order with 2849:** if 2849 is done,
  `SECURITY.md` exists; change its *Supported versions* line from
  `main` to the latest 0.x release, and check the rest against this
  list; if not, write it as above. 1132, parked, grows this file if it
  lands later.
  - **Either order with 3512:** if 3512 is done, `SECURITY.md` gets one
    line saying how a fix is released (a patch release of every crate,
    a GHSA, affected versions yanked), linking 3512's ADR, and the
    version-policy ADR cites it on fixes; if not, add that line here
    linking the ADR 3512 will write, and 3512 makes its skill name
    `release.yml` as the publish step.
  - **Either order with 2846:** if 2846 is done, `SECURITY.md` links
    its page `docs/cyber-resilience.md` in one line (what the engine
    gives a game's maker for the EU Cyber Resilience Act); if not,
    nothing to do here (2846's page names the Security tab's private
    reporting, and it links `SECURITY.md` once this ticket writes it,
    so the PR closing 2846 adds the link to `SECURITY.md` if this one
    landed first).
- **Owners and the credential:** `Zafnok` stays the crates' one owner,
  and the release's only credential is the trusted publisher's. The
  list of accounts (`docs/continuity.md`) is written by 2849, or by 2016
  if it is unparked first. At the release, add its crates.io row: the
  trusted publisher.
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
- **From 3050 (done)** (release a game from a tag): the tag workflow
  also exports Hello, Grid through the engine's
  `.github/workflows/export.yml` (a byte-equal copy of the template's;
  its inputs `target`, `host`, `container`, `dir: examples/hello-grid`)
  for each `build` entry of `cargo shuyi export --plan --json` in
  `examples/hello-grid`, made without `--tag` (the workflow has already
  checked the tag against the crates' version). It attaches the
  packages, a `SHA256SUMS` and attestations (`actions/attest`, as the
  templates' `release.yml` does; the repository is public) to the
  engine's GitHub Release. This ticket adds that job.
- **2544 is done** (`bots` folded into `playtest`, which removed the
  `bots` operation and `GameTools::bots`): its break landed before the
  first release, so nothing is left here.
- **From 2919 (done):** ADR-0128's one break, 2262 (done: the bundle's
  and `DecodeCx`'s reads, and `AppConfig::files`), carries its own
  `Breaking:` lines. Paths stay `&'static`; a change to
  them for the parked DS (2264) would come after this release, a 0.x
  break as this policy says.
- The names are held already: on 2026-10-21 Nick had the six crates
  published as empty 0.0.0 placeholders, `shuyi` on 2026-10-22, and
  `shuyi-alloc-count` and `shuyi-playdate` on 2026-11-01, all owned by
  `Zafnok` (`docs/decisions.md`). The other five engine crates
  (`shuyi-embed`, `shuyi-embed-macros`, `shuyi-script`, `shuyi-sound`,
  `shuyi-sound-out`) were held the same way on 2026-11-24. The first release is the
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
- The docs.rs metadata, feature labels and the CI job that imitates a
  docs.rs build: 2842. **Either order with 2842:** if 2842 is done,
  after the publish open each crate's docs.rs page and check that its
  build passed; if not, nothing to do (2842 does it, and the next
  release carries it).
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
   `cargo publish --dry-run --workspace`.
4. Read the settings with `gh api`, GET only: `rules/branches/main`
   holds `pull_request` and `required_status_checks` naming `ci-result`
   and `mutants (diff)`, and `private-vulnerability-reporting` is
   enabled. Anything else goes to Nick with its click.
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
- [ ] `SECURITY.md` exists, and on the day of the release `main` requires
      `ci-result` and `mutants (diff)` and private vulnerability
      reporting is on.
- [ ] After the release, `/v<x.y>/llms-full.txt` is live and
      `/llms.txt` links it.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `cargo package` (with verification) and `cargo publish --dry-run
  --workspace` of the fourteen together, in CI; 3510's step runs
  `--no-verify` until the templates, targets and skills move into the
  packages here.
- Integration (2003's operations): `shuyi modules` and `shuyi add` on
  a game whose `shuyi` comes from a registry, not a checkout.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
