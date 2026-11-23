---
id: "3531"
title: Every seam between two engine systems is reviewed, and each one missing is wired or ticketed
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3530"]
nick_input: none
completed:
---

# 3531 — Every seam between two engine systems is reviewed, and each one missing is wired or ticketed

## Context

3530 lists the engine's systems in `docs/systems.ron`. Each one says how
it works with every system before it: `wired` with a test, `planned` by
a ticket, or `none` with a reason. The seed leaves the entries it
couldn't settle as `unreviewed`, and their count sits in
`tools/systems-unreviewed.txt`.

This ticket settles them, and with them the drift Nick fears most: a
system written before another one existed, and never wired to it
afterwards (3530's Context). Every `unreviewed` entry is a question:

- Does the older system need to know about the newer one?
  - Does the renderer pause, shake or flash with what the sound does?
  - Does the debug view show the sound?
  - Does the replay record what the script decided?
- Does the newer one need to know about the older one?
  - Does sound mute in the background, keep to reduced motion, and
    show captions?

## Nick input

None.

## Scope

**In:**

- **Settle each `unreviewed` entry** in `docs/systems.ron`. Read both
  systems' code and their tickets, then pick one:
  - **`none(why)`:** nothing passes between them, with the reason;
  - **`wired(test)`:** the seam exists. Point at a test that proves it,
    or write a small test if there is none;
  - **`planned(ticket)`:** the seam is missing. Write a ticket for it
    with the `write-ticket` skill.
- **Group the missing seams by the system they change.** One ticket per
  system wires all of that system's seams (for example "sound keeps to
  pause, reduced motion and the debug view"), so a dozen gaps make a
  few tickets rather than a dozen. A seam that touches a parked
  ticket's work becomes a parked ticket too.
- **Work the ratchet down** in `tools/systems-unreviewed.txt` to 0.

**Out (do not do):**

- Building the missing seams: the tickets this one writes do that.
- Changing 3530's check or file format, except a fix the review shows
  it needs, named in the completion notes.

**Estimated size:** about 900 lines (the data edits 400, the proving
tests 400, notes 100; the tickets it writes count apart).

## Implementation steps

1. List the `unreviewed` entries, grouped by the newer system of each.
2. For each group, read both systems and grep `tickets/open` for the
   seam. Settle each entry as Scope says.
3. Write the gap tickets, one per system the gaps change, each listing
   the seams it wires. Each one's `planned` entries name it.
4. Set `tools/systems-unreviewed.txt` to 0, and run
   `cargo xtask check-systems` and `ticket-lint`.

## Acceptance criteria

- [ ] `docs/systems.ron` has no `unreviewed` entry, and
      `tools/systems-unreviewed.txt` says 0.
- [ ] Every `none` gives a reason a reader can check against the code.
- [ ] Every new `wired` test runs both systems together.
- [ ] Every `planned` entry names an open ticket written by this PR or
      before it, and those tickets pass the ticket lint.
- [ ] The completion notes list the gap tickets written and the seams
      each one wires.
- [ ] The book's chapter is not updated: this is for engine developers
      only, so the completion notes have a `Book:` line saying so.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: one test per new `wired` entry, running both systems.

## Completion notes

*(Filled in by the session that completes the ticket.)*
