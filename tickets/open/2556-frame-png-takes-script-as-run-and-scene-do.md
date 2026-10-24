---
id: "2556"
title: "frame-png takes --script, as run and scene do"
type: bug
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2556 — `frame-png` takes `--script`, as `run` and `scene` do

## Context

Found by the skill pack's authors (1134), writing the skills' worked
examples. `cargo shuyi frame-png stage.png --script "Enter Right"`
exits 2 with `unknown option: --script`:

- `docs/contract.md`, *The operations*, says a step is `--script "Enter
  Up"` (or `--keys`) for every operation that plays one, and `run` and
  `scene` take both (`crates/cli/src/contract/run.rs`, `parse_args`).
- `Script::parse_step` (`crates/cli/src/frame_png.rs`) takes `--script`
  as `--keys`. But `frame_png::parse_args`, in the same file, matches only
  `"--keys" | "--pad" | "--wait"` before it hands a step to
  `parse_step`, so `--script` falls to the game's own options and is
  refused.
- The grid template's `crates/rules/AGENTS.md` lists `cargo shuyi
  frame-png stage.png --script "Enter Right"` in its table of tools, so a
  new `grid` game's own rules show a command that fails.
- The skill pack works around it: `skills/shuyi-cinematic/SKILL.md`
  (step 6, "its steps are `--keys`, `--pad` and `--wait`") and
  `skills/shuyi-test/SKILL.md` (step 6, "`--keys`, `--pad` and `--wait`
  are the steps") name only `--keys` for `frame-png`.

## Nick input

None.

## Scope

**In:**
- `frame_png::parse_args` takes `--script` as `--keys`: `"--keys" |
  "--script" | "--pad" | "--wait"`, each handed to `Script::parse_step`
  as now.
- `frame_png::usage` lists `--script "F2 Up f"` beside `--keys`, as the
  same step ("press and release each key chord in turn").
- The skill pack says `frame-png` takes `--script` too:
  - `skills/shuyi-cinematic/SKILL.md`, step 6: "its steps are
    `--script` (or `--keys`), `--pad` and `--wait`", as `run`'s are;
  - `skills/shuyi-test/SKILL.md`, step 6: the same words;
  - any other skill that says `frame-png` takes only `--keys` (grep
    `skills/` for `frame-png`).
  `cargo shuyi check-skills skills --tickets tickets/open` passes.

**Out (do not do):**
- new step kinds, or `--act` (2524 adds actions by name to every
  operation's script);
- changing `run`'s or `scene`'s parsing;
- the grid template's `AGENTS.md` table: once `--script` works, its
  line is right as it is.

## Implementation steps

1. In `crates/cli/src/frame_png.rs`, add `"--script"` to the step flags
   `parse_args` matches.
2. Add `--script` to `frame_png::usage`'s steps.
3. Add the tests below to `crates/cli/src/frame_png/tests.rs`.
4. Change the skills' lines named in Scope, and run
   `cargo xtask skill-examples shuyi-cinematic shuyi-test`.
5. Run the gates.

## Acceptance criteria

- [ ] `cargo shuyi frame-png stage.png --script "Enter Right"` works on a
      game made with `shuyi new --template grid`, and draws the same
      frame as `--keys "Enter Right"`.
- [ ] `frame-png --help` lists `--script`.
- [ ] No skill says `frame-png` takes only `--keys`, and
      `check-skills` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `parse_args` with `--script "Enter Right"` gives the same
  `Script` as `--keys "Enter Right"`; `--script` with a key name the
  Harness doesn't know is an error, as with `--keys`; the usage names
  `--script`.
- Integration: the worked examples of `shuyi-cinematic` and `shuyi-test`
  (CI's `skills` job).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
