---
id: "2801"
title: "The website: a Zola site on GitHub Pages with a home page, Learn, News, Examples, Community and Sponsor, ready for a custom domain"
type: infra
milestone: "Engine: public"
model: sonnet-5
effort: high
status: todo
blocked_by: []
nick_input: sign-off
completed:
---

# 2801 — The website

## Context

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md), *everything
on Bevy's homepage, and a website like Bevy's*): extensive docs, a
tutorial, a news page with an automatic changelog, a blog "hosted in a
larger dev website the way Bevy does it", and a sponsor segment, on
GitHub Pages first and a custom domain later. The website is part of the
MVP, in place of the docs site 1132 used to hold (1132 keeps the
community files).

Today `.github/workflows/pages.yml` builds Hello, Grid for the web and
writes a one-page index by hand. This ticket replaces that index with a
real site, and the examples move under it.

bevy.org is built with Zola. This ticket uses Zola too:

- one static binary, no Node or npm, with Markdown content, Tera
  templates, Sass, a search index and Atom feeds built in;
- MIT licensed (check its `LICENSE` on the day, and record it);
- a build tool only: nothing of it ships in a game, so ADR-0002 §8 is
  about the license, not the size.

mdBook was considered and rejected. It is MPL-2.0, which ADR-0002 §8
denies, and it makes books, not a site with news and pages.

This ticket sets the frame. The book (2802), the tutorial (2803), the
changelog (2804), the blog (2805) and the sponsor page (2806) fill it.

## Nick input

**Sign-off:** Nick looks at the live site, on a desktop and on his phone,
and says whether the home page says what the engine is.

## Scope

**In:**
- **An ADR** (the `write-adr` skill): the site in `site/`, built by Zola
  at a pinned version, its sections, the URL rule below, and why not
  mdBook or a JS framework.
- **`site/`**, a Zola project:
  - `config.toml` with `base_url` read from the build. The workflow
    passes `--base-url` from the Pages URL now, and a custom domain
    later (2807) changes only that value. No page writes an absolute
    link to `zafnok.github.io`.
  - **Templates of our own** (`site/templates/`), no third-party theme,
    so nothing needs a license check. System fonts only, so no font
    files ship. Light and dark from `prefers-color-scheme`. Works at
    phone width with no sideways scroll.
  - A text wordmark, "Shuyi Engine". No logo: branding is Nick's, and
    he hasn't picked one.
- **The sections**, each a page or a Zola section:
  - **Home:** what Shuyi is in two lines (from `README.md`), a "Get
    started" button to 2802's quick start, and a feature grid in the
    style of bevy.org. Each feature card has a title, two lines, a
    **status** (Ready, In progress or Planned), and a link to its docs
    page or ticket. The cards come from `site/data/features.toml`,
    whose first rows are the table in `docs/ROADMAP.md`, *Bevy's
    homepage*, plus what the engine has that Bevy's homepage doesn't:
    determinism and replays, the tool contract for agents, targets as
    data. It also has a strip for the latest two News posts and a
    Sponsor strip (2806 fills it; until then a link to the Sponsor
    page).
  - **Learn:** a landing page linking the book (2802), the tutorial
    (2803) and the API docs (2802).
  - **News:** a Zola section with an Atom feed. 2804 and 2805 fill it;
    this ticket ships it with one post, "The website is up", in
    `draft = true`, for Nick to publish or delete.
  - **Examples:** one card per example, each playing in the browser at
    `/examples/<name>/`. Hello, Grid moves there from `/hello-grid/`.
    The old path redirects (a Zola `aliases` page).
  - **Community:** GitHub issues, `CONTRIBUTING.md`, the code of
    conduct, the security policy (from 1132; if 1132 isn't done, link
    the repository and add the rest in 1132), and `GOVERNANCE.md` (from
    2016; if 2016 isn't done, 2016 adds the link).
  - **Sponsor:** a page saying sponsorship is coming. 2806 fills it.
  - A footer with the licenses (MIT OR Apache-2.0), the repository, and
    the feed.
- **`pages.yml`:**
  - downloads Zola at the pinned version from its GitHub release and
    checks its SHA-256 against a value in the workflow;
  - builds the examples (as now), then `zola build`, copies each
    example's `dist/web` into `public/examples/<name>/` whole, so each
    example's `NOTICES.txt` (3039) goes with it, and the Examples page
    links each example's notices beside it (if 3039 isn't done, 3039
    adds the link); then it uploads `public/`.
- **CI** (`ci.yml`, on pull requests): `zola check` (internal links and
  anchors), and `cargo xtask check-links` over `site/content/`, which
  leaves Zola's internal links (`@/…`) to `zola check`: `is_relative` in
  `crates/xtask/src/links.rs` skips them, with a test. A pull request
  that breaks the site fails before it merges.
- **Locally:** `site/README.md` says how to preview: `winget install -e
  --id getzola.zola`, then `zola serve` in `site/`. Nothing in the
  engine's build needs Zola.

**Out (do not do):**
- the book's chapters, the tutorial, the changelog, blog tooling, sponsor
  accounts (2802–2806);
- `llms.txt`, `llms-full.txt` and Markdown twins of the Learn pages
  (2808), which also adds their links on the Learn page and in the
  footer;
- an Assets page of modules, templates, tools and games, the engine's
  own and, if Nick says yes, other people's: 2809, after the first
  release (2005), which adds an Assets item to the base template's menu
  after Examples;
- a custom domain (2807, parked);
- search (Zola's built-in index is a follow-up if Learn grows past a
  few dozen pages);
- comments, analytics or anything that sets a cookie;
- animated previews on the Examples cards: 2514 makes an APNG or a GIF
  of a game's frames; putting them on the cards is a follow-up after
  both;
- an automatic check that each example's web build loads in a browser
  (2019, after the MVP, adds every build `pages.yml` publishes to its
  `web-render` job);
- playing the examples on a phone, which has no keyboard (2407,
  parked);
- Hello, Grid's Quit, which freezes its web build: 3053 hides it where
  the platform can't quit (either order; moving the example to
  `/examples/hello-grid/` doesn't change it). An example added to the
  Examples page offers Quit only where `ctx.platform.can_quit()` once
  3053 is done;
- a logo or a brand (Nick's).

## Implementation steps

1. Write the ADR.
2. `zola init` in `site/`, then write the base template, the CSS and
   the sections' templates.
3. Write `features.toml` from the roadmap's table, and the home page.
4. Move the examples index into the Examples section, with the redirect.
5. Change `pages.yml`, and add the CI checks.
6. Run the gates, push, and check the live site.

## Acceptance criteria

- [ ] The live site has Home, Learn, News, Examples, Community and
      Sponsor, and its links work (`zola check` passes in CI).
- [ ] Hello, Grid plays at `/examples/hello-grid/`, and `/hello-grid/`
      redirects there.
- [ ] Changing `base_url` alone moves every link (checked by building
      with two base URLs and grepping the output for the other one).
- [ ] The home page's feature grid has a card for every row of the
      roadmap's *Bevy's homepage* table, with a status.
- [ ] No page loads anything from another host: no fonts, scripts or
      images (checked by grepping `public/` for `http` in `src` and
      `href` of `link` and `script`).
- [ ] At 375 px wide, no page scrolls sideways.
- [ ] Zola's pinned version and checksum are in the workflow, and its
      license is recorded in the ADR.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- CI: `zola check` and the link check on pull requests; the Pages build
  on `main`.
- The base URL test and the no-external-host test above, as a script in
  the workflow.

## Completion notes

*(Filled in by the session that completes the ticket.)*
