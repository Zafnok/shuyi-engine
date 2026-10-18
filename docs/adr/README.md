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
| [0003](0003-game-styles-stages-two-kinds-of-simulation-cameras-and-projections.md) | Game styles: stages, two kinds of simulation, cameras and projections | Accepted (was game ADR-0064); its tickets are parked; its "without 3D" is lifted by ADR-0011 |
| [0004](0004-assets-standard-files-in-the-engines-own-data-out-and-a-deterministic-movement-kit.md) | Assets: standard files in, the engine's own data out; a deterministic movement kit, physics optional | Accepted (was game ADR-0065); extended by ADR-0007 |
| [0005](0005-ai-native-tooling-one-contract-three-surfaces-and-visual-scripting-as-a-view-over-text.md) | AI-native tooling: one contract, three surfaces (CLI, MCP, skills); visual scripting as a view over text | Accepted (was game ADR-0066) |
| [0006](0006-networking-in-layers-transports-sessions-sync-models-services.md) | Networking in layers: transports, sessions, sync models, services; determinism is the contract | Accepted (was game ADR-0067); its tickets are parked |
| [0007](0007-formats-are-plugins-standard-files-always-optimized-ones-optional.md) | Formats are plugins: standard files always, optimized ones optional, text stays the source | Accepted |
| [0008](0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md) | Export targets: profiles as data, platform backends, controls that fit, 1-bit graphics, performance budgets | Accepted; the Switch, mobile and console tickets are parked; extended by ADR-0012 (capabilities) |
| [0009](0009-modules-a-small-default-engine-and-opt-in-crates.md) | Modules: a small default engine, a direction per stage, and opt-in crates that say what they are for | Accepted; extends ADR-0003 §6 with perspective markers |
| [0010](0010-procedural-animation-bodies-limbs-and-strips.md) | Procedural animation: bodies of points in fixed-point, limbs that find their footing, and strips to draw them | Accepted; its tickets are parked |
| [0011](0011-3d-fixed-point-space-a-3d-view-in-the-frame-and-hd-2d-as-a-skin.md) | 3D: a fixed-point space, a 3D view in the frame, four directions, and HD-2D as a skin | Accepted; its tickets are parked |
| [0012](0012-gba-and-ds-targets-capabilities-indexed-colour-two-screens-and-homebrew.md) | The GBA and the DS: target capabilities, indexed colour, two screens, and homebrew backends | Accepted; extends ADR-0008; its tickets are parked |

Template: [`0000-template.md`](0000-template.md).
