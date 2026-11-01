---
id: "3085"
title: "Refuse live-link features in the resolved export build"
type: bug
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3004", "3075"]
nick_input: none
completed:
---

# 3085 — Refuse live-link features in the resolved export build

## Context

[PR 183](https://github.com/Zafnok/shuyi-engine/pull/183) (3004) and
[PR 212](https://github.com/Zafnok/shuyi-engine/pull/212) (3075) require
exports to refuse the debug live link. The 2026-11-01 PR review found
that `export::no_live_link` checks only strings after Cargo's feature
flags. A package with `default = ["link"]`, or a requested
`devtools = ["link"]` alias, passes that check without a literal
`--features link` argument. An ordinary dependency can also enable
`shuyi-app/link` on the exported game without such an argument.

`ExportCx::of` already reads the package's feature map, but retains only
its names. `export::disclosure::resolved_features` walks that package's
map for reporting; it does not resolve the actual dependency build or
enforce the live-link rule.

The listener still requires debug tools and `SHUYI_LINK` at runtime;
this is a build-policy bypass, not an automatically listening release.
Exports must nevertheless uphold the rule before packaging, in debug
and release configurations alike
([ADR-0047](../../docs/adr/0047-the-live-link-json-lines-over-loopback-tcp-to-a-running-debug-build-read-only-but-a-players-input.md)).

## Nick input

None.

## Scope

**In:**
- Validate the features Cargo actually enables for the selected game
  package, artifact and target, with the exact export build flags.
  Include default features, local aliases and dependency feature
  forwarding, including renamed dependencies. Refuse the selected
  package's `link` feature or the engine's live-link feature
  (`shuyi-app/link` or `shuyi-platform/link`) when enabled in that build.
- Resolve normal/build dependencies for the exported artifact. A
  dev-dependency, an unrelated workspace member or a dependency only
  enabled for another target must not cause a false refusal. This
  matters here: `shuyi-app` enables its own `link` in dev-dependencies
  to test it, while normal game exports must continue to work.
- Run the check before compilation, baking or packaging, for each build
  returned by `Exporter::cargo_args`: desktop slices, web, and the
  Playdate's selected library builds. Name the package and feature that
  violates the rule. Resolution/tool failure refuses the export with an
  actionable error; it never means the build is safe.
- Keep flags used by resolution and compilation together so `--without`
  and any default-feature behavior are interpreted identically. Do not
  add an export bypass or silently remove features Cargo would enable.
- **Either order with 3082:** if 3082 is done, validate its edition's
  selected package and final default-feature/feature flags too; if not,
  nothing to add for editions here (3082 passes its selected flags through
  the shared guard when it lands).
- Use Cargo's resolution information with existing JSON/process tools;
  no new dependency or custom replacement for Cargo's whole resolver.
  Keep the existing literal guard for immediate diagnostics if useful.
- Document the resolved-build rule and its error in the export rustdoc
  and the book's export chapter. Preserve public context compatibility
  where possible; any API/schema break follows the usual gates.
- About 800–1,400 changed lines, including fixture work, in one PR.

**Out (do not do):**
- Implementing named demo editions and new default-feature controls
  (3082), beyond the either-order integration above.
- Capture-feature or general debug-feature policy, runtime listener
  authentication, or redesigning the exposure report (3081, done).
- Disabling the game's default features to make a failing export pass.

## Implementation steps

1. Read `crates/cli/src/export.rs`, `export/{desktop,web,playdate}.rs`,
   `contract/export.rs`, and the package-feature reporting helper in
   `export/disclosure.rs`. Confirm which builds each exporter runs.
2. Add one shared resolved-build check and invoke it from the common
   export path before each exporter's preparation or build. Use the
   same package, target and feature arguments as that build.
3. Add tiny local Cargo fixture projects demonstrating default, alias,
   dependency-forwarded, renamed-dependency and target-specific cases.
   Keep them free of new outside dependencies and SDK requirements.
4. Test refusal before build/prepare through the contract/export test
   seams, and the successful ordinary export path with engine
   dev-dependencies present. Update docs and run the gates.

## Acceptance criteria

- [ ] Direct `link`, `default = ["link"]`, and one- and two-level
      requested aliases that enable `link` are refused, naming the
      activated package and feature.
- [ ] A normal dependency enabling an engine live-link feature is
      refused, including dependency renaming and feature forwarding.
- [ ] `--without devtools` passes only if the resolved build has no
      other path enabling the link; a default path still causes refusal.
- [ ] A harmless feature named `linked`, link in a dev-dependency or
      unrelated workspace member, and a link dependency excluded by the
      selected target all pass the guard.
- [ ] The guard runs before preparation/build/package work for desktop,
      web and Playdate exporter plans; a refusal cannot produce a new
      distributable or invoke `--run`.
- [ ] A feature-resolution failure is an export error with its cause,
      and never a successful or unverified package.
- [ ] Existing Hello, Grid exports pass with their normal feature sets;
      no test disables engine dev-dependencies to hide false positives.
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: exact build flags handed to resolution, errors and diagnostics,
  exporter-plan coverage and refusal before preparation/build.
- Integration: local Cargo fixture graph cases above, using the actual
  Cargo resolver and no network or device SDK.
- Regression: ordinary engine-backed exports with dev-dependency-only
  link features still succeed; normal dependency activation is refused.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
