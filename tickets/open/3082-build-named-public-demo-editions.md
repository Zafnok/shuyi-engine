---
id: "3082"
title: "Build named demo editions from separate code and content roots"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3081", "3075"]
nick_input: none
completed:
---

# 3082 — Build named public-demo editions

## Context

A runtime lock does not hide content already packaged. The default template embeds its whole files directory with include_dir!, while Cargo features are additive. A game needs a reproducible smaller program and content root for a public demo. The final-artifact gate (3081) measures the result; export feature support (3075) supplies the build seam.

**From 3081 (done):** every export already runs the gate, `export::disclosure::gate`, after its exporter, against `shuyi.ron`'s `disclosure` part, so nothing here invokes it again: an edition needs its own part (or its own lists in one), since the part is one per project today. `exposure-report.json`'s `inputs` holds the package, program, cargo arguments, default-features flag and resolved features, read from `Exporter::cargo_args`, so a build with an edition's flags shows there if `cargo_args` returns them; add `edition` and `content root` to `BuildInputs`, and raise the contract's version. Compressed bytes a game makes itself aren't searched: the fixture's canaries stay plain.

## Nick input

None.

## Scope

**In:**
- Named distribution editions in shuyi.ron, independent of hardware targets.
- An edition selects a game package or binary, explicit Cargo default-feature behavior and features, content root and allowed loose files before compilation and baking.
- The exporter records the resolved edition and invokes the exposure gate on its final artifact.
- **Either order with 3085:** if 3085 is done, pass the edition's selected
  package and final default-feature/feature flags through its shared
  resolved-build live-link guard before preparation or compilation; if
  not, retain those flags in the exporter plan so 3085 validates edition
  builds when it lands, with no separate edition bypass.

**Out (do not do):**
- Automatic guesses about which mechanic, story or art a game considers secret.
- Runtime menu locking as a substitute for omission.
- A full-game API change without a Breaking: completion note.

## Implementation steps

1. Add edition parsing and validation to the CLI config and export contract.
2. Build the selected package with resolved Cargo flags and selected input tree; make the template show separate public-demo and full roots.
3. Audit the final archive and teach the book and shuyi-ship skill the disclosure boundary.

## Acceptance criteria

- [ ] A fixture game's public-demo and full editions both play on desktop and web.
- [ ] Future maps, dialogue and code canaries appear only in full, even after pack extraction and binary string inspection.
- [ ] Export reports name edition, package, features and content root.
- [ ] The book's chapter is updated (2802), or the completion notes have a Book: line saying why not.
- [ ] All gates in the run-gates skill pass.

## Tests required

- Integration: build both editions and inspect the final files, packs and binary/Wasm strings.
- Regression: default Cargo features do not silently turn full-only code on in public-demo.

## Completion notes

*(Filled in by the session that completes the ticket.)*
