# ADR-0111: A game's private files: a second repository, laid over its files whenever it is checked out

- **Status:** Accepted
- **Date:** 2026-11-12
- **Related tickets:** 2590, 2591, 2592, 2593, 2594, 2595, 3050

## Context

Nick decided on 2026-11-12 that `shuyi new` sets up a game's
repositories: a public game, with its writing, art and sounds in a
private repository beside it, so they can't be copied from the public
code ([decisions](../decisions.md)). Visions of Shuyi spent many commits
getting there, and its two ADRs are where this one comes from:

- **game ADR-0040**: one private repository, checked out into a
  git-ignored folder at the commit a pin file names, fetched by a
  read-only deploy key whose private half is the Actions secret
  `PRIVATE_ASSETS_KEY`. A fork, or a run without the key, builds on the
  public files.
- **game ADR-0117**: the game's own data lives in that repository's
  `content/`, which the build lays over `assets/` **whenever the
  checkout is there**, with no cargo feature (§2). Every CI job that
  loads files checks it out, so the gates read the real game (§3).
  Nothing that embeds it becomes a public workflow artifact: anyone
  signed in to GitHub can download a public repository's artifacts (§4).

The engine already fetches: `shuyi private-assets` clones a sparse
checkout at the pin, and `--pin` moves the pin (`shuyi.ron`'s
`private_assets`, `crates/cli/src/private_assets.rs`). The bundle
already has private layers (`Layer::private`, `Bundle::is_private`). A
new game had neither the layout, the build side nor the CI step.

## Decision

### 1. The private repository's layout

| Folder | Holds | Built |
| ------ | ----- | ----- |
| `files/` | Files at the path they have in the game's files folder (`files/sprites/hero.png` ↔ `crates/game/files/sprites/hero.png`), laid over it: one subfolder per kind picked (`text/`, `sprites/`, `sounds/`, `music/`, `data/`) and any the developer names | Yes, a private layer |
| `docs/` | The story and design documents | No |
| `library/` | Bought packs as they came, sorted; fetched only by `private-assets --library` | No |

2591 (`shuyi repos`) makes the repository with this layout, and 2592
asks which kinds go there. The game checks it out into `private/`, at
the commit `private.rev` names; `/private/` is git-ignored.

### 2. The template's part `private-files`

`templates/shuyi-default`'s question `private-files` (`pick: One`,
`yes`/`no`, default `no`) picks the part `private-files`, which needs
`base`. `shuyi new --private-url https://github.com/<owner>/<repo>.git`
fills its placeholder `{{private_url}}`: the flag is required when the
part is picked and refused when it isn't, and a URL of another shape is
refused (GitHub only: 2591 makes the repository with `gh`). The part
writes:

