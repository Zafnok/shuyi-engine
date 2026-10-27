# The website

Shuyi Engine's website: a [Zola](https://www.getzola.org/) project, built
by Zola **0.21.0** and published on GitHub Pages by
[`pages.yml`](../.github/workflows/pages.yml)
([ADR-0048](../docs/adr/0048-the-website-zola-0-21-pinned-in-site-sections-of-our-own-templates-and-the-base-url-passed-by-the-build.md)).
Nothing in the engine's build needs Zola.

## Preview it

Install Zola 0.21.0, the version the site is built with. Later releases
are under the EUPL-1.2, which this repository doesn't use (ADR-0048).

```bash
winget install -e --id getzola.zola --version 0.21.0
```

On Linux or macOS, take `zola-v0.21.0-*` from
[its release](https://github.com/getzola/zola/releases/tag/v0.21.0).
Then, in this folder:

```bash
zola serve --drafts
```

and open <http://127.0.0.1:1111>. `--drafts` shows the posts marked
`draft = true`. An example's page shows how to build it: the Pages build
copies each example's web build there.

## What is where

| Path | What |
| ---- | ---- |
| `config.toml` | The site's settings. `base_url` is for `zola serve`; the build passes `--base-url` |
| `content/` | The pages: `_index.md` (the home page's text), `learn.md`, `community.md`, `sponsor.md`, `news/` (one Markdown file a post) and `examples/` (one page an example, named as its folder in `examples/`) |
| `data/features.toml` | The home page's feature cards, the first rows from `docs/ROADMAP.md`, *Bevy's homepage* |
| `templates/` | The HTML, our own (no theme): `base.html` has the menu and the footer |
| `static/style.css` | The one stylesheet: system fonts, light and dark |
| `check-build.sh` | The build checks CI runs |

## The rules

- Link to a page of the site with Zola's `@/` links (`[News](@/news/_index.md)`)
  in Markdown and `get_url` in templates, never with an address:
  changing `base_url` alone must move every link.
- Load nothing from another host: no fonts, scripts, stylesheets or
  images.
- A new example gets its page in `content/examples/`, or the Pages build
  fails.

## The checks

CI's `site` job, on every pull request:

```bash
zola check --skip-external-links
bash check-build.sh
```

The first checks the `@/` links and their anchors. The second builds
the site with two base URLs and fails if either output names the
other's, and fails if a page loads anything from another host.
`cargo xtask check-links` reads `content/` like the rest of the
repository's Markdown, and leaves the `@/` links to Zola.
