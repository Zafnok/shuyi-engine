# The docs map

Where each kind of fact is written, by who reads it. Every fact has one
home. A session that changes one updates that home, as the `document`
skill ([`.claude/skills/document/`](../.claude/skills/document/SKILL.md))
says. `cargo xtask check-docs` fails when a Markdown file in the
repository has no row here.

## People making games, and their agents

| The fact | Its home | Update it on a change that … |
| -------- | -------- | ---------------------------- |
| What a crate, a module or a public item does | Rustdoc in the code. The crate doc (`//!` in each crate's `src/lib.rs`) names every `pub mod` | adds, renames or changes a public item |
| The tool contract | [`contract.md`](contract.md) for the prose and the example answer; [`crates/cli/schemas/`](../crates/cli/schemas/) for the shapes, generated from the types ([ADR-0022](adr/0022-the-tool-contract-answers-with-limits-schemas-by-version-and-a-session-over-the-harness.md)) | adds or changes an operation or a field |
| A problem code and its fix | [`crates/cli/errors/`](../crates/cli/errors/), one `SHnnnn.md` each, read by `shuyi explain` ([ADR-0028](adr/0028-problem-codes-a-stable-code-by-class-a-help-line-a-page-and-exit-code-3-for-the-setup.md)); the website's `/learn/errors/` is written from them by [`site/errors.sh`](../site/errors.sh) | adds or changes an error the contract gives |
| The engine's file formats and the registry | [`formats.md`](formats.md) | adds or changes a format, an asset kind or the registry |
| What a platform backend must do | [`platform-backends.md`](platform-backends.md) | changes the platform boundary or adds a backend |
| How to build a game for the Playdate and sideload it, what `export playdate` writes, and what doesn't work there yet | [`targets/playdate.md`](targets/playdate.md) | changes the Playdate exporter or backend |
| What each file of the Playdate backend is | [`crates/playdate/README.md`](../crates/playdate/README.md) | moves a file of the backend, or adds one |
| What each desktop package (`export windows`, `linux`, `macos`) holds and runs on, a release build's renamed paths and debug file, and the Linux package made by hand | [`targets/desktop.md`](targets/desktop.md) | changes a desktop exporter, its package or what it runs on |
| The modules, by concern: what each answers and serves, its status and tickets | [`modules.md`](modules.md), the catalog; [`modules.ron`](modules.ron), the concerns and their answers as data, which `shuyi modules` reads ([ADR-0009](adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)) | adds a module, a format plugin, a backend, a concern or an answer, or changes one's status |
| What a new game starts with: its rules (`AGENTS.md`) and its README | [`templates/`](../templates/): the engine's template, `templates/shuyi-default`, its files by part | changes a template, or a rule a game on the engine keeps |
| The template format: `template.ron`, parts, `When`, placeholders, merged files | [`templates.md`](templates.md) | changes the template format, the composer, or the engine's template's parts |
| What the engine is, its crates, and where to start | [`README.md`](../README.md) | adds a crate, or changes what one is for |
| How to contribute | [`CONTRIBUTING.md`](../CONTRIBUTING.md) | changes who may contribute or how |
| An example game | [`examples/`](../examples/), each example's `README.md` | changes what an example does or how to run it |
| How an agent makes a game on the engine, task by task | [`skills/`](../skills/), the skill pack: each skill's `SKILL.md`, its worked example `example.ron` (CI runs it on a new game), and the pack's [`README.md`](../skills/README.md); the plugin is `.claude-plugin/marketplace.json` ([ADR-0043](adr/0043-the-skill-pack-skills-at-the-root-worked-examples-run-by-the-xtask-check-skills-in-the-cli-and-the-pack-folder-as-the-plugin.md)) | changes what a skill teaches (an operation, a field, a problem code, a template, a format, a rule a game keeps), or adds a skill |
| How to make a game on the engine, idea by idea: the quick start, then a chapter per concept, with code CI builds and tests | The book, [`site/content/learn/book/`](../site/content/learn/book/), one page a chapter; its code in [`site/snippets/`](../site/snippets/), the crate `book-snippets`, shown by the `snippet` shortcode ([`site/templates/shortcodes/snippet.md`](../site/templates/shortcodes/snippet.md), [`site/README.md`](../site/README.md)) | adds or changes something public: a public item a game names, an operation or a field of the contract, a format, a rule a game keeps, a template |
| How to make a whole game on the engine, from nothing, for a reader new to programming: a box-pushing puzzle in ten steps, each a whole project CI builds and tests and plays in the browser, with pictures of real edits | The tutorial: its pages in [`site/content/learn/tutorial/`](../site/content/learn/tutorial/), each step's project in [`site/tutorial/`](../site/tutorial/), shown by the `snippet` shortcode's `step`; its pictures listed in [`site/tutorial/tries.toml`](../site/tutorial/tries.toml), made by `cargo xtask tutorial-tries` and shown by the [`try_it`](../site/templates/shortcodes/try_it.md) shortcode ([`site/README.md`](../site/README.md)) | changes what a step uses of something public, or how a game starts (`shuyi new`, the template), or changes what a picture shows (`cargo xtask tutorial-tries` remakes them) |
| The docs as agents read them: `llms.txt`, `llms-full.txt`, a Markdown twin of each Learn page at its URL with `index.md` on the end, and the contract's schemas at `/contract/schemas/v<N>/` | Generated at each build of the site by `cargo xtask llms` from Learn ([`site/content/learn/`](../site/content/learn/)), `crates/cli/schemas/` and the hand-written start of `llms.txt`, [`site/llms-header.md`](../site/llms-header.md) ([`site/README.md`](../site/README.md), *The docs for agents*). Change Learn or the header, never a twin | changes what Shuyi is in a sentence (the header), or adds a shortcode to Learn (its expansion in `crates/xtask/src/llms.rs`) |
| Every third-party file | [`THIRD_PARTY_ASSETS.md`](../THIRD_PARTY_ASSETS.md) | adds or removes a non-crate asset |
| The website's pages: the home page's text, Learn, News, the Examples cards, Community and Sponsor | [`site/content/`](../site/content/), and the home page's feature cards in [`site/data/features.toml`](../site/data/features.toml) ([ADR-0048](adr/0048-the-website-zola-0-21-pinned-in-site-sections-of-our-own-templates-and-the-base-url-passed-by-the-build.md)) | adds or changes a section or a page of the site, adds an example, or changes a feature card's tickets, or the status of one with none |

## Engine developers (people and coding agents)

| The fact | Its home | Update it on a change that … |
| -------- | -------- | ---------------------------- |
| The rules | [`AGENTS.md`](../AGENTS.md) | changes how everyone must work here |
| Why a technical choice was made | [`adr/`](adr/), indexed in its [README](adr/README.md) (the `write-adr` skill) | makes a significant technical choice, or supersedes one |
| Research findings, comparisons and measurements | [`research/`](research/), one report per research ticket | completes research or revisits its evidence |
| How to do a recurring task here | [`.claude/skills/`](../.claude/skills/), the shared source; [`.agents/skills/`](../.agents/skills/), generated Codex loaders | changes a recurring task's steps, or adds one |
| What is to be done | [`tickets/`](../tickets/) (the `write-ticket` skill) | finds work, or finishes a ticket |
| The order tickets are done in | [`ROADMAP.md`](ROADMAP.md) | adds a ticket to a phase, or moves one |
| What came from the game | [`carve.md`](carve.md) | moves more of the game's code here |
| Which engine crate may depend on which, the outside crates of the pure ones, and where devices and floats may be | [`tools/layers.ron`](../tools/layers.ron), checked by `cargo xtask check-layers` ([ADR-0032](adr/0032-the-layers-as-data-checked-over-cargo-metadata-a-float-ban-in-the-simulations-and-a-new-games-rules-crate-kept-pure.md)) | adds an engine crate, an edge between engine crates, or an outside crate of a pure crate |
| What a new game's compile may cost: its rebuild's seconds on CI's runners, and its dependency count | [`tools/compile-budget.ron`](../tools/compile-budget.ron), checked by `cargo xtask compile-times` in CI's `compile-times` job ([ADR-0049](adr/0049-fast-compiles-a-new-games-rebuild-timed-in-ci-against-a-budget-no-debug-info-in-dev-and-no-dynamic-engine.md)) | adds a crate to a new game, or makes its rebuild faster or slower |
| The Sonar policy and its script | [`tools/sonar-triage/README.md`](../tools/sonar-triage/README.md) | changes how Sonar findings are triaged |
| The MCP server sessions here use: `shuyi mcp`, run as `cargo run -q -p shuyi-cli -- mcp` | [`.mcp.json`](../.mcp.json) and [`.claude/settings.json`](../.claude/settings.json), what `cargo shuyi mcp install --dry-run --command "cargo run -q -p shuyi-cli --"` prints for Claude Code (a test checks) | changes how the server is run or registered |
| How to preview and build the website, and its rules, and the docs for agents `cargo xtask llms` writes into it: the twins' rule, the URL rule and the full file's budget | [`site/README.md`](../site/README.md) | changes how the site is built, checked or published (`pages.yml`, the `site` job, the pinned Zola, `crates/xtask/src/llms.rs`) |
| The web shell a game's web build runs in, and how CI opens the web builds in headless Chromium (*Checked in CI*: `tools/web-check/check.py`, the `web-render` job) | [`crates/cli/web/README.md`](../crates/cli/web/README.md) | changes the page, the loader or the files `shuyi web` writes, or what `check.py` and the `web-render` job check |
| The sound parity gates: how to run them, what each cell of the matrix means, each platform's evidence contract and what each replacement stage adds | [`tools/sound-parity/README.md`](../tools/sound-parity/README.md); the tolerances and limits in its `budget.ron` | changes what `capture.py`, the `sound-parity` job or `cargo xtask sound-parity` do, or what a stage must pass |
| Where each fact is written | this file | makes a new home for docs (rule 5) |

## Nick

| The fact | Its home | Update it on a change that … |
| -------- | -------- | ---------------------------- |
| His answers | [`decisions.md`](decisions.md) (the `ask-nick` skill) | needs a product decision |
| The plan, in product terms | [`ROADMAP.md`](ROADMAP.md) | changes a phase, or what is parked |

`ROADMAP.md` is in two tables, and that's fine: Nick reads it for the
plan, and sessions read it only to choose a ticket.

## What each task reads

A session reads `AGENTS.md`, this file and its ticket, then the docs its
ticket's block names here, and the ADRs, decisions and tickets the
ticket links. The blocks are [`tickets/README.md`](../tickets/README.md)'s.
A ticket from `1119`–`1150` reads the row of its area.

| Block | Area | Read first |
| ----- | ---- | ---------- |
| `20xx` | Repository and foundation | the book's [quick start](../site/content/learn/book/quick-start.md) and [Fast compiles](../site/content/learn/book/fast-compiles.md), the `run-gates` skill, `.github/workflows/ci.yml`, [ADR-0002](adr/0002-inherited-rules-layers-determinism-skins-actions-tests-licenses.md), [ADR-0017](adr/0017-sonar-backlog-becomes-batched-github-issues.md), [ADR-0037](adr/0037-dependency-upkeep-dependabot-a-weekly-advisory-check-workflow-audits-and-bot-work-as-its-own-ticket.md) |
| `21xx` | Core and simulation | the book's [chapter 6](../site/content/learn/book/rules-as-commands.md) and [chapter 8](../site/content/learn/book/testing.md), [ADR-0002](adr/0002-inherited-rules-layers-determinism-skins-actions-tests-licenses.md), [ADR-0003](adr/0003-game-styles-stages-two-kinds-of-simulation-cameras-and-projections.md), [ADR-0004](adr/0004-assets-standard-files-in-the-engines-own-data-out-and-a-deterministic-movement-kit.md), [ADR-0027](adr/0027-bots-that-plan-see-a-determinized-copy-integer-rollouts-shared-seeds-and-work-units.md) |
| `22xx` | Assets and formats | the book's [chapter 5](../site/content/learn/book/assets-and-formats.md), [`formats.md`](formats.md), [ADR-0007](adr/0007-formats-are-plugins-standard-files-always-optimized-ones-optional.md), [ADR-0019](adr/0019-the-format-registry-kinds-are-types-a-format-reads-one-file.md), [ADR-0023](adr/0023-the-pack-one-indexed-file-of-baked-assets-proven-equal-to-its-sources.md), [ADR-0057](adr/0057-import-settings-a-meta-file-beside-a-file-and-folder-defaults-typed-sections-the-nearest-wins.md) |
| `23xx` | Frame, screens and skins | the book's [chapter 2](../site/content/learn/book/games-and-screens.md) and [chapter 4](../site/content/learn/book/frame-and-skins.md), the MCP session tools (`.mcp.json`: `start`, `step`, `scene`, `frame_png` on `examples/hello-grid`) to see a change, [ADR-0002](adr/0002-inherited-rules-layers-determinism-skins-actions-tests-licenses.md) §4, [ADR-0018](adr/0018-screen-results-a-screen-pops-with-a-value-and-hears-when-it-is-covered.md), [ADR-0020](adr/0020-menu-items-carry-a-value-and-views-hold-labels-the-skin-translates.md), [ADR-0026](adr/0026-one-bit-by-role-a-pattern-map-beside-the-palette-colours-find-their-roles-and-the-renderer-takes-its-shades.md) |
| `24xx` | Input | the book's [chapter 3](../site/content/learn/book/actions.md), the `shuyi-input` skill ([`skills/shuyi-input/`](../skills/shuyi-input/SKILL.md)) and the `input-actions` skill, [ADR-0025](adr/0025-controls-fit-screens-declare-actions-targets-have-button-tables-and-the-harness-holds-back-the-rest.md) |
| `25xx` | Tools and AI support | the book's [chapter 10](../site/content/learn/book/tools-for-agents.md) and [chapter 8](../site/content/learn/book/testing.md), [`contract.md`](contract.md), [`crates/cli/errors/`](../crates/cli/errors/), [`skills/`](../skills/), [ADR-0005](adr/0005-ai-native-tooling-one-contract-three-surfaces-and-visual-scripting-as-a-view-over-text.md), [ADR-0022](adr/0022-the-tool-contract-answers-with-limits-schemas-by-version-and-a-session-over-the-harness.md), [ADR-0028](adr/0028-problem-codes-a-stable-code-by-class-a-help-line-a-page-and-exit-code-3-for-the-setup.md) |
| `26xx` | Networking | [ADR-0006](adr/0006-networking-in-layers-transports-sessions-sync-models-services.md) |
| `27xx` | Game styles and examples | the book's [quick start](../site/content/learn/book/quick-start.md) and [chapter 2](../site/content/learn/book/games-and-screens.md), the MCP session tools (`.mcp.json`: `start`, `step`, `scene`, `frame_png`) to play what changed, [`templates/`](../templates/), [`examples/`](../examples/), [ADR-0003](adr/0003-game-styles-stages-two-kinds-of-simulation-cameras-and-projections.md), [ADR-0009](adr/0009-modules-a-small-default-engine-and-opt-in-crates.md), [ADR-0013](adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md) |
| `28xx` | Docs and community | [the book](../site/content/learn/book/) and [`site/README.md`](../site/README.md), this file, the `document` skill, [ADR-0005](adr/0005-ai-native-tooling-one-contract-three-surfaces-and-visual-scripting-as-a-view-over-text.md) §4 |
| `29xx` | Research | the [`ROADMAP.md`](ROADMAP.md) section the ticket names, the [ADR index](adr/README.md) |
| `30xx` | Export targets | the book's [chapter 9](../site/content/learn/book/targets-and-export.md), [`platform-backends.md`](platform-backends.md), [ADR-0008](adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md), [ADR-0012](adr/0012-gba-and-ds-targets-capabilities-indexed-colour-two-screens-and-homebrew.md), [ADR-0024](adr/0024-the-platform-boundary-app-runs-a-frame-time-in-milliseconds-storage-handed-over-and-the-harness-on-the-headless-backend.md), [ADR-0025](adr/0025-controls-fit-screens-declare-actions-targets-have-button-tables-and-the-harness-holds-back-the-rest.md) |
| `31xx` | 3D and HD-2D | [ADR-0011](adr/0011-3d-fixed-point-space-a-3d-view-in-the-frame-and-hd-2d-as-a-skin.md) |

## The rules for writing docs

1. **One home.** A fact is written in its home. Anywhere else links to
   it, or says it in one line with the link, and never with detail that
   could drift.
2. **Generated beats written.** A shape the code can produce (a schema,
   a list of operations) is generated or tested against the code. A
   written copy is never the source.
3. **Pairs.** A fact that two readers read in two places is a pair,
   listed under *Pairs* with both homes. A change to one changes the
   other in the same PR. A pair the code can check says which test
   checks it.
4. **The reader decides the place.** Nothing technical goes in Nick's
   files. Nothing of this repository's process (tickets, Nick, the
   game's pin) goes in what people making games read: rustdoc, the
   templates, `contract.md`, the error pages, the skill pack and the
   book.
5. **A new home gets a row.** A change that makes a new home for docs (a
   new file under `docs/`, a new folder of pages) adds its row here in
   the same PR.

## Pairs

| The fact | One home | The other | Checked by |
| -------- | -------- | --------- | ---------- |
| The layer, skin and key rules | [`AGENTS.md`](../AGENTS.md), rules 5, 8 and 9 | `templates/shuyi-default/base/AGENTS.md`, *The layers* | nothing yet |
| The contract's operations | [`contract.md`](contract.md), *The operations* | [`README.md`](../README.md), the `shuyi-cli` row | `crates/cli/tests/it/docs.rs` |
| The local gates | the `run-gates` skill | `.github/workflows/ci.yml` | nothing yet |
| The input rules every game keeps | the `shuyi-input` skill ([`skills/shuyi-input/`](../skills/shuyi-input/SKILL.md)), the source | the `input-actions` skill, which keeps only the engine's own and links it | nothing yet |
| The gates every game runs | the `shuyi-gates` skill ([`skills/shuyi-gates/`](../skills/shuyi-gates/SKILL.md)), the source | the `run-gates` skill, which keeps only the engine's own and links it; and `templates/shuyi-default/base/AGENTS.md`, *The gates* | nothing yet |
| The actions' pins and the shared crates' requirements | `.github/workflows/`, and the root's `[workspace.dependencies]` in `Cargo.toml` | `templates/shuyi-default/base/.github/workflows/` and `templates/shuyi-default/stages/top-down-grid/.github/workflows/` (its engine call at `@{{engine_ref}}` aside), and every `Cargo.toml` of the template's parts (`templates/shuyi-default/**/Cargo.toml`) | `cargo xtask check-pins` ([ADR-0037](adr/0037-dependency-upkeep-dependabot-a-weekly-advisory-check-workflow-audits-and-bot-work-as-its-own-ticket.md)) |
| What a module answers | its crate's `[package.metadata.shuyi]` | its row in [`modules.md`](modules.md), and its answers in [`modules.ron`](modules.ron) | `cargo xtask check-modules` |
| What a public concept is and how a game uses it | its rustdoc, the source | its chapter of the book ([`site/content/learn/book/`](../site/content/learn/book/)), which teaches it and links the API docs, with its code in `site/snippets/` | the `test` job builds `site/snippets/`, so a public item the book's code uses can't change without the book; its words: CI's `api` job (`cargo xtask api`, ticket 2815) fails a change to the public API that changes no file of the book and has no `Book:` line in its ticket's notes saying why not. Which chapter it changes isn't checked |
| The features on Bevy's homepage | [`ROADMAP.md`](ROADMAP.md), *Bevy's homepage* | the first cards of [`site/data/features.toml`](../site/data/features.toml), one a row | their statuses only: a card's `tickets` make it at each build (`cargo xtask site-data`, ticket 2815); its text, nothing yet |
| The device crates | [`tools/layers.ron`](../tools/layers.ron), `devices` | `templates/shuyi-default/stages/top-down-grid/crates/rules/tests/layers.rs`, and its list in `shuyi_cli::contract::lint::NEVER` | `layers.rs`'s `a_new_games_rules_test_keeps_a_copy_of_the_devices` (`crates/xtask`); `lint`'s `the_crates_never_under_the_rules_are_the_templates` |

Coming homes get their rows when they land: the guides, `docs/guides/`
(2004).
A skill of the pack that teaches what one of this repository's skills
teaches is the source for what holds in every game; ours keeps only what
holds here and links it, and the pair gets a row above.
