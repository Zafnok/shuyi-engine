---
id: "2592"
title: "shuyi new asks whether to put the game on GitHub, each repository's visibility and which kinds of files stay private, then runs shuyi repos"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2590", "2591"]
nick_input: sign-off
completed:
---

# 2592 — `shuyi new` asks about the game's repositories

## Context

Nick, 2026-11-12 (`docs/decisions.md`, *`shuyi new` sets up a game's
repositories*): `shuyi new` should ask whether to set up the git
repositories, name them from the game (`visions-of-shuyi` and
`visions-of-shuyi-assets`), make them with every key and secret, default
the game's to public and the files' to private while asking the
developer, and lay out the private repository the opinionated way
(writing, sprites, sounds…) or with the folders the developer names.

2590 is the template part (`--set private-files=yes --private-url …`)
and the layout; 2591 is `shuyi repos`, which makes the repositories,
keys, secrets and ruleset in a game's folder. This ticket asks the
questions in `shuyi new` and joins the two.

## Nick input

**Sign-off:** run `shuyi new "Repo Test"` in a terminal, answer the
questions, and say whether the wording is clear and the defaults are the
ones you want. Then delete the two repositories it made.

## Scope

**In:**
- **Flags of `shuyi new`** (`crates/cli/src/contract/new.rs`'s
  `parse_args`), each answering one question:
  - `--github` / `--no-github`;
  - `--owner <login or organization>`, `--visibility public|private`,
    `--assets-visibility private|public` (passed on to 2591);
  - `--private <kinds>`: a comma list of `text`, `sprites`, `sounds`,
    `music`, `data`, or `none` for no private repository; default all
    five;
  - `--private-folder <path>`, repeatable: a folder of the developer's
    own (under `files/`, or `docs`/`library`), checked as 2591 checks
    them.
- **The questions**, asked only when stdin and stdout are a terminal
  and no flag answered them, after the template's own questions, each
  with its default in brackets:
  1. "Put <name> on GitHub? It needs the GitHub CLI (gh), signed in.
     [Y/n]" (when `gh` isn't installed, the question says so and the
     default is `n`);
  2. "<owner>/<crate name>: public (GitHub's CI is free on a public
     repository) or private? [public]";
  3. "Keep some files in a second, private repository,
     <owner>/<crate name>-assets, so they aren't in the public code?
     [Y/n]";
  4. "Which kinds? [text, sprites, sounds, music, data]" with each kind
     shown as the folder it makes (`files/text/` — writing: dialogue,
     scripts…), several picked by number or name;
  5. "Other folders? (comma list, empty for none) []";
  6. "<owner>/<crate name>-assets: private or public? [private]".
  Not asked: anything a flag answered. A non-terminal run, `--json`
  and the MCP `new` tool ask nothing and default `--no-github`.
- **What `new` does with the answers:** with a private repository,
  compose with `private-files=yes` and `--private-url
  https://github.com/<owner>/<crate name>-assets.git` (the owner from
  `--owner` or `gh api user`); write the game; then, with `--github`,
  run 2591's command in the new folder with the same owner,
  visibilities and folders, and print its report after `new`'s. A
  failure of `repos` leaves the game written and says to run
  `cargo shuyi repos` in it to finish.
- **The MCP `new` tool and the contract's `new` operation never make a
  repository**: an agent that wants one asks the developer and runs
  the CLI (2593's skill says so). Their JSON schema gains nothing that
  runs `repos`; `--private`/`--private-folder` compose the part only
  when `--private-url` is also given there.
- **A question is a function** from the input and a writer to the
  answer, so the tests feed scripted lines; the terminal check is one
  place, passed in.
- **Docs** (the `document` skill): `shuyi new`'s page in the CLI
  reference, the book's chapter on starting a game, `docs/templates.md`
  if the question list is there.
- **Either order with 2502:** if 2502 is done, these questions are
  asked in its wizard, after the catalog's and the template's, in its
  prompt style; its `--questions --json` lists them with a field saying
  an agent must not answer `github: yes` itself; and its `--no-input`
  means `--no-github`. If not, a plain prompt in `new` as above, and
  2502 moves them into the wizard (its clause says so).

**Out (do not do):**
- The template part and the private layer: 2590.
- Making the repositories, keys and secrets: 2591.
- The skill: 2593.
- Hosts other than GitHub.

## Implementation steps

1. `new.rs`: the flags into the input; their checks; the private URL.
2. A `questions` module beside `new.rs` for the six questions, over a
   `BufRead` and a `Write`.
3. The CLI's `new` (`crates/cli/src/contract/cli.rs`): the terminal
   check, the questions, compose and write, then `repos::run`.
4. Docs, `cargo xtask gates`.
5. Run it once for real, then delete what it made, before the sign-off.

## Acceptance criteria

- [ ] `shuyi new "Moon Garden"` in a terminal, with every default taken,
      makes the game, `<you>/moon-garden` (public) and
      `<you>/moon-garden-assets` (private, with `files/text`,
      `files/sprites`, `files/sounds`, `files/music`, `files/data`,
      `docs`, `library`), and the game's first CI run passes.
- [ ] `--no-github` asks nothing and makes only the folder, as today.
- [ ] `--private none --github` makes only the public repository, with
      no `private_assets` in `shuyi.ron`.
- [ ] `--private sprites --private-folder files/levels` makes only
      `files/sprites` and `files/levels` (with `docs` and `library`).
- [ ] A run with stdin not a terminal, `--json` and the MCP `new` tool
      ask nothing and make no repository.
- [ ] A `repos` failure leaves the game folder and names `cargo shuyi
      repos`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each flag's parse and refusal; each question with a scripted
  answer, an empty line (the default) and a wrong answer asked again;
  the private URL from the owner and name.
- Integration: `new` with scripted answers and 2591's test host: the
  composed files and the recorded `repos` calls; a non-terminal run
  recording no call.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
