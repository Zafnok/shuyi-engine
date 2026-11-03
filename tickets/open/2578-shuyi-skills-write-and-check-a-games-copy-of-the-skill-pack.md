---
id: "2578"
title: "shuyi skills: write a game's copy of the skill pack, and --check that it is this shuyi's"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2578 — `shuyi skills [--check]`

## Context

A request from the game.

1. **The game ticket:** game#1224, which deletes the game's own copier
   once this lands
   ([`1224-keep-the-skill-pack-with-shuyi-skills.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1224-keep-the-skill-pack-with-shuyi-skills.md)).
   It is not a blocker of this one.
2. **The job in one sentence:** after moving to a newer engine, bring the
   game's `.claude/skills/shuyi-*` up to that engine's pack, and fail in
   CI when someone forgot.
3. **The friction numbers:** no friction review: found by the
   game/engine boundary audit of 2026-11-03. The game wrote it itself
   (game ticket 1192, game ADR-0097): `crates/xtask/src/skill_pack.rs`,
   114 production lines and about 100 of tests, over `Pack::embedded`.
4. **The generic shape:** `shuyi new` writes the pack once
   (`template::compose::with_skills`: every skill but the modules'), and
   `shuyi add` copies a module's skill once (`catalog::copy_skill`).
   Nothing keeps that copy current when the game's engine moves, and
   `check-skills` checks a pack's own rules, not whether a game's copy
   is this `shuyi`'s. `shuyi skills` writes the copy, and `--check`
   lists each difference.
5. **An example from a game that isn't tactics:** a platformer made
   with `shuyi new` in March moves its engine dependency in June; its
   agent still reads March's `shuyi-screen` and calls a renamed
   function. `cargo shuyi skills` brings June's pack in.

## Nick input

None.

## Scope

**In:**
- **Which skills a game holds** (`skills::wanted(pack, catalog,
  game) -> Vec<&Skill>`): every skill of `Pack::embedded()` but a
  module's (`Catalog::is_module`, as `with_skills` does), plus the skill
  of each module the game's `Cargo.toml` turns on
  (`catalog::features_on`), plus a module skill whose folder the game
  already holds (it kept one from `shuyi add`, or by hand).
- **`skills`** writes each wanted skill's files into `.claude/skills/`
  (`catalog::SKILLS`) with `\r\n` made `\n` in text files, after
  removing that skill's folder; it removes a `shuyi-*` folder the pack
  has no skill of (one the pack dropped). Folders not named `shuyi-*`
  are the game's own and never touched. It prints what it wrote and
  removed. With no `.claude/skills/` it says so and writes nothing
  (exit 0), as `copy_skill`'s `NoSkills` does.
- **`skills --check`** writes nothing and prints one line for each
  difference, `.claude/skills/<skill>/<path>: <what>`: `missing`, `not
  this shuyi's` (bytes differ after `\r\n` → `\n`), `not in this shuyi's
  pack` (a file the skill lacks), and `<skill>/: this shuyi's pack has
  no such skill`. Exit 0 with none, 1 with some, 2 for wrong arguments,
  as `check-skills` does.
- `"skills"` in `COMMANDS`, a line in `usage`, and its arm in
  `lib.rs` beside `check-skills`.
- `skills/README.md` (how a game gets the pack) says that `cargo shuyi
  skills` keeps the copy current after the engine moves.
- Additive, no API break. The game's PR that moves the pin follows this
  one (game#1224).

**Out (do not do):**
- Rewriting a game's code or files after a break (2517's and 2523's
  jobs); this keeps only the skill copy.
- Adding `skills --check` to a new game's gates in the template.
- Changing `shuyi new`, `shuyi add` or `check-skills`.
- A contract operation or an MCP tool for it.

## Implementation steps

1. Read `crates/cli/src/skills.rs` (`Pack`, `embedded_files`),
   `catalog.rs` (`SKILLS`, `is_module`, `features_on`, `copy_skill`) and
   `template/compose.rs::with_skills`, and the game's
   `crates/xtask/src/skill_pack.rs` at its `main` for the behavior to
   match.
2. New `crates/cli/src/skills/install.rs` (`pub mod install` in
   `skills.rs`): `pub fn wanted`, `pub fn problems(game, pack, catalog)
   -> Vec<String>` and `pub fn write(game, pack, catalog) ->
   Result<String, String>`, tests first. Public, so a game's own test
   can run the check in `cargo test`.
3. `lib.rs`: the `skills [--check]` arm, `COMMANDS` (`[&str; 14]`) and
   `usage`.
4. `skills/README.md`.
5. **Measure:** about 120 production lines and 120 test lines, judged by
   the game's 114.

## Acceptance criteria

- [ ] On a game made by `shuyi new`, `shuyi skills --check` passes; after
      editing a pack file, adding a file to a pack skill, deleting one, or
      adding `.claude/skills/shuyi-gone/`, it fails naming each.
- [ ] `shuyi skills` then makes `--check` pass, and leaves a game's own
      `.claude/skills/write-ticket/` byte for byte.
- [ ] A module's skill is written only when the module is on or its
      folder is already there.
- [ ] A file with `\r\n` endings and the pack's `\n` bytes is no
      difference.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`skills/install.rs`, on a temporary folder and a small `Pack`
  built in the test): each kind of difference; write then check; a
  game's own folder untouched; a dropped `shuyi-*` folder removed; the
  module rule both ways; the line-ending rule.
- Integration (`crates/cli/tests/it/skills.rs`): the binary's `skills
  --check` exit codes 0, 1 and 2.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
