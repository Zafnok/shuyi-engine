---
id: "3528"
title: "Nick decides the first release's guards: an agent smoke test, the game passing on the release commit, the five crate names, our own advisories on RustSec, a successor, and which audit tickets join the MVP"
type: decision
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 3528 — Nick decides the first release's guards

## Context

A blind-spot audit of the engine (2026-11-23) found six things about
the first release (2005) that only Nick can decide. Each changes what
the release waits for, what it promises, or one of his accounts. The
evidence below was checked in this repository on 2026-11-23.

1. **Nothing measures how well an agent does before the release.**
   Nick's end goal (`docs/decisions.md`, 2026-10-19) is an agent
   making a game in under an hour and in few lines. The eval that
   measures it (2524), the paid scheduled runs (2525) and the A/B
   against Bevy (2562) all come after the release. 2524's blockers
   (1134, 2803, 2566) are done, so it could start now. The audit also
   found drift an agent would hit: the blank template's game code is
   about 200 lines (`lib.rs` 119, `title.rs` 73, `main.rs` 5,
   `app_info.rs` 3, counted without blank and comment lines), while
   2511 (done) set a cap of 120. 2512 (done, 2026-11-23) now caps it
   in CI, at 220 lines by its count (211 that day), lowered as it
   shrinks. Snake, the end goal's own example, waits for real time
   (2026-10-20), so a test before the release uses a turn-based game,
   such as the tutorial's box pusher (2803).
2. **A release doesn't wait for Visions of Shuyi to pass.** Since
   2026-10-26 (*no build of Visions of Shuyi here*) the game finds its
   own breaks when it moves its pin, within minutes for an API change
   and within 48 hours otherwise. In three of the game's first five
   pin moves the game needed a change. Nothing says a crates.io
   release must wait until the game, pinned to that exact commit, is
   green. A version on crates.io can't be replaced, so a break found
   after it needs a new release. 2028 (checks against the last
   release) and 2035 (a changed golden value needs a `Breaking:` line)
   are After MVP.
3. **Five crate names are not held.** The release must publish
   `shuyi-embed`, `shuyi-embed-macros`, `shuyi-script`, `shuyi-sound`
   and `shuyi-sound-out` (`crates/assets/Cargo.toml:51` needs
   `shuyi-embed-macros`, for one). crates.io's index answers 404 for
   all five (checked 2026-11-23; `shuyi-core` answers 200). The rule of
   2026-10-21 (*Only these six*) says a name is published "when its
   crate is real", and all five are real now. 3510 makes the publish
   list match the workspace and has this as its setup. **Answered:**
   Nick held all five on 2026-11-24 (`docs/decisions.md`, *the last
   five engine crate names are held*); skip question 3.
4. **Our own security advisories may not reach RustSec.** RustSec is
   the database `cargo audit` and `cargo deny` read, our own weekly
   advisory check included (ADR-0037). Its guide takes a new advisory
   as a pull request or an issue on its repository, or by email. Rule
   12 (`AGENTS.md`; 2026-11-10, *no PRs or issues on other projects*)
   forbids a PR or an issue on another project, and nobody has decided
   whether that covers reporting our own crate's hole. 3512 writes the
   security-release runbook and leaves this step to this ticket.
5. **One person holds everything.** `Zafnok` is the only owner of
   every crate name and the only admin of the repository. No GitHub
   account successor is set. On 2026-11-01 Nick parked 2016
   (governance) and said "No backup is named". 2016's own option B, a
   successor only, costs nothing and gives the successor no rights
   while Nick is alive (GitHub hands over only after it confirms the
   account holder has died). A second crates.io owner can publish,
   yank and change owners, so it is a real grant.
6. **The audit's tickets are all After MVP.** Several of them change
   the API, so done after the release they are breaking versions, which
   Nick wants avoided (2026-11-14, *no run of breaking versions after
   the MVP*). Others make the first publish work at all. 2842, 2843,
   3413, 3518 and 2847 already say their place is decided here.
   2099 (open) will make the ticket lint fail an MVP ticket that 2005
   doesn't wait for, so each yes moves both the milestone and 2005's
   `blocked_by`.

Questions about unparking work are in the sibling decisions: what 1.0
is for (3525), the editor (3526), platforms and devices to test on
(3527), and the legal stance and community files (3529). This ticket
asks only about the release's guards and about todo tickets.

## Nick input

**Decision:** six questions, asked with the `ask-nick` skill, one at a
time, in these words. Before asking, re-check each number in
*Context* on the day and correct it if it moved.

