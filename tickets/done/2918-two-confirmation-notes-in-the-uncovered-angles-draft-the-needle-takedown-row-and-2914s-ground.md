---
id: "2918"
title: "Two confirmation notes in the uncovered-angles draft: the Needle takedown row and 2914's ground"
type: research
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-09
---

# 2918 — Two confirmation notes in the uncovered-angles draft

## Context

On 2026-11-09 Nick asked whether agentic reverse engineering can be
prevented at the execution layer on a local machine, and whether visual
deconstruction and cloning (the Rotris and Needle In A Haystack cases)
can be, and asked for a second check of the track's studies
([2912](../../docs/research/2912-game-cloning-threat-paths.md),
[2913](../../docs/research/2913-human-readable-ai-disruptive-clips.md),
[2914](../../docs/research/2914-protected-cold-rules-dynamic-analysis.md),
[2915](../../docs/research/2915-capture-controls-marks-and-ai-signals.md))
and of the 2917 draft
([`docs/research/2917-reverse-engineering-angles-not-yet-covered.md`](../../docs/research/2917-reverse-engineering-angles-not-yet-covered.md)).
The check found two things the draft should say before 2917's full
confirmation pass:

- The draft's section 11 row for Needle In A Haystack Simulator claimed
  a code-copy clone was removed within about nine days after the
  original developer's notice. The sources say the removed title was a
  clone of a clone, accused by the other cloner, gone from Steam around
  2026-10-21 with no reason disclosed; no clone of the original is
  reported removed.
- The draft cites 2914's result as if its timing were the evidence.
  2914's measured delay is one analyst per arm; the stronger ground is
  that the fixture's hidden phase is one of three values and the trace
  emits `BonusOre` on every gather, so the rule is learnable by play
  and no code-level defense could have scored.

Ticket 2917 (open) owns the full confirmation pass and the report's
final form. This ticket records only these two entries, so that the
draft does not carry a wrong takedown claim while 2917 waits. It is a
research ticket (`29xx`).

## Nick input

None.

## Scope

**In:**
- In the 2917 draft: correct the Needle row and the section 11 candidate
  text, start the "What changed on confirmation" section with the two
  entries and their sources, and point section 6 at it. Note the Rotris
  case as 2913's description bypass in the wild.
- A row for this ticket in the ROADMAP's public-demo-defenses table and
  a sentence in 2917's Context saying these entries exist.

**Out (do not do):**
- Any other claim's confirmation, the report's final header and
  "Decision" section, the book's plain statements and the line in
  2912's report: all 2917.
- Any change to the 2912–2915 reports or their study code.
- Any experiment, build or measurement.

## Implementation steps

1. Edit `docs/research/2917-reverse-engineering-angles-not-yet-covered.md`
   as the Scope says.
2. Add the ROADMAP row and the sentence in
   `tickets/open/2917-….md`.
3. Run `typos`, `cargo xtask check-links`, `cargo xtask check-docs` and
   `cargo xtask ticket-lint` with the PR's title.

## Acceptance criteria

- [x] The Needle row no longer claims a takedown of the original's clone,
  and names the corrected facts and their sources.
- [x] The "What changed on confirmation" section exists with both
  entries, and section 6 points at it.
- [x] 2917's Context names this ticket.
- [x] The book's chapter is updated (2802), or the completion notes have
  a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- None: a documentation change. The docs gates (typos, links, docs map,
  ticket lint) are the checks.

## Completion notes

- Corrected the Needle In A Haystack row and the section 11 candidate
  text, added "What changed on confirmation" with the two entries and
  the Rotris note, and pointed section 6 at it. Added this ticket's
  ROADMAP row and a sentence in 2917's Context.
- The check also read the 2914 kernel (`tools/research/2912/fixture/src/kernel.rs`,
  `build.rs`), its runner and oracle, and the 2912 fixture's trace. 2914's
  attempt ledgers are not committed (2913's and 2915's are), so its
  protocol and code were checked, not its attempts. That limit is now in
  the draft.
- Nothing in the engine, a game-facing API, contract, format or template
  changed. No `Breaking:` line.
- Book: nothing to update. The book's plain statements about web demos,
  runtime extraction and sprites are 2917's work and unchanged here.
- Nick input: none. The answer to his questions was given in the session:
  no execution-layer measure on a player's machine resists an agent with
  the game as its oracle, and no pixel treatment reaches a viewer who
  describes a clip; the levers left are release hygiene (3099, 3300),
  identity and detection speed (2816, 3301).
