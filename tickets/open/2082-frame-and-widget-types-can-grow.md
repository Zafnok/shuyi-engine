---
id: "2082"
title: "Before the first release, the frame's and the widgets' types that open tickets grow move under ADR-0039, and the ADR says every such type moves before a release"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2082 — The frame's and the widgets' types that open tickets grow move under ADR-0039 before the first release

## Context

Nick, 2026-11-14 (`docs/decisions.md`): "I want to ensure we're not gonna
have a thousand version bumps after mvp. We'll follow semver. So I want
all major feature plans to be surfaced to me now if they will require a
ver bump." An audit of every open ticket that day found about 120 that
would break a game's code as planned. Most of them only grow a public
type that is still exhaustive: a new field on a struct games build by
literal, or a new variant on an enum games match. Under
[ADR-0039](../../docs/adr/0039-public-types-that-can-grow-non-exhaustive-config-and-views-with-constructors-plain-values-left-open.md)
the type moves under its rule "in the PR that next adds to it". After
2005 that PR is a breaking release, one per type.

Before 2005 a break costs nothing: no version has been published, and
the game moves its pin anyway. So every type an open ticket grows moves
now, in five tickets by area. Each one is mechanical: the attribute, the
constructor the ADR's kind asks for, and the call sites in the engine,
the templates and the examples.
- 2082: the frame and the widgets (this ticket);
- 2083: input and controllers;
- 2084: sound, music and cinematics;
- 2085: the assets' and the platform's data;
- 2086: `shuyi-cli`'s contract.

The types this ticket moves, checked exhaustive on 2026-11-14, with the
open tickets that grow each:

| Type | Where | Kind (ADR-0039) | Grown by |
| ---- | ----- | --------------- | -------- |
| `Sprite` | `crates/ui/src/frame/glyph_buffer.rs:219` | 3 (it has `new` and `painted`) | 2334, 2335, 2327, 2222, 2703, 1120 |
| `Paint` | `glyph_buffer.rs:187` | 6 | 2337, 1120 |
| `Overlay` | `glyph_buffer.rs:168` | 3 | 2337 |
| `Layer` (the frame's) | `glyph_buffer.rs:149` | 6 | 1120 |
| `PanelLook` | `crates/ui/src/frame/draw.rs:96` | 2 | 2338 |
| `MenuView` | `crates/ui/src/widgets/menu/view.rs:11` (it has `new`) | 3 | 2355, 2405 |
| `AskView` | `crates/ui/src/widgets/ask/view.rs:9` | 3 | 2409 |
| `GridView` | `crates/ui/src/widgets/name_entry.rs:338` | 3 | 2409 |
| `GridCell` | `name_entry.rs:172` | 6 | 2346 |
| `SkipView` | `crates/ui/src/dialogue/text_box.rs:237` | 3 | 2347 |
| `GlyphStyle` | `crates/ui/src/grid/glyph/style.rs:26` | 2 | 3203 |
| `GlyphSkin` | `crates/ui/src/grid/glyph.rs:37` (it has `const fn new`) | 2 | 2740 |
| `Projection` | `crates/ui/src/grid/layout.rs:29` | 6 | 1126, 1127, 2740 |
| `Grid` (the view's) | `layout.rs:49` | 3 | 2740, 2769, 1126, 2405 |
| `FitRule` | `crates/ui/src/input/fit.rs:80` | 6 | 2367 |
| `ScreenActions` | `fit.rs:41` | 3 | 2367 |

`Item` (`glyph_buffer.rs:334`) is already `#[non_exhaustive]`, so 2336's
new `Item::Shape` breaks nothing. Its planned `Breaking:` line is out of
date, and this ticket's PR corrects it.

## Nick input

None.

## Scope

**In:**
- Each type in the table under its ADR-0039 kind:
  - kind 2: `#[non_exhaustive]`, a `new` with what every caller sets
    (`const fn` where it can be), `with_<field>` where callers build it
    in a `const`;
  - kind 3: `#[non_exhaustive]`, a `new` for what it can't be without;
  - kind 6: `#[non_exhaustive]` on the enum, and on each struct-like
    variant that may gain fields.
- Every engine, template, example and tutorial-step call site moved
  onto the constructors.
- ADR-0039's **When** paragraph gains a sentence: before a crate's
  first release, every exhaustive type an open ticket plans to grow
  moves under the ADR at once (2082–2086), not in the PR that next adds
  to it.
- The listed tickets' text: a planned `Breaking:` line for one of these
  types becomes additive, and 2336's line is corrected.
- A `Breaking:` line in the completion notes (ADR-0038), naming each
  type. Before it, search the game for literals of each type
  (`gh search code --repo Zafnok/visions-of-shuyi '<Type> {'`) and list
  what the game's pin move has to change.

**Out (do not do):**
- Any new field or variant the listed tickets add.
- Plain values (kind 1: `Rect`, `Pos`, `Rgb`, `Cell`): they stay
  exhaustive.
- The widget painters' signatures (`menu::glyph::paint`,
  `ask::glyph::paint`, the name entry's painters). Each ticket that
  changes one adds a new function beside the old.
- Input, sound, assets and contract types (2083–2086).

**Estimated size:** about 900 lines (code 500, tests 250, docs 150).

## Implementation steps

1. For each type, add the attribute and its constructor in its module,
   with a doc line naming the kind.
2. `cargo build --workspace --all-targets` and fix each struct literal
   or exhaustive match it reports. Do the same in
   `templates/shuyi-default` (the templates' job, the `run-gates`
   skill) and the tutorial steps.
3. ADR-0039's **When** sentence.
4. Edit the listed tickets' planned `Breaking:` lines (grep
   `tickets/open` for each type's name).
5. The `document` skill: the book's pages that show a literal of these
   types.

## Acceptance criteria

- [ ] Every type in the table is `#[non_exhaustive]`, and a test outside
      its crate builds it through its constructor (a doc test does).
- [ ] `cargo xtask api` lists each change, and the completion notes'
      `Breaking:` line names each type.
- [ ] The templates' job, the tutorial steps and the examples pass.
- [ ] ADR-0039's **When** says what moves before a first release.
- [ ] No open ticket still plans a `Breaking:` line for one of these
      types.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: one test per kind-2 type that its `new` and `with_<field>`
  build the value a literal built before.
- The existing snapshots stay unchanged: no frame or skin output moves.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