1. **Before the first release, should a fresh agent try to make a small
   game with Shuyi, given only the website's `llms.txt` and the MCP
   server, and have every name it guessed wrong, every failed tool call,
   its minutes and its lines written down?**
   - **Yes, once before the release.** It runs in a normal session in
     an empty folder outside this repository, on the box pusher. It
     costs about one session of your usual use; no API key, no money
     beyond that. Each miss becomes a ticket, and the release waits for
     the ones the session judges are breaks.
   - **Yes, and again before every release:** the same, as a line in
     the release checklist.
   - **No:** the eval stays after the release (2524, 2525).
   - The monthly money for scheduled runs stays 2525's own question.
   - Unblocks: a new ticket for the test, blocking 2005.
2. **Should an engine release wait until Visions of Shuyi, pinned to
   that exact commit, builds and passes its tests?**
   - **Yes, checked by a machine:** the game's watcher reports back to
     this repository, and the release can't be tagged without a green
     report. Needs a small change in each repository.
   - **Yes, checked by you:** the release's sign-off (2005) asks you
     to confirm that the game's latest pin move is at the release
     commit and green. Nothing to build.
   - **No:** as today; a break the game finds after a release needs a
     new release.
   - Unblocks: with a machine check, a new ticket here and one in the
     game; with your check, a line in 2005's sign-off.
3. **May the five crate names (`shuyi-embed`, `shuyi-embed-macros`,
   `shuyi-script`, `shuyi-sound`, `shuyi-sound-out`) be held now as
   empty 0.0.0 placeholders, as the nine were?**
   - **Yes, now:** the same token steps as on 2026-11-01, about five
     minutes of yours. The release can't go out without them.
   - **Later, at the release's sign-off.** Someone else could take a
     name before then.
   - Unblocks: 3510's setup.
4. **When a security hole is found in our own crates, may the warning
   be sent to RustSec, so every Rust project that checks for warnings
   hears of it?**
   - **Yes, as a pull request on RustSec's list**, the way RustSec
     asks for it. This would be the one exception to the rule of never
     opening a PR on another project, for our own warnings only.
   - **Yes, by email only.** No PR or issue anywhere; RustSec's people
     file it.
   - **No:** the warning goes only on this repository's GitHub page.
     Projects that check RustSec don't hear of it unless someone else
     files it.
   - Unblocks: 3512's RustSec step.
5. **Before the first release, should someone be able to act if you
   can't?**
   - **A successor only:** a person you trust becomes your GitHub
     account successor. It costs nothing and gives them nothing while
     you're alive. Two clicks.
   - **A successor and a second crates.io owner:** they can also
     publish, yank and change owners of the crates. Good if you trust
     them with that; it can't be limited.
   - **Nobody, as on 2026-11-01.**
   - Ask too whether a short list of the project's
     accounts and where each is set up (no passwords or keys) should be
     written now, and whether the person's GitHub name may appear in
     the repository. Unless Nick says yes, it says "a successor the
     owner has named".
   - Unblocks: Nick's clicks, and a small follow-up ticket for the list
     of accounts, taken out of 2016.
6. **Which of these should the first release wait for?** Each one
   added makes the release later. You want no real version until the
   engine is stable anyway (2026-10-21), so the delay costs little.
   Pick any, all or none:
   - **The publish itself** (the agent recommends yes for all three):
     3510, without which the first publish fails as planned; 3511, the
     oldest Rust a game may use is stated apart from ours; 3512, the
     security-release runbook. **3510 answered:** Nick put it in the
     MVP on 2026-11-25 (`docs/decisions.md`, *the publish list (3510)
     joins the MVP*); ask about 3511 and 3512 only.
   - **Changes that would break games if they came later** (the agent
     recommends yes): 3413, names agents guess find something, and one
     spelling; 3518, an outside module tries the extension API and how
     it is versioned is settled; 3520, the grid, dialogue, cutscene and
     bot kits can be left out of a game that doesn't use them.
   - **Guards against drift and bad files:** 2020, fuzzing the file readers (its first part
     only, if the session splits it); 2035, a moved test value always
     says so.
   - **What a newcomer sees first:** 2842, an example on every main
     type and every feature on docs.rs; 2843, the README rewritten as a
     front door with a picture, tested code and links, plus how-to
     pages and site search on the website; 2786, a cookbook of small
     examples.
   - **Measuring agents:** 2524, the graded eval tasks.
   - Unblocks: each pick becomes an MVP ticket that 2005 waits for.

## Scope

