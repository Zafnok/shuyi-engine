---
id: "3234"
title: Toggle, slider, choice, scroll panel, tab bar, tooltip and image node are standalone widgets a screen or a .ui.ron places
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3230", "2330"]
nick_input: none
completed:
---

# 3234 — Toggle, slider, choice, scroll panel, tab bar, tooltip and image node are standalone widgets

## Context

A blind-spot audit of the engine (2026-11-23) found that the in-game UI
has menus, but not the basic controls other engines ship with. Godot,
Unity's UI Toolkit, Defold and egui all have a checkbox, a slider, a
drop-down or stepper, tabs, a scroll area and a tooltip.

What exists today, checked in the code:

- `crates/ui/src/widgets/` holds four widgets: `ask`, `list`
  (`ListCursor`), `menu` and `name_entry` (`widgets.rs`).
- A toggle, a choice and a volume slider exist only as kinds of an
  options row: `RowValue::Toggle`, `RowValue::Choice` and
  `RowValue::Volume` in `crates/ui/src/screens/options.rs` (from line
  103), with `fn` getters and setters over `Ctx`. Their stepping is
  private to that screen (`stepped`, `stepped_index`, `stepped_volume`),
  and so is their look (`volume_text` and `paint_row` in
  `screens/options/glyph.rs`). A screen other than Options, or a
  `.ui.ron`, can't place one.
- 2330's `.ui.ron` widget list is `Text`, `Menu`, `Meter`, `TitledBox`,
  `Question`, `TextBox` and `Slot`. A game's own control can only be a
  `Slot` closure.
- No ticket plans a scroll panel for any content. 2329's Out says
  "scrolling containers" and leaves menus to `ListCursor` (2307, done).
  2409's Out says scroll bars have no ticket. 3201's credits roll their
  own scroll.
- No ticket plans a tab bar, a tooltip or an image node. 2355's Out
  leaves shoulder buttons between tabs as "a follow-up if a game asks",
  because no `UiAction` exists for them (`crates/assets/src/keymap/action_set.rs`).
- `GlyphBuffer::blit` (`crates/ui/src/frame/glyph_buffer.rs`) copies a
  whole buffer, clipped to the target. It cannot copy a window of the
  source, which a scroll panel needs.

Games that would write these by hand: a farming game's shop with tabs
for seeds and tools, a settings sub-page of a platformer with a toggle
for screen shake, a puzzle game's help text longer than the screen, a
card game's tooltip on a card's keyword.

This ticket builds them on the widget contract of 3230, so layout
(2329), focus (2355), hits (2405), the look and the accessibility
description (2322, parked) see each one. It adds each one as a
`.ui.ron` node (2330).

## Nick input

None.

## Scope

