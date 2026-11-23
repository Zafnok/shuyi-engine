---
id: "2920"
title: "Parked: Research: Epic Online Services behind an optional eos feature, and the online rules consoles impose on sessions and accounts"
type: research
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3527"]
nick_input: none
completed:
---

# 2920 — Research: Epic Online Services and console online rules

> **Parked**: this waits on Nick's answer to decision 3527 (whether
> Epic Online Services may sit behind an optional feature, like Steam).
> Nobody starts it until Nick unparks it. If he says no to EOS, the EOS
> half is dropped and the console half waits with the rest of
> networking (1140–1150). Whoever unparks it sets `status: todo` and
> drops `Parked:` from the title.

## Context

A blind-spot audit of the engine (2026-11-23) found two gaps in the
networking plan. Both are about services the engine doesn't host
itself.

**No free online path off Steam.** ADR-0006 §5 says "No proprietary
SDK except Steam's behind its feature"
([`docs/adr/0006-networking-in-layers-transports-sessions-sync-models-services.md`](../../docs/adr/0006-networking-in-layers-transports-sessions-sync-models-services.md)).
So the only free internet path is Steam's (1144). Anything else needs a
relay server someone pays for. Epic Online Services (EOS) has a free
SDK. It is said to offer peer-to-peer with NAT traversal and relays,
lobbies, sessions, achievements, leaderboards, player storage,
cross-platform identity and anti-cheat, on desktop, consoles and phones
(check all of it on the day). Nothing in the repository evaluates it:

- 3323 (a release publishes to the Epic Games Store) puts "Epic Online
  Services in the game" under **Out**;
- 3054 (leaderboards) and 3055 (presence) name EOS only as a later
  backend; 3036 (`StoreServices`) names the Epic Games Store the same
  way;
- 2910 (voice chat) names EOS Voice as an opt-in a game brings.

**Console online rules are not modelled.** Consoles are tier 2
(`docs/decisions.md`, the tiers). Their certification asks a game to
check whether a user may play online, to honour the platform's block
list and its chat and user-content limits, and to take the platform's
invites. A search of `tickets/open` for `privilege`, `block list` and
`parental` finds nothing. 3016 covers console certification only "in
broad terms". Today's shapes have no place for these rules:

- 1145's `SessionBackend` has `join(JoinToken)` and
  `SessionEvent::JoinRequested(JoinToken)`, but no check before
  `create` or `join`;
- 1148's accounts are "A device id, or a Steam auth ticket"; there is
  no other platform account id;
- 1149's chat has a client-side block per account, not the platform's.

Adding these hooks after 1145 and 1148 ship would break their API. Every
backend could answer "allowed" for now, but the shape must be known
first.

**The GNU toolchain.** Nick's machine builds with the GNU toolchain and
never MSVC (`AGENTS.md`, *Environment*). The EOS SDK ships native
libraries. Whether a GNU build links them, or loads them at run time,
is unknown. 1144 and 3037 have the same question for Steam's
`steam_api64`; this ticket answers it for EOS.

This ticket is research: a report, an ADR and parked tickets for the
build. It promises no crate.

## Nick input

`None.` Decision 3527 asks the product question first. If the report
finds a duty that 3527 didn't cover (a fee, a sign-up per game, a data
or age duty, terms a game must accept), ask Nick again with the
`ask-nick` skill before any build ticket is written, and record his
answer in `docs/decisions.md`.

## Scope

**In:**
- **EOS, from its public docs and SDK terms**, dated: what it offers
  (P2P and relays, lobbies, sessions, identity, achievements, stats,
  leaderboards, player storage, presence, anti-cheat, voice); which
  targets it supports; what it costs and what a game must do to use it
  (an account per game, a product id, terms the game accepts, data it
  sends to Epic). Whether its SDK license lets the engine ship code
  that loads it behind an off-by-default `eos` feature, with the game
  bringing the SDK itself, as the Steam feature does (ADR-0002 §8,
  `AGENTS.md` rule 7). Check each Rust binding found with `cargo deny
  check licenses` in a scratch crate.
- **The build**: whether a Windows GNU build links the SDK, or loads it
  at run time, and what Linux, macOS and the web need. Try it in a
  scratch crate never committed. If GNU fails, say what CI's MSVC
  runners would do instead; never require MSVC locally.
