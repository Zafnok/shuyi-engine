# Architecture Decision Records

Technical decisions for Shuyi Engine. Product decisions (name, license,
going public, what is parked) are Nick's and live in
[`../decisions.md`](../decisions.md).

An ADR is never edited to change its meaning after it is `Accepted`. To
change a decision, write a new ADR that supersedes it and set the old
one's status to `Superseded by ADR-NNNN`. Use the `write-adr` skill.

"game ADR-NNNN" means the game repository's ADRs
(`github.com/Zafnok/visions-of-shuyi/tree/main/docs/adr`).

| ADR | Title | Status |
| --- | ----- | ------ |
| [0001](0001-record-architecture-decisions.md) | Record architecture decisions | Accepted |
| [0002](0002-inherited-rules-layers-determinism-skins-actions-tests-licenses.md) | Inherited rules: layers, determinism, skins, actions, tests, licenses | Accepted; extended by ADR-0008 (platform backends) |
| [0003](0003-game-styles-stages-two-kinds-of-simulation-cameras-and-projections.md) | Game styles: stages, two kinds of simulation, cameras and projections | Accepted (was game ADR-0064); its tickets are parked; its "without 3D" is lifted by ADR-0011; extended by ADR-0015 (command sims, and §4's spike for RTS units) |
| [0004](0004-assets-standard-files-in-the-engines-own-data-out-and-a-deterministic-movement-kit.md) | Assets: standard files in, the engine's own data out; a deterministic movement kit, physics optional | Accepted (was game ADR-0065); extended by ADR-0007 |
| [0005](0005-ai-native-tooling-one-contract-three-surfaces-and-visual-scripting-as-a-view-over-text.md) | AI-native tooling: one contract, three surfaces (CLI, MCP, skills); visual scripting as a view over text | Accepted (was game ADR-0066); extended by ADR-0022 (the contract as built) |
| [0006](0006-networking-in-layers-transports-sessions-sync-models-services.md) | Networking in layers: transports, sessions, sync models, services; determinism is the contract | Accepted (was game ADR-0067); its tickets are parked |
| [0007](0007-formats-are-plugins-standard-files-always-optimized-ones-optional.md) | Formats are plugins: standard files always, optimized ones optional, text stays the source | Accepted; extended by ADR-0019 (the registry's rules) |
| [0008](0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md) | Export targets: profiles as data, platform backends, controls that fit, 1-bit graphics, performance budgets | Accepted; the Switch, mobile and console tickets are parked; extended by ADR-0012 (capabilities) and ADR-0024 (the platform boundary as built) |
| [0009](0009-modules-a-small-default-engine-and-opt-in-crates.md) | Modules: a small default engine, a direction per stage, and opt-in crates that say what they are for | Accepted; extends ADR-0003 §6 with perspective markers; extended by ADR-0013 (templates made of parts) and ADR-0015 (the *Kind of game* concern) |
| [0010](0010-procedural-animation-bodies-limbs-and-strips.md) | Procedural animation: bodies of points in fixed-point, limbs that find their footing, and strips to draw them | Accepted; its tickets are parked |
| [0011](0011-3d-fixed-point-space-a-3d-view-in-the-frame-and-hd-2d-as-a-skin.md) | 3D: a fixed-point space, a 3D view in the frame, four directions, and HD-2D as a skin | Accepted; its tickets are parked |
| [0012](0012-gba-and-ds-targets-capabilities-indexed-colour-two-screens-and-homebrew.md) | The GBA and the DS: target capabilities, indexed colour, two screens, and homebrew backends | Accepted; extends ADR-0008; its tickets are parked |
| [0013](0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md) | Starting a game: a wizard over the catalog, and templates made of parts | Accepted |
| [0014](0014-midi-music-sequences-and-a-sound-bank-in-a-module.md) | MIDI music: sequences and a sound bank, in a module | Accepted; its tickets are parked |
| [0015](0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md) | Real-time strategy at scale: command sims, units in columns, flow fields, and AI in layers | Accepted; its tickets are parked until after the game launches |
| [0016](0016-visual-novels-and-detective-games-a-story-stage-a-case-file-and-a-fair-play-check.md) | Visual novels and detective games: a story stage, a case file, and a fair-play check | Accepted; its tickets are parked until after the game launches |
| [0017](0017-sonar-backlog-becomes-batched-github-issues.md) | SonarCloud in CI, and its backlog as batched GitHub issues | Accepted; adopts game ADR-0071; extended by ADR-0037 |
| [0018](0018-screen-results-a-screen-pops-with-a-value-and-hears-when-it-is-covered.md) | Screen results: a screen pops with a value, and hears when it is covered | Accepted |
| [0019](0019-the-format-registry-kinds-are-types-a-format-reads-one-file.md) | The format registry: kinds are types, a format reads one file, conflicts are per kind | Accepted; extends ADR-0007 |
| [0020](0020-menu-items-carry-a-value-and-views-hold-labels-the-skin-translates.md) | Menu items carry a value, and views hold labels the skin translates | Accepted |
| [0021](0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md) | The platform layer in-house: written from specifications, in crates of their own, with wgpu and winit for graphics and the window | Accepted; extends ADR-0002 §8 and §9 and ADR-0008 §2; its tickets are parked |
| [0022](0022-the-tool-contract-answers-with-limits-schemas-by-version-and-a-session-over-the-harness.md) | The tool contract: answers with limits, schemas by version, and a session over the Harness | Accepted; extends ADR-0005 §1–§2 |
| [0023](0023-the-pack-one-indexed-file-of-baked-assets-proven-equal-to-its-sources.md) | The pack: one indexed file of baked assets, a bundle layer, proven equal to its sources | Accepted; extends ADR-0007 §4 and ADR-0019 |
| [0024](0024-the-platform-boundary-app-runs-a-frame-time-in-milliseconds-storage-handed-over-and-the-harness-on-the-headless-backend.md) | The platform boundary as built: App runs a frame, time in milliseconds, storage handed over, and the Harness on the headless backend | Accepted; extends ADR-0008 §2 and ADR-0003 §2 |
| [0025](0025-controls-fit-screens-declare-actions-targets-have-button-tables-and-the-harness-holds-back-the-rest.md) | Controls that fit: screens declare their actions, targets have button tables, and the Harness holds back the rest | Accepted; extends ADR-0008 §3 |
| [0026](0026-one-bit-by-role-a-pattern-map-beside-the-palette-colours-find-their-roles-and-the-renderer-takes-its-shades.md) | One bit by role: a pattern map beside the palette, colours find their roles, and the renderer takes its shades | Accepted; extends ADR-0008 §4 |
| [0027](0027-bots-that-plan-see-a-determinized-copy-integer-rollouts-shared-seeds-and-work-units.md) | Bots that plan see a determinized copy: integer rollouts, shared seeds and work units | Accepted; extends ADR-0008 §6; extended by ADR-0029 (the turn planner) |
| [0028](0028-problem-codes-a-stable-code-by-class-a-help-line-a-page-and-exit-code-3-for-the-setup.md) | Problem codes: a stable code by class, a help line, a page, and exit code 3 for the setup | Accepted; extends ADR-0022 §1 |
| [0029](0029-the-turn-planner-a-beam-over-the-sides-own-commands-copies-kept-and-the-best-continuation-backed-up.md) | The turn planner: a beam over the side's own commands, copies kept, and the best continuation backed up | Accepted; extends ADR-0027 |
| [0030](0030-the-log-plain-lines-kept-per-app-output-only-and-handed-to-the-backends-sinks.md) | The log: plain lines with a level, a category and a frame, kept per App, output only, and handed to the backend's sinks | Accepted; extends ADR-0022 and ADR-0024 |
| [0031](0031-the-mcp-server-hand-written-json-rpc-the-game-in-its-tool-program-and-session-tools-as-ops.md) | The MCP server: hand-written JSON-RPC, the game in its tool program, and session tools as operations | Accepted; extends ADR-0005, ADR-0022 and ADR-0028 |
| [0032](0032-the-layers-as-data-checked-over-cargo-metadata-a-float-ban-in-the-simulations-and-a-new-games-rules-crate-kept-pure.md) | The layers as data, checked over cargo metadata, a float ban in the simulations, and a new game's rules crate kept pure | Accepted; extends ADR-0002 §1–§2, ADR-0008 §2 and ADR-0024 §1 |
| [0033](0033-dungeon-crawlers-a-labyrinth-of-cells-and-walls-a-view-as-plain-data-skins-from-lines-to-3d-and-a-party-of-any-size.md) | Dungeon crawlers: a labyrinth of cells and walls, a view as plain data, skins from lines to 3D, a party of any size, and the bar for engine, template and example | Accepted; its tickets are parked until after the game launches |
| [0034](0034-action-rpgs-a-hero-in-real-time-loot-rolled-from-affix-tables-skills-with-cooldowns-and-packs-of-monsters.md) | Action RPGs: a hero in real time, loot rolled from affix tables, skills with cooldowns, and packs of monsters | Accepted; extends ADR-0033; its tickets are parked until after the game launches |
| [0035](0035-the-mcp-server-follows-the-sessions-folder-roots-a-forwarded-server-per-workspace-and-a-restart-when-the-engine-moves.md) | The MCP server follows the session's folder: roots, a forwarded server per workspace, and a restart when the engine moves | Accepted; extends ADR-0031 |
| [0036](0036-the-short-path-ready-made-parts-beside-the-long-form.md) | The short path: ready-made parts beside the long form | Accepted; extends ADR-0019 and ADR-0024 |
| [0037](0037-dependency-upkeep-dependabot-a-weekly-advisory-check-workflow-audits-and-bot-work-as-its-own-ticket.md) | Dependency upkeep: Dependabot, a weekly advisory check, workflow audits, and bot work as its own ticket | Accepted; adopts game ADR-0008's supply-chain rows; extends ADR-0017 §3 |
| [0038](0038-the-public-api-gate-a-listing-diff-and-semver-checks-on-every-pr-and-a-breaking-line-for-each-break.md) | The public API gate: a listing diff and semver checks on every PR, and a `Breaking:` line for each break | Accepted; extends ADR-0002 §7 and ADR-0017 |
| [0039](0039-public-types-that-can-grow-non-exhaustive-config-and-views-with-constructors-plain-values-left-open.md) | Public types that can grow: non-exhaustive config and views with constructors, plain values left open | Accepted |
| [0040](0040-modules-join-a-running-game-a-library-first-a-module-trait-with-declared-hooks-and-state-by-type-on-the-context.md) | Modules join a running game: a library first, a `Module` trait with declared hooks, and state by type on the context | Accepted; extends ADR-0009 §6 and §8 and ADR-0024 §2 and §5 |
| [0041](0041-the-module-catalog-concerns-as-data-metadata-answers-as-pairs-and-modules-and-add-reading-the-engine-the-game-builds-with.md) | The module catalog: concerns as data, metadata answers as pairs, and `modules` and `add` reading the engine the game builds with | Accepted; extends ADR-0009 §2, §4 and §6 and ADR-0040 §5 |

Template: [`0000-template.md`](0000-template.md).
