---
id: "3251"
title: "Help bars in styled text: wrap that keeps a key cap whole, joined hints, a HelpBar of shown hints, and a frame's widths from the context"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3251 — Help bars in styled text

## Context

A request from the game.

1. **The game ticket:** game#1412, which deletes its own copies once this
   lands
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1412-help-bars-on-the-engines-styled-help.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** build a screen's help line from the
   hints its state allows. Each key the line names is drawn as a key
   cap on a keyboard and as plain text on a pad. When the line doesn't
   fit, wrap it without ever breaking a key cap, or the key from the
   `,` after it.
3. **The friction numbers:** from the game's third engine-friction
   sweep (game#1397, `docs/engine/friction.md`):
   - **Measured:** 193 production lines in the game's
     `crates/ui/src/widgets/help.rs`, at the game's pin `230080d`. They
     cover `for_player`, `key_styled`, `join_hints`, `HelpBar`,
     `frame_widths`, `styled_lines`, `wrap_styled`, `hint_column` and
     `pad_last_hint`, plus the test helpers `key_spans` and `spans_of`.
   - **Estimate:** about 30 lines for what the game would keep: its
     `HelpKeys` alias, `action_key` and `key_pair` over its `Action`,
     and `text_with_keys` (below).
   - **Ratio:** 6.4×.
   - **Signal:** a copy of engine code. The game's `frame_widths` is the
     engine's `KeyCaps::frame_widths` (`crates/ui/src/frame/key_cap.rs:255`),
     which is `pub(crate)`.
   - **Outcome:** 1, engine feature.
   - **Callers in the game, outside `help.rs`:** `frame_widths` 9,
     `join_hints` 6, `HelpBar::new` 4, `text_with_keys` 4.
4. **The generic shape:** in `shuyi_ui::input::help_keys` and
   `shuyi_ui::text`, the styled side of what `help_line` and `word_wrap`
   already do for plain text.
5. **Not a tactics example:** a farming game's bar at the bottom of its
   shop says "`E` buy · `Q` sell · `Esc` leave". It hides "sell" when
   the pack is empty, shows caps on a keyboard and button names on a
   pad, and wraps on the Steam Deck's narrow layout. That is exactly
   this.

The engine today has these pieces:
- `help_line_styled` (`crates/ui/src/input/help_keys.rs:229`);
- `Ctx::text_styled` and `Ctx::text_styled_with_keys`
  (`crates/ui/src/ctx.rs:679`, `:687`).

What is missing:
- `word_wrap` (`crates/assets/src/text.rs:160`) wraps plain text only.
- Nothing wraps `StyledText` or joins hints.
- `text_styled_with_keys` takes its key arguments as strings, so a key
  pair already built as spans (`w/s`) can't go in.

## Nick input

None.

## Scope

**In:**
- `help_keys::for_player(keys, &StyledText) -> StyledText`: on a pad,
  and wherever a key span says `HelpKeys::not_mapped`, each key or
  arrows span becomes plain text. A no-break space reads as a space.
- `help_keys::key_styled(keys, name) -> StyledText`: one key span, as
  `for_player` shows it.
- `help_keys::join_hints(parts) -> StyledText`: the parts joined with
  `SEPARATOR`.
- `HelpBar` (`#[non_exhaustive]`, `HelpBar::new()`):
  - `hint(key)` and `hint_if(key, shown)` add a hint;
  - `hints()` lists them;
  - `line(&Ctx) -> StyledText` gives each shown hint through
    `Ctx::text_styled`, joined.
  - A hint is one text key, so a language may put the key after the
    verb.
- `text::styled_lines(&StyledText) -> Vec<StyledText>`: split at line
  breaks, roles kept.
- `text::wrap_styled(text, room, widths) -> Vec<StyledText>`:
  - The plain spans break as `word_wrap` breaks them (spaces; Chinese
    and Japanese between characters).
  - A span with a role stays whole, together with what follows it up to
    the next break.
  - A key wider than `room` gets a row of its own.
- `help_keys::hint_column(hints, index, widths) -> usize` and
  `pad_last_hint(hints, column, widths) -> StyledText`: line a last hint
  up in a column, counted from the hints, never by searching the joined
  line.
- `Ctx::frame_widths() -> CellWidths`: `KeyCaps::frame_widths` over the
  context's widths and key caps. The `pub(crate)` function stays.
- `Ctx::text_styled_with_spans(key, &[(&str, &StyledText)])`: like
  `text_styled_with_keys`, with arguments that are spans built by the
  caller.
- Test helpers, behind the `testing` feature where `help_keys` has
  some:
  - `key_spans(&StyledText) -> Vec<String>`;
  - `spans_of(keys, &[&str]) -> Vec<String>`, which takes an action by
    its keymap name and `"Cursor"` for the cursor keys.
- No API break, additions only. The game's PR that moves the pin follows
  this one (game#1412).

**Out (do not do):**
- moving the engine's own screens' help lines onto `HelpBar`: write a
  ticket if it would shrink them;
- the one-bit skins' help rows (engine#2371);
- any key name in code: every name still comes from the keymap.

**Estimated size:** about 600 lines (code 220, tests 300, docs 80).

## Implementation steps

1. Read the game's `crates/ui/src/widgets/help.rs` and
   `widgets/help/tests.rs` at the pin. They are the behaviour to keep.
2. Add the functions above. `wrap_styled` measures each styled span as
   a run of `x` its drawn width, wraps through `word_wrap`, and maps
   the rows back. This is the game's method.
3. Port the game's tests: key caps kept whole, `F,` kept together, a
   key wider than the row, Chinese text, a pad showing plain text, a
   not-mapped key.
4. Doc examples on `HelpBar` and `wrap_styled`. Add a note to the input
   chapter of the book.

## Acceptance criteria

- [ ] `wrap_styled` never splits a key span, nor a key from the `,` right
      after it (unit test with the game's cases).
- [ ] `HelpBar::new().hint("a").hint_if("b", false).line(ctx)` holds only
      hint `a`.
- [ ] `for_player` on a pad device gives no key span.
- [ ] `Ctx::frame_widths` equals `KeyCaps::frame_widths(ctx.widths(),
      ctx.key_caps())`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each function, with the game's cases ported.
- Property: `wrap_styled`'s rows joined back give the text's characters
  in order, and no row is wider than `room` unless it holds one key.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
