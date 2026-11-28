---
id: "2864"
title: "An itch.io tool page kit: the page's text, tags and AI disclosure, screenshots, the examples' web zips, and Nick's upload steps"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2859", "2861", "2863"]
nick_input: setup
completed:
---

# 2864 — An itch.io tool page kit

## Context

itch.io is where many game makers already have an account, and its Tools
section lists open-source engines (<https://itch.io/tools>). A new page
gets little browse traffic, but it is the page a jam entry (2868) and the
devlogs link to, and its examples play in the browser there. itch.io's
quality guidelines (<https://itch.io/docs/creators/quality-guidelines>)
ask for accurate tags, an honest AI disclosure, and self-announcements on
the Release Announcements board only.

The page text is a listing, not a community post, so an agent may draft
it and Nick reads it (2859, question 3, is about posts). Only if 2859 says
yes to an itch page.

## Nick input

**Setup** (about 30 minutes):

1. Sign in to itch.io, or create an account (yours to do).
2. Dashboard → Create new project. Kind of project: Tool.
3. Paste the title, short description and description from
   `docs/launch/itch/page.md`. Classification and fields as it says.
4. Upload the two web zips it names, each marked "This file will be
   played in the browser".
5. Add the tags it lists, no others.
6. AI disclosure: answer as `page.md` says.
7. Pricing: No payments.
8. Save as Draft; set it Public on launch day.
9. On launch day, one thread in itch.io's Release Announcements board, in
   your own words, from the fact sheet (2865). No other boards, and no
   other projects' comments.

## Scope

**In:**
- `docs/launch/itch/page.md` with sections Title, Short description,
  Description, Tags, AI disclosure, Uploads, Devlog: the description says
  "grid" in its first sentence, what is planned, the license, and links
  the site, the book, the Codespaces quick start and the honest page
  (2861); no store link, no sponsor link.
- Screenshots from `frame_png` of Hello Grid, Snake and the demo puzzle
  (2863), at itch's cover and screenshot sizes.
- Web zips of Hello Grid and Snake, built as the website builds them, by
  one command written in `page.md`.
- The GIF from 2863.
- The *Devlog* section: how each release's News post becomes an itch
  devlog (Nick pastes it).
- `docs/launch/` and its docs-map row, if not there yet.

**Out (do not do):**
- A launch announcement written by an agent.
- Payments, donations, a sponsor or store link.
- Any Visions of Shuyi material.

## Implementation steps

1. Draft `page.md` from the honest page.
2. Make the screenshots and zips with one command, written in `page.md`.
3. Write Nick's steps (above) into `page.md`'s *Uploads*.
4. Ask Nick to read the text.

## Acceptance criteria

- [ ] `page.md` has the seven sections named in *In*.
- [ ] It lists 10 tags or fewer, each describing the engine (no "roguelike" unless a roguelike made with it exists).
- [ ] The description's first sentence says "grid", and it has no word from 2861's keep-out list.
- [ ] Each zip opens its game from `index.html` in a browser.
- [ ] The screenshots are PNG files made by `frame_png`, one per example.
- [ ] The completion notes have `Book:` and `Skills:` lines saying why neither changes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None beyond the gates: no code. The zip command is the website's build.

## Completion notes

*(Filled in by the session that completes the ticket.)*
