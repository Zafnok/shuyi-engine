---
id: "1149"
title: "Parked: Networking, step 10: chat channels for lobbies, matches and friends, with filter, block, mute and report"
type: feature
milestone: "Engine: networking"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["1148"]
nick_input: none
completed:
---

# 1149 — Chat channels with moderation tools

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0053 picks an online mode. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0006 §8. The engine gives chat as channels over sessions and
accounts: lobby, match and friends. It also gives the tools a game needs
to keep chat safe. It does **not** turn on global chat. Talking with
strangers brings legal duties to whoever runs the game:

- the EU DSA's notice-and-action;
- the UK Online Safety Act;
- COPPA for mixed audiences;
- GDPR for chat logs;
- ESRB's "Users Interact" notice.

1150 studies global chat. For our game, global chat is Nick's decision.

## Nick input

None to build it. Turning chat on in our game comes with 0053.

## Scope

**In:**
- `ChatChannel` (lobby, match, direct or friends) over the session
  (peers) or the services (accounts).
  - Messages have a size limit and a rate limit.
  - History is kept only as long as the game configures.
- **Word filter:** `rustrict` (MIT/Apache) by default. On Steam, Steam's
  text filter, which follows each player's own settings.
- **Block and mute** on the client, with block saved per account.
- **Report:** a message plus context goes to a moderation queue in
  `shuyi-services`. A small moderation command lets someone list, act on
  and close reports.
- A text-input widget that respects `check-text`, and an IME note for
  later languages.
- A chat panel in `shuyi-ui` (logic, view and skin) that games can place.
- Tests:
  - filtering;
  - limits;
  - block;
  - a report reaches the queue;
  - history expiry.

**Out (do not do):**
- global chat;
- voice chat;
- anything in our game.

## Implementation steps

1. Write the channels and the limits.
2. Write the filter, block and report.
3. Build the moderation command and the panel.
4. Write the tests and the docs: "what running chat means", with the
   legal list above and links.

## Acceptance criteria

- [ ] The tests pass.
- [ ] `cargo deny check` passes.
- [ ] The docs exist.
- [ ] All gates pass.

## Tests required

- Unit and integration, as above.

## Completion notes
