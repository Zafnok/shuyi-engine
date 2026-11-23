---
id: "3527"
title: "Nick decides platform and service scope: signed packages, Steam services, phones' browsers, Epic Online Services, web requests, analytics, cloud saves and the devices to test on"
type: decision
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 3527 — Nick decides platform and service scope

## Context

A blind-spot audit of the engine (2026-11-23) found eight places where
the next step is a product choice, not a technical one. Each one is
parked or waits on money, an account or a store, so only Nick can
decide it (`AGENTS.md`, *Who decides what*). Several of them were
already listed on 2026-10-20 as "decisions left for later"
(`docs/decisions.md`: "paid code signing", "phones' browsers as a
target", "stores beyond Steam").

What each question rests on (read on 2026-11-23):

1. **Signed packages (3051, parked).** An exported game is signed ad
   hoc only (3004). 3051 records what players see without a real
   signature, checked 2026-10-20: macOS 15 removed the Control-click
   way to open an app that isn't notarized, and Windows SmartScreen
   blocks an unsigned download hard. The engine never buys anything for
   a game: 3051 plans for each developer to bring their own Apple and
   Windows signing accounts, kept as CI secrets. Visions of Shuyi sells
   through Steam (game ticket 0903), so it doesn't need this.
2. **Steam services (3036, 3037, parked).** Nick asked on 2026-10-20
   that Steam achievements and cloud saves have tickets, and they are
   parked until after the MVP. Meanwhile 3321 (a release publishes to
   Steam) and 3012 (the Steam Deck target) are After MVP and not
   parked. `targets/steam-deck.ron` says `status: Supported`. 3012
   leaves the Deck's on-screen keyboard to the game's own Steam code
   (game ticket 0903, open at the game's pin
   `50472bbcd3084555a61728a5d07518f720bd6514`; `gh search code --repo
   Zafnok/visions-of-shuyi 'steamworks'` finds 0903, 1024 and game
   ADR-0013). So the first Steam release from a new game would ship
   with no achievements, no Steam Cloud and no Deck keyboard from the
   engine.
3. **Phones' browsers (2407, parked).** The engine reads the mouse and
   touches now (2405 and 2413, done), so 2407's only blocker is done.
   2407 adds on-screen controls and a web touch target. It costs no
   store fee and no account. Native Android and iOS (3015) stay parked
   either way. 3507 (more desktop browsers in CI) leaves phones out
   until this is answered.
4. **Epic Online Services (2920, parked).** ADR-0006 §5 allows no
   outside company's online kit except Steam's, behind its own feature.
   So a game not on Steam has no free way to play online: anything else
   needs a relay server someone pays for. Epic's kit is free to use
   (2920 checks the terms on the day). 2920 is the research; it waits
   on this answer.
5. **Web requests (2601, parked).** A game can't call a web address at
   all today, and a web build never can without the engine (2601's
   context). 2601 has no blocker of its own: it is parked only because
   networking is (2026-10-14). Sending crash reports (3047), online
   high scores (3054) and remote data (2608) all need it.
6. **A game's own analytics (2608, parked).** 2608 plans two parts:
   remote data (a game changes a number or shows news without a patch),
   and, if Nick agrees, a channel a game may use to send its own play
   events, only after the player says yes. The engine itself would
   never send anything. 2608's events half waits on this answer.
7. **Cloud saves beyond Steam (2607, parked).** Cloud saves exist only
   through Steam (3037). 2607 plans one layer every store uses, with a
   screen that asks the player which copy to keep when two machines
   disagree, and asks the browser to keep a web game's saves. 2607
   waits on this answer.
8. **Devices to test on.** Tier 1 is Windows, Linux, macOS and the web
   (decision of 2026-11-08). Only Nick's Windows 11 machine is at hand.
   `targets/playdate.ron` and `steam-deck.ron` say `Supported`, but
   neither game nor engine has run on a Playdate or a Deck. 3011
   (Playdate) and 3013 (Deck) need the device (`nick_input: setup`).
   3340 measures frame times on Windows and leaves other machines to
   this answer. 3338 labels what has run where, and leaves buying to
   this ticket.

Other questions from the same audit are in 3525 (what 1.0 is for),
3526 (the editor), 3528 (the release's guards) and 3529 (legal stance).

## Nick input

**Decision:** eight questions, asked one at a time with the `ask-nick`
skill, in plain words as below. No question is technical. Each says what
it costs and what each answer starts.

1. **Signed packages.** Should the engine support real code signing now,
   with each developer bringing their own Apple and Windows accounts?
   Without it, a player who downloads a Shuyi game from itch.io or a
   website sees a warning that looks like malware, and on a Mac has to
   dig in System Settings to open it. The engine pays nothing; a game
   that signs pays Apple 99 USD a year and about 10 USD a month or more
   for Windows (3051's figures, re-checked when asked).
   - **Yes, now (recommended):** 3051 is unparked. Then ask 3051's own
     three questions in the same session (paid signing now; who checks
     the real path once; how the engine's own example packages are
     signed).
   - **Later:** 3051 stays parked until a game sells a direct download.
2. **Steam services.** Should Steam achievements, Steam Cloud saves and
   the Steam Deck's on-screen keyboard come from the engine, ready for
   the first Steam release? That means unparking them now. The game's
   Steam account, app id and store page stay the game's, as decided on
   2026-10-15.
   - **Yes (recommended):** 3036 (achievements as data, any store) and
     3037 (Steam under it) are unparked. Ask 3036's own question (which
     stores beyond Steam to plan for) in the same session. The Deck
     keyboard goes into 3037's Steam module instead of the game's own
     code.
   - **No:** they stay parked; a Steam release has none of them unless
     the game writes its own.
3. **Phones' browsers.** Should a web build be playable on a phone's
   browser, with on-screen controls, before and apart from real
   Android and iOS apps? It costs no store fee or account. New games
   from the templates would get on-screen controls on a phone; Visions
   of Shuyi only if it asks.
   - **Yes (recommended):** 2407 is unparked, as 2407's own *Nick
     input* says.
   - **No:** 2407's plan moves into 3015 (parked mobile apps) and 2407
     closes.
4. **Epic Online Services.** May the engine use Epic's free online
   service, behind an optional switch a game turns on, the way Steam's
   is? It would be the free way for a game not on Steam to play online.
   It brings Epic's terms, which a game that turns it on accepts.
   - **Yes:** 2920 (the research) is unparked. It reports Epic's terms
     and duties before any build ticket.
   - **No:** 2920 drops its Epic half; its console half waits with the
     rest of networking.
5. **Web requests.** Should games be able to call a web address (to
   send a crash report, post a high score, fetch news) ahead of the
   rest of networking? The engine never sends anything by itself; a
   game that sends something about a player takes on the privacy duties
   for it.
   - **Yes (recommended):** 2601 and the two backend tickets its step 2
     writes are unparked.
   - **No:** 2601 stays parked with networking.
6. **A game's own analytics.** Should the engine offer games a way to
   send their own play events (where players stop, how long they play),
   off unless the game sets an address and the player says yes? The
   engine itself would never use it.
   - **Yes:** 2608 keeps its events half. A yes lets Nick unpark it:
     a game's own events, sent to the game's own address after 2606's
     consent.
   - **No:** 2608's events half is dropped, as 2608's *Nick input*
     says, and 2608 becomes remote data only.
   - Either answer: 2608 is unparked only if question 5 is yes (it
     sends through 2601), and only when Nick says so.
7. **Cloud saves beyond Steam.** Should the engine have one cloud-save
   layer that every store, and later a game's own server, plugs into,
   with a screen that asks the player which save to keep when two
   machines disagree?
   - **Yes:** 2607 is unparked.
   - **No, Steam only:** 2607 closes; 3037's Auto-Cloud is the only
     cloud save.
8. **Devices to test on.** Before a release, which real devices should
   the project buy or borrow to check the engine on: a Mac, a Linux
   laptop, a low-end Windows laptop, a Steam Deck, a Playdate, an
   Android phone, an iPhone? Each costs money or a loan. A device
   nobody has means its target is not called supported (3338).
   - Nick names the devices, and who pays for or lends each. None is
     a valid answer.
   - **Either order with 3340:** if 3340 is done, the answer writes a
     follow-up ticket (After MVP, `33xx`) that runs 3340's `bench
     --wall` steps on each device it names; if not, nothing to do
     (3340 writes that ticket from the answer).

2606 (one consent record shared by every online feature) waits on
questions 5 to 7: it is unparked with the first yes among them. Its
other blockers stay in its `blocked_by`.

## Scope

**In:**
- Ask the eight questions with `ask-nick` and record each answer in
  Nick's words in `docs/decisions.md`, one entry for the session.
- For each answer, edit the tickets it names, as its option says:
  - an unparked ticket gets `status: todo` (or `blocked` if another
    blocker is still open), loses `Parked:` from its title and its
    Parked note, and loses `3527` from `blocked_by` (find them with
    `grep -rln 3527 tickets/open`; 2920, 2606, 2607 and 2608 hold it
    on 2026-11-23);
  - a ticket that stays parked keeps its note, rewritten to name the
    decision of this date instead of 3527, and loses `3527` from
    `blocked_by`;
  - a ticket that closes moves to `tickets/done/` with a completion
    note naming the decision.
- The follow-ups an answer asks for (3051's, 3036's and 2407's own
  questions, asked in the same session when their answer is yes).
- For question 8, a device ticket (After MVP, `33xx`, `nick_input:
  setup`, the `write-ticket` skill) for each device Nick names that no
  ticket covers yet (3011 and 3013 cover the Playdate and the Deck).
- 3340's follow-up, as question 8's either-order clause says (3340's
  step 9 writes it instead if 3340 is not done yet).
- `docs/ROADMAP.md`: each unparked ticket moves out of the parked list.

**Out (do not do):**
- Building anything an answer starts: each ticket is worked on its own.
- Buying a device, a certificate or an account. Nick does that, or a
  game does.
- Whether real-time online play is in 1.0 (3525), the agent eval's
  budget and the release's guards (3528), and legal questions such as
  the community files or a policy on agent-written code (3529).
- Technical questions under these answers (whether Steam's library
  links with the GNU toolchain is 3037's and 1144's; outside signing
  tools and their licenses are settled in 3051's Out).

## Implementation steps

1. Re-read each ticket named under its question (3051, 3036, 3037,
   3012, 3321, 2407, 3015, 2920, 2601, 2608, 2607, 2606, 3011, 3013,
   3338, 3340) and refresh any cost or fact the question quotes that
   is dated (3051's prices, Epic's terms), with the date checked.
2. Ask the eight questions with `ask-nick`, one at a time, in the words
   above. Offer the recommended option first where one is marked.
3. Record the answers in `docs/decisions.md`.
4. Apply each answer to its tickets as the Scope says, and ask the
   follow-up questions of 3051, 3036 and 2407 if their answer is yes.
5. Write the device tickets of question 8, and 3340's follow-up if 3340
   is done.
6. Update `docs/ROADMAP.md`, and run `cargo xtask ticket-lint`.

## Acceptance criteria

- [ ] `docs/decisions.md` has all eight answers in Nick's words.
- [ ] No open ticket has `3527` in `blocked_by` or its Parked note.
- [ ] Each ticket a "yes" unparks has `status` `todo` or `blocked` (by
      a real open blocker) and no `Parked:` prefix; each "no" leaves
      its ticket as its option says.
- [ ] Each device Nick names in question 8 has a ticket, and 3340's
      follow-up exists if 3340 is done.
- [ ] `docs/ROADMAP.md` matches the answers.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None (a decision). `cargo xtask ticket-lint` passes over the edited
  tickets.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
