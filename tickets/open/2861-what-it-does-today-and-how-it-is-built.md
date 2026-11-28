---
id: "2861"
title: "One honest page: what Shuyi does today, what it doesn't yet, how it is built, where it came from, and how to contribute"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2859"]
nick_input: sign-off
completed:
---

# 2861 — One honest page: today, not yet, how it is built

## Context

Every launch post (2865) links one page that answers, before anyone asks,
the threads the research expects (2859, *Context*): "is this AI slop?",
"can it do a platformer?", "is this an ad for a paid game?", "open source
with closed PRs?". With the answer already linked, those threads stay
short. The page brings no traffic itself, but every channel leans on it.

The facts come from `README.md`, `docs/ROADMAP.md` and `docs/decisions.md`:
grid games today, turn-based and in real time (Snake, ADR-0148); free
movement, side-scrollers, 3D and networking planned or parked; the code
written by coding agents and reviewed by the gates (`AGENTS.md`, rule 6);
carved out of Visions of Shuyi, its first user; outside pull requests
closed (`docs/decisions.md`, 2026-11-01).

## Nick input

**Sign-off:** read the page once (about 10 minutes) and say yes, or name
the sentence that is wrong. No technical questions.

## Scope

**In:**
- A website page, `site/content/about.md` (title "What it is, honestly"
  or similar), linked from the home page and from `README.md` above its
  first `##` heading, with five sections:
  - **Today:** turn-based and real-time grid games (Hello Grid and Snake
    playing in the browser); run, step, frame to PNG, test and replay with
    no window, through the CLI, the MCP server and the Agent Skills;
    deterministic; everything is text; graphics are a skin; every key
    rebinds.
  - **Not yet:** free movement, side-scrollers, 3D, networking, more
    targets, each with its `docs/ROADMAP.md` status (planned or parked).
  - **How it is built:** coding agents write almost all the code; Nick
    decides the product and doesn't review code line by line; the gates
    are the review, each named (tests, clippy pedantic, coverage,
    SonarCloud, CodeQL, zizmor, the public API and semver checks, mutation
    testing in CI), with a link to a green CI run and to the ADRs.
  - **Where it came from:** carved out of a commercial, closed tactics
    game that is its first user; the engine is MIT OR Apache-2.0, no paid
    tier, no telemetry. No store link.
  - **Contributions:** issues are open and read; outside pull requests are
    closed for now, because one unpaid person can't review them; forking
    is free; link the decision.
- Words to keep out of the page: "revolutionary", "the first",
  "production-ready", "battle-tested", "community-driven", "contributions
  welcome", "faster than", and any engine compared by name.

**Out (do not do):**
- Any comparison with another engine and any benchmark (3432, 2562 are
  parked).
- Naming Playdate, GBA, DS, consoles or online play as available.
- The FAQ line on the held crate names: 2862 adds it to this page.
- The launch fact sheet and checklist (2865).

## Implementation steps

1. Read `README.md`, `docs/ROADMAP.md` and `docs/decisions.md` for the
   facts; keep each claim to what they say.
2. Write the page (the `document` skill; the website row of the docs map).
3. Link it from the home page and from `README.md`'s first screen.
4. Open each browser example and the playground in a fresh browser
   profile, with no login, and note the result in the PR.
5. Ask Nick to read it.

## Acceptance criteria

- [ ] The page builds on the site with exactly the five section headings above.
- [ ] `README.md` links the page above its first `##` heading, and the home page links it.
- [ ] The page names every gate listed under *How it is built* and links one CI run.
- [ ] A grep of the page finds none of the words to keep out.
- [ ] *Not yet* lists free movement, side-scrollers, 3D and networking, each with a ROADMAP status.
- [ ] Nick's yes is quoted in the PR.
- [ ] The completion notes have `Book:` and `Skills:` lines saying why neither changes.
- [ ] All gates in the `run-gates` skill pass (the site's and the docs map's included).

## Tests required

- The site's build and link check (`site/check-build.sh`) cover the page.

## Completion notes

*(Filled in by the session that completes the ticket.)*