**In:**
- Ask the six questions and record each answer in Nick's words in
  `docs/decisions.md` (the `ask-nick` skill).
- **Question 1, a yes:** write the smoke-test ticket (`write-ticket`):
  the task, the folder outside the repository, what is logged, and
  that each miss becomes a ticket. Milestone MVP, in 2005's
  `blocked_by`. With "every release", the release checklist's line
  too (2005's sign-off).
- **Question 2:** with a machine check, an engine ticket (the release
  gate reads the game's report) and a game ticket for the watcher's
  report, linked both ways, the engine one in 2005's `blocked_by`;
  with Nick's own check, a line in 2005's *Nick input* sign-off list.
- **Question 3:** a yes is 3510's setup.
  - Answered: Nick held the five names on 2026-11-24
    (`docs/decisions.md`, *the last five engine crate names are held*),
    and `cargo xtask check-publish --index` runs in CI's `layers` job
    since 3510. Skip question 3.
- **Question 4:** with a PR, a narrow exception line under `AGENTS.md`
  rule 12 (our own crates' advisories on `rustsec/advisory-db` only),
  citing the decision; with email or no, rule 12 is unchanged.
  - **Either order with 3512:** if 3512 is done, Nick's answer on
    filing our own advisory with RustSec updates the RustSec step of
    `.claude/skills/security-release/SKILL.md` (which sends advisories
    to the GHSA only until this answer) and its ADR; then run
    `pwsh -File scripts/sync-agent-skills.ps1 -Check`; if not, nothing
    to do (3512 reads the answer from `docs/decisions.md`).
- **Question 5:** Nick's steps, as 2016's *Setup* steps 5 and 6 say:
  `https://github.com/settings/admin`, *Successor settings*; for a
  crates.io owner, `cargo owner --add <login> <crate>` for each held
  name, run by Nick. With the list of accounts, a follow-up ticket for
  `docs/continuity.md` (accounts and where each is configured, no
  values), and 2016's Scope narrowed to point at it.
- **Question 6:** for each ticket picked, set `milestone: MVP`, add it
  to 2005's `blocked_by`, and replace its "Nick decides in 3528" line
  with the answer. For each not picked, the line says it stays After
  MVP, with the date.
  - **Either order with 3518:** if 3518 is done, its completion notes
    list the friction tickets it filed, each saying whether it breaks
    the API, and the answer places each of them (2005's `blocked_by`,
    or After MVP); if not, nothing to do (3518 files them where the
    answer puts them).
  - **Either order with 3526:** if 3526 is done, leave out of the
    question the editor groundwork tickets its answer settled (2244,
    3417, 2515, 3418, 3416, 2316); if not, nothing to do (3526 asks
    about those six itself).

**Out (do not do):**
- The work of any ticket named here: each ticket does its own. This
  one only moves milestones and blockers, and writes the follow-ups.
- Changing any setting, account or crates.io owner. Nick does those
  himself; a session never sees a token.
- How much the scheduled agent runs may spend a month: 2525's own
  question.
- Unparking anything, devices to test on, and the community files:
  3525, 3526, 3527 and 3529.
- Opening a PR or an issue on `rustsec/advisory-db` or any project.
  Even a yes to question 4 only allows it in 3512's runbook, for a
  real advisory.

## Implementation steps

1. Re-check the numbers in *Context*: the template's line count, the
   five names on `https://index.crates.io/sh/uy/<name>`, 2524's
   blockers, and RustSec's guide on the day.
2. Ask the six questions with `ask-nick`, one at a time, in the words
   above, with the corrected numbers.
3. Record each answer verbatim in `docs/decisions.md`.
4. Apply each answer as the Scope says: the follow-up tickets
   (`write-ticket`, ids from `cargo xtask next-id`), the milestones and
   2005's `blocked_by`, the `AGENTS.md` line if any, and Nick's steps
   in the PR body.
5. Run `cargo xtask ticket-lint` and the `run-gates` skill.

## Acceptance criteria

- [ ] `docs/decisions.md` has the six answers in Nick's words.
- [ ] Each yes has its follow-up ticket, its edit, or Nick's steps in
      the PR body; each no is recorded with nothing else changed.
- [ ] Every ticket picked in question 6 has `milestone: MVP` and is in
      2005's `blocked_by`; every ticket not picked still says After
      MVP, and no open ticket still says "Nick decides in 3528".
- [ ] `AGENTS.md` rule 12 changed only if Nick allowed a PR to
      RustSec, and then only for our own crates' advisories.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None (a decision). `cargo xtask ticket-lint` passes over the edited
  and new tickets.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
