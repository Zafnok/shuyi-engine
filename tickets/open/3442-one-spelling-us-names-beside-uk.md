---
id: "3442"
title: "One spelling of identifiers: the US names beside deprecated UK ones, and a spelling check in api-text"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3438"]
nick_input: none
completed:
---

# 3442 — One spelling of identifiers: the US names beside deprecated UK ones, and a spelling check in api-text

## Context

Split off 3438 at its size valve: its steps 3–5 (the ADR for one
spelling, the renames and the spelling check) and the part of its step 6
that follows them. 3438 gave the domain preludes and `cargo xtask
check-imports` (ADR-0161); 3413 gave `cargo xtask api-text`
(`crates/xtask/src/api_text.rs`) and its `--check`. The blind-spot audit
of 2026-11-23 (*Agent experience*, candidate 68) found that an agent
writes US spellings from memory, and each UK name it misses costs a Rust
compile round.

**What is true today** (checked 2026-11-27):

- The public names mix spellings. `Color`, `EngineColor`, `ColorSet`
  and `Colors` are US. Eight names are UK: `print_centred`
  (`crates/ui/src/screens.rs:44`, `frame/draw.rs:205`),
  `print_centred_styled` (`screens.rs:53`), `centre_x` (`screens.rs:37`),
  `centre` (`crates/ui/src/cinematic.rs:192`, `debug/camera.rs:125`),
  `centred_on` (`crates/ui/src/grid/camera.rs:21`), and the variants
  `NarrationAlign::Centred` (`dialogue/text_box/glyph.rs:26`),
  `Scroll::Centred` (`widgets/list.rs:20`) and `Rule::SharedColour`
  (`crates/assets/src/onebit.rs:523`). The `centre` family is used about
  246 times in 64 files outside `tickets/`.
- `shuyi_platform::prelude` exports `print_centred`; 3438's
  `shuyi::prelude::text` exports `print_centred_styled`.
- The precedent for a rename: 2226 kept `Lang::english` one release as a
  `#[deprecated]` alias of `source_text`, with one `#[expect(deprecated)]`
  test per alias, and needed no `Breaking:` line.
- Its milestone, After MVP, is one of the audit's that 3528 asks Nick
  about: a rename after the release is a deprecation games see.

**The game** (pin `50472bbc`, `gh search code --repo
Zafnok/visions-of-shuyi`): `print_centred` in 8 files
(`crates/ui/src/screens/*/glyph.rs`), `centre_x` in 8, `centred_on` in 6
(`map_view/place.rs`, `screens/battle/mod.rs`), `NarrationAlign` in
`screens/dialogue/glyph.rs`. The old names stay as deprecated aliases, so
the game builds; its PR that moves the pin renames its calls, as each
deprecation warning points at one.

## Nick input

None. The spelling of identifiers is a technical choice, recorded in an
ADR.

## Scope

**In:**

- **One spelling.** US spelling for identifiers, matching `Color` and
  Rust's own crates; prose keeps the house spelling. A new ADR records
  it (the `write-adr` skill). Each of the eight UK names gets its US name
  (`print_centered`, `print_centered_styled`, `center_x`, `center`,
  `centered_on`, `NarrationAlign::Centered`, `Scroll::Centered`,
  `Rule::SharedColor`); the old name stays one release as a
  `#[deprecated(note = "renamed …")]` alias, as 2226 did. A variant's
  alias is an associated `const` (the three enums derive `PartialEq` and
  `Eq`, so it still matches in a pattern). Nothing in `crates/`,
  `templates/`, `examples/`, `site/` or `skills/` calls an old name but
  one `#[expect(deprecated)]` test per alias. `shuyi_platform::prelude`
  exports `print_centered` beside the deprecated `print_centred`, and
  `shuyi::prelude::text` `print_centered_styled` beside
  `print_centred_styled`. `check_text.rs`'s list of text sinks keeps both
  names (`crates/cli/src/check_text.rs:842`).
- **The spelling check** in `cargo xtask api-text --check`: no public
  item's name holds a UK spelling of `api_text::UK_SPELLINGS` (`centre`,
  `centred`, `colour`, `behaviour`, `normalise`, `initialise`, `favour`,
  `licence`), unless it is `#[deprecated]` (the index, `rustdoc_index`,
  reads the deprecation).
- **Public types** (ADR-0039): no new type. The renamed variants keep
  `NarrationAlign`'s `#[non_exhaustive]`; `Scroll` and `Rule` stay as
  they are. `grey` (`lighting.rs:78`) is not renamed: it is a US
  spelling too, and the list leaves it out.

**Out (do not do):**

- Removing an old name: a later release, by its own ticket.
- Renaming anything for taste beyond the UK spellings, or changing the
  house prose style.
- The preludes and the import check: 3438 (done).

**Estimated size:** about 900 lines (code 250, tests 150, call sites
400, docs 100), counted from the steps and doubled. The renames are
about 250 call sites, each a line removed and a line added.

## Implementation steps

1. **The ADR** for one spelling of identifiers (`write-adr`).
2. **The renames**: the US names, the deprecated aliases with their
   tests, every call site in this repository (`templates/` still passes
   `cargo xtask check-imports`), the preludes' new names, and
   `check_text.rs`'s list of text sinks.
3. **The spelling check** in `api_text.rs`'s `--check`, with
   `UK_SPELLINGS`, reading `#[deprecated]` from the rustdoc index.
4. **Docs** (the `document` skill): the book's *Games and screens* uses
   the new names; the shipped skills' text follows.

## Acceptance criteria

- [ ] `print_centered`, `print_centered_styled`, `center_x`, `center`
      (both), `centered_on`, `NarrationAlign::Centered`,
      `Scroll::Centered` and `Rule::SharedColor` exist; each old name is
      `#[deprecated]`, gives what its new name gives (one
      `#[expect(deprecated)]` test each), and still matches in a pattern
      for the three variants.
- [ ] No code in this repository calls an old name but those tests.
      `shuyi check` reads a text sink by either name.
- [ ] `cargo xtask api-text --check` fails on a public, non-deprecated
      item named with a UK spelling, with its own message, and passes on
      this branch.
- [ ] The ADR for one spelling is in `docs/adr/` and its README.
- [ ] Completion notes: a `Behaviour:` line per renamed item (the old
      names warn), or, if the `api` job counts a deprecation as a semver
      finding, a `Breaking:` line with a `migration` block whose `find`
      lists the old names.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: one `#[expect(deprecated)]` test per alias, comparing it with
  its new name (so mutation testing of the diff kills each alias's
  mutants).
- Unit: the spelling check on a fixture index.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
