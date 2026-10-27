# ADR-0048: The website: Zola 0.21 pinned, in `site/`, sections of our own templates, and the base URL passed by the build

- **Status:** Accepted
- **Date:** 2026-10-26
- **Related tickets:** 2801, 2802–2809, 2811

## Context

Nick asked for a website like Bevy's (2026-10-20, `docs/decisions.md`,
*everything on Bevy's homepage, and a website like Bevy's*): docs, a
tutorial, news with a changelog made automatically, a blog, a sponsor
page, on GitHub Pages first and a custom domain later. Until now
`pages.yml` built Hello, Grid for the web and wrote a one-page index by
hand.

bevy.org is built with Zola: one static binary with Markdown content,
Tera templates, Sass, Atom feeds, a link checker and redirects
(`aliases`) built in, and no Node or npm. The ticket chose it for that,
on the condition that its license be checked on the day.

The check, 2026-10-26: Zola's `LICENSE` changed from MIT to **EUPL-1.2**
in commit `3c9131db` (2026-01-09, "Change license to EUPL"). Every
release from **0.22.0** (2026-01-09) on is under the EUPL; **0.21.0**
(2025-07-14) is the last release under MIT. ADR-0002 §8 denies the EUPL
by name. Zola ships in no game, but the ticket reads §8 as covering a
build tool's license too: that is why mdBook (MPL-2.0) was rejected.

## Decision

1. **The site is a Zola project in `site/`**, built by **Zola 0.21.0**,
   the last MIT release (its `LICENSE` at the tag `v0.21.0`: "The MIT
   License (MIT)"). It stays at 0.21.0: a later Zola is EUPL-1.2, denied
   by ADR-0002 §8. Moving off it is a new ADR (see *Alternatives*).
2. **The pin is in one place**, the composite action
   `.github/actions/setup-zola/action.yml`: the version and the SHA-256
   of `zola-v0.21.0-x86_64-unknown-linux-gnu.tar.gz` from Zola's GitHub
   release (`5c37a8f7…32d6`, the release's own digest). It downloads the
   archive, checks the sum, and puts `zola` on `PATH`. `ci.yml` and
   `pages.yml` both use it. Dependabot doesn't track it, on purpose.
3. **The sections:** Home (`templates/index.html`, with the feature grid
   from `site/data/features.toml`, the latest two News posts and a
   Sponsor strip), Learn (`content/learn.md`), News (`content/news/`, a
   section with an Atom feed at `news/atom.xml`), Examples
   (`content/examples/`, one page per example), Community
   (`content/community.md`) and Sponsor (`content/sponsor.md`). The menu
   is in `templates/base.html` in that order; 2809 adds Assets after
   Examples.
4. **The URL rule.** `config.toml`'s `base_url` is a placeholder for
   `zola serve`. The build passes `--base-url`: `pages.yml` reads the
   Pages site's `html_url` from the API, so a custom domain (2807)
   changes only what GitHub answers. Every link is made from the base
   URL (`get_url`, `@/` links, `page.permalink`); no file in `site/`
   names `zafnok.github.io`. `site/check-build.sh` builds twice with two
   base URLs and fails if either output names the other's, and fails if
   a page loads a script, a stylesheet, a font or an image from another
   host.
5. **An example is a page.** `content/examples/<name>.md` is the card:
   its title, two lines, its `aliases` (an old path, as `hello-grid/`)
   and its notices. Zola renders a stub at `/examples/<name>/` for
   `zola serve`; `pages.yml` builds each `examples/<name>/` whose
   `shuyi.ron` has a `web` section for the web and copies its `dist/web`
   whole over that folder, `NOTICES.txt` with it. An example with no page
   fails the Pages build.
6. **Our own templates and CSS**, no theme: system fonts, light and dark
   from `prefers-color-scheme`, one stylesheet (`static/style.css`, plain
   CSS: Sass isn't needed yet), and nothing loaded from another host. No
   logo: a text wordmark until Nick picks a brand.
7. **The checks.** On every pull request, `ci.yml`'s `site` job runs
   `zola check --skip-external-links` (the `@/` links and their anchors)
   and `site/check-build.sh`; `cargo xtask check-links` reads
   `site/content/` like any Markdown and leaves the `@/` links to Zola.
   Nothing in the engine's build needs Zola.

## Consequences

- One binary, fetched and checked in seconds; no Node, no npm, no
  lockfile of JavaScript to keep.
- A custom domain is one value, and the base-URL check proves it.
- Zola 0.21 gets no fixes. It only reads our own Markdown and templates
  and writes static files, so an unfixed bug is a build bug, not a hole
  in what ships. A feature that needs a later Zola means a new ADR.
- `winget install getzola.zola` installs the newest (EUPL) Zola, so
  `site/README.md` names the version to install.
- The examples and the site build in one job, and a broken example
  build stops the site from publishing.

## Alternatives considered

- **A Zola after 0.21 (EUPL-1.2)** — denied by ADR-0002 §8.
- **mdBook** — MPL-2.0, denied by ADR-0002 §8, and it makes books, not a
  site with news and pages.
- **Hugo** (Apache-2.0, one static binary) — the fallback if 0.21 stops
  being enough. It lost for now because the tickets after this one
  (2802–2811) are written for Zola's `@/` links, `aliases`, Tera and
  `zola check`, and Hugo's templates and link checking differ.
- **A JS framework (Astro, Docusaurus, VitePress)** — Node and an npm
  tree to audit and keep, for a site of static pages.
- **A generator of our own in `xtask`** — pulldown-cmark is already
  here, but templates, feeds, redirects and a link check are a project
  of their own.
- **A third-party Zola theme** — a license to check and a look to fight;
  our templates are a few files.
