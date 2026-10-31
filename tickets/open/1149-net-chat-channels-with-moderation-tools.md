---
id: "1149"
title: "Parked: Networking, step 10: chat channels for lobbies, matches and friends, with filter, block, mute and report"
type: feature
milestone: "Engine: networking"
model: reasoning
effort: high
status: blocked
blocked_by: ["1148", "2346"]
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
- **Word filter:** `rustrict` (MIT/Apache) by default, which 1148
  already added for display names: reuse it. On Steam, Steam's
  text filter, which follows each player's own settings.
  **Either order with 1144 and 3037:** if either is done, with the
  `steam` feature on and Steam running, the filter is Steam's, asked
  through its `steam` module's handle. If neither is, `rustrict` alone,
  the completion notes say so, and 1144 adds Steam's filter (its clause
  on this ticket).
- **Block and mute** on the client, with block saved per account.
- **Report:** a message plus context goes to a moderation queue in
  `shuyi-services`. A small moderation command lets someone list, act on
  and close reports.
- A text-input widget that respects `check-text`, built on the name
  box's typing (2346: the letters of any language the font draws, the
  IME on only while the box is open through `Ctx::takes_text`).
  - The IME's composing text comes from 3034's tickets; miniquad 0.4.11
    has none.
  - Paste from the clipboard, on an action the player binds, never a key
    (the `input-actions` skill). miniquad 0.4.11 has
    `window::clipboard_get` and `clipboard_set` on desktop. On the web,
    its loader (`crates/cli/web/mq_js_bundle.js`) keeps the text of the
    browser's last paste event for `clipboard_get`, and writes
    `clipboard_set`'s text on the browser's next copy or cut. So no JS
    plugin of our own is needed; the PR says how paste behaves there.
    3034's replacement may change this.
- A chat panel in `shuyi-ui` (logic, view and skin) that games can place.
- **Either order with 2367** (overlays that let the screen below run):
  if 2367 is done and the panel opens as a screen over a running match,
  its `passes` is `Passes::Undeclared` and it declares only the chat
  actions, so the match keeps running and keeps its own actions; if 2367
  isn't done, the panel is a widget the match screen draws and feeds
  itself, and no chat screen is pushed. Either way, while the chat box
  takes typed text, the match gets none of its actions, so a letter
  typed in chat doesn't also fire the match's action on that key: the
  chat screen then declares nothing (`None` takes every action, and only
  time goes down), or the match screen stops reading its actions.
- **Either order with 2910** (voice chat research): if 2910 is done,
  the "what running chat means" page (step 4) takes the voice section
  of `docs/research/2910-voice-chat.md`, or links it; if not, nothing
  to do (2910 links this page when it lands).
- Tests:
  - filtering;
  - limits;
  - block;
  - a report reaches the queue;
  - history expiry.

**Out (do not do):**
- global chat;
- voice chat (2910 researches it);
- anything in our game.

## Implementation steps

1. Write the channels and the limits. Read 2367's ADR if 2367 is done.
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
