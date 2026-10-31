---
id: "2801"
title: "The website: a Zola site on GitHub Pages with a home page, Learn, News, Examples, Community and Sponsor, ready for a custom domain"
type: infra
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: []
nick_input: sign-off
completed: 2026-10-26
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
    started" button to the Learn page (2802 points it at its quick
    start), and a feature grid in the style of bevy.org. Each feature
    card has a title, two lines, a
    **status** (Ready, In progress or Planned), and a link to its docs
    page or ticket. The cards come from `site/data/features.toml`,
    whose first rows are the table in `docs/ROADMAP.md`, *Bevy's
    homepage*, plus what the engine has that Bevy's homepage doesn't:
    determinism and replays, the tool contract for agents, targets as
    data. It also has a strip for the latest two News posts and a
    Sponsor strip (2806 fills it; until then a link to the Sponsor
    page).
  - **Learn:** a landing page for the book (2802), the tutorial (2803)
    and the API docs (2802). None of them exists yet (2802 and 2803
    are blocked by this one), so it names them without links, with a
    line saying they are coming, and 2802 and 2803 each add their own
    links here when they land.
  - **News:** a Zola section with an Atom feed. 2804 and 2805 fill it;
    this ticket ships it with one post, "The website is up", in
    `draft = true`, for Nick to publish or delete.
  - **Examples:** one card per example, each playing in the browser at
    `/examples/<name>/`. Hello, Grid moves there from `/hello-grid/`.
    The old path redirects (a Zola `aliases` page).
    - **Either order with 2108:** if 2108 is done,
      `examples/ecs-garden/` gets its card, and `pages.yml` builds it
      beside Hello, Grid; if not, nothing to do (2108 adds its card).
    - **Either order with 2735, 2736 and 2737:** if one of them is done
      (for 2736, its part 2, the match-3 played), its example gets a
      card here as Hello, Grid does, and `pages.yml` keeps the loop
      over every example, if that ticket made it; if not, nothing to
      do (each adds its own card).
    - **Either order with 2738 and 2739** (the brick-breaker and snake,
      parked): if one is done, its example gets its card too, and its
      old path (`/bricks/`, `/snake/`) redirects there as `/hello-grid/`
      does; if not, nothing to do (each adds its own card when it
      lands).
    - **Either order with 2740:** if 2740 is done, Hello, Hex is in
      `pages.yml`'s index too: give it a card and move it the same way.
      If not, nothing to do (2740 adds its card).
    - **Either order with 2754:** if 2754 is done, its crawler example
      gets a card here like every other; if not, nothing to do (2754
      adds its card when it lands).
    - **Either order with 2764:** if 2764 is done, its
      `examples/action-rpg` gets a card like every other example; if
      not, nothing to do (2764 adds its card).
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
    links each example's notices beside it, as today's index links
    `hello-grid/NOTICES.txt`; then it uploads `public/`.
- **CI** (`ci.yml`, on pull requests): `zola check` (internal links and
  anchors), and `cargo xtask check-links` over `site/content/`, which
  leaves Zola's internal links (`@/…`) to `zola check`: `is_relative` in
  `crates/xtask/src/links.rs` skips them, with a test. A pull request
  that breaks the site fails before it merges.
- **Locally:** `site/README.md` says how to preview: `winget install -e
  --id getzola.zola`, then `zola serve` in `site/`. Nothing in the
  engine's build needs Zola.
- **Either order with 2019** (the web build checked in CI): if 2019 is
  done, each example's web build `pages.yml` makes gets a run of
  `check.py boot` in its `web-render` job: add one for any that lacks
  it. If not, nothing to do (2019 boots every build `pages.yml`
  publishes when it is worked).
- **From 2810 (done):** the docs map (`docs/README.md`) gets the
  site's rows when it lands, and `cargo xtask check-docs` fails on any
  Markdown file no row covers. Add a row for `site/content/` under
  *People making games* (the site's pages, updated by a change that
  adds or changes a section or a page; the book gets its own row in
  2802), and one for `site/README.md` under *Engine developers* (how
  to preview and build the site). Take the site out of the map's
  closing line on coming homes, which keeps `llms.txt` (2808).

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
  of a game's frames, and 2811 puts them on the cards, after both;
