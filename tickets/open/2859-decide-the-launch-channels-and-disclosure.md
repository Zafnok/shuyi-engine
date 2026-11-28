---
id: "2859"
title: "Decide the launch: the channels, the agent-written disclosure, the placeholder crates, the listings, a jam, and what counts as traction"
type: decision
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 2859 — Decide the launch: channels, disclosure, placeholders, listings, a jam, traction

## Context

The first release (2005) is the public launch (`docs/ROADMAP.md`, phase 2,
Nick's choice of 2026-11-05). On 2026-11-28 Nick asked how to get some
traction after it "without overpromising or getting heat", spending little
or nothing: a few developers who try it for a weekend, maybe a video like
Fireship's. A research pass (six channel researchers and a skeptic, with
sources) found that most of the risk sits in product choices only Nick can
make. Tickets 2860–2868 carry out the answers.

What the research found, in short:

- **The biggest risk is being read as "AI slop".** The code is written
  almost entirely by coding agents and Nick does not review it
  (`AGENTS.md`, *Who decides what*). In 2026 several communities pushed
  back on that: r/opensource banned projects "made by AI, or intended for
  AI" (2026-10-24), Hacker News restricted Show HN for new accounts and
  bans generated text in posts and comments
  (<https://news.ycombinator.com/newsguidelines.html>), This Week in Rust
  takes project links only from r/rust since 2026-08-10 and wants
  human-written ones
  (<https://github.com/rust-lang/this-week-in-rust/issues/8575>). Hiding
  it is worse than saying it: the advice is to say it in the first two
  sentences, with the CI gates as the review, and for Nick to write every
  post and reply in his own words.
- **Empty 0.0.0 crates read as name squatting.** The engine holds its
  crates' names as empty 0.0.0 placeholders until the first release
  (`docs/decisions.md`, 2026-10-21 and later). Each is a workspace crate
  that 2005 publishes (3510), so after the release none should stay
  empty, but some are thin for now (`shuyi-net`, `shuyi-playdate`), and a
  reader who finds a 0.0.0 page, or a crate that says less than it does,
  calls it squatting. crates.io's usage policy (RFC 3463,
  <https://rust-lang.github.io/rfcs/3463-crates-io-policy-update.html>)
  is aimed at names held without real work. 2862 checks it.
- **Overclaiming:** "a 2D game engine" with no qualifier, while it does
  grid games only; any comparison with Bevy or Godot (the benchmark,
  3432 and 2562, is parked and has no numbers).
- **Unsolicited pitches to YouTubers read as spam** for an unknown
  project. Coverage like Fireship's comes from a clip people already
  share, so 2863 makes one and nobody is contacted.
- **Hosting our own itch.io jam** breaks itch.io's guidance against a jam
  made to promote one's own project
  (<https://itch.io/docs/creators/quality-guidelines>), and would get no
  entries without users. Entering 7DRL (a week-long roguelike jam that
  allows engines and earlier code, <https://itch.io/jam/7drl-challenge-2026>)
  is the honest version, but the roguelike pieces (2119, 2120) are parked
  until Visions of Shuyi launches (`docs/decisions.md`, 2026-10-22).
- **Rule 12 (no PR or issue on another project)** rules out awesome lists,
  an arewegameyet.rs entry and This Week in Rust PRs. The official MCP
  Registry (a CLI with GitHub sign-in, no PR) and Anthropic's plugin
  directory (a web form) are not PRs.
- **Traction has no number.** 1132 (the community files) and outside pull
  requests come back "if I get any traction" (`docs/decisions.md`,
  2026-11-01), and nothing defines it.

## Nick input

**Decision.** The questions, each with options and a recommendation, asked
with the `ask-nick` skill:

1. **Channels on launch day**, each yes or no: an r/rust text post; Show
   HN; one users.rust-lang.org Announcements topic; macroquad's Discord
   (a showcase channel, if there is one); r/ClaudeAI (its showcase thread
   if its rules say so); a short clip on X and Bluesky; an r/roguelikedev
   intro in launch week; an itch.io tool page (2864). *Recommended:* all,
   a day or more apart, one post a channel, all from Nick's own existing
   accounts.
2. **The disclosure:** every post says in its first two sentences that
   coding agents wrote almost all the code, that Nick does not review it
   line by line, and that the CI gates are the review. *Recommended:* yes.
3. **Who writes the posts:** Nick, in his own words, from a fact sheet
   (2865), with agents writing no post or reply text. *Recommended:* yes;
   HN bans generated text and the Rust channels flag it.
4. **The held crate names:** no launch post until no `shuyi` crate on
   crates.io is still at 0.0.0 and each crate's page says what it does
   today, thin ones included (2862). *Recommended:* yes. If 2005 ever
   leaves a crate out of the release, label it (a README saying what it
   is held for and its ticket) or hand the name back: which?
5. **Listings:** does a listing that needs a PR or an issue on another
   project (awesome lists, arewegameyet.rs) fall under rule 12?
   *Recommended:* yes, and let others list it. Is the MCP Registry
   (published from the release job) and a web form to Anthropic's plugin
   directory, submitted by Nick, allowed? *Recommended:* yes (2866).
6. **7DRL 2027** (late February to early March 2027) with a small
   original roguelike made with the engine, which unparks 2119 and 2120
   early? *Recommended:* Nick's call; it is the best trial channel found,
   at 10–25 hours of his time in the jam week.
7. **The paid game:** one plain sentence in posts ("carved out of a
   commercial tactics game I'm making, its first user; the engine is MIT
   or Apache, no paid tier, no telemetry"), never a store link, never its
   art. Confirm.
8. **Sponsors and Ko-fi** (2806): no sponsor link in launch posts.
   Confirm.
9. **Traction:** which number brings back 1132 and outside pull
   requests? For example a count of people other than Nick who opened an
   issue in 30 days, games made with the engine by others, or stars over
   30 days, as 2860 measures them.

## Scope

**In:**
- One `ask-nick` session with the nine questions above.
- The answers, as one dated entry in `docs/decisions.md`, each saying which
  of 2860–2868 it unblocks or changes.
- Each of 2860–2868 edited to follow the answers (a channel dropped from
  2865's checklist, 2868 unparked or left parked, 2862's rule for a
  crate a release leaves out).
- If Nick says yes to 7DRL: 2119 and 2120 unparked (`status: todo`,
  `Parked:` off their titles) and 2868 unparked, in this PR.

**Out (do not do):**
- Writing any post, page or kit (2861, 2865 and the others).
- Moving 2005's `blocked_by` (Nick's call; ask only if an answer needs it).
- Technical choices, which are the agent's.

## Implementation steps

1. Read `docs/decisions.md` on the name and the held crates (2026-10-21,
   2026-10-22 and later), contributions and 1132 (2026-11-01), the
   website and sponsors (2026-10-20), and the benchmark (3432).
2. Write the nine questions with their options, one line on why each
   recommendation, and what each answer unblocks.
3. Ask with `ask-nick`; record the answers in `docs/decisions.md`.
4. Edit 2860–2868 (and 2119, 2120 if 7DRL is yes) to match.

## Acceptance criteria

- [ ] `docs/decisions.md` has one entry, dated the day Nick answers, with an answer to each of the nine questions, quoting him.
- [ ] The entry lists each launch-day channel as yes or no.
- [ ] The entry names a traction number (a metric, a threshold and a window) or says Nick chose none.
- [ ] 2860–2868 agree with the entry (a channel Nick said no to appears in no checklist).
- [ ] The PR changes only `tickets/`, `docs/ROADMAP.md` and `docs/decisions.md` (a `Ticket 2859: …` PR).
- [ ] `cargo xtask ticket-lint` passes.

## Tests required

- None: a decision. `cargo xtask ticket-lint` checks the tickets it edits.

## Completion notes

*(Filled in by the session that completes the ticket.)*
