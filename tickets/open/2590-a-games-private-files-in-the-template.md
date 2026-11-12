---
id: "2590"
title: "A new game's private files: a second repository checked out over its files whenever it is there, as a part of the template"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2590 — A new game's private files, as a part of the template

## Context

Nick, 2026-11-12 (`docs/decisions.md`, *`shuyi new` sets up a game's
repositories*): Visions of Shuyi spent many commits keeping its writing,
art and sounds in a private repository beside its public one, so that
they can't be copied from the public code. A new game should get that
from `shuyi new`, already working.

What the game built, and what this ticket carries into the engine's
template (generic, with nothing of the game):

- **game ADR-0040** (`docs/adr/0040-private-assets.md` in
  `Zafnok/visions-of-shuyi`): one private repository, checked out into a
  git-ignored folder at a commit a pin file names, fetched by a
  read-only deploy key whose private half is the Actions secret
  `PRIVATE_ASSETS_KEY`; a fork or a run without the key builds on the
  public files.
- **game ADR-0117**: the game's own data and scripts live in that
  repository's `content/`, which the build lays over `assets/`
  **whenever the checkout is there**, with no cargo feature; every CI
  job that loads files checks it out; nothing that embeds it becomes a
  public workflow artifact (anyone signed in can download a public
  repository's artifacts).

The engine already has the fetching half: `shuyi private-assets`
(`crates/cli/src/private_assets.rs`) clones a sparse checkout at the
pin, and `--pin` moves the pin; `shuyi.ron`'s `private_assets`
(`PrivateAssets` in `crates/cli/src/project.rs`) configures it. The
bundle already has private layers (`Layer::private`,
`Bundle::is_private`, `crates/assets/src/bundle.rs`). What is missing is
a template part that wires them into a new game, and the build side.

Searched the game at the engine revision it pins
(`234de697e435026301a653b0963800663db1ef7f`):
`gh search code --repo Zafnok/visions-of-shuyi 'private_assets'` finds
its `shuyi.ron` block (url, `assets-private`, `assets-private.rev`,
folders, library, copies, credits), its xtask and its content crate's
`build.rs`. This ticket changes neither `PrivateAssets` nor the
`private-assets` command's behaviour for a project that has a pin, so
nothing in the game must change.

The tickets that follow: 2591 (`shuyi repos` makes the two GitHub
repositories, the keys and the secrets), 2592 (`shuyi new` asks about
them), 2593 (the shipped skill for changing the private files).

## Nick input

None. Nick decided the defaults (public game, private files) on
2026-11-12; the rest is technical.

## Scope

**In:**
- **An ADR** (the `write-adr` skill): a game's private files are a
  second repository, checked out at a pinned commit into a git-ignored
  folder, whose `files/` is laid over the game's files as a private
  layer whenever the checkout is there; its other folders are never
  built. It records the layout 2591 writes and 2592 asks about:

  | Folder of the private repository | Holds | Built |
  | -------------------------------- | ----- | ----- |
  | `files/` | Files at the path they have in the game's files folder (`files/sprites/hero.png` ↔ `crates/game/files/sprites/hero.png`), laid over it: one subfolder per kind picked (`text/`, `sprites/`, `sounds/`, `music/`, `data/`) and any the developer names | Yes, a private layer |
  | `docs/` | The story and design documents | No |
  | `library/` | Bought packs as they came, sorted; fetched only by `private-assets --library` | No |

  It cites game ADR-0040 and ADR-0117 as where this came from, and says
  why there is no cargo feature (ADR-0117 §2: the checkout is the
  switch) and why the gates run on the private files when CI has the
  key (ADR-0117 §3).
- **A template part `private-files`** in
  `templates/shuyi-default/template.ron`, written when a new question
  `private-files` (`pick: One`, answers `yes` / `no`, default `no`) is
  `yes`. Its folder `templates/shuyi-default/private-files/` holds:
  - `shuyi.ron`, merged into the base's (docs/templates.md, *merged by
    the engine*): `private_assets: (url: "{{private_url}}", checkout:
    "private", pin_file: "private.rev", folders: ["files", "docs"],
    library: "library", no_access: "…")`, the `no_access` text saying
    the game builds and runs on its public files without access;
  - `.gitignore` lines `/private/` (merge or a second ignore file:
    whichever the composer supports; YAML and ignore files aren't RON);
  - the CI step (below).
- **A placeholder `{{private_url}}`** and a flag `shuyi new
  --private-url <https url>`: required when `private-files=yes`, refused
  otherwise; a URL that isn't `https://github.com/<owner>/<repo>.git`
  is refused with a problem code (GitHub only, for now: 2591 makes it
  with `gh`). `docs/templates.md`'s table of placeholders gains it.
- **The build lays the private files over the game's files.** In the
  base's `crates/game/build.rs`: when `<root>/private/files/` exists,
  `SHUYI_PRIVATE_FILES` names it; else an empty folder in `OUT_DIR`
  (an `include_dir!` path must exist). It watches the folder when it is
  there, else the checkout's `.git/index` if any, else `build.rs` (a
  watched path that doesn't exist rebuilds the crate every time:
  game ADR-0117 §2). `src/lib.rs` embeds it and builds the bundle with
  `Layer::dir("private", &PRIVATE).private()` above `files`. A base
  game with no checkout has an empty private layer and works as now.
  This goes in the **base** part, not the private one: the code is the
  same for every game, so a game can add the private part later.
- **Editions:** an edition with a content root of its own
  (`SHUYI_CONTENT`, `shuyi export --edition`) gets **no** private layer:
  a public demo built from its own folder holds none of the private
  files. A test says so.
