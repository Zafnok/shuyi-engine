---
id: "2805"
title: "The blog: Nick's posts in News, written by hand or drafted by the agent, with authors, images, drafts and a feed"
type: infra
milestone: "Engine: public"
model: implementation
effort: medium
status: todo
blocked_by: ["2801"]
nick_input: sign-off
completed:
---

# 2805 — The blog

## Context

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md)): a devlog
"if I want to post a manual update to community", as "just a normal blog
that's hosted in a larger dev website the way Bevy does it... I can pick
if I wanna use claude or use my own hands and brain".

Bevy's News is its blog: release posts and others, side by side, one
feed. Here the release posts are automatic (2804), and this ticket adds
everyone else's: Nick's updates first.

Two ways to write a post, and Nick picks each time:

- **by hand:** a Markdown file, which he can make in GitHub's web editor
  with no tools installed, then merge;
- **with the agent:** he says what he wants to share, and a skill drafts the
  post in his voice and opens a PR. It never publishes on its own:
  Nick merges.

## Nick input

**Sign-off:** Nick reads the "How to post" page and the draft example,
and says whether it is easy enough.

## Scope

**In:**
- **Posts** are `site/content/news/YYYY-MM-DD-slug.md`, with front
  matter:
  ```toml
  +++
  title = "…"
  date = 2026-11-01
  draft = true            # true until it should go live
  description = "…"       # one line, for cards and the feed
  [taxonomies]
  tags = ["devlog"]
  [extra]
  author = "Nick"         # a key in site/data/authors.toml
  image = "cover.png"     # optional, in the post's folder
  +++
  ```
  A post with pictures is a folder (`YYYY-MM-DD-slug/index.md` and its
  images), Zola's page bundle.
- **`site/data/authors.toml`:** name, GitHub user and an optional link
  per author. Nick is the first.
- **Templates:**
  - News's list shows every post with its date, author, tag and
    description, ten per page;
  - a post page has its cover image and author, and links to the
    previous and next posts;
  - tag pages (`/tags/devlog/`, `/tags/release/`);
  - one Atom feed for all of News, and one per tag.
  - drafts build with `zola serve` and never on Pages (`zola build`
    leaves them out).
- **With 2804 (done):** 2804's generated posts (a release, or a
  month before the first tag) have a `description` and the `release`
  tag, and no `extra.author`, so the list and the post page show no
  author for a post without one. 2804 is done: extend the plain
  News list it wrote (`site/templates/news.html`, `macros.html`'s
  `post_list`) and keep the `tags` taxonomy it declared in
  `site/config.toml`, with its two plain templates
  (`taxonomy_list.html`, `taxonomy_single.html`). **From 2804 (done):**
  its generated posts are in `site/content/news/generated/` (never
  committed), and `site/content/news/intro/vX.Y.Z.md` (with `draft =
  true` front matter) holds a release's optional introduction; the News
  page links the Changelog page by `get_url(path='changelog/')`, and the
  feed would list that page too unless this ticket's feed template
  leaves it out (it has no `date`).
- **`site/content/news/_template.md`**, the file to copy, with each
  field explained in a comment.
- **A "How to post" page** for Nick (`site/README.md` and a page under
  Community), in plain steps:
  - in the browser: open `site/content/news/` on GitHub, "Add file",
    paste the template, write, set `draft = false`, "Commit changes"
    to a new branch, open the PR, merge it once its checks pass;
  - with the agent: say "write a blog post about …" in a session.
- **A `write-post` skill** (`.claude/skills/write-post/SKILL.md`) for the
  The agent path:
  - it asks Nick what the post says if he hasn't said, and drafts in his
    words: plain, short, no hype;
  - every fact in it (a feature, a date, a number) comes from the
    repository (the roadmap, tickets, decisions) or from Nick;
  - it opens a PR with the post as `draft = false`, and never merges it;
  - an image Nick didn't make goes in `THIRD_PARTY_ASSETS.md` with its
    license, or not at all (`AGENTS.md` rule 7).
- A draft example post (`draft = true`) that shows every field.

**Out (do not do):**
- Nick's first post (his to write, or to ask for);
- comments under posts (a follow-up, if Nick wants a community space;
  GitHub Discussions would be his call);
- cross-posting to other sites.

## Implementation steps

1. Write the templates, the authors file and the template post.
2. Write the "How to post" page and check its browser steps on GitHub's
   web editor, on a test branch, then delete the branch.
3. Write the `write-post` skill.
4. Run the gates.

## Acceptance criteria

- [ ] A post made from `_template.md` with `draft = false` shows on
      News, in the feed and on its tag page.
- [ ] A post with `draft = true` doesn't show on the live site (checked
      by building and grepping `public/`).
- [ ] The browser steps on the "How to post" page work as written.
- [ ] The `write-post` skill exists and its rules are as above.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `zola check` over the example post and its tag pages.
- A CI step that builds the site and checks that the draft example is
  absent from `public/`.

## Completion notes

*(Filled in by the session that completes the ticket.)*
