---
id: "1132"
title: "The engine's community files, contribution policy, and a docs site with the examples playable in the browser"
type: infra
milestone: "Engine: public"
model: sonnet-5
effort: medium
status: todo
blocked_by: ["game#1131"]
nick_input: none
completed:
---

# 1132 — Community files, contribution policy, docs site

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
- A docs site on Pages built from `docs/` with a static site
  generator under an allowed license. It has a getting-started page that
  uses `shuyi new`, a page per style linking its playable example, a
  page per crate, and the skills and MCP setup once 1133 and 1134 are
  done.
- `AGENTS.md` at the repository root.

**Out (do not do):**
- a forum or Discord (ask Nick first; that is a time commitment for him);
- trademark registration (Nick's, outside the repository).

## Implementation steps

1. Write the files.
2. Build the docs site and Pages workflow, and link the examples.

## Acceptance criteria

- [ ] The docs site is live, and each example plays from it.
- [ ] The repository shows the community files in GitHub's community
      profile.
- [ ] All gates pass.

## Tests required

- Pages build in CI.

## Completion notes