- **The packed build** (ADR-0110, `Export::packs` and `bake_pack` in
  `crates/cli/src/export.rs`): the pack is baked from the same layers,
  the private files included when the checkout is there, and the bake's
  read-back check covers them.
- **`shuyi run --watch`** reloads a changed private file as it does one
  of `files/`.
- **CI.** The private part's CI step, before the first cargo command of
  the base's `ci.yml` `gates` job, checks out `files/` at the pin with
  the secret `PRIVATE_ASSETS_KEY` (an SSH deploy key, so it clones
  `git@github.com:<owner>/<repo>.git` with
  `GIT_SSH_COMMAND="ssh -i <key file> -o IdentitiesOnly=yes"`, the key
  written to `$RUNNER_TEMP` and removed after). Read the game's
  `.github/actions/private-assets/action.yml` for the approach and
  carry what is generic. Rules:
  - a run on the game's own repository without the key **fails**, naming
    the secret and `shuyi repos` (2591): the gates must run on the real
    files (game ADR-0117 §3);
  - a pull request from a fork has no secrets: it warns and runs on the
    public files;
  - no step uploads an artifact that embeds the private files. A YAML
    file isn't merged by the composer, so either the step is in the
    base's `ci.yml` guarded by `hashFiles('private.rev') != ''`, or the
    private part writes its own workflow: decide in the ADR, and keep
    zizmor clean (`.github/zizmor.yml` only with a reason).
- **Dependabot:** the template's Dependabot runs get only Dependabot
  secrets, so the step reads `PRIVATE_ASSETS_KEY` there too; 2591 sets
  it. The step's comment says so.
- **No pin yet.** `shuyi private-assets` with a pin file missing or
  empty says `no pin yet: push the private repository's first commit,
  then run \`shuyi private-assets --pin\``, instead of the current
  `40-character commit id` error. That is the only change to the
  command. `Behaviour:` line for it.
- **Docs** (the `document` skill): the template's `AGENTS.md` and
  `README.md` say, inside the private part's text or behind the
  `private-files` answer, where the private files are and that they
  never go in the public repository; `docs/templates.md` lists the part
  and its question; the book's chapter on starting a game names it.
- `PrivateAssets` and `CopyOut` are not changed; nothing public is
  added to `shuyi-assets`.

**Out (do not do):**
- Making the repositories, keys and secrets: 2591.
- Asking about them in `shuyi new`, and which kinds of files go
  private: 2592. This ticket's part takes the answer by `--set
  private-files=yes --private-url …`.
- The shipped skill for changing the private files: 2593.
- Credits for bought files in the private repository: the existing
  `private_assets.credits` part, left unset by the template.
- A public web build or Pages deploy for the template: the template has
  none today.
- **Either order with 3050:** if 3050 is done, add this ticket's
  checkout step to every package job of its release workflow, run when
  the game has `private.rev`: a release from the game's own repository fails
  without the key (game ADR-0040 §6), a dry run or a fork warns and
  builds on the public files; and its release notes in the template's
  `README.md` say that a release ships the private files inside the
  program. If not, nothing to do (3050 adds the step: its clause says
  so).
- Anything in the game repository: it keeps its own setup.

## Implementation steps

1. Write the ADR (the table above, the CI rules, editions).
2. `crates/cli/src/contract/new.rs`: `--private-url`, its check, the
   `{{private_url}}` placeholder in the fill (`template::Fill`) and the
   module doc's placeholder table.
3. `templates/shuyi-default/template.ron`: the question and the
   `private-files` part (`requires: ["base"]`); its folder with
   `shuyi.ron`, the ignore lines and the CI step.
4. The base's `crates/game/build.rs` and `src/lib.rs`: the private
   layer, empty without a checkout, absent for an edition with its own
   content root.
5. `crates/cli/src/export.rs`: the pack holds the private layer's files.
6. `crates/cli/src/private_assets.rs`: the *no pin yet* message.
7. Docs, then `cargo xtask gates` and the template job's checks
   (`templates/` escape `cargo fmt --all`: run the template job's own
   fmt).

## Acceptance criteria

- [ ] `shuyi new g --template blank --set private-files=yes --private-url https://github.com/o/g-assets.git`
      makes a game whose `shuyi.ron` has the `private_assets` block with
      that URL, whose ignore file has `/private/`, and whose CI has the
      checkout step; the same command without `--private-url` fails
      with a problem code, and `--private-url` without the answer too.
- [ ] `shuyi new g --template blank` makes the same files as before but
      the base's `build.rs` and `lib.rs`, and its gates pass.
- [ ] In a made game, a `private/files/sprites/x.png` laid in the
      checkout replaces `crates/game/files/sprites/x.png` in the bundle,
      a file only in the checkout is added, `Bundle::is_private` is true
      for both, and with `private/` removed (and a rebuild) the public
      file is back.
- [ ] An edition with its own content root holds none of the checkout's
      files.
- [ ] A packed export of a game with a checkout holds the private files,
      and its read-back check passes.
- [ ] `shuyi private-assets` with no pin file says *no pin yet* and how
      to make one.
- [ ] The CI step fails on the game's own repository without the key,
      warns on a fork's pull request, and no workflow step of the
      template uploads an artifact.
- [ ] The ADR is written; `docs/templates.md` lists the part, its
      question and `{{private_url}}`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `--private-url` parsing and its refusals; the fill of
  `{{private_url}}`; the *no pin yet* message.
- Snapshot / integration: the files `new` makes with and without the
  part (the template's own tests in `crates/cli/src/template/`); a made
  game's bundle with a fixture checkout laid over (the replace, the add,
  `is_private`), and without it; an edition without the private layer;
  the packed build's pack holding a private file.
- The template's CI job builds a game made with the part and no
  checkout (the empty layer), so a fork's build is covered.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
