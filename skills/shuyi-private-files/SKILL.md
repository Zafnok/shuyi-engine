---
name: shuyi-private-files
description: Change a game's private files, the writing, art and sounds kept in a second, private repository beside its public one (`shuyi new --set private-files=yes`). The checkout in `private/` (`cargo shuyi private-assets`), a branch there made with `--no-track`, a pull request in the private repository, then the game's pull request moving the pin `private.rev` (`private-assets --pin`), two pull requests that both move it, what never goes in public, and setting the repositories up (`cargo shuyi repos`, only once the developer says yes). Use when a change edits, adds or removes a file of `private/files/` or `private/docs/`, when the game's CI needs private files that aren't on the private repository's main yet, or when `private.rev` conflicts. Not for the game's public files (`crates/game/files/`) or a release (shuyi-ship).
license: MIT OR Apache-2.0
metadata:
  version: "1.0"
---

# Change the private files

A game made with `shuyi new --set private-files=yes --private-url
<repository>` keeps the files it may not hand out (its writing, its art,
its sounds) in a second, private repository, the one `shuyi.ron`'s
`private_assets.url` names. The public repository holds the code and
public placeholders. The game builds with one commit of the private
repository, the **pin**, which `private.rev` names. A change there is
two pull requests: the private repository's first, then the game's,
which moves the pin.

## When to use it

- A change edits, adds, renames or removes a file of `private/files/`
  (what the game reads) or `private/docs/` (the story and design
  documents).
- The game's code needs a private file that isn't on the private
  repository's `main` yet.
- Two pull requests both move `private.rev`.
- The repositories, the keys or the secret aren't there yet, or broke.

## When not to

- A public file of the game (`crates/game/files/`): an ordinary change.
- A release, its packages and their credits: `shuyi-ship`.
- A game with no `private_assets` in `shuyi.ron`: it has no private
  files.

## 1. What is where

| Where | Holds | Built |
| ----- | ----- | ----- |
| `private/` | The checkout of the private repository, git-ignored by the game | |
| `private/files/` | Files at the path they have in `crates/game/files/` (`private/files/sprites/hero.png` ↔ `crates/game/files/sprites/hero.png`) | Yes, laid over the game's files |
| `private/docs/` | The story and design documents | Never |
| `private/library/` | Bought packs as they came, only with `--library` | Never |
| `private.rev` | The pin: the private repository's commit the game builds with | |

The build lays `private/files/` over `crates/game/files/` **whenever the
checkout is there**, with nothing to turn on: a private file replaces
the public one at its path, and a file only there is added. A fork's
pull request, or a machine without access, builds and runs on the public
files. CI checks the pin out before the gates, with the secret
`PRIVATE_ASSETS_KEY`, so the gates read the real game.

## 2. The checkout

From the game's folder:

```bash
cargo shuyi private-assets
```

It clones the private repository into `private/` at the pin, or fetches
and puts a checkout that is there at the pin. `--library` brings
`library/` too. Without access it says so, and the game still builds on
its public files, but this skill can't be done: **say so to the
developer and stop.**

A build made before the checkout may still hold the public files. The
game's test `the_private_files_are_the_checkouts` catches it and names
the command: `cargo clean -p <the game's crate>`.

## 3. A branch there

```bash
git -C private fetch origin
git -C private switch --no-track -c <branch> origin/main
```

