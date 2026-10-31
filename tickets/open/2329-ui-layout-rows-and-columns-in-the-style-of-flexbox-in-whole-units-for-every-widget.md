---
id: "2329"
title: "UI layout: rows and columns in the style of flexbox, in whole units, placing every widget"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2329 — UI layout in the style of flexbox

## Context

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md)): every feature
on Bevy's homepage. Bevy UI's layout is "a flexbox-style model"
(through `taffy`). Shuyi's widgets (`crates/ui/src/widgets/`: menus,
the text box, the question widget, meters and titled boxes from 2311)
are placed by hand, at positions each screen computes. A screen with a
title, a menu, a panel and a footer does the same arithmetic every
time, and redoes it for each console size (2323, parked).

This ticket adds a layout pass: a tree of boxes with rules, solved into
rectangles that widgets draw in.

`taffy` (MIT) was considered. It is complete CSS flexbox and grid, in
`f32`. The engine's frames are whole cells (glyph skins) or whole
pixels (sprite skins), the views are tested by equality, and `shuyi-ui`
is `no_std` (3005). A small integer subset of flexbox of our own is
simpler to test and to keep deterministic. If a game needs CSS grid,
a follow-up can wrap `taffy` behind a feature.

## Nick input

None.

## Scope

**In:**
- **`shuyi_ui::layout`**, `no_std`, in `i32` units of the frame (cells
  or pixels):
  - `Node { style: Style, children: Vec<Node>, key: Option<NodeKey> }`;
  - `Style`: `direction` (`Row`, `Column`), `width` and `height`
    (`Fixed(n)`, `Fill`, `Fit`, `Percent(p)`), `grow` (weight),
    `min` and `max`, `padding`, `margin`, `gap`, `align` (`Start`,
    `Center`, `End`, `Stretch`), `justify` (`Start`, `Center`, `End`,
    `SpaceBetween`), `wrap` (on or off);
  - `layout(&Node, available: Size) -> Layout`, a map from each
    `NodeKey` to its `Rect`;
  - leftover units from a division go to the first children, in order,
    so the result is exact and the same everywhere;
  - a node too big for its place is cut at its parent's edge, and
    `Layout::overflows()` lists it (for a check like 2209's).
- **Widgets measure themselves:** a `Measure` trait (the minimum size a
  widget needs: a menu's longest label and its rows, a text box's
  lines) used by `Fit`.
- **Either order with 2334:** if 2334 is done, its `TextLabel`
  implements `Measure` in pixels, with `shuyi_ui::frame::text::measure`;
  if not, 2334 does it.
- **Screens use it:** the options screen (`crates/ui/src/screens/`) and
  one other engine screen move to a layout, with their `frame-png`
  snapshots unchanged (or changed on purpose, said in the PR).
- **Either order with 2353:** if 2353 is done, the two moved screens
  also pass its walk in the pseudo-language (no cut text), so a layout
  leaves room for translated text. If not, 2353's walk covers them when
  it lands.
- **Either order with 2762** (the action HUD, parked): if 2762 is
  done, give its `PoolGauge` and `SkillBar` a `Measure`, so a game
  lays the HUD out in rows and columns. If not, nothing to do (2762
  does it, its clause says so).
- **From 2802 (done):** the book's chapter 4, *The frame and skins*
  (`site/content/learn/book/frame-and-skins.md`), gets a section on
  laying widgets out with `shuyi_ui::layout`, its code in
  `site/snippets/`.

**Out (do not do):**
- UI declared in a data file (2330);
- CSS grid, absolute positioning, scrolling containers (the list cursor,
  2307, already scrolls a menu);
- moving the focus between laid-out widgets with the d-pad (2355, which
  waits for this ticket);
- hit areas for the pointer: painters record them in the frame as they
  paint inside this layout's rectangles (2405), and `Layout` is not a
  second list of them;
- a reading direction and mirrored rows for right-to-left languages
  (2908, parked research). Here `Start` and `End` mean left and right on
  a horizontal axis (`justify` in a `Row`, `align` in a `Column`), and
  `Style`'s docs say so;
- moving every screen at once.

## Implementation steps

1. Write the layout types and the solver, rows first, then columns,
   then wrap.
2. Add `Measure` for the widgets, and for 2334's `TextLabel` if it is
   done.
3. Move the two screens over, keeping their snapshots (and 2353's walk,
   if it is done).
4. Run the gates.

## Acceptance criteria

- [ ] Children's rectangles stay inside their parent minus padding, and
      siblings don't overlap without wrap (property test over random
      trees).
- [ ] `Fill` children share the leftover exactly: the sum of sizes plus
      gaps equals the parent's inner size (property test).
- [ ] The two moved screens' snapshots are unchanged, or the PR says why.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each `Style` field on a small tree; overflow listing.
- Property: the two above, and that layout is the same twice.
- Snapshot: the moved screens.

## Completion notes

*(Filled in by the session that completes the ticket.)*