**In:**
- **Lifted out of the options screen**, in `crates/ui/src/widgets/`,
  each with a plain-data view and a glyph painter as `menu` and `ask`
  have:
  - `Toggle`: on or off. Left turns it off, Right on, Confirm flips it
    (today's `RowValue::Toggle` rule).
  - `Choice`: one of several values, given as text keys. Left and Right
    step and stop at the ends, Confirm goes round (today's
    `RowValue::Choice` rule, and `stepped`).
  - `Slider`: an `i32` value between a `min` and a `max`, moved by a
    `step` (Left and Right). The volume is a `Slider` from 0 to
    `MAX_VOLUME` by `VOLUME_STEP`. Its blocks are today's volume
    painter.
  - `OptionsScreen`'s rows build these widgets and draw them. Every
    snapshot of the options screen stays unchanged. `RowValue` keeps
    its constructors and fields, so no game changes (no break). The
    volume row's number box (`NumberBox`) stays in the options screen.
- **New widgets:**
  - `ScrollPanel`: one child laid out at its own height (2329's `Fit`)
    and shown through the panel's rectangle. The child is painted into a
    buffer of its own and copied in through a new
    `GlyphBuffer::blit_window(other, src: Rect, dest_x, dest_y)`, which
    copies only `src`. Its items and hits are offset and clipped the same
    way. When the focus moves to a child out of view, the panel scrolls
    it in (the `Scroll::KeepInView` rule of `ListCursor`). The pointer's
    wheel (`PointerInput::wheel`) scrolls it by one row per notch. It
    shows `▲` and `▼` marks when more is above or below.
  - `TabBar`: a row of tab names (text keys) and the page of the chosen
    tab. Left and Right on the bar change the tab. Two new `UiAction`s,
    `PrevPage` and `NextPage` (shoulder buttons), change it from
    anywhere on the screen. As ADR-0036 says, a new `UiAction` stays
    out of `UiAction::ALL`, so no keymap file must name it. A click on a
    tab chooses it (a `Hit` per tab).
  - `Tooltip`: a text key on any widget. It shows beside the widget
    while the widget has the focus or the pointer is over it, after a
    delay counted in frames (no clock). It stays on the screen: it goes
    to the other side when it would leave it. The same rule works with
    a pad and with a mouse.
  - `Image`: a picture from the game, named by text in the file. It is
    resolved through a new `GameData::ui_image(&self, name: &str) ->
    Option<UiImage>` (an `ImageId` and its source rectangle), with a
    default of `None`. An unknown name is an error from `shuyi check`.
    It measures as its picture's size and is drawn as an `Item::Sprite`.
- **`.ui.ron` nodes** in 2330's format: `Toggle`, `Choice`, `Slider`,
  `ScrollPanel(child)`, `TabBar(tabs: [(label, page)])`, `Image(name)`,
  and an optional `tip: "text.key"` on any widget node. A value comes
  from the game's code through `DocScreen` by the node's `id` (a
  `set_value` beside 2330's `set_arg`). A change comes back in the
  `ScreenResult` as `(id, value)`. Every label is a text key, and no
  field names a key (AGENTS.md rule 8).
- Each widget implements all of 3230's contract (measure, input, focus,
  paint, hits, describe). Its description gives its role: a toggle a
  check box, a slider a slider with its value.
- Every new public type follows ADR-0039: `#[non_exhaustive]` with a
  constructor, since each may grow fields. `UiAction` is already
  `#[non_exhaustive]`, so its two new variants are additive.
- **Either order with 3233:** if 3233 is done, carry its drag on the
  options screen's volume row over to the `Slider` this ticket lifts,
  and build the scroll panel's bar drag (if it draws a bar) on
  `PointerInput::drag`; if not, nothing to do (3233 puts the drag on the
  `Slider`, and on the scroll panel's bar if it draws one).
- **Either order with 3235:** if 3235 is done, the toggle, slider and
  choice nodes take a `value` binding from `UiValues` (`Flag` for a
  toggle, `Number` for a slider and a choice, declared in the doc's
  `binds`), and the image node takes `shown`; if not, nothing to do
  (3235 binds them). Either way `DocScreen::set_value` stays.
- **Either order with 2371:** if 2371 is done, each new widget's painter
  takes a `WidgetLook` and has a one-bit snapshot; if not, 2371 gives
  them their one-bit look with the other widgets.
- **Either order with 2409:** if 2409 is done, its pointer handling of
  the options rows moves into `Toggle`, `Choice` and `Slider`, with its
  tests unchanged; if not, 2409 puts the pointer on these widgets
  instead of on the rows.

**Out (do not do):**
- The widget contract itself: 3230.
- Dragging and drop targets: 3233 (see the clause above).
- Bindings to game data, visibility rules and repeated rows in a
  `.ui.ron`, and the theme file: 3235.
- Text fields and the shared editing core: 3232.
- The options screen's typed number box (`NumberBox`): it stays as it
  is.
- 2314's enum rows: they build `RowValue::choice`, which stays and now
  draws a `Choice`. Nothing to settle between the two.
- 3201's credits: they roll with time, not with a scroll panel.
- The inventory's tabs of 2759 (parked): it may use `TabBar` once it is
  unparked.
- A drop-down list that opens over the screen: a `Choice` steps in place.
  Write a ticket when a game asks.
- A scroll bar that the pointer drags, before 3233 is done: 3233 (the
  clause above).

**Estimated size:** about 2,200 lines (code 1,100, tests 800, docs 300).

**Split:** past 1,000 lines, split off steps 5–7 (the scroll panel, the
tab bar and the tooltip, with their `.ui.ron` nodes and the two
`UiAction`s) into a new ticket, and keep the lifted toggle, choice and
slider and the image node here.

## Implementation steps

1. Read 3230's contract as it landed, 2329's `layout` module, 2330's
   `UiDoc` and `DocScreen`, and `widgets/menu.rs` with its `view.rs`
   and `glyph.rs`, which are the pattern for a widget.
2. Write `widgets/toggle.rs`, `widgets/choice.rs` and
   `widgets/slider.rs`. Each has its state, a view, a glyph painter and
   the contract. Move `stepped`, `stepped_index` and `stepped_volume`
   from `screens/options.rs`, and the volume painter (`volume_text` and
   the slider's part of `paint_row`) from `screens/options/glyph.rs`.
3. Make `OptionsScreen` rows build and draw these widgets. Run the
   options snapshots and tests (`screens/options/tests.rs`): none may
   change.
4. Write `widgets/image.rs`, `UiImage` and `GameData::ui_image`, and the
   `Image` node with its `shuyi check` error.
5. Write `GlyphBuffer::blit_window`, with its hits offset and clipped,
   then `widgets/scroll_panel.rs`: lay out the child at `Fit`, paint it
   into its own buffer, copy the window in, keep the focused child in
   view, scroll with the wheel and show the marks.
6. Add `UiAction::PrevPage` and `UiAction::NextPage` (the
   `input-actions` skill; outside `ALL`), then write
   `widgets/tab_bar.rs`.
7. Write `widgets/tooltip.rs`: the delay in frames, the placement and the
   flip at the screen's edge, and the `tip` field on any node.
8. Add the `.ui.ron` nodes to 2330's format, `DocScreen::set_value`, and
   the `(id, value)` in its `ScreenResult`. Add one example screen in
   `examples/` that uses every new widget from a `.ui.ron`.
9. Re-export the widgets from `widgets.rs`, and update the docs that
   2330 and the widget pages name (the `document` skill).

## Acceptance criteria

- [ ] `Toggle`, `Choice`, `Slider`, `ScrollPanel`, `TabBar`, `Tooltip`
      and `Image` are public in `shuyi_ui::widgets`, and each implements
      3230's contract.
- [ ] Every snapshot of the options screen is unchanged, and
      `RowValue`'s constructors keep their signatures.
- [ ] A `Slider` from 0 to 100 by 5 at 100 stays at 100 on Right, and at
      0 stays at 0 on Left. A `Slider` from 0 to 10 by 3 at 9 goes to 10
      on Right (the max, not 12).
- [ ] A `Choice` at its last value stays there on Right, and goes to the
      first on Confirm.
- [ ] A `ScrollPanel` 5 rows high over a child 12 rows high shows rows
      0–4. When the focus moves to a widget on row 8, it shows rows 4–8.
      Two notches of the wheel down from the top show rows 2–6, and the
      wheel never scrolls past row 7 as the first row shown.
- [ ] `blit_window` copies only its source rectangle. A sprite or a hit
      that is half inside it is cut at the window's edge, and one fully
      outside is dropped.
- [ ] A `TabBar` of three tabs goes from the third tab to the first on
      `NextPage`, and from the first to the third on `PrevPage`. A click
      on the second tab chooses it.
- [ ] A game whose keymap maps no `PrevPage` or `NextPage` still loads
      its keymap file unchanged.
- [ ] A `Tooltip` shows after its delay in frames, and not before. One on
      a widget at the screen's right edge is drawn to its left, fully on
      the screen.
- [ ] An `Image` node with a name `GameData::ui_image` doesn't know fails
      `shuyi check`, naming the file and the node path.
- [ ] The example's `.ui.ron` builds a screen with every new widget, and
      `shuyi check` passes on it.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each widget's actions (Left, Right, Confirm, the page actions);
  `blit_window` at each edge, with sprites, painted pictures and hits;
  the tooltip's placement at all four edges; each new `.ui.ron` node's
  parse and its errors.
- Property: for any child height, panel height, focus and number of
  notches, the focused child is in view after the move and the first row
  shown is between 0 and child height minus panel height. For any
  slider's `min`, `max`, `step` and actions, the value stays between
  `min` and `max`.
- Snapshot / integration: the options screen's snapshots unchanged; a
  snapshot of each new widget's glyph look; a harness test that drives
  the example screen by actions only (tabs, a toggle, the slider, a
  scroll) and reads its `ScreenResult`; a harness test of the same screen
  by the pointer.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