- **The mapping**: how each EOS service fits the engine's planned
  traits, piece by piece:
  - P2P and relays onto 1140's `Transport`: EOS P2P is weighed as a
    backend behind `shuyi-net`'s `Transport`, beside 2605's
    `RelayTransport` (our own self-hosted UDP path with NAT traversal).
    2605 leaves EOS P2P to this ticket;
  - lobbies and sessions onto 1145's `SessionBackend` and `JoinToken`;
  - identity onto 1148's accounts, as a platform account id beside the
    device id and the Steam ticket;
  - achievements, stats and leaderboards onto 3036's `StoreServices`.
- **A generic console online model**, from public sources only, never
  from anything under an agreement (3016's rule):
  - a platform account identity a session and an account carry;
  - an online check before a session is created or joined, with three
    answers (allowed, denied, restricted) and what a restricted user
    may still do;
  - platform invites as a `JoinToken`;
  - the platform's block list, and how it joins the client-side block
    1149 plans;
  - limits on chat and on content other players make.
  Say which layer each belongs in: the online check is a fact the
  platform hands in (`AGENTS.md` rule 5), so the rules never read it.
- **Either order with 1145 and 1148:** if they are done, map onto what
  they built and write a parked ticket for each hook they lack; if not,
  add the hooks to their scope (the online check before `create` and
  `join`, the platform account id), each answered "allowed" by every
  backend they build.
- **Either order with 3036:** if it is done, the EOS backend implements
  the `StoreServices` it built; if not, name what an EOS backend needs
  from `StoreServices` and add a line to 3036's scope.
- **Either order with 2605:** if it is done, compare EOS's P2P and
  relays with its UDP path and say whether EOS is a backend of it; if
  not, write the comparison as a section 2605 can link.
- **Either order with 2606:** if it is done, the online check and any
  age rule feed its consent record; if not, name what the console
  model needs from it and add a line to 2606's scope.
- **Outputs:**
  - the report, `docs/research/2920-eos-and-console-online.md`, with
    the facts dated and sourced;
  - an ADR (the `write-adr` skill): whether an `eos` feature amends
    ADR-0006 §5, and the shape of the console online model;
  - the build tickets (the `write-ticket` skill), each one PR, each
    `Parked:` with milestone "Engine: networking": the hooks in the
    session and account traits; the `eos` backend if Nick said yes to
    3527; each named in the report.
- **Size:** if the PR passes about 600 changed lines with the report
  and the ADR, the build tickets become a follow-up ticket blocked by
  this one; the PR says so.

**Out (do not do):**
- building any of it: the tickets it writes do;
- publishing to the Epic Games Store (3323) and to GOG (3322);
- GOG Galaxy's services: name them in one line if they fit the same
  mapping, nothing more;
- the console programs themselves, how to apply, their costs and
  toolchains (3016, 3014); applying, signing, or anything under an
  agreement;
- Steam's own transport, lobbies and services (1144, 3037);
- voice chat (2910);
- anything in Visions of Shuyi.

## Implementation steps

1. Read ADR-0002 §8, ADR-0006 and ADR-0008, and 1140, 1144, 1145,
   1148, 1149, 2605, 2606, 3016, 3036, 3037 and 3323 as they stand.
   Note which are done, for the either-order rules above.
2. Read EOS's public docs and SDK license on the day. Write the offer,
   the costs, the duties and the license into the report.
3. In a scratch crate, try the GNU link (or a run-time load) of the
   SDK on Windows, and check any Rust binding's license with `cargo
   deny`. Record what worked.
4. Write the mapping onto `Transport`, `SessionBackend`, the accounts
   and `StoreServices`.
5. From public sources, write the console online model and where each
   piece lives.
6. Write the recommendation, ask Nick again if step 2 found a new duty,
   write the ADR and the parked build tickets.
7. Run the gates.

## Acceptance criteria

- [ ] `docs/research/2920-eos-and-console-online.md` exists, with a
      dated table of every EOS service (what it does, targets, cost,
      the trait it maps onto) and its sources.
- [ ] The report says whether the EOS SDK's terms allow an
      off-by-default `eos` feature under ADR-0002 §8, and cites them.
- [ ] The report records the Windows GNU link (or run-time load)
      result, and what CI would do if it failed.
- [ ] The console online model names, for each piece (identity, the
      online check, invites, the block list, chat and content limits),
      the trait or platform fact it lives in.
- [ ] Every console fact cites a public source; nothing comes from an
      agreement.
- [ ] The ADR is in `docs/adr/`, linked from the report.
- [ ] The parked build tickets are in `tickets/open/`, or the
      completion notes say why none were written.
- [ ] The either-order clauses above are settled in the PR.
- [ ] `cargo xtask ticket-lint` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None shipped (research). The tickets it writes say their tests.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
