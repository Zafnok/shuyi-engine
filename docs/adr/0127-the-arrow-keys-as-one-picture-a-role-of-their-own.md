# ADR-0127: The cursor's arrow keys as one picture: a role of their own, a picture in the key caps' style

- **Status:** Accepted; extends ADR-0124
- **Date:** 2026-11-17
- **Related tickets:** 3217 (this); 3215 (key caps); game#0250, game#0254, game#0255

## Context

ADR-0124 drew every `TextRole::Key` span as a key cap. The cursor's four
keys were one key span: the language's word for the arrows (`arrows`)
when the four cursor actions were on the arrow keys, else the four keys
run together (`wasd`), so caps drew one box round either.

The game decided (game#0250) that the arrow keys, when they move the
cursor, are one small picture of the arrow cluster (four 8 × 8 caps in an
upside-down T, 24 × 16 px, which the game draws), and any other four keys
four caps. The picture is the game's; the engine has to know where it
goes, how many cells it takes, and that the text there is the arrow keys.

## Decision

1. **A role of their own.** `TextRole::Arrows` is the four arrow keys
   that move the cursor, together; its text is the language's word for
   them. Not a flag on a `Key` span: a span is text and one role, and
   every place that already matches on a role (the palette's colours, the
   frame) handles a new value of a `#[non_exhaustive]` enum where it
   handles keys. The palette paints it as a key.
2. **The styled cursor keys.** `help_keys::cursor_keys_styled` gives one
   `Arrows` span when the four cursor actions' primary chords are the
   plain arrow keys; else four `Key` spans, up, left, down, right, with a
   plain space between them; a controller's buttons, or the not-mapped
   word, are one key span. `cursor_keys_name` is its `plain()`, so the
   other four keys now read `w a s d`, not `wasd`.
3. **`{Cursor}` in styled text.** `fill_styled` takes a lookup that gives
   a key's name as text; `fill_styled_with` takes one that gives spans,
   and keeps their roles (their spaces made no-break, as a key's name's
   are). `GameData::key_text_styled` is what `Ctx::text_styled` looks a
   placeholder up with: by default the styled cursor keys where the
   game's `key_text` gives the cursor keys' name, else one key span of
   what `key_text` gives. A game that overrides `key_text` keeps working,
   and gets the cluster wherever it names the cursor keys.
4. **The picture is in the key caps' style.** `KeyCapStyle::arrows` is an
   image and its part (`ImageId`, `PxRect`). It is drawn one line tall,
   its width scaled as much, rounded to the pixel, and takes the whole
   cells that width needs (`KeyCaps::arrows_cells`), centred in them.
   `CellWidths` carries those cells (`arrows_cells`) beside a cap's extra
   cells, so `StyledText::width` and `cut` count them without the font,
   and `cut` keeps the picture whole or not at all, as a cap.
5. **Drawn as a sprite.** With caps on and a picture, `print_styled`
   writes the picture's cells as blanks on plain text's background and
   adds a `Sprite` over them; it advances by the same cells. With caps
   on and no picture, an `Arrows` span is a cap of its word; with caps
   off, the word in the key's colours: as before this ADR.

## Consequences

- With no picture, nothing a game draws changes; with caps off, no
  snapshot changes.
- The four other cursor keys read `w a s d` in every text, also plain
  (`cursor_keys_name`, `text_with`): a behaviour change a game's tests
  see.
- `GlyphBuffer` keeps its `KeyCaps` behind an `Rc`: the style grew by
  the picture, and a buffer is held by value in enums.
- A controller's direction pad as a picture is game#0255's, through the
  same role or one beside it.

## Alternatives considered

- **A flag on a `Key` span.** Every span compares roles to join and to
  paint; a flag would be a second field each place reads, for one case.
- **The picture in the palette or a theme file.** The picture is part of
  how keys are drawn, which `KeyCapStyle` already says, and a game picks
  its image ids in code.
- **A lookup that returns spans, in place of `fill_styled`'s.** A break
  for every caller of `fill_styled`; a second function costs one name.