- an automatic check that each example's web build loads in a browser,
  beyond the clause with 2019 above;
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

- [x] ~~The live site has Home, Learn, News, Examples, Community and
      Sponsor, and its links work (`zola check` passes in CI).~~ (not done: the live site is Nick's sign-off after the first Pages run)
- [x] Hello, Grid plays at `/examples/hello-grid/`, and `/hello-grid/`
      redirects there.
- [x] Changing `base_url` alone moves every link (checked by building
      with two base URLs and grepping the output for the other one).
- [x] The home page's feature grid has a card for every row of the
      roadmap's *Bevy's homepage* table, with a status.
- [x] No page loads anything from another host: no fonts, scripts or
      images (checked by grepping `public/` for `http` in `src` and
      `href` of `link` and `script`).
- [x] At 375 px wide, no page scrolls sideways.
- [x] Zola's pinned version and checksum are in the workflow, and its
      license is recorded in the ADR.
- [x] `docs/README.md` has the site's two rows, and `cargo xtask
      check-docs` passes.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- CI: `zola check` and the link check on pull requests; the Pages build
  on `main`.
- The base URL test and the no-external-host test above, as a script in
  the workflow.

## Completion notes

- **Zola is pinned at 0.21.0, not the newest.** The ticket's license check
  on the day failed: Zola moved from MIT to **EUPL-1.2** in commit
  `3c9131db` (2026-01-09), and every release from 0.22.0 on is EUPL, which
  ADR-0002 §8 denies by name (the same reason mdBook lost). 0.21.0
  (2025-07-14) is the last MIT release, and the site needs nothing newer.
  ADR-0048 records the license, the pin, and Hugo (Apache-2.0) as the
  fallback if 0.21 ever stops being enough. So `site/README.md` says
  `winget install -e --id getzola.zola --version 0.21.0`, not the bare
  command the ticket gave.
- **The pin lives in one place,** the composite action
  `.github/actions/setup-zola/action.yml` (the version and the release's
  SHA-256 of the Linux archive), used by `ci.yml`'s new `site` job and by
  `pages.yml`. It installs to `/usr/local/bin`, because zizmor flags a
  write to `GITHUB_PATH`.
- **The base URL** comes from the Pages API (`html_url`, with `pages:
  read` on the build job), so a custom domain (2807) changes only what
  GitHub answers. `site/check-build.sh` builds with two base URLs and
  fails if either output names the other's or `config.toml`'s; it also
  fails on any `src`, `link href` or CSS `url()` on another host.
  `pages.yml` runs that host check again on the real output, with the
  examples copied in.
- **An example is a page,** `site/content/examples/<name>.md` (its card,
  and `aliases` for an old path). `pages.yml` loops over every
  `examples/*/shuyi.ron` with a `web` section, as the example tickets
  expect, copies `dist/web` whole over the page's stub, and fails an
  example with no card or no `NOTICES.txt`.
- **Either-order clauses:** 2108, 2735–2740, 2754 and 2764 aren't done,
  so Hello, Grid is the only card; 1132 and 2016 aren't done, so
  Community links the repository, its issues, `CONTRIBUTING.md` (it
  already exists) and News; 2019 isn't done. Each of those tickets now
  says what it adds to the site.
- **Not checked here:** the live site. The first Pages run on `main`
  publishes it, and Nick's sign-off reads it on a desktop and a phone.
  Locally: `zola check`, the build checks, every page at 375 px with no
  sideways scroll in light and dark, and Hello, Grid playing at
  `/examples/hello-grid/` under a sub-path after the `/hello-grid/`
  redirect.
- **For Nick:** the News post "The website is up" is `draft = true`;
  publish it by deleting that line, or delete the file. The home page's
  two lines come from `README.md`.
- **Docs:** `docs/README.md` has the rows for `site/content/` (with
  `features.toml`) and `site/README.md`, a Pairs row for the Bevy table
  and its cards, and no longer lists the site as coming. `README.md`'s
  map has a `site/` row, and Hello, Grid's web address is updated.
- No break: no crate's public API changes (only `xtask`'s link check
  skips Zola's `@/` links, with a test).
