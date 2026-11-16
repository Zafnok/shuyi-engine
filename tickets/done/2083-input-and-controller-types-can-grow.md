---
id: "2083"
title: "Before the first release, the input and controller types that open tickets grow move under ADR-0039, and PadKind::ALL becomes a slice"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-15
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

- [x] Every type in the table is `#[non_exhaustive]`; `PadKind::ALL` is a
      slice.
- [x] Every keymap file in the repository and the templates still loads
      unchanged (the existing keymap tests pass with no file edited).
- [x] `cargo xtask api` lists each change, and the `Breaking:` line names
      each type.
- [x] No open ticket still plans a `Breaking:` line for one of these
      types.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `PadState::new` builds what a literal built before.
- The existing input, keymap and Harness tests pass unchanged.

## Completion notes

What was done:

- Every type of the table is `#[non_exhaustive]`, each with a doc line
  naming its ADR-0039 kind:
  - kind 6: `RawInputEvent`, `Key`, `Button` (in their `keys!` and
    `buttons!` macros), `UiAction`, `PadKind`, `Device` and the input
    state's `Input`;
  - kind 5: `StickDef` (built as kind 2: `StickDef::new(press_percent,
    release_percent)`, a `const fn`, beside `Default`) and `KeymapDef`
    (built as kind 4: `from_source` or `Default`, fields assigned; it
    needs no constructor). Neither gained a field, so their serde forms
    are unchanged and every keymap file loads as before: no keymap file
    was edited, and the keymap tests pass unchanged;
  - kind 3: `PadState::new(buttons, left_stick, right_stick)`, a `const
    fn`; `from_standard` builds through it.
- `PadKind::ALL` is `&'static [PadKind]`. Its users were the tests of
  `shuyi-ui`'s pads and `shuyi-platform`'s script and session tests (the
  keymap's checks and the pad pictures iterate `Button::ALL`, not it).
- Call sites: the backends' `PadState`s (`shuyi-app`'s gilrs pads,
  `shuyi-playdate`'s input), `StickDef` literals in `shuyi-app`'s and
  `shuyi-ui`'s tests, and a `_` arm on the matches of `UiAction` in the
  toy grid, the ask widget, the name entry and hello-grid's `stage.rs`.
  A button added later is named on screen as its keymap file names it
  (the `_` arm of `names` in `pad.rs`), until it gets names of its own.
  The templates, the tutorial's steps and the book's snippets already
  had their `_` arms.
- The planned breaks of 2405 (`RawInputEvent`), 2316 and 2357
  (`UiAction`) and 2407 (`PadKind`) now say the new variants are
  additive. 1121, 2402, 2406, 3012, 3015, 3023 and 3024 planned none for
  these types (2406 already counted on this ticket).
- Docs: `docs/platform-backends.md` says a backend builds a `PadState`
  with `PadState::new` and matches the input types with a `_` arm; the
  book's *Actions* chapter says a `match` on `UiAction` ends with a `_`
  arm.
- Tests: `pad_states_constructor_builds_what_a_literal_built`
  (`shuyi-ui`) and `a_sticks_constructor_builds_what_a_literal_built`
  (`shuyi-assets`).

Deviations:

- Some existing tests changed, against "pass unchanged": tests outside
  `shuyi-assets` built `StickDef` by literal, and `shuyi-app`'s built
  `PadState` by literal; they call the constructors now. Their
  assertions are unchanged.
- `shuyi-app`'s `code_for` (`keys.rs`'s tests) matched `Key`
  exhaustively so that a new key without a key code failed the build. It
  can't across crates now: its `_` arm panics, and
  `every_key_is_mapped_from_a_key_code`, which walks `Key::ALL`, fails
  instead.
- `cargo xtask api` runs in CI only (it needs a nightly); its listing is
  read there.

The game, searched with `gh search code --repo Zafnok/visions-of-shuyi`
for `UiAction::`, `RawInputEvent::`, `PadKind::`, `PadState`, `StickDef`,
`KeymapDef`, `Device::`, `Input::Key`, `Input::Pad` and `Key::`, and the
files found read (`content/src/keymap.rs`, `playdate/src/pads.rs`,
`playdate/src/start.rs`, `ui/src/game.rs`, `ui/src/input.rs`,
`ui/src/tips.rs`, `ui/src/screen.rs`, `battle/mode.rs`,
`battle/split_keys.rs` and others). The search for `Button::` hit the
code-search rate limit; it is a set built by name, matched nowhere in the
files read.

