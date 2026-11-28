---
name: write-post
description: Draft a post for the website's News (site/content/news/) in Nick's voice and open a PR for him to merge. Use when Nick says "write a blog post about …", "a news post", "a devlog", or asks to announce something on the site. Not for release notes (cargo xtask changelog writes those) or a release post's introduction alone (site/content/news/intro/).
---

# Write a post

Nick posts on News by hand or through this skill, his pick each time
(ticket 2805, [decision of 2026-10-20](../../../docs/decisions.md)). The
skill drafts and opens a PR; **Nick merges it. Never merge it, never
turn on auto-merge**, and never push a post to `main`.

How a post is laid out: [`site/content/news/_template.md`](../../../site/content/news/_template.md)
(every field, explained) and the draft example
`site/content/news/2026-11-27-an-example-post/`. Nick's own steps:
[How to post](../../../site/content/community/how-to-post.md).

## 1. Know what the post says

- If Nick hasn't said what the post is about and the points it makes,
  ask him: what happened, what he wants readers to take away, and
  anything he wants to say in his own words. This is a product
  question to him, not a technical one.
- Don't add points he didn't ask for. A short post that says his two
  things is the post.

## 2. Gather the facts

Every fact in the post (a feature, a date, a number, a name, what works
and what doesn't yet) comes from one of:

- the repository: `docs/ROADMAP.md`, `tickets/done/` and
  `tickets/open/` (a ticket's `status`), `docs/decisions.md`, an
  example's `README.md`, the ADRs, `git log`;
- Nick, in this conversation.

Nothing from memory or guesswork. Check each claim against its source
before writing it, and keep the source list for the PR body.

- **Done or not:** something is "here" only when its tickets are in
  `tickets/done/`. Work in `tickets/open/` is "coming" or "started",
  never "here".
- **Names:** an example, a game or a feature is called what the
  repository calls it. Never name, compare to or style a post after
  another company's game or product where `docs/decisions.md` says not
  to (trails, for one, is never named after another game).
- **Nothing of the game** (`AGENTS.md` rule 4) unless Nick says so: the
  game's story, characters, art and plans are his to announce.
- **Links:** to a page of the site with `@/…` (`[News](@/news/_index.md)`),
  so `zola check` checks it; to a file or ticket on GitHub at `main`.

## 3. Write it in his voice

- Plain and short: a few short paragraphs, the way the existing posts
  read (`site/content/news/the-website-is-up.md`).
- No hype: no "exciting", "blazing", "revolutionary", "game-changer", no
  exclamation marks, no emoji. Say what it does.
- First person ("I") only where Nick speaks for himself; otherwise say
  what the engine does.
- Name keys through actions, never a key (`AGENTS.md` rule 8): "the turn
  actions", not "the arrow keys".

## 4. Make the file

- `site/content/news/YYYY-MM-DD-short-name.md`, the date it goes up
  (today unless Nick says otherwise), copied from `_template.md`. A post
  with a picture is a folder: `YYYY-MM-DD-short-name/index.md` with the
  picture beside it.
- Front matter: `title`, `date` (the file's), `description` (one line),
  `tags = ["devlog"]` for an update, `[extra] author = "Nick"` (a key of
  `site/data/authors.toml`). Delete the template's comment lines.
- **`draft = false`**: the PR is the draft. Nick reads it there and
  merges when he is happy.
- **Pictures:** one Nick made, or one the engine makes (an example's
  `preview.png`, `shots/`, or a `frame_png` from the MCP tools), with an
  `image_alt` that says what is on screen. A picture anyone else made
  goes in `THIRD_PARTY_ASSETS.md` with its license (`AGENTS.md` rule 7),
  or not at all.

## 5. Check and open the PR

1. In `site/`: `zola check --skip-external-links` (after `bash errors.sh`,
   `cargo xtask site-data --site .` and `bash copy-shots.sh`, as
   `site/README.md` says), and `zola serve` to read the post on News, its
   tag's page and the feed.
2. Run the `run-gates` skill (a post alone touches no crate, so
   `cargo xtask gates --fast` is all of it locally; CI runs the website
   gates).
3. A branch `post-YYYY-MM-DD-short-name` (in a desktop worktree, rename
   its `claude/…` branch before the first push), a commit and a PR both
   titled `Post: <title>`. It has no ticket: a post is Nick's content,
   not engine work, and CI lets a `Post: …` PR through when it changes
   only `site/content/news/` and `THIRD_PARTY_ASSETS.md`
   (`tickets/README.md`, *Pull requests*).
4. The PR body: the post's text, then **Sources**, each fact with the
   file, ticket or message it came from, then the attribution line. Ask
   Nick to read it in **Files changed** and merge it when it reads
   right; change it as he asks.
