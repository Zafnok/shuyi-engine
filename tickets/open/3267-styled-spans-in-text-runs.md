---
id: "3267"
title: "Styled spans in text runs: colour roles, bold and italic as fonts by id, underline and strikethrough, a background, inline pictures, and spans the pointer clicks"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2334"]
nick_input: none
completed:
---

# 3267 — Styled spans in text runs

## Context

2334 places proportional text in pixels as runs, with an outline and a
shadow and fonts by id. 2347 (done) styles cell text with colour roles
and key names. Both **Out** lists leave styles inside one run ("styled
spans inside a run … a ticket of its own when a game asks"), with
bold, italic, inline icons and clickable spans. The Bevy coverage
audit of 2026-11-28 (`docs/ROADMAP.md`, *Bevy's game coverage*) found
the games that need them: RPGs and card games whose text colours a
keyword, shows an item's icon in a line or a button's glyph in a hint,
and lets the player click a keyword for its glossary entry; visual
novels with emphasis. Bevy has text spans, inline boxes and
decorations (`crates/bevy_text/src/text.rs`, `inline_box.rs`;
`crates/bevy_ui/src/widget/inline_image.rs`;
`examples/ui/text/inline_image.rs`, `strikethrough_and_underline.rs`,
`text_background_colors.rs`).

## Nick input

None.

## Scope

**In:**
- `StyledRun`: a list of spans, each with a font id (so bold and italic
  are fonts a game bakes, as 2224 bakes weights), a colour role,
  underline and strikethrough, an optional background rectangle, and an
  optional `HitId`.
- **Inline pictures:** a span that is a sprite, measured as a box with
  the line's ascent so wrapping keeps it whole and the baseline lines
  up.
- **Markup in text files:** a small span syntax in translated strings
  (`[b]`, `[i]`, `[role=warn]`, `[icon=coin]`, `[link=fire]`), checked
  by `shuyi check` (an unknown role, icon or unclosed tag is an error
  with its key and position), and the pseudo-language and cut-text walk
  (2353) keep the tags whole.
- **Pointer:** a span with a `HitId` records a hit area for each of its
  line pieces, so a click on a keyword reports it (2409).
- **Snapshot text** prints the run with span markers, so tests read
  which word is which style without reading pixels (rule 9).
- New types follow ADR-0039.

**Out (do not do):**
- synthesised bold and italic (a game bakes those fonts);
- shaping and right-to-left text (2908);
- cell text: 2347 already styles it;
- letter spacing and per-span sizes (one size per run).

**Estimated size:** about 1,300 lines (code 600, tests 550, docs 150).

**Split:** past 1,000 lines, split off the markup in text files into a
new ticket, leaving spans built in code here.

## Implementation steps

1. Read 2334's run layout, 2347's styled text, 2224's font ids and
   2409's hits.
2. Add `StyledRun` and measure spans through 2334's line breaker.
3. Add inline sprite boxes and the baseline rule.
4. Add decorations and backgrounds in every renderer.
5. Add hits per span piece.
6. Add the markup and its checks.
7. Write the docs page: a card's rules text with a keyword and an icon.

## Acceptance criteria

- [ ] A run whose spans all share one style lays out exactly as 2334's
  plain run.
- [ ] An inline icon is never split from the word after it by a wrap
  when no space separates them.
- [ ] A keyword span wrapped over two lines gives two hit areas, and a
  click on either reports its `HitId`.
- [ ] `[role=nope]` in a strings file fails `shuyi check` with the key
  and position.
- [ ] The snapshot of a styled run equals its golden text.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: span measurement, decorations, the markup parser.
- Property: any styled run's text, with styles dropped, wraps at the
  same places as the plain run.
- Snapshot / integration: goldens in frame-png; snapshot text.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
