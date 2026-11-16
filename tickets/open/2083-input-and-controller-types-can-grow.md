---
id: "2083"
title: "Before the first release, the input and controller types that open tickets grow move under ADR-0039, and PadKind::ALL becomes a slice"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: in-progress
blocked_by: []
nick_input: none
completed:
---

# 2083 — The input and controller types that open tickets grow move under ADR-0039 before the first release

## Context

The second of five tickets that move, before 2005, every exhaustive
public type an open ticket plans to grow (2082 has the reasons and
Nick's words of 2026-11-14). Nick asked that the controller backend be
worked out before the release. The audit found that replacing gilrs
with our own code (2903, 2402, 2403) stays internal: `PadInput`'s fields
are private (`crates/app/src/pads.rs:17`). But every input type games
see is exhaustive, and nine open tickets grow them.

| Type | Where | Kind (ADR-0039) | Grown by |
| ---- | ----- | --------------- | -------- |
| `RawInputEvent` | `crates/ui/src/game.rs:43` | 6 | 1121, 2405, 2406, 3024 |
| `Key` | `crates/assets/src/keymap.rs:40` | 6 | 1121 |
| `Button` | `keymap.rs:270` | 6 | 2407 |
| `StickDef` | `keymap.rs:527` | 5 | 1121 |
| `KeymapDef` | `keymap.rs:698` | 5 | 1121, 2406, 2407 |
| `UiAction` | `crates/assets/src/keymap/action_set.rs:17` | 6 | 2316, 2357, 3023 |
| `PadKind` | `crates/ui/src/input/pad.rs:49`, with `ALL: [PadKind; 6]` at `:71` | 6 | 3012, 3015, 2407 |
| `PadState` | `pad.rs:252` (public `f32` sticks) | 3 | 1121, 2402 |
| `Device` | `crates/ui/src/input/state.rs:43` | 6 | 2405, 2406 |
| `Input` (the input state's) | `state.rs:54` | 6 | 2405, 2406 |

`PadKind::ALL`'s length is part of its type, so a new kind changes the
type even with the attribute. It becomes a `&'static [PadKind]`.
`ButtonSet` (`pad.rs:195`, a `u64` of bits) is a plain value and stays
as it is. Which pad an event came from is a change of the events'
payload, which no attribute softens: 2412 (done) did it.

## Nick input

None.

## Scope

**In:**
- Each type in the table under its ADR-0039 kind. Kind 5 types (read
  from files) take `#[serde(default)]` on any field they add later, as
  the ADR says, so keymap files keep loading.
- `PadKind::ALL` as `pub const ALL: &'static [PadKind]`, and its callers.
- Every engine, template, example and tutorial-step call site; a game's
  `match` on `UiAction` gains its `_` arm (hello-grid's does).
- The listed tickets' planned `Breaking:` lines for these types become
  additive.
- A `Breaking:` line in the completion notes, after a search of the
  game for matches and literals of each type
  (`gh search code --repo Zafnok/visions-of-shuyi 'UiAction::'`, and so
  on).

**Out (do not do):**
- The pad id on pad events, `PadRemoved`, and seats (2412, 2406).
- Pointer, touch or analog events (2405, 1121).
- `ButtonSet` and the other plain values.

**Estimated size:** about 600 lines (code 300, tests 200, docs 100).

## Implementation steps

1. The attributes and constructors, module by module; `PadState::new`
   with its sticks and buttons.
2. `PadKind::ALL` as a slice; fix its users (the keymap's checks, the
   pad pictures).
3. Build the workspace, the templates and the tutorial steps, and fix
   each literal and exhaustive match.
4. Edit the listed tickets' planned `Breaking:` lines.

## Acceptance criteria

- [ ] Every type in the table is `#[non_exhaustive]`; `PadKind::ALL` is a
      slice.
- [ ] Every keymap file in the repository and the templates still loads
      unchanged (the existing keymap tests pass with no file edited).
- [ ] `cargo xtask api` lists each change, and the `Breaking:` line names
      each type.
- [ ] No open ticket still plans a `Breaking:` line for one of these
      types.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `PadState::new` builds what a literal built before.
- The existing input, keymap and Harness tests pass unchanged.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
