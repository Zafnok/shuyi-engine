---
id: "2868"
title: "Parked: Enter 7DRL 2027 with a small original roguelike made with Shuyi, and note what the engine lacked"
type: feature
milestone: "Parked: community"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: blocked
blocked_by: ["2859", "2867", "2119", "2120"]
nick_input: setup
completed:
---

# 2868 — Enter 7DRL 2027 with a small original roguelike

> **Parked** (2026-11-28, written from Nick's ask for "some devs who might wanna try it out for a weekend"): it waits for Nick's yes to 7DRL in 2859, and needs the roguelike pieces 2119 and 2120, parked until Visions of Shuyi launches (`docs/decisions.md`, 2026-10-22). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick asked for a weekend's try by developers, and a jam is the usual way.
Hosting our own jam now goes against itch.io's guidance on jams made to
promote one's own project
(<https://itch.io/docs/creators/quality-guidelines>), and with no users it
would get no entries. Entering one is the honest version: 7DRL, the
Seven Day Roguelike Challenge, allows engines and code written before it
(<https://itch.io/jam/7drl-challenge-2026>), had about 190 entries in
2026, and its people (roguelike makers, turn-based, on grids) are the
engine's. A finished game that plays in the browser says more than any
announcement, and it gives Sharing Saturday and Screenshot Saturday
(2867) something real.

The game is new and original, made in the jam week, in a repository of
its own: nothing of Visions of Shuyi (`AGENTS.md`, rule 4), and not in
this repository (it is a game, not an example). GMTK's jam is out because
it bars generative AI for parts of an entry; Ludum Dare is out because its
theme may need free movement and it costs more hours for few trials.

## Nick input

**Setup:**

1. In January 2027, find the "7DRL Challenge 2027" jam on itch.io, read
   its rules (any AI rule included), and give the agent the dates.
2. Create the game's repository (public, MIT OR Apache-2.0) and click
   "Join jam" with your itch account.
3. In the jam week, direct the agent and playtest (about 10–25 hours).
4. Submit the entry yourself: the browser build, the source link, the
   description from `entry.md`, the AI disclosure as it says.
5. Rate a few other entries honestly. Never promote the engine in other
   entrants' comments.
6. The week after, write the postmortem yourself from the agent's notes,
   for News, Sharing Saturday and Screenshot Saturday.

## Scope

**In:**
- `docs/launch/7drl/plan.md`: the dates; the engine pieces a roguelike
  needs (field of view 2120, level generators 2119, pathfinding) with
  their status; a ticket (the `write-ticket` skill) for each missing
  piece that blocks the entry.
- `docs/launch/7drl/entry.md`: "Made with Shuyi Engine (open source,
  early)", the site's link, "turn-based grid", the agent-written
  disclosure, the itch AI disclosure answers; no paid-game mention.
- `docs/launch/7drl/postmortem-notes.md`, from the jam's session logs:
  what the agent did with no window (step, screenshot, replay), what
  broke, what the engine lacked; each gap a ticket here.

**Out (do not do):**
- The game's code in this repository.
- Building 2119, 2120 or other engine pieces here (their own tickets).
- Ludum Dare, GMTK, and hosting a jam of our own (revisit after ten or
  more people other than Nick have made something with the engine,
  as 2860 counts them).

## Implementation steps

1. Before February 2027: write `plan.md` and the gap tickets.
2. Rehearse the jam's loop on a throwaway prototype, not used in the
   entry.
3. In the jam week: build the game in its own repository, as Nick
   directs.
4. After: write `entry.md` and `postmortem-notes.md`.

## Acceptance criteria

- [ ] `plan.md` lists field of view, level generation and pathfinding, each with a ticket id and its status.
- [ ] `entry.md` has the disclosure sentence and "turn-based grid", and no store link.
- [ ] The entry plays in a browser from its itch page (Nick's link in the completion notes).
- [ ] The game repository's first commit is dated inside the jam's week.
- [ ] `postmortem-notes.md` has the sections: What the agent did with no window, What broke, What the engine lacked.
- [ ] The completion notes have `Book:` and `Skills:` lines saying why neither changes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None here: the game's tests live in its repository.

## Completion notes

*(Filled in by the session that completes the ticket.)*
