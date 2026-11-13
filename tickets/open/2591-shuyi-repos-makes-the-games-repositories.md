---
id: "2591"
title: "shuyi repos: makes a game's public repository and its private files' repository on GitHub, with the read-only keys, the secrets and a ruleset, and repairs them when run again"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2590"]
nick_input: sign-off
completed:
---

# 2591 — `shuyi repos`: a game's repositories, keys and secrets

## Context

Nick, 2026-11-12 (`docs/decisions.md`, *`shuyi new` sets up a game's
repositories*): naming a game "Visions of Shuyi" should give a
repository `visions-of-shuyi` and a private one `visions-of-shuyi-assets`,
with every token and secret made for you, the game's repository public
by default (free CI on GitHub) and the files' private (they need no CI).

Each step below was done by hand for Visions of Shuyi, across several
game tickets, and two of them went wrong:

- game ticket 0116 (`tickets/done/0116-upload-private-assets-and-build-key.md`):
  `gh repo create … --private --source … --push`; an ed25519 key made
  with `ssh-keygen`, its public half a read-only deploy key of the
  private repository, its private half the game's Actions secret
  `PRIVATE_ASSETS_KEY`, then both files deleted. The first version of
  the command was written for the wrong shell and failed on Nick's
  machine.
- game ticket 0156: Dependabot runs read only Dependabot secrets, so the
  key had to be stored a second time; the private half had been lost,
  so a **second** read-only deploy key ("… dependabot (read-only)")
  was made for it. A ruleset on the private repository's `main` (no
  force push, no deletion, a pull request required, the owner may
  bypass) was made with `gh api`.

**From 2590 (done, ADR-0111):** a game made with the private part has
an empty `private.rev` (no pin yet, which this ticket's step 4 fills),
and its CI step is the action `.github/actions/private-files`, called
from `ci.yml`'s `gates` job with `PRIVATE_ASSETS_KEY`. Every file of
the private repository's `files/` is embedded in the game, a
`README.md` too (ADR-0111 §3): so step 3's README of `files/` and of its
kind folders would ship inside the game. Put what explains them in the
repository's top `README.md` (or in `docs/`), and keep each empty kind
folder with a `.gitkeep`. ADR-0111 is accepted: the list of what `repos`
makes goes in a new ADR that extends it (its `Status:` line naming the
new one), not in ADR-0111's text.

2590 makes the game side: `shuyi.ron`'s `private_assets` (its `url`
names the private repository), the `private/` checkout laid over the
game's files, `private.rev`, and the CI step that reads
`PRIVATE_ASSETS_KEY`. This ticket makes the GitHub side, as a command a
developer runs once in a game's folder, and again to repair what is
missing. 2592 runs it from `shuyi new`.

## Nick input

**Sign-off:** in a scratch folder, run `shuyi new "Repo Test"
--template blank --set private-files=yes --private-url
https://github.com/<you>/repo-test-assets.git`, then `shuyi repos` in
it. Look at the two repositories on GitHub: is anything you set up by
hand for Visions of Shuyi missing? Then delete both repositories.

## Scope