- `shuyi.ron`'s `private_assets` (`checkout: "private"`, `pin_file:
  "private.rev"`, `folders: ["files", "docs"]`, `library: "library"`),
  merged into the base's;
- an empty `private.rev`: no pin yet;
- `/private/` in `.gitignore`;
- the CI step's action, `.github/actions/private-files/action.yml` (§5);
- a section of `AGENTS.md` and of `README.md`: where the private files
  are, and that they never go in the public repository.

The composer merges the last two: a `.gitignore` gains the lines it
lacks, and a Markdown file gains each later part's text after a blank
line (`docs/templates.md`, *Merged files*).

The pin is a commit of the private repository, which only exists once
2591 has made it, so `private.rev` starts empty. An empty or missing pin
file is *no pin yet*: `shuyi private-assets` says so and how to make the
first, and the CI step fails a run that has the key with the same words.
The file being there from the start is what tells the build to make the
folder it watches (§3).

### 3. The build lays `private/files/` over the game's files, whenever it is there

The code is the **base** part's, the same in every game, so a game can
add the private part later. There is no cargo feature: the checkout is
the switch (game ADR-0117 §2).

- `crates/game/build.rs` names the folder to embed as
  `SHUYI_PRIVATE_FILES`: `<root>/private/files/` when it is a folder,
  else an empty folder in `OUT_DIR` (an `include_dir!` path must exist).
- `src/lib.rs` embeds it and hands it to `BasicGame::private`: a private
  layer named `private`, read before every other layer, so a file there
  replaces the public one at its path and a file only there is added.
  `Bundle::is_private` is true for both.
- **An edition with a content root of its own** (`SHUYI_CONTENT`,
  ADR-0100) gets the empty folder: a public demo built from its own
  folder holds none of the private files.
- **Cargo's watch.** A watched path that doesn't exist reruns the build
  script, and rebuilds the crate, on every build. So the build watches
  `private/files/` when it is there; else the checkout's `.git/index`;
  else, when the game has `private.rev` or `private/` is there, the
  folder `private/`, made empty if missing, for `shuyi private-assets`
  or a clone by hand to fill (as game ADR-0117 §2 does); else only
  `build.rs`. A build that missed a checkout anyway (a base game given
  one by hand, with no `private.rev`) is caught by the template's test
  `the_private_files_are_the_checkouts`, which reads every file of the
  checkout in the bundle and names `cargo clean -p <crate>`.
- **Every file of `files/` is embedded**, a `README.md` too: what explains
  the private repository's folders goes in its own top `README.md`, or
  in `docs/`.
- **The packed build** (ADR-0110) bakes its pack from the tool program's
  bundle, which is the same layers: the pack holds the private files when
  the checkout was there, and the bake's read-back check covers them.
  The packed program reads them from the pack, as one layer.

### 4. `run --watch` watches the private files too

`BasicGame::private`'s layer stays above the layer
`BasicGame::over` lays (`run --watch`'s folder, a page's files, a mod):
the public placeholder in `crates/game/files/` never hides the private
file. `cargo shuyi run --watch` also hands the game `<checkout>/files`,
when the project has `private_assets` and that folder is there, as
`SHUYI_WATCH_PRIVATE` (`shuyi_platform::WATCH_PRIVATE_VAR`). `run_game`
reads it in place of the embedded private layer and watches it, so a
changed private file reloads as one of `files/` does.

### 5. CI: the gates run on the private files

The base's `ci.yml` calls the action before its first cargo command,
guarded by `hashFiles('.github/actions/private-files/action.yml')`: a
workflow is YAML, which the composer doesn't merge, and the step must
sit in the `gates` job, so the part writes the action and the base calls
it when it is there. The action:

- checks out `files/` of the private repository at `private.rev`'s
  commit into `private/`, with `actions/checkout`'s `ssh-key` (the
  secret `PRIVATE_ASSETS_KEY`, an SSH deploy key) and
  `persist-credentials: false`, as the game's
  `.github/actions/private-assets` does;
- **fails** a run on the game's own repository without the key, naming
  the secret and `shuyi repos` (2591): the gates must read the real
  files (game ADR-0117 §3). A pin that isn't a commit fails too, naming
  `private-assets --pin`;
- **warns** and builds on the public files on a pull request from a fork
  (which gets no secrets) and on a run in someone else's copy (whose
  owner isn't the private repository's);
- Dependabot's runs get only Dependabot secrets, so 2591 sets a
  Dependabot secret of the same name, and the step reads it there too.

No workflow of the template uploads an artifact, and none may upload one
that embeds the private files (game ADR-0117 §4). A release workflow
(3050) runs the same action in every job that packages the game.

## Consequences

- A game made with `private-files=yes` keeps its writing and art out of
  its public repository from its first commit, and its gates read them.
  A fork, or a clone without access, builds and runs on the public
  files.
- Every game, private part or not, embeds `private/files/` if it is
  there. A base game has an empty private layer and works as before.
- A record's or a save's `made` fingerprint differs between a build with
  the checkout and one without: they are different games' content.
- Two pins move in one line of `private.rev`, so two pull requests that
  each move it conflict (game ADR-0040 §3).
- The engine's reusable playtest workflow checks out no private files:
  a game's playtest runs on its public files until ticket 2595.
- `check-keys` scans the folders `shuyi.ron` lists, which can't name a
  checkout that may be missing: the private text isn't scanned until
  ticket 2594.

## Alternatives considered

- **A cargo feature** (`private-assets`, as game ADR-0040 had for its
  art). Lost as in game ADR-0117 §2: a build or a gate that forgets the
  feature quietly reads the public files.
- **The step in a workflow of the part's own.** A separate workflow
  can't put the files under the `gates` job's cargo commands, so the
  gates would run on the public files.
- **Cloning with a key file and `GIT_SSH_COMMAND`** in a shell step.
  `actions/checkout`'s `ssh-key` does the same with no key file of our
  own to write and remove, and the template already pins the action.
- **The private layer under the watched folder** (`BasicGame::over`'s
  place). The public placeholder on disk would hide the private file
  under `run --watch`.
- **`/private/` in the base's `.gitignore`.** Harmless, but a game
  without private files would carry a line about them; the composer
  merging ignore files keeps it in the part.
