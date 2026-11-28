---
id: "2865"
title: "The launch-day kit: a fact sheet, a technical FAQ and a checklist with one row per channel, for Nick to post in his own words"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2859", "2861", "2862", "2863"]
nick_input: setup
completed:
---

# 2865 — The launch-day kit: fact sheet, FAQ, checklist

## Context

Hacker News bans generated or AI-edited text in posts and comments
(<https://news.ycombinator.com/newsguidelines.html>). This Week in Rust
takes project links only from r/rust and wants human-written ones
(<https://github.com/rust-lang/this-week-in-rust/issues/8575>). Readers on
r/rust, r/roguelikedev and the Rust forum flag LLM prose. So, under 2859's
answer, agents write no post or reply text: they prepare the facts and a
checklist, and Nick types the words.

Show HN is restricted for new or inactive accounts
(<https://news.ycombinator.com/showhn.html>), so Nick's ordinary use of
his HN account has to start weeks before. The channels are the ones Nick
said yes to in 2859.

The pitch lines the heat check found accurate and still interesting, for
the fact sheet:

- A small Rust engine for grid games that a coding agent can run, step,
  screenshot and replay with no window.
- The same seed and the same inputs give the same game, so a playtest is
  a text snapshot a unit test diffs.
- Coding agents wrote almost all the code; the CI is the review (the
  gates, named, with a run to judge).
- Grid games today, turn-based and in real time; no free movement,
  side-scrollers, 3D or online play yet.
- Play Snake and Hello Grid in the browser in seconds; the Codespaces quick
  start builds one in minutes.

## Nick input

**Setup**, by Nick, starting now and on launch day:

1. Now: use your own HN account as you normally would (read, comment
   now and then). No new accounts for the launch.
2. 1–2 weeks before launch: one reply of 3–5 sentences of your own in
   users.rust-lang.org's weekly "What's everyone working on this week"
   thread.
3. On launch day, for each row of `docs/launch/checklist.md`, in its
   order and a day or more apart where it says: read that channel's rules
   yourself (the row's link) and tick its rule box; write the post in your
   own words from the fact sheet, the disclosure in its first two
   sentences and "grid" in its title; post it from your own account; ask
   nobody to upvote; stay in the thread the hours the row says, answering
   in your own words ("I'll check and follow up" is fine); paste the post's
   link into the row.

## Scope

**In:**
- `docs/launch/facts.md`: the pitch lines above; today and not yet (from
  the honest page, 2861); the disclosure; the gates with a CI run's link;
  the one sentence on the paid game (2859, question 7); contributions;
  links (the browser examples, the Codespaces quick start, the honest
  page, the demo clip, the install lines); one open question for readers
  ("what should an agent-facing game API expose that this doesn't?").
- `docs/launch/faq.md`: short technical answers Nick can put in his own
  words: determinism and the float ban in simulations, why macroquad, the
  build and web sizes, why the names were held (2862), and "why not
  Bevy?" ("Bevy is a far bigger, general engine; this one is narrow, grid
  games driven by an agent; use Bevy if you need more").
- `docs/launch/checklist.md`:
  - pre-launch gates: crates live on crates.io; no `shuyi` crate at 0.0.0
    (2862); the web examples load; the honest page is live; the demo
    clip is ready;
  - one row per channel Nick said yes to: the rules link, a rule box, the
    kind of post, when (US weekday morning for r/rust and Show HN), the
    hours to stay, the claims never to make, and an empty link box;
  - the claims never to make: "a 2D game engine" with no "grid"; "AI makes
    your game", "any game in one prompt"; any comparison or benchmark;
    "the first", "revolutionary", "production-ready", "battle-tested";
    "reviewed" or "audited" as if a person read the code; "community
    driven" or "contributions welcome"; any target or online play that
    isn't there; any count of users or stars beyond the day's.
- **Either order with 2866:** if 2866 is done, the checklist has a row
  for Nick's plugin directory form; if not, nothing to do (2866 adds the
  row).
- `docs/launch/` and its docs-map row, if not there yet.

**Out (do not do):**
- Any post body, title or reply text.
- Posting, signing up or messaging anyone.
- Channels the heat check rules out: r/opensource (bans AI-made
  projects), Lobsters, Handmade Network, Bevy's spaces, r/gamedev,
  Product Hunt, a Crate of the Week self-nomination, any This Week in Rust
  PR (rule 12).
- The directory listings (2866) and the itch page (2864).

## Implementation steps

1. Collect the facts from the honest page, the release and 2859's entry.
2. Write the three files.
3. Grep them for each never-make claim (outside the list itself).
4. Ask Nick to read the checklist once.

## Acceptance criteria

- [ ] `facts.md`, `faq.md` and `checklist.md` exist in `docs/launch/`.
- [ ] `checklist.md` has exactly one row per channel Nick said yes to in 2859, each with a rules link, a time, the hours to stay and an empty link box.
- [ ] Its pre-launch gates include "crates live on crates.io" and "no `shuyi` crate at 0.0.0".
- [ ] None of the three files has a paragraph written as a ready-to-paste post: each item is a fact, a link or a box.
- [ ] A grep finds no never-make claim in the three files outside the list itself.
- [ ] The completion notes have `Book:` and `Skills:` lines saying why neither changes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None: no code.

## Completion notes

*(Filled in by the session that completes the ticket.)*