**In:**
- **`shuyi repos`**, a CLI command on a project (`crates/cli/src/repos.rs`,
  wired in `crates/cli/src/lib.rs` beside `private-assets`). Flags:
  - `--owner <login or organization>`: default the signed-in user
    (`gh api user --jq .login`);
  - `--name <repository>`: default `contract::new::crate_name` of
    `shuyi.ron`'s `app` name ("Visions of Shuyi" → `visions-of-shuyi`);
  - `--visibility public|private`: the game's repository, default
    `public`;
  - `--assets-visibility private|public`: default `private`; `public`
    asks for `--yes` (it defeats the point) and says why;
  - `--new-keys`: replace both deploy keys and both secrets (a lost
    private half);
  - `--dry-run`: print the plan and change nothing; `--json`: the plan
    or the outcome as JSON.
  The private repository is the one `private_assets.url` names (2590):
  this command never edits `shuyi.ron`. A project without
  `private_assets` makes only the game's repository and says how to add
  the private part (2590's docs).
- **What it does, in order, each step skipped when already done** (so a
  second run makes nothing, and a run after a failure finishes the
  rest):
  1. `gh` is installed and signed in (`gh auth status`) with the scopes
     it needs; else a problem code naming `gh auth login` /
     `gh auth refresh -s <scope>`. `git` and `ssh-keygen` are on `PATH`
     (Windows has `ssh-keygen` with its OpenSSH, and Git for Windows
     brings one).
  2. **The game's repository:** if the folder isn't a git repository,
     `git init -b main`, add everything, first commit. If it has no
     `origin`, `gh repo create <owner>/<name> --<visibility> --source .
     --remote origin`, **without pushing yet**: a push now would start
     a CI run before the key exists, and 2590's step fails a run on the
     game's own repository without it. An existing `origin` is left as
     it is and named in the report.
  3. **The private repository**, if it doesn't exist
     (`gh repo view`): its first commit made in a temporary folder
     outside the game: a `README.md` (what it is, that it must stay
     private, the folders and what each holds, how a change lands: a
     pull request here, then the game's pull request moving the pin
     with `shuyi private-assets --pin`), a `.gitattributes` marking
     pictures and sounds binary, a `.gitignore`, and each folder of
     2590's layout with a `README.md` saying what goes in it:
     `files/` and its kind folders, `docs/`, `library/`. Which kind
     folders: `--private-folders <list>`, default `text,sprites,sounds,
     music,data` (2592 asks); a folder name is a relative path of
     letters, digits, `-` and `_` under `files/`, or `docs`/`library`.
     Then `gh repo create … --<assets-visibility> --source <tmp>
     --push` and the description "Private files of <name>. Must stay
     private."
  4. **The pin:** if `private.rev` is missing or empty, the private
     repository's `main` commit (`git ls-remote`) is written there, and
     `shuyi private-assets` is run to make the checkout.
  5. **The keys:** for each of *builds* (Actions) and *dependabot*: an
     ed25519 key from `ssh-keygen -t ed25519 -N "" -C "<name> <use>
     (read-only)"` in a temporary folder; its public half added with
     `gh repo deploy-key add --repo <private repo> --title "<name>
     <use> (read-only)"` (read-only: never `--allow-write`); its private
     half given to `gh secret set PRIVATE_ASSETS_KEY --repo <game repo>`
     (with `--app dependabot` for the second) **on stdin**; the
     temporary folder removed whatever happens (a guard that removes it
     on drop). A step is done when the secret exists and a deploy key
     of that title exists; with `--new-keys`, the old deploy keys of
     those titles are deleted first.
  6. **The ruleset** on the private repository's default branch: no
     deletion, no force push, a pull request required, the repository
     admin role may bypass (`gh api repos/<o>/<r>/rulesets -X POST`,
     its JSON built with `serde_json`). GitHub refuses rulesets on a
     private repository of a free personal account: that refusal is a
     **warning** in the report ("pushing to main is not blocked"),
     never a failure.
  7. **The push:** if step 4 wrote `private.rev`, commit it ("Pin the
     private files"); then, if `main` has never been pushed, `git push
     -u origin main`. The first CI run has the key. A branch already
     pushed is never pushed by this command.
  8. The report: both URLs, what was made, what was already there,
     each warning, and the next step (`cargo shuyi run`).
- **The private half of a key is never** in an argument, the output,
  the JSON, a log line, the game's folder or a file left behind.
- **It acts on the developer's account**, so it is CLI only: not an
  operation of the contract (`contract::Op`) and not an MCP tool. Its
  `--help` says it creates repositories and secrets. 2593's skill
  tells an agent to ask the developer before running it.
- **The plan is data.** A pure function from the inputs and what
  exists (repositories, keys, secrets, `origin`, pin) to a list of
  `Step`s; a `Host` trait runs a command with its arguments and stdin
  and returns its output. The real host spawns the programs (as
  `private_assets::git` does, with `GIT_TERMINAL_PROMPT=0` and
  `GH_PROMPT_DISABLED=1`); the tests' host records the calls and
  answers from a script. No test reaches GitHub.
- **New problem codes** in `crate::contract::codes`, each with its
  `explain` text: no `gh`, not signed in, a missing scope, a name
  GitHub refuses, a repository that exists but isn't the owner's.
- **`shuyi doctor`** lists `gh` and `ssh-keygen` as optional, "only for
  `shuyi repos`".
- **Docs** (the `document` skill): the command's page in the CLI
  reference, the book's chapter on starting a game, the template's
  `README.md` (*Putting the game on GitHub*), and 2590's ADR gains the
  list of what `repos` makes.
- **A new dependency**: none planned (`ssh-keygen` and `gh` are run,
  not linked). If the worker finds `ssh-keygen` unusable, `ssh-key`
  (RustCrypto, MIT OR Apache-2.0, checked with `cargo deny check`) is
  the fallback, and the ticket's notes say why.

**Out (do not do):**
- Asking these questions in `shuyi new`: 2592.
- The skill for changing the private files: 2593.
- A ruleset or branch protection on the game's repository, Pages, a
  Sonar token or any other secret than `PRIVATE_ASSETS_KEY`.
- Hosts other than GitHub.
- Writing to `shuyi.ron`: the URL is 2590's `--private-url`.
- Anything in the game repository: Visions of Shuyi keeps its setup.

## Implementation steps

1. `crates/cli/src/repos.rs`: the flags, the observed state, the plan
   (`fn plan(&Inputs, &State) -> Vec<Step>`), the `Host` trait and its
   real implementation, the temporary-folder guard.
2. Observe the state with read-only calls: `gh auth status`, `gh api
   user`, `gh repo view` for both, `gh repo deploy-key list`, `gh
   secret list` and `gh secret list --app dependabot`, `gh api
   …/rulesets`, `git remote get-url origin`, the pin file.
3. Run the steps; stop at the first error, with what was done so far
   in the report, so a second run finishes.
4. The private repository's first commit: write its files from text in
   the binary (a folder under `crates/cli/src/repos/`), not from the
   game.
5. Codes, `doctor`, `--help`, docs; `cargo xtask gates`.
6. Run it once for real against a scratch owner before asking for the
   sign-off, and delete what it made.

## Acceptance criteria

- [ ] In a game made by `shuyi new … --set private-files=yes
      --private-url https://github.com/<o>/<n>-assets.git`, `shuyi repos`
      makes `<o>/<n>` (public) holding the game, `<o>/<n>-assets`
      (private) holding the README and the folders, two read-only deploy
      keys there, `PRIVATE_ASSETS_KEY` as an Actions and a Dependabot
      secret of the game, the ruleset (or the warning), `private.rev` at
      the private repository's commit and the `private/` checkout; the
      game's CI run on `main` passes with the private step taking the
      key.
- [ ] A second run makes nothing and says everything is there.
- [ ] With the Dependabot secret deleted by hand, a run makes only that
      key and secret.
- [ ] `--new-keys` replaces both deploy keys and both secrets.
- [ ] `--dry-run` prints the plan and runs no command that changes
      anything (the test host sees only read-only calls).
- [ ] The private half of a key appears in no argument, output, JSON or
      file left behind (a test over the recorded calls and the
      temporary folder).
- [ ] A refused ruleset is a warning and the command succeeds.
- [ ] Without `gh`, or signed out, it fails with its code and says how
      to fix it.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the plan for each state (nothing there; everything there; each
  piece missing alone; `--new-keys`; no `private_assets`); the name
  from `app.name`; the folder list's refusals; the ruleset JSON.
- Integration with the test host: a full run's recorded calls, in
  order; the stdin of `gh secret set` is the key and no argument is;
  the temporary folder is gone after a success and after a failure at
  each step; a failure half-way then a second run finishing.
- No test reaches the network.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
