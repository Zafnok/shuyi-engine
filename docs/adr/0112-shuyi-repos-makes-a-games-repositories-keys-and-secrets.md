# ADR-0112: `shuyi repos` makes a game's repositories, keys and secrets, and repairs them when run again

- **Status:** Accepted
- **Date:** 2026-11-12
- **Related tickets:** 2591, 2592, 2593
- **Extends:** ADR-0111 (§1 and §5: who makes the private repository, its
  first commit, the deploy keys and the secret)

## Context

Nick decided on 2026-11-12 that a new game's repositories, tokens and
secrets are made for the developer ([decisions](../decisions.md)): the
game's repository public by default (free CI on GitHub), its private
files' repository private. ADR-0111 laid out the private repository and
the game's side (`private_assets`, `private.rev`, the CI step reading
`PRIVATE_ASSETS_KEY`) and left making the GitHub side to ticket 2591.

Visions of Shuyi did each step by hand, and two went wrong (game tickets
0116 and 0156): the first commands were written for the wrong shell, and
the private half of the key was lost, so Dependabot's runs, which read
only Dependabot secrets, needed a second deploy key. A ruleset on the
private repository's `main` was then made with `gh api`.

## Decision

### 1. A command of the CLI, never of the contract

`shuyi repos` (`crates/cli/src/repos.rs`) runs on a project, in the
game's folder. It acts on the developer's GitHub account, so it is not an
operation of the contract (`contract::Op`) and not a tool of the MCP
server; its `--help` says it creates repositories, deploy keys and
secrets. The module is private to `shuyi-cli`: nothing of it is public
API. It is run with GitHub's own CLI, `gh`, signed in, and `git` and
`ssh-keygen` (OpenSSH's), each run as a program with its arguments, never
through a shell. No new dependency.

### 2. What it makes, in order

Each step is skipped when what it makes is there, so a second run makes
nothing and a run after a failure finishes the rest. It stops at the
first error, with the steps done so far in its report.

1. **The checks.** `gh`, `git` and, for a game with private files,
   `ssh-keygen` start; `gh auth status` is signed in to github.com with
   the scopes `repo` and `workflow` (pushing `.github/workflows/` needs
   the second). A token whose scopes `gh` doesn't list isn't refused for
   them.
2. **The game's folder as a git repository**, if it isn't one of its own
   (a folder inside another repository isn't): `git init -b main`, then
   everything committed ("First commit").
3. **The game's repository**, if the folder has no `origin`: `gh repo
   create <owner>/<name> --<visibility> --source . --remote origin`,
   without pushing: a push now would start a CI run before the key
   exists. The owner is `--owner` or the signed-in user; the name is
   `--name` or `contract::new::crate_name` of `shuyi.ron`'s `app.name`. An
   existing `origin` on GitHub is left as it is and is the game's
   repository; one elsewhere is refused.
4. **The private repository** `private_assets.url` names, if it isn't
   there: its first commit made in a temporary folder outside the game,
   from text in the binary (`crates/cli/src/repos/`): a top `README.md`
   (what it is, that it must stay private, each folder and what it
   holds, how a change lands), a `.gitattributes` marking pictures,
   sounds, fonts and archives binary, a `.gitignore`, and a `.gitkeep`
   in `files/<kind>/` for each kind (`--private-folders`, default
   `text,sprites,sounds,music,data`), in `docs/` and in `library/`. No
   README goes inside `files/`: every file there ships in the game
   (ADR-0111 §3). Then `gh repo create … --<visibility> --source <tmp>
   --push`, described "Private files of <name>. Must stay private."
5. **The pin**, if `private.rev` is missing or empty: the private
   repository's `main` commit (`gh api …/commits/main`), then the
   checkout (`shuyi private-assets`). A checkout git can't make (git
   without access to GitHub over HTTPS) is a warning naming `gh auth
   setup-git`, not a failure: the keys and the push don't need it.
6. **The keys**, one for *builds* (Actions) and one for *dependabot*,
   each titled `<name> <use> (read-only)`: an ed25519 key from
   `ssh-keygen -t ed25519 -N "" -C <title>` in a temporary folder, its
   public half a deploy key of the private repository (`gh repo
   deploy-key add`, read-only: never `--allow-write`), its private half
   the game's secret `PRIVATE_ASSETS_KEY` (`gh secret set`, with `--app
   dependabot` for the second), given **on stdin**. A key is there when
   the secret exists and a deploy key of its title does. When a key is
   made, the old deploy keys of its title are deleted first: their
   private half is gone, and a key nobody holds is only a risk.
   `--new-keys` makes both again.
7. **The ruleset** on the private repository's default branch, named
   "Protect main": no deletion, no force push, a pull request, and the
   repository role admin may bypass. GitHub refuses rulesets on a
   private repository of a free personal account, even to list them:
   that refusal (HTTP 403) is a warning, "pushing to its main is not
   blocked", and the run succeeds.
8. **The push**: the pin committed if step 5 wrote it or it has a change
   ("Pin the private files", only `private.rev`), then the folder's
   branch pushed with `--set-upstream` if GitHub hasn't it. A branch
   already pushed is never pushed by `repos`.

A game without `private_assets` gets steps 1 to 3 and 8, and a warning
saying how a game gets private files.

### 3. The private half of a key never leaves the pipe

It is in no argument, output, JSON, log line, error message, file of the
game's folder or file left behind. The temporary folder (`TempDir`) is
removed on drop: after a success, an error or a panic; the key's folder
goes once its public half is a deploy key, before `gh secret set` runs.

### 4. The plan is data

`observe` looks with read-only calls only (`gh auth status`, `gh api` on
the repositories, their branches, secrets, deploy keys and rulesets,
`git rev-parse`, `git remote get-url`, `git status`, the pin file);
`plan(&Inputs, &State) -> Vec<Step>` is pure; every program runs through
a `Host` trait (a command, its arguments, its folder and its stdin). The
real host runs them with `GIT_TERMINAL_PROMPT=0` and
`GH_PROMPT_DISABLED=1`. The tests' host is a scripted `gh`, `git` and
`ssh-keygen` that keeps what they change, so a second run sees the first
one's work, and records every call. No test reaches GitHub. `--dry-run`
runs only the reads and prints the plan.

### 5. Codes, and `doctor`

New problem codes: `SH0006` `repo-name` (a name GitHub refuses, or taken:
a repository with the folder's branch and no `origin` naming it),
`SH1007` `no-gh`, `SH1008` `gh-signed-out`, `SH1009` `gh-scope`,
`SH3013` `not-yours` (a repository the signed-in user can't administer),
`SH3014` `github` (a step `gh` or `git` refused). A missing `git` or
`ssh-keygen` is `SH1003`. `doctor` lists `gh` and `ssh-keygen` as
optional checks (`Check::optional`, contract version 50): they say how
to install them, and never fail it.

## Consequences

- A new game reaches GitHub, with its private files, keys, secrets and
  first green CI run, in one command; 2592 runs it from `shuyi new`, and
  2593's skill tells an agent to ask the developer first.
- A lost key, a deleted secret or a run that stopped half-way is
  repaired by running it again.
- It knows GitHub only, through `gh`; another host is another command.
- A free personal account's private repository gets no ruleset, and
  every run says so.
- The game's repository gets no ruleset, Pages or other secret: that is
  the developer's.
- `doctor::Check` is `#[non_exhaustive]` now (ADR-0039), and `Probe`
  gains `starts`, with a default body.

## Alternatives considered

- **`ssh-key` (RustCrypto) to make the keys in the process.** No file at
  all, but a new dependency for one command, while every machine that
  has `git` for Windows or OpenSSH has `ssh-keygen`. Kept as the
  fallback if `ssh-keygen` proves unusable.
- **One deploy key for both secrets** (one private half set twice). Fewer
  keys, but the two uses couldn't be replaced apart, and game ticket 0156
  shows the second secret is often made later, when the first half is
  gone.
- **`git ls-remote` for the private repository's commit, and `gh repo
  view` for whether a repository is there.** `gh api` reads both with
  `gh`'s own sign-in, where git over HTTPS may have none, and answers
  JSON with the permissions.
- **Pushing with `gh repo create --push`.** The first CI run would start
  before the secret exists, and fail.
- **An operation of the contract, or an MCP tool.** An agent could then
  make repositories and secrets on the developer's account without
  asking; the CLI keeps that to a person running it.
- **Keeping a key that lost its secret.** Its private half is gone, so
  it can only be deleted later by hand; deleting it when its replacement
  is made keeps one key per title.
