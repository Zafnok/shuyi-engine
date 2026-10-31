---
id: "2371"
title: "A one-bit look for the engine's screens and widgets: Options, Key bindings, Profiles, the layout picker, menus, questions and name entry painted for a one-bit screen"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3053"]
nick_input: none
completed:
---

# 2371 — A one-bit look for the engine's screens and widgets

## Context

A request from the game.

1. **The game ticket:** game#1184, which deletes the game's one-bit skins
   of this engine's screens once this lands
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1184-the-engines-one-bit-look-replaces-the-games-one-bit-skins-of-engine-screens.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** a game that runs on a one-bit screen gets
   the engine's own Options, Key bindings, Profiles and layout picker
   screens, and its menu, question and name entry widgets, drawn for
   that screen out of the box, instead of writing a painter for each.
3. **The friction numbers:** from the game's first sweep (game
   `docs/engine/friction.md`, the row "the Playdate front end … 17×",
   game#1153), whose outcome 4 said "choosing a screen's skin by target
   … revisit with engine 3001 and 3010"; 3001 closed on 2026-10-20 and
   the revisit never happened, which the game's retro of its last 50 PRs
   found (2026-10-22). Today the game paints this engine's screens in one
   bit with **617 production lines that name no game concept** (game
   ADR-0091: "the engine's screens painted by the game's"): a shared
   window, menu and question painter (251), Key bindings (155), Profiles
   (80), Options (74), name entry (57). It also copies this engine's menu
   skin for its own menus, because `widgets/menu/glyph.rs` `paint`
   hard-codes `BoxStyle::Single` and its `EngineColor` roles.
4. **The generic shape:** each engine screen and widget gets a second
   painter, `onebit`, beside `glyph`, for a one-bit target's ground and
   ink (white ground, black text, a double frame, the focused row as an
   inverted bar, what can't be chosen in a dotted role, a long list
   scrolled with `Scroll::Centred` and its `▲`/`▼` marks); a game picks
   it with the screens' existing `painted_by`, and the target profile
   (3001) can name it as the default for a one-bit target. The widget
   painters take a **look** (box style, the roles of the frame, the
   focus bar, disabled text and the note rule) so that a game's own
   menus can use the same painter in either look.
5. **Examples from games that aren't tactics:** a puzzle game on the
   Playdate needs its Options and Key bindings screens and a "Quit?"
   question; a farming game's one-bit handheld build needs Profiles and
   name entry. Each would write the same painters.

## Nick input

None.

## Scope

**In:**
- `crates/ui/src/widgets/menu/glyph.rs`, `widgets/ask/glyph.rs`,
  `widgets/name_entry/glyph.rs`: the paint functions take a `WidgetLook`
  (box style and roles), with `WidgetLook::GLYPH` (today's look; every
  existing snapshot unchanged) and `WidgetLook::ONE_BIT`.
- An `onebit` painter for `screens/options`, `screens/key_bindings`,
  `screens/profiles` and `screens/layout_picker`, sized for the one-bit
  frame (`frame/onebit.rs`, 400 × 240 pixels; the cells that fit at the
  one-bit font the frame draws with), using the widgets in `ONE_BIT`.
- The target profile (3001) names a default painter set (`glyph` or
  `onebit`) that the screens use when the game gives none. It reaches
  a running game through 3053's facts: `PlatformFacts::from_profile`
  copies it, and the screens read it from `ctx.platform`.
- Snapshots of every one-bit painter, as cells and as a one-bit frame
  (`frame/onebit.rs` `to_one_bit`).
- `docs/`: where screens and widgets are documented, a line on the one-bit
  look and how to pick it.

**Out (do not do):**
- The game's screens (its battle, title, lead select: the game's).
- A one-bit look for anything but these screens and widgets.
- The Playdate backend (3010; this ticket needs no device, only the
  one-bit frame).

**From 3010 (done):** Hello, Grid (`examples/hello-grid`) runs on the
Playdate (`src/playdate.rs`, its console `PLAYDATE_CONSOLE` of 50 × 15,
its `Hello::one_bit_ctx`). Its Options screen there is the engine's
glyph options, painted through the pattern map, and cut off where they
don't fit 15 rows. This ticket moves it onto these painters, and
`docs/targets/playdate.md`'s line on the engine's screens goes with it.

**One look type, not two.** `PanelLook` (`frame/draw.rs`) already holds
a panel's `BoxStyle` and its border, title, text and background
colours. `WidgetLook` builds on it: its box style and frame roles are
what make the painter's `PanelLook`, and it adds only the focus bar,
disabled text and the note rule.

**Either order with 2338:** if 2338 is done, `PanelLook` has a
`frame: Option<NineSlice>`, and the question and name box painters take
it as an extra argument: move that frame into `WidgetLook` (a `framed`
method, passed on to the `PanelLook` it makes) so each painter takes
one look, and the engine's callers set it from `ctx.panel_frame(style)`
as 2338's do; the menu's note rule still leaves out its joins when the
look has a frame. If not, nothing to do (2338 puts its frame in
`WidgetLook`, its clause says so).

**Either order with 2355:** if 2355 is done, `MenuView` has `active`:
`WidgetLook::ONE_BIT` paints an inactive menu's focused item as an
ordinary row, with no focus bar and no note, as `GLYPH` does, and
`size` doesn't change with it, with a one-bit snapshot. If not, nothing
to do (2355 paints the inactive menu in every look, its clause says
so).

**The game's look as the reference:** the game's one-bit skins
(`crates/ui/src/screens/playdate.rs`, `options/playdate.rs`,
`key_bindings/playdate.rs`, `profile/playdate.rs`,
`widgets/name_entry/playdate.rs` in `Zafnok/visions-of-shuyi`) are a
working one-bit look for exactly these screens; their snapshots show
it. Read them for the layout, take nothing game-specific (the game's
palette names stay the game's; here the roles are `EngineColor`'s).

**Size:** if the change passes about 600 lines, split it: this ticket
keeps the widgets' `WidgetLook` and the Options and question painters;
a follow-up (written in the same PR) takes Key bindings, Profiles, name
entry and the layout picker.

## Implementation steps

1. **Read** `widgets/menu/glyph.rs`, `widgets/ask/glyph.rs`,
   `widgets/name_entry/glyph.rs`, `widgets/list.rs` (`Scroll`), the four
   screens' `glyph.rs` and `view.rs`, `frame/draw.rs` (`panel`,
   `PanelLook`, `BoxStyle`), `frame/onebit.rs` and `frame/onebit/roles.rs`,
   3001's target profiles, and the game's one-bit skins named above.
2. **`WidgetLook`** and the widget paints taking it; `GLYPH` keeps every
   snapshot.
3. **The one-bit painters**, one screen at a time, each with snapshots.
4. **The profile's default painter set**.
5. **Docs**.

## Acceptance criteria

- [ ] Every existing glyph snapshot is unchanged.
- [ ] Each of the four screens and three widgets has a one-bit snapshot
      (cells and one-bit frame) that fits the one-bit frame.
- [ ] A menu longer than its window scrolls with `Scroll::Centred` and
      shows `▲`/`▼` in the one-bit look (a test).
- [ ] A game whose target profile names `onebit` gets the one-bit Options
      screen without calling `painted_by` (a test with a fake game).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Snapshot: the one-bit painters.
- Unit: `WidgetLook` roles; the default painter set from a profile.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
