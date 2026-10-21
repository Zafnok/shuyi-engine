---
id: "2006"
title: "check-keys and check-text on the engine's own code: a shuyi.ron at the root and both checks in CI"
type: infra
milestone: MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: ["2002"]
nick_input: none
completed:
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
- Both checks in CI (`ci.yml`, beside the ticket lint 2002 adds) and in
  the `run-gates` skill.

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

- [ ] `cargo shuyi check-keys` and `cargo shuyi check-text` pass at the
      root and run in CI.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None beyond the checks' own tests.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
