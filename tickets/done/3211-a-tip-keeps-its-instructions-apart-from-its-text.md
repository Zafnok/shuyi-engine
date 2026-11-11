---
id: "3211"
title: A tip keeps its instructions apart from its text
type: feature
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-06
---

# 3211 — A tip keeps its instructions apart from its text

## Context

1. **The game ticket:** game#0475
   ([`tickets/open/0475-vixs-lines-apart-from-the-objectives.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0475-vixs-lines-apart-from-the-objectives.md)).
2. **The job:** a tip says something a recorded voice can speak, and
   apart from it, what to press.
3. **No friction review:** a missing feature.
4. **The generic shape:** a tip (`shuyi_assets::tip::Tip`) gains an
   optional `how`, plain instructions with `{Action}` placeholders, kept
   to the box like its text, with its own line limit
   (`TipLimits::how_lines`). A voice clip is made ahead of time and can't
   say a key the player rebound, so a game whose tips are spoken keeps
   the keys in `how`.
5. **Not tactics:** a platformer's narrator says "Mind the gap, the
   ledge crumbles", and under it the box shows "Jump: `{Jump}`".

## Nick input

None.

## Scope

**In:** `Tip::how`, `TipLimits::how_lines`, their checks (line count,
line width, placeholders) and a test.

**Out (do not do):** what a game shows of `how`, and a language pack's
keys for it (the game's own, as its tip text keys are).

## Acceptance criteria

- [x] A tip with no `how` loads as before (`None`).
- [x] A `how` past the limit, empty, too wide or with an unknown
  placeholder is reported (test).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `a_tips_how_is_checked_like_its_text`.

## Completion notes

Done as scoped. The game's PR that moves the pin to this change is
game#0475's.

Breaking: shuyi-assets: TipLimits: a new field, `how_lines`, the most lines of a tip's `how` (0 if the game's tips have none).

```migration
since: 0.2.0
what: TipLimits has a new field, how_lines, the most lines of a tip's how.
old: TipLimits { text_lines, line_chars, title_chars }
new: TipLimits { text_lines, line_chars, title_chars, how_lines }
before: |
  TipLimits { text_lines: 3, line_chars: 60, title_chars: 30 }
after: |
  TipLimits { text_lines: 3, line_chars: 60, title_chars: 30, how_lines: 0 }
```

Breaking: shuyi-assets: Tip: a new field, `how`, a tip's instructions apart from its text (`None` when a tip built in code has none).

```migration
since: 0.2.0
what: Tip has a new field, how, the tip's instructions apart from its text.
old: Tip { id, trigger, title, text, face }
new: Tip { id, trigger, title, text, how, face }
before: |
  Tip { id, trigger, title, text, face: None }
after: |
  Tip { id, trigger, title, text, how: None, face: None }
```

Book: none, the book names tips only for how their limits count cells
(`assets-and-formats.md`), which `how` keeps to as the text does.
