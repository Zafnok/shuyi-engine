---
id: "2917"
title: "Research: the reverse-engineering angles the track had not covered, confirmed at their sources, and the plain statements the book owes"
type: research
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-14
---

# 2917 — Confirm the uncovered-angles research and write the book's plain statements

## Context

On 2026-11-08 a session surveyed, without attempting anything, the
angles of AI-assisted and visual reverse engineering that the
public-demo-defenses track (2912–2915, 3080–3082, 2816, and the parked
2602, 2603 and 3083; [`decisions.md`](../../docs/decisions.md),
2026-10-31) had not covered. Eleven research agents ran, one over this
repository and ten over the web, and the coordinating session checked
the repository findings in the code. The draft is
[`docs/research/2917-reverse-engineering-angles-not-yet-covered.md`](../../docs/research/2917-reverse-engineering-angles-not-yet-covered.md).

The draft's repository findings became tickets 3099 (release-build
hygiene), 3300 (a shipped pack), 3301 (a store watch) and 3302 (a
per-recipient overlay), and a step in 2816. Its web findings rest on
search summaries, not primary reads, and each section says so. Ticket
2918 (done 2026-11-09) already wrote the first two entries of the draft's
"What changed on confirmation" section: the section 11 Needle In A
Haystack row corrected at its sources, and 2914's ground restated as the
behavioural oracle rather than its timing. Keep both entries and add to
them. The docs
map ([`docs/README.md`](../../docs/README.md)) keeps one research report
per research ticket, so the draft needs a ticket, a pass over its
sources, and a home for the plain statements it says the book owes:

- a web demo is a published copy of its assets and rules (**From 3082
  (done):** the book's *A public demo*, in `targets-and-export.md`, says
  it, with how an edition leaves the rest out; check its words against
  the confirmed claim);
- anything decoded, drawn, heard or simulated on the player's machine
  can be extracted, and no engine setting changes that;
- art perturbation tools are unproven on small palette-limited sprites;
- Steam Playtest, override keys and a password branch are a controlled
  preview at no cost, with the gaps named;
- attacker capability moved in 2026: whole-game ports from shipped
  binaries by agents are routine, so any later measurement in the track
  assumes a REA-class harness and a frontier model as its floor.

## Nick input

None.

## Scope

**In:**
- Read the primary source behind each numbered claim in the draft's
  sections 2 to 11 that the draft marks as a summary, inference or
  unverified; keep, correct or strike the claim and say which in the
  report's "What changed on confirmation" section. Spend under US$10 of
  model reading and under a day.
- The five plain statements above, in the book's export, release and
  (if one exists) art chapters, each in one or two sentences with a link
  to the report, through the `document` skill.
- The report's final form: the draft header replaced by the standard
  research header, a "Decision" section that lists the tickets each
  angle became and the angles dropped with their reasons, and the
  ROADMAP's public-demo-defenses table updated to say which rows came
  from it.
- A line in 2912's report pointing at section 6 (attacker capability).

**Out (do not do):**
- Any experiment, attempt, build or measurement. 2912's protocol exists
  for that, and a new experiment is its own research ticket.
- Prompt-injection notices aimed at agents (2915 excludes them).
- Hosted networking (parked) and console or mobile store test tracks.
- Legal conclusions. The report stays "research, not legal advice".
- New tickets beyond those already written, unless a confirmed source
  overturns a dropped angle; then a `write-ticket` follow-up, named in
  the completion notes.

## Implementation steps

1. List every claim in sections 2–11 flagged as unverified, inference or
   summary; for each, fetch the named source and record keep, correct or
   strike with a one-line reason.
2. Rewrite the header and add the "Decision" and "What changed on
   confirmation" sections; keep the method section honest about the
   first pass.
3. Write the five statements where the docs map puts them; run
   `cargo xtask check-links` and the docs-map check.
4. Update the ROADMAP table rows and 2912's report line.
5. Run `cargo xtask ticket-lint`.

## Acceptance criteria

- [x] Every flagged claim in sections 2–11 has a recorded keep, correct
      or strike with its source.
- [x] The report has the standard header, a "Decision" section and the
      confirmation section.
- [x] The five plain statements are in the book with links, and the
      docs-map and links checks pass.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Docs: `cargo xtask check-links`, the docs map check, typos.
- Tickets: `cargo xtask ticket-lint`.

## Completion notes

Done 2026-11-14.

- **Confirmation.** Six source-reading agents on a smaller model (one per
  group of sections), well under the US$10 cap, each returning keep,
  correct or strike with the URL. The report's "What changed on
  confirmation" entry 3 records all 53 verdicts; the sections carry the
  corrected text and the primary links. Three claims were struck: the
  Space Cadet Pinball cost and the 30-minute REA clone (no source found)
  and the Half-Life 2 lore (not needed); the rest were kept or corrected. Notable corrections: `goldberg` is
  GPL-3.0; Tkachenko et al. rank instruction substitution most resistant;
  Valve's 2026-10-28 remarks were about AI clones on Steam, not the
  browser ports; the reversedwithai.com counts; Freya Holmér's prototype
  was knocked off within five hours, not a day. Some primary pages
  refused automated reads (EUR-Lex, OpenAI's policy, the USPTO and EUIPO
  fee pages, the Glaze site); the table says which verdicts rest on a
  secondary page or memory.
- **Report.** The draft header is replaced by the standard research
  header, a Decision section lists what each angle became (3099, 3303,
  3300, 3301, 3302, 2816's checklist, the book) and the dropped angles
  with their reasons. Section 1 is kept as the 2026-11-08 finding, now
  noted as fixed by 3099, 3303 and 3300.
- **Book.** The five statements are in *Targets and export*: the web demo
  as a published copy (*A public demo*, 3082's sentence kept and
  linked), runtime extraction (*What a package gives away*), Steam's and
  itch.io's own preview tools with their gaps (*Releasing*), attacker
  capability and art perturbation (*Before the reveal*; the book has no
  art chapter). Each links the report's section.
- **ROADMAP and 2912.** The public-demo-defenses rows from this report
  say "from 2917"; 2912's report points at section 6.
- **Handover.** 3301's context restated the Needle and Wordle cases as
  the draft had them; it now carries the confirmed facts as a
  "From 2917 (done)" paragraph.
- No confirmed source overturns a dropped angle, so no new ticket.

Book: updated, `site/content/learn/book/targets-and-export.md`.