- `StickDef`: 3 literals (`content/src/keymap.rs`'s test,
  `playdate/src/pads.rs`, `ui/src/game.rs`'s test).
- `PadState`: 2 literals (`playdate/src/pads.rs`, `ui/src/game.rs`'s
  test).
- `PadKind::ALL`: `ui/src/input.rs`'s test iterates it with
  `.into_iter()`, which still compiles over a slice.
- `UiAction`, `RawInputEvent`, `PadKind`, `Device`, `Input`, `Key`:
  built by name, no exhaustive match found. `KeymapDef`: read, never
  built by literal.

Game: nothing to adopt beyond the pin move: the game's next pin move
past this change makes the five edits above; no game ticket is needed
before it.

Book: the *Actions* chapter (`site/content/learn/book/actions.md`) says a
`match` on `UiAction` ends with a `_` arm.

Breaking: shuyi-assets: StickDef: #[non_exhaustive]; build it with StickDef::new or Default

```migration
since: 0.1.0
what: A StickDef is built with StickDef::new (or Default), not by a literal.
old: StickDef { press_percent, release_percent }
new: StickDef::new(press_percent, release_percent)
before: |
  let stick = StickDef { press_percent: 50, release_percent: 35 };
after: |
  let stick = StickDef::new(50, 35);
find: ["StickDef {"]
```

Breaking: shuyi-assets: KeymapDef: #[non_exhaustive]; build it with KeymapDef::from_source or Default, and assign its fields

```migration
since: 0.1.0
what: A KeymapDef is read from a keymap file, or made with Default and its fields assigned, not built by a literal.
old: KeymapDef { layouts, .. }
new: KeymapDef::default(), then assign the fields
before: |
  let def = KeymapDef { layouts, ..KeymapDef::default() };
after: |
  let mut def = KeymapDef::default();
  def.layouts = layouts;
find: ["KeymapDef {"]
```

Breaking: shuyi-assets: Key, Button, UiAction: #[non_exhaustive]; a match on one has a `_` arm

```migration
since: 0.1.0
what: A match on Key, Button or UiAction has a wildcard arm.
old: UiAction (exhaustive)
new: UiAction (#[non_exhaustive])
before: |
  match ui { UiAction::Up => up(), UiAction::Down => down(), UiAction::Left => left(), UiAction::Right => right(), UiAction::Confirm => confirm(), UiAction::Cancel => cancel(), UiAction::Debug => {} }
after: |
  match ui { UiAction::Up => up(), UiAction::Down => down(), UiAction::Left => left(), UiAction::Right => right(), UiAction::Confirm => confirm(), UiAction::Cancel => cancel(), _ => {} }
find: ["UiAction::Debug =>", "UiAction::Debug) =>"]
```

Breaking: shuyi-ui: PadState: #[non_exhaustive]; build it with PadState::new

```migration
since: 0.1.0
what: A PadState is built with PadState::new (or Default), not by a literal.
old: PadState { buttons, left_stick, right_stick }
new: PadState::new(buttons, left_stick, right_stick)
before: |
  let state = PadState { buttons, ..PadState::default() };
after: |
  let state = PadState::new(buttons, (0.0, 0.0), (0.0, 0.0));
find: ["PadState {"]
```

Breaking: shuyi-ui: PadKind::ALL: a slice, `&'static [PadKind]`, not an array `[PadKind; 6]`

```migration
since: 0.1.0
what: PadKind::ALL is a slice; iterate it by reference.
old: "PadKind::ALL: [PadKind; 6]"
new: "PadKind::ALL: &'static [PadKind]"
before: |
  const KINDS: [PadKind; 6] = PadKind::ALL;
  for kind in PadKind::ALL { use_kind(kind); }
after: |
  const KINDS: &[PadKind] = PadKind::ALL;
  for &kind in PadKind::ALL { use_kind(kind); }
find: ["[PadKind; 6]", "in PadKind::ALL"]
```

Breaking: shuyi-ui: RawInputEvent, PadKind, Device, Input: #[non_exhaustive]; a match on one has a `_` arm

```migration
since: 0.1.0
what: A match on RawInputEvent, PadKind, Device or the input state's Input has a wildcard arm.
old: Device (exhaustive)
new: Device (#[non_exhaustive])
before: |
  match device { Device::Keyboard => keys(), Device::Pad(kind) => pad(kind) }
after: |
  match device { Device::Keyboard => keys(), Device::Pad(kind) => pad(kind), _ => keys() }
find: ["Device::Pad(kind) =>"]
```
