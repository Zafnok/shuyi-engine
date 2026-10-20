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
| [0017](0017-sonar-backlog-becomes-batched-github-issues.md) | SonarCloud in CI, and its backlog as batched GitHub issues | Accepted; adopts game ADR-0071 |
| [0018](0018-screen-results-a-screen-pops-with-a-value-and-hears-when-it-is-covered.md) | Screen results: a screen pops with a value, and hears when it is covered | Accepted |
| [0019](0019-the-format-registry-kinds-are-types-a-format-reads-one-file.md) | The format registry: kinds are types, a format reads one file, conflicts are per kind | Accepted; extends ADR-0007 |
| [0020](0020-menu-items-carry-a-value-and-views-hold-labels-the-skin-translates.md) | Menu items carry a value, and views hold labels the skin translates | Accepted |
| [0021](0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md) | The platform layer in-house: written from specifications, in crates of their own, with wgpu and winit for graphics and the window | Accepted; extends ADR-0002 §8 and §9 and ADR-0008 §2; its tickets are parked |
| [0022](0022-the-tool-contract-answers-with-limits-schemas-by-version-and-a-session-over-the-harness.md) | The tool contract: answers with limits, schemas by version, and a session over the Harness | Accepted; extends ADR-0005 §1–§2 |
| [0023](0023-the-pack-one-indexed-file-of-baked-assets-proven-equal-to-its-sources.md) | The pack: one indexed file of baked assets, a bundle layer, proven equal to its sources | Accepted; extends ADR-0007 §4 and ADR-0019 |
| [0024](0024-the-platform-boundary-app-runs-a-frame-time-in-milliseconds-storage-handed-over-and-the-harness-on-the-headless-backend.md) | The platform boundary as built: App runs a frame, time in milliseconds, storage handed over, and the Harness on the headless backend | Accepted; extends ADR-0008 §2 and ADR-0003 §2 |
| [0025](0025-controls-fit-screens-declare-actions-targets-have-button-tables-and-the-harness-holds-back-the-rest.md) | Controls that fit: screens declare their actions, targets have button tables, and the Harness holds back the rest | Accepted; extends ADR-0008 §3 |

Template: [`0000-template.md`](0000-template.md).
