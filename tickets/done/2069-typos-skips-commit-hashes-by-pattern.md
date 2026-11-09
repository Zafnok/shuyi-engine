---
id: "2069"
title: "Typos skips commit hashes by pattern"
type: infra
milestone: After MVP
model: haiku-4.5 / gpt-6-luna
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-08
---

# 2069 — Typos skips commit hashes by pattern

## Context

game#0149
([ticket](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0149-typos-ignores-commit-hashes-by-pattern-not-by-allowing-ba-everywhere.md)).

The job: `typos` reads a commit hash written in a ticket or a doc as
prose, and fails on it.

Typos splits a hash like `c1ba9ec8` into three fragments and flags the
middle one as a misspelling of "by". The game passed that once by
allowing the fragment as a word, which lets that typo through everywhere; the retro of 2026-11-08 found
it, and this repository hit a short pin hash too (#314). Here
`_typos.toml` skips only 16-digit snapshot hashes, so a commit hash of
any other length is still read as words.

No friction review: a gate's configuration.

## Nick input

None.

## Scope

**In:**
- `_typos.toml` skips a hex run that holds a digit, the same pattern as
  game 0149: `\b[0-9a-f]*[0-9][0-9a-f]*\b`. A hex-only word such as
  `deadbeef` is still read. Typos's regex has no look-ahead, so the
  pattern can't also require a hash's 7 to 40 characters; a short hex
  run with a digit is never a word anyway.

**Out (do not do):**
- Other typos rules. No API or behaviour change for a game.

## Implementation steps

1. Add the pattern to `extend-ignore-re` beside the snapshot hashes.
2. Run `typos`.

## Acceptance criteria

- [x] `typos` passes on the tree.
- [x] A file with `c1ba9ec8` passes `typos`; one with that fragment alone as a word fails.

## Tests required

- Manual: the typos runs above, in the notes.

## Completion notes

Done with game 0149, whose PR drops the game's allowed hash fragment for
the same pattern. Checked with typos 1.50.3: `typos` passes on the tree; a file
holding `c1ba9ec8` and a 40-digit hash passes; a file with the fragment
alone as a word fails. The 16-digit snapshot pattern
stays: it also skips a snapshot hash with no digit.
