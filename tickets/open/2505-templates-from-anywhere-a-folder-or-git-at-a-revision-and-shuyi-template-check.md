---
id: "2505"
title: "Templates from anywhere: a folder or a git repository at a revision, no code run, shuyi template check and shuyi template new"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: ["2502"]
nick_input: none
completed:
---

# 2505 — Templates from anywhere

## Context

Nick, 2026-10-17 (`docs/decisions.md`): templates, "and let it be
extensible too".
[ADR-0013](../../docs/adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)
§7: anyone can write a template in the same format as the engine's own
(2501). `shuyi new --template` takes a folder or a git repository at a
pinned revision. No code from a template ever runs when a game is made,
unlike cargo-generate's and Yeoman's hooks. The template's own questions
join the wizard's (2502).

## Nick input

None.

## Scope

**In:**
- **`--template <source>`** takes:
  - a short name of the built-in template's answer sets (`grid`,
    `blank`), as today;
  - a path to a folder holding `template.ron`;
  - `git+<url>@<rev>`, where `<rev>` is a commit hash or a tag. A
    missing `@<rev>` is an error that says why it is required (the same
    command must always make the same game). `shuyi-cli` runs the `git`
    program into a temporary folder (`git clone --no-checkout`, then
    `git checkout <rev>`), and `shuyi doctor` (1129) reports whether
    `git` is found. No git library is added.
- **Nothing runs.** The loader reads `template.ron` and files, and
  nothing else. A template folder holding `build.rs`, scripts or
  binaries is fine: they are copied as files and run only when the
  developer builds the game. A test checks that composing a fixture
  template with a `build.rs` that would write a marker file leaves no
  marker.
- **Before writing,** for any template not built in, the wizard shows
  its source, revision, `name`, `version` and `license`, and asks to go
  on. `--yes` skips that question; `--no-input` without `--yes` refuses,
  so CI never uses a stranger's template by accident. A license outside
  ADR-0002 §8 adds a warning line; it isn't a refusal.
- **`shuyi template check <dir> [--build]`**, a contract operation with
  JSON output:
  - everything `Template::load` checks (2501);
  - every placeholder in every file is known;
  - every combination of the template's own answers and the perspective
    answers its parts name composes, or fails with a `requires` or
    `conflicts` rule the template states;
  - with `--build`: for each combination that changes the set of parts,
    make a game in a temporary folder and `cargo build` it. Cap it at 64
    games, and say in `limits` when the cap cut it short.
- **`shuyi template new <dir>`** writes a small template: a `base` part
  and one `stage-top-down-grid` part, copied from the built-in one, with
  `id`, `name` and `license` set from flags or questions. `shuyi template
  check` passes on it.
- **The built-in template passes `shuyi template check --build`** in the
  engine's CI.
- **`docs/templates.md`** (2501) gets "Writing your own template" and
  "Sharing it": a git repository, the command with `@<rev>`, and what the
  wizard shows users before it writes.
- **The skill pack:** if 1134 is done, the `shuyi-new-game` skill learns
  `--template` sources and that it must show the user the source and
  license before passing `--yes`. If not, add that to 1134's scope.

**Out (do not do):**
- a template registry or a search command (ADR-0013 §7);
- hooks, scripts or any code run by `shuyi new` (never);
- templates written in another format (cargo-generate's).

## Implementation steps

1. Write the source parsing and the git fetch, with errors that say how
   to fix each case.
2. Show the template's details, and add `--yes`.
3. Write `template check`, then `--build` with the cap.
4. Write `template new`.
5. Add the engine CI job, the docs and the skill.

## Acceptance criteria

- [ ] `shuyi new demo --template <fixture folder> --yes --no-input`
      makes a game from a fixture template outside `templates/`.
- [ ] `shuyi new demo --template git+file://<fixture repo>@<hash> --yes
      --no-input` makes the same game, and without `@<hash>` fails with
      the reason.
- [ ] `--no-input` without `--yes` refuses a template not built in.
- [ ] The `build.rs` marker test passes: nothing from the template ran.
- [ ] `shuyi template check` passes on the built-in template and on the
      output of `shuyi template new`, and fails on fixture templates for
      each rule, naming the file and rule.
- [ ] `shuyi template check --json` matches its schema.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: source parsing; each `template check` rule on fixtures.
- Integration: a template from a folder and from a local git repository
  made in the test (`git init`, commit, use the hash); the marker test;
  `template new` then `template check --build`.

## Completion notes
