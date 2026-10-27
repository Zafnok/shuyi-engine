---
id: "2006"
title: "check-keys and check-text on the engine's own code: a shuyi.ron at the root and both checks in CI"
type: infra
milestone: MVP
model: sonnet-5
effort: medium
status: done
blocked_by: ["2002"]
nick_input: none
completed: 2026-10-27
---

# 2006 — check-keys and check-text on the engine's own code

## Context

`cargo shuyi check-keys` fails when code or text names a key instead of
an action, and `cargo shuyi check-text` when a screen writes its text as a
literal (`AGENTS.md`, rule 8; the `input-actions` skill). In the game's
repository they scanned only the game's crates: the engine's code was in
`engine/`, which its `shuyi.ron` left out on purpose while the carve was
moving things (game ADR-0063). The code came here with the game's ticket
1131 (game#1131), and nothing scans it yet.

The engine's own key pipeline is allowed to name keys: `shuyi-app`'s
`keys.rs` and `pads.rs`, the keymap format (`shuyi-assets::keymap`), the
input runtime (`shuyi-ui::input`), the rebinding screen's capture, the
test Harness, and the sample game's actions (`keymap/sample.rs`).

## Nick input

None.

## Scope

**In:**
- A `shuyi.ron` at the repository's root with `check_keys` and
  `check_text` over `crates/` and `examples/`: the key pipeline above as
  `key_files`, `crates/app/src/keys.rs` as the `platform_file`, the test
  support files, and the engine's own screens' words.
- Fix what the checks find, or list a file with the reason beside it.
- **From 2511 (done):** `UiAction`'s fixed key (plain `Escape` for
  `Cancel`, `crates/assets/src/keymap/action_set.rs`, ADR-0036) is
  part of the keymap format: list `action_set.rs` in `key_files` with
  the other keymap files, and keep the key.
- Both checks in CI and in the `run-gates` skill, as one command:
  `cargo run -q -p shuyi-cli -- lint` (2534), which runs `check-keys` and
  `check-text` and gives each finding a problem code, a file and a line.
  The engine's `shuyi.ron` names no `rules` package, so the `lint`
  answer's limit about the rules crate's fence is expected: the engine's
  own layers are `cargo xtask check-layers`' (ADR-0032). The CI step
  goes in the `test` job or a job of its own gated like it, since it
  needs the code built.
- **Either order with 2330:** if 2330 is done, an engine screen is
  built from a `.ui.ron`: list that file's folder in `check_keys`'
  `assets`, so its strings are scanned as any `.ron` text is. If not,
  nothing to do (2330 adds its folder to this `shuyi.ron`).

**Out (do not do):**
- Changing what the checks look for.
- Checking that every text key the engine's code names has English:
  2351's test does that (`crates/ui/tests/it/engine_english.rs`).
  `check-text` keeps counting literals.

## Implementation steps

1. Write the `shuyi.ron` and run both checks.
2. Fix or list each finding.
3. Add them to CI and the skill.

## Acceptance criteria

- [x] `cargo run -q -p shuyi-cli -- lint` at the root has no finding,
      and runs in CI and in `run-gates`.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- None beyond the checks' own tests.

## Completion notes

Added the root `shuyi.ron`: `check_keys` over `crates` and `examples`,
`check_text` over the engine's own screens (`crates/ui/src/screens`), and a
step in CI's `test` job (Linux leg) running `cargo run -q -p shuyi-cli --
lint`. `run-gates` and `input-actions` say so.

- `key_files` holds, each with its reason in the file: the platform
  readers (`keys.rs`, `pads/native.rs`), the Harness and the link's key
  scripts, the keymap format with `action_set.rs` (the fixed `Cancel` key,
  ADR-0036) and `sample.rs`, the input runtime, the toy game, the
  hello-grid example's actions, and the tools that take or print key names
  as text (`cli`, `xtask`). `picture_files` lists the two rebinding
  pictures' files.
- Fixed in code: five `Debug` type names and one view-owned `.name` got
  the checks' own `// check-text: not ...` comments in the engine's
  screens; a doc comment in `crates/cli/src/project.rs` no longer quotes
  the marker text.
- `check_text` scans the screens' folder only: its `dir` is one folder,
  and the rest of `crates/ui/src` holds the toy game, log, snapshot and
  fit messages that are not screens' text. `max_literals` stays 0.
- 2330 and 2351 each got a "From 2006 (done)" line for their folders.

No `Breaking:` change; no game change. Nick input: none.
