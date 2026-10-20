---
id: "1132"
title: "The engine's community files and contribution policy"
type: infra
milestone: "Engine: public"
model: sonnet-5
effort: medium
status: todo
blocked_by: ["game#1131"]
nick_input: none
completed:
---

# 1132 — Community files and contribution policy

## Context

0059's fourth question decided who may send code to the engine. The
recommendation was issues only at first, then pull requests under a
Developer Certificate of Origin.

The research (the game repository's [feasibility report](https://github.com/Zafnok/visions-of-shuyi/blob/main/docs/engine/feasibility.md)) found two things:

- Engines that welcome AI-written code attract low-quality pull
  requests. Godot's foundation banned them from its own repository in
  2026.
- Open engines die when nobody answers (Moai, comfy, Amethyst).

ADR-0005 §4 says pull requests are judged by the gates and a clear
description, however they were written.

This ticket used to hold the docs site too. Nick asked on 2026-10-20 for
a website like Bevy's (`docs/decisions.md`), so the site is 2801–2806
now, and this ticket keeps the community files. 2801 links them from its
Community page.

## Nick input

None. The policy comes from 0059.

## Scope

**In (in the engine repository):**
- `CONTRIBUTING.md` holds the policy from 0059: what is accepted, the
  DCO sign-off if chosen, and that the gates are the review. It covers
  how AI-assisted work is judged and that issues come first for large
  changes.
- `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1), `SECURITY.md`, and
  issue and PR templates.
- `AGENTS.md` at the repository root.

**Out (do not do):**
- a forum or Discord (ask Nick first; that is a time commitment for him);
- trademark registration (Nick's, outside the repository);
- the website (2801–2806).

## Implementation steps

1. Write the files.
2. If 2801 is done, link them from its Community page.

## Acceptance criteria

- [ ] The repository shows the community files in GitHub's community
      profile.
- [ ] All gates pass.

## Tests required

- The link check (`cargo xtask check-links`) over the new files.

## Completion notes
