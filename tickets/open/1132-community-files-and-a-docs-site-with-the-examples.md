---
id: "1132"
title: "The engine's community files and contribution policy"
type: infra
milestone: "Engine: public"
model: implementation
effort: medium
status: todo
blocked_by: ["game#1131", "2016"]
nick_input: decision
completed:
---

# 1132 — Community files and contribution policy

## Context

0059's fourth question, who may send code to the engine, is still open
(`docs/decisions.md`, *Still open*). The recommendation is issues only
until the first public release, then pull requests under a Developer
Certificate of Origin. This ticket asks Nick, unless the game's 0059
has settled it by then.

The research (the game repository's [feasibility report](https://github.com/Zafnok/visions-of-shuyi/blob/main/docs/engine/feasibility.md)) found two things:

- Engines that welcome AI-written code attract low-quality pull
  requests. Godot's foundation banned them from its own repository in
  2026.
- Open engines die when nobody answers (Moai, comfy, Amethyst).

ADR-0005 §4 says pull requests are judged by the gates and a clear
description, however they were written.

This ticket used to hold the docs site too. Nick asked on 2026-10-20 for
a website like Bevy's (`docs/decisions.md`), so the site is 2801–2806
now, and this ticket keeps the community files. 2801 (done) gave the
website a Community page, `site/content/community.md`, which links the
repository, its issues and `CONTRIBUTING.md`; this ticket adds the rest
there.

2016 comes first: it turns on private vulnerability reporting, the
channel `SECURITY.md` names, and records the response time Nick chose.

## Nick input

**Decision**, with the `ask-nick` skill: which community channel the
engine opens at its first public release. The choices:

- none yet (issues only);
- GitHub Discussions in this repository (free, no new account; Nick
  turns it on in the repository's settings);
- a Discord server (a time commitment: someone must answer, and the
  server belongs to Nick's account).

Recommended: GitHub Discussions. Record it in `docs/decisions.md`.

**Decision**, in the same `ask-nick` session, unless the game's 0059
has settled it by then: who may send code to the engine. Recommended:
issues only until the first public release, then pull requests under a
Developer Certificate of Origin. Record it in `docs/decisions.md` and
take it out of *Still open*.

## Scope

**In (in the engine repository):**
- `CONTRIBUTING.md` holds Nick's contribution policy (above): what is
  accepted, the DCO sign-off if chosen, and that the gates are the
  review. It covers how AI-assisted work is judged and that issues come
  first for large changes.
- `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1), and issue and PR
  templates.
- `SECURITY.md`, with:
  - a *Supported versions* line: before the first release, `main` only;
    after it, the latest 0.x release only, with no backports or LTS
    (2005's version policy; Bevy too supports only its latest minor
    version);
  - how to report: GitHub's private vulnerability reporting (the
    Security tab's "Report a vulnerability", which 2016's setup turns
    on), never a public issue;
  - the response time Nick chose in 2016 (`docs/decisions.md`);
  - a link to the threat model for untrusted files (`docs/formats.md`,
    *Untrusted files*, from 2020). It says that, for a file the engine
    reads, a panic, memory or time without bound, or two different
    results from the same bytes is a security bug to report privately.
    **Either order with 2020:** if 2020 isn't done, `SECURITY.md` leaves
    this out, and 2020 adds the link when it lands.
- `CONTRIBUTING.md` and `SECURITY.md` link `GOVERNANCE.md` (2016), and
  `GOVERNANCE.md`'s line on security reports gets a link to
  `SECURITY.md`.
- The channel Nick picks, linked from `CONTRIBUTING.md` and from the
  website's Community page.
- `AGENTS.md` is at the root already (2001). It gets a line pointing
  outside contributors, and their agents, to `CONTRIBUTING.md`.
- **Either order with 2352** (parked: the engine's screens
  machine-translated): if 2352 is done, `CONTRIBUTING.md` says whether
  people may review and correct the engine's machine-translated
  language packs, from Nick's answer in `docs/decisions.md`; if not,
  nothing to do (2352 asks Nick and adds that line).

**Out (do not do):**
- a forum or Discord, unless Nick picks one in the decision above.
  Creating the server or turning Discussions on is Nick's (his account,
  the repository's settings);
- trademark registration (Nick's, outside the repository);
- the website (2801–2806);
- `GOVERNANCE.md`, the list of accounts and secrets
  (`docs/continuity.md`) and the repository's settings (2016).

## Implementation steps

1. Ask Nick which community channel and, unless the game's 0059 has
   settled it, the contribution policy (`ask-nick`), and record his
   answers in `docs/decisions.md`.
2. Write the files.
3. Link the code of conduct, the security policy and the channel from
   the website's Community page (`site/content/community.md`).

## Acceptance criteria

- [ ] `docs/decisions.md` has Nick's answer on the community channel.
- [ ] `docs/decisions.md` has Nick's contribution policy, and *Still
      open* no longer lists it.
- [ ] `SECURITY.md` has the supported versions, the private channel and
      the response time from 2016, and the link to *Untrusted files* if
      2020 is done.
- [ ] The repository shows the community files in GitHub's community
      profile.
- [ ] All gates pass.

## Tests required

- The link check (`cargo xtask check-links`) over the new files.

## Completion notes
