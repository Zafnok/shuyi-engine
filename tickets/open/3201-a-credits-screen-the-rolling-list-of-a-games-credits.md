---
id: "3201"
title: "A credits screen: the rolling list of a game's credits, grouped, with title, author, license and link"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3201 — A credits screen

## Context

A request from the game.

1. **The game ticket:** game#1211, which deletes the game's own credits
   screen once this lands
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1211-use-the-engines-credits-screen.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** show every third-party work a game
   uses, grouped by kind, each with its title, author, license and
   link (what CC BY asks for), rolling up by itself, with the player
   able to pause it or scroll it by hand.
3. **The friction numbers:** no friction review: found by the
   game/engine boundary audit of 2026-11-03. The game's
   `CreditsScreen` (`crates/ui/src/screens/credits.rs`, `view.rs` and
   `glyph.rs`, about 270 production lines and 640 of tests) names no
   game concept. It reads this engine's `shuyi_assets::credits::Credits`
   by `CreditGroup::ALL`; its only game inputs are a music cue and wrap
   widths hard-coded for a 100-cell console. 2223 left this screen Out
   until a game asks, and `skills/shuyi-ship/SKILL.md` tells every game
   to write its own.
4. **The generic shape:** `shuyi_ui::screens::credits::CreditsScreen<G>`,
   given a `&Credits` (what `credits::merge` makes), beside `options`,
   `profiles` and `key_bindings`. Its view (`CreditsView`) is plain data
   and `credits::glyph::paint` paints it. The list is wrapped to the
   console the frame is drawn on (`ctx.console`), so it works on a
   50-cell handheld as well as a 100-cell PC.
5. **An example from a game that isn't tactics:** a farming game with
   CC-BY music and a CC0 tile pack opens the same rolling list from its
   main menu, and a platformer shows it after its last level.

## Nick input

None.

## Scope

**In:**
- `crates/ui/src/screens/credits.rs` with `view.rs` and `glyph.rs`,
  ported from the game's screen (behavior unchanged):
  - rows: each group with entries under its heading, a blank row, then
    each entry as `credits.entry` (`{title}`, `{author}`) and
    `license · source`, wrapped with `shuyi_assets::text::word_wrap`;
  - rolling: a row every `row_secs` (default 0.5), a rest of
    `rest_secs` (default 2.0) at the top and the bottom, at most once
    round the list in one frame; a list that fits stays put;
  - `UiAction::Up`/`Down` stop rolling and scroll a row (`MenuSound::Move`,
    silent at the ends), `Confirm` toggles rolling (`Select`), `Cancel`
    pops (`Cancel`). `actions()` declares those four through
    `ui_actions`.
- Builders: `CreditsScreen::new(ctx, &Credits)`, `.with_music(cue)`
  (asked for once on the first update; none by default),
  `.rolling(row_secs, rest_secs)`, and `.painted_by(paint, layout)`
  like `ProfilesScreen::painted_by`.
- `CreditsLayout` (rows shown, wrap width per `CreditRowKind`) and
  `glyph::layout(ConsoleSpec)`: a panel 2 cells in from each side and
  3 rows shorter than the console, so 100 × 32 gives 25 rows and
  widths 91/89/87, exactly the game's today.
- Text keys as constants, the game's names: `credits.title`,
  `credits.group.{music,sound_effects,art,fonts,software}`,
  `credits.entry`, `credits.help.rolling`, `credits.help.stopped`.
- `CreditsView` and `CreditRowView` are `#[non_exhaustive]` with
  constructors (ADR-0039); `CreditRowKind` is a plain enum. Additive:
  no break to the public API.
- Docs: `screens.rs`'s module comment, `skills/shuyi-screen/SKILL.md`'s
  list of ready-made screens, `skills/shuyi-ship/SKILL.md` step 5 (use
  this screen instead of writing one), and the book's
  `site/content/learn/book/games-and-screens.md`, *The engine's own
  screens and widgets*.

**Either order with 2351:** if 2351 is done, the credits keys' English
goes in its engine English file; if not, put it in
`crates/ui/src/toy/en.ron` (2351 moves it with the rest).

**Either order with 2371:** if 2371 is done, add `credits::onebit` in
`WidgetLook::ONE_BIT` with its layout, picked like the other screens'
one-bit painters; if not, nothing to do (2371 adds it with the others).

**Out (do not do):**
- License lists, the credits pass of `check` and the credits in
  `NOTICES.txt` (2223): this screen only shows `Credits`, which 2223
  doesn't change.
- A screen of `NOTICES.txt` or of license texts.
- Hello, Grid opening it: it has no credits file until 2223.

## Implementation steps

1. Port the three files into `crates/ui/src/screens/credits*`, generic
   over `G: GameData`, with `EngineColor` roles (`Highlight` title,
   `Focus` headings, `Text` entries, `TextDim` details and marks).
   About 230 production lines; the game's file is the reference.
2. Take widths and rows from `CreditsLayout` instead of constants; the
   screen builds its rows in `new` from `(self.layout)(ctx.console)`.
3. Add the builders and the text-key constants, then the English
   (either-order clause above).
4. Port the game's unit tests (`credits/tests.rs`,
   `credits/glyph/tests.rs`) on a toy `Ctx` with made-up credits, and
   add a glyph snapshot on 100 × 32 and one on 50 × 15.
5. Update the docs and skills listed; run `check-skills`.
6. The PR says the game's PR that moves the pin (game#1211) follows it.

## Acceptance criteria

- [ ] `CreditsScreen::new(&ctx, &credits)` on a 100 × 32 console wraps
      headings, entries and details at 91, 89 and 87 characters.
- [ ] Ported tests pass: rolling every `row_secs` after a rest, rest
      then restart at the bottom, at most once round per frame, Confirm
      toggles, cursor stops rolling, Cancel pops with its sound.
- [ ] `.with_music("m")` asks for `m` once; without it, no music request.
- [ ] Snapshots of the glyph look at 100 × 32 and 50 × 15.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: rows, wrapping, scrolling, rolling, actions, sounds, music,
  view and help, on a toy context. Snapshot: the glyph look at two sizes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