`--no-track` matters: a branch made from `origin/main` that tracks it
pushes to `main` on a plain `git push`. Name the branch after the
change (the game's ticket, if it has tickets).

**Never push the private repository's `main`**, even where its ruleset
lets the owner, or a session with the owner's token, bypass it. A file
removed or renamed there rides into the next pin move of whoever comes
next, and breaks their build.

Running `cargo shuyi private-assets` again puts the checkout back at
the pin: commit and push the branch first.

## 4. Change, commit, push, pull request

Edit `private/files/` or `private/docs/`, and the game's code as the
change needs. `cargo build`, `cargo test` and `cargo shuyi run --watch`
read the checkout as it is, so the game's gates (`shuyi-gates`) check
the change before anything is pushed.

```bash
git -C private add files docs
git -C private commit -m "<what changed>"
git -C private push origin HEAD:refs/heads/<branch>
gh pr create --repo <owner>/<private repository> --base main --head <branch> --title "<title>" --body-file <file>
```

Push by the branch's full name, never `HEAD` alone. The `--repo` is
the private repository's `<owner>/<name>`, from `private_assets.url`.

Open the game's pull request with its code beside it, and link the two:
each names the other. **While the private one isn't merged, the game's
CI reads the old pin**, so a game change that needs the new files fails
there until the pin moves. Once the private pull request is merged:

```bash
git -C private fetch origin
git -C private switch --detach origin/main
cargo shuyi private-assets --pin
```

`--pin` writes the checkout's commit into `private.rev`, after checking
it has no uncommitted change and is on the private `main` (a squash
merge leaves the branch's own commit on no branch: that is why the
checkout moves to `origin/main` first). If it refuses with "isn't
merged into the private repository's main branch yet", the private pull
request isn't merged: wait for it, then fetch, switch to `origin/main`
and run `--pin` again, as the refusal says. Commit `private.rev` in the game's pull request, push, and
merge it once CI is green.

Moving the pin brings in every commit on the private `main` since the
old pin, not only this change's. Before committing it, list them:

```bash
git -C private log --oneline "$(cat private.rev)..origin/main"
```

Each one that isn't this change's must have its own game pull request
merged already, or the game's `main` would build with files its code
doesn't expect. If one hasn't, wait for it, or ask the developer.

## 5. Two pull requests moving the pin

The second to merge has a conflict on `private.rev`'s one line. On its
branch, merge the game's `main`, then:

```bash
git -C private fetch origin
git -C private switch --detach origin/main
cargo shuyi private-assets --pin
```

List what the pin carries again (step 4), commit, push. The new pin
holds both changes.

## 6. Never in public

The public repository, its history, its issues and pull requests, and
its workflows' artifacts are anyone's. So:

- no private file is copied into the public repository, not even for a
  test or as a fixture;
- none is quoted in an issue, a pull request, a commit message or a
  review, beyond the file's path and what the change needs to be
  understood;
- no workflow uploads an artifact that embeds them, but a release's
  packages, which publish them inside the program anyway;
- no job that checks them out caches `target/`: its `rust-cache` step
  keeps `cache-targets` false when the private files' action is there,
  and any other cache waits on the same guard. A pull request's run may
  restore its base branch's caches.

A failing test's log may quote a line of a private file. That is the
price of testing the real files: keep it to what the failure needs.

## 7. Setting up

`cargo shuyi repos` makes, or repairs when run again, the game's
repository, the private one with its folders, the pin and the checkout,
one read-only deploy key whose private half is the Actions secret
`PRIVATE_ASSETS_KEY`, and a ruleset on the private `main`. Dependabot
gets no key: its pull requests build on the public files, so a
dependency's new version never runs its build next to the private
files before a person merges it. A Dependabot secret an older `repos`
made is warned of, with the commands that delete it. **It acts on
the developer's GitHub account: ask the developer before running it, and
never run it on your own.** With their yes, show them `cargo shuyi repos
--dry-run`'s plan first, then run it with their answers as flags.
`--new-keys` replaces the key and its secret: only when they ask.

Who can read the private files: the game's own CI, and so **anyone with
write access to the game's repository**, since a workflow they push runs
with the key. Tell the developer this before they add a collaborator:
write access to the public repository is read access to the private
one.

The same holds for `shuyi new --github`. The MCP `new` tool and `new
--json` never make a repository (`--github` is refused with `--json`),
so an agent that wants one asks the developer, then runs the CLI's `new
--github`, or `repos` in the game, with their answers.

Its problem codes, each explained by `cargo shuyi explain <code>`:

- `SH1007`, `SH1008`, `SH1009`: `gh` isn't installed, isn't signed in,
  or lacks a scope. The developer fixes it: say which.
- `SH0006`: GitHub refuses the repository's name, or it is taken.
- `SH3013`: the repository exists, and the account can't administer it.
- `SH3014`: a step on GitHub or in git failed; running it again goes on
  from there.

On a free account's private repository the ruleset is a warning, not a
failure: GitHub keeps rulesets for paid plans there, so step 3's rule is
the only guard.

## Worked example

[`example.ron`](example.ron) runs on a new `blank` game, with no private
repository: the title says the public subtitle; a file laid in
`private/files/` by hand ([`example/en.ron`](example/en.ron)) is caught
by `the_private_files_are_the_checkouts` until `cargo clean -p`; then the
title says the private file's subtitle, and the public file is
unchanged.
