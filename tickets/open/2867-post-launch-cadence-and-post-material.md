---
id: "2867"
title: "After the launch: one page of what Nick posts where and how often, with a stop rule each, and the material for his technical post"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2864", "2865"]
nick_input: setup
completed:
---

# 2867 — After the launch: the cadence and the technical post's material

## Context

After launch day, most channels take updates only in their own threads:
users.rust-lang.org's weekly "What's everyone working on this week"
thread, r/roguelikedev's Sharing Saturday, an itch.io devlog (2864). This
Week in Rust picks project links from r/rust only. Screenshot Saturday is
for games, not engines, so it gets only a game made with the engine (the
7DRL entry, 2868).

The research found that a post that teaches something does better than
a second announcement, and lasts: for example how a deterministic loop,
text snapshots and frame-to-PNG let an agent test a game like a unit
test, with what the agent got wrong. HN and This Week in Rust want it in
the author's words, so agents prepare its material only, and Nick writes
it (2859, question 3). It is not written with the `write-post` skill.

## Nick input

**Setup**, weekly while there is real progress, as `docs/launch/cadence.md`
says:

1. The Rust forum's weekly thread: 3–5 sentences of your own, one link,
   the one-line disclosure; launch week and up to 2 weeks after, then
   stop.
2. r/roguelikedev's Sharing Saturday: what changed, in your own words,
   only when something did.
3. Each release: paste its News post as an itch devlog.
4. Weeks 2–4 after launch: write the technical post yourself from
   `docs/launch/post-material/`, put it on News (a `Post:` PR; an agent
   can place your text), and share it once on r/rust.
5. Screenshot Saturday only for a game made with the engine, never the
   engine.

## Scope

**In:**
- `docs/launch/cadence.md`: one row per recurring channel, with its
  thread, its rules link, how often, and a stop rule; the rule "one post
  per community; a second r/rust or Show HN post only for a release that
  adds a kind of game, never a version bump".
- `docs/launch/post-material/`: real, unedited transcripts of an agent
  calling `start`, `step` and `frame_png` on Snake and on the demo puzzle
  (2863); the PNG files; a replay test passing; a list of what the agent got
  wrong, taken from the transcripts; a list of the limits (grid games
  only).
- A per-release line in the release's checklist: itch devlog, Sharing
  Saturday if there is real progress, the listings check (2866).

**Out (do not do):**
- Any post or reply text.
- A `write-post` News post meant for r/rust or This Week in Rust.
- r/gamedev posts, Ludum Dare, hosting a jam of our own.

## Implementation steps

1. Write `cadence.md`.
2. Run the agent on Snake and on the demo puzzle through the MCP server;
   save the transcripts unedited (no personal paths or tokens in them).
3. Write the "what went wrong" list from the transcripts, nothing
   invented.
4. Add the release-checklist line.

## Acceptance criteria

- [ ] `cadence.md` has one row per recurring channel, each with how often and a stop rule.
- [ ] `post-material/` holds at least two transcripts, at least four PNG files made by `frame_png`, and a "what went wrong" list of at least three items, each pointing at a transcript line.
- [ ] `cadence.md` says Screenshot Saturday is for games made with the engine only.
- [ ] The release checklist has the per-release line.
- [ ] The completion notes have `Book:` and `Skills:` lines saying why neither changes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None: no code.

## Completion notes

*(Filled in by the session that completes the ticket.)*
