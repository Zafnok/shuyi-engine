---
id: "2846"
title: shuyi export writes an SBOM beside each package, and a page says what the engine gives a game for the EU Cyber Resilience Act
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2846 — shuyi export writes an SBOM beside each package, and a page says what the engine gives a game for the EU Cyber Resilience Act

## Context

The EU Cyber Resilience Act (CRA, Regulation (EU) 2024/2847) covers
"products with digital elements" sold in the EU. A paid game is very
likely one. Its maker, the "manufacturer", answers for the components
inside it, the engine included. Two dates matter:

- 11 September 2026: the duty to report actively exploited
  vulnerabilities and severe incidents applies (Article 14). It applies
  now.
- 11 December 2027: the essential requirements apply in full. Among them
  (Annex I, Part II): identify the components and draw up a software
  bill of materials (SBOM) in a common machine-readable format, covering
  at least the top-level dependencies; handle vulnerabilities; ship
  security updates for a support period. The SBOM is part of the
  technical documentation (Annex VII). It need not be public.

Free open-source software that is not monetised is largely outside the
CRA, and an "open-source software steward" (Article 24) has lighter
duties. So the engine's own position is a product question for Nick
(decision ticket 3529). A game's maker has the duties whatever the
engine's position is. An engine that hands them an SBOM and a clear page
saves every game that work.

What exists, checked on 2026-11-23:

- `grep -rniE 'cyber resil|sbom|cyclonedx|cargo-auditable'` over
  `tickets`, `docs`, `.github` and `crates` finds one line:
  `tickets/done/2011-…md:369`, which put "cargo-auditable and SBOMs" under
  **Out** with no follow-up.
- `NOTICES.txt` already lists the exact crates a build compiles in.
  `crates/cli/src/notices.rs`: `built_crates` (line 330) runs `cargo tree`
  with the build's own flags, then `cargo metadata`, and `shipped`
  (line 505) drops the game's own crates (workspace members outside the
  engine's repository). `Work` (line 230, `#[non_exhaustive]`) holds each
  crate's name, version, license expression, source URL and texts. The
  `Package` it reads (line 434) has Cargo's `source` too, which `Work`
  doesn't keep.
- Three callers build it: `ExportCx::native_notices`
  (`crates/cli/src/export.rs:813`, used by the three desktop exporters,
  `export/desktop.rs:330` and `:369`), the Playdate exporter
  (`export/playdate.rs:930`) and the web build (`crates/cli/src/web.rs:293`).
  The exporters are `EXPORTERS` (`export.rs:210`): windows, linux, macos,
  web and playdate.
- The disclosure check (`crates/cli/src/export/disclosure.rs`, around
  line 798) refuses any file in the export's folder outside the package
  that this export didn't make (`FindingKind::Beside`), except the
  `known` set: `export-report.json`, `exposure-report.json`, the archive
  and the debug companion. A new file there joins that set.
- `export-report.json` is `contract::export::ExportOutput`
  (`crates/cli/src/contract/export.rs:365`). It is not
  `#[non_exhaustive]`, so a new field is a `Breaking:` line, as 3300's
  `pack` was. The contract is version 54 (`contract.rs:76`).
- The template's release workflow
  (`templates/shuyi-default/base/.github/workflows/release.yml`) checksums
  and attests every package (`actions/attest`), which says where it was
  built, not what is in it. It moves into `release/` every file but
  `*.json`. Its `export.yml` uploads the packages, `*.debug` and the two
  reports.
- `SECURITY.md` doesn't exist yet: 2005 writes it.

The game, checked on 2026-11-23 at its pin
`50472bbcd3084555a61728a5d07518f720bd6514`:
`gh search code --repo Zafnok/visions-of-shuyi 'ExportOutput'` finds only
`docs/engine/pins.md`, which says twice "the game builds none of them
field by field". `scripts/check-release-package.sh` reads
`export-report.json` and lists the files of the package itself, never
the folder around it, so a file beside the package changes nothing
there. `'SBOM'` finds nothing.

Builds on ADR-0058 (packages carry nothing of the build machine),
3050 (release from a tag), 3300 (the pack beside the program) and the
disclosure check. Source: the engine blind-spot audit of 2026-11-23,
finding 94.

## Nick input

None. The engine's own CRA position (non-commercial project or steward)
is Nick's, asked in 3529; this ticket works with either answer.

## Scope

**In:**
- Every `shuyi export` writes a CycloneDX 1.6 JSON SBOM beside its
  package: `<folder>.cdx.json` in the export's folder
  (`game-0.1.0-linux.cdx.json`), never inside the package. It lists what
  `NOTICES.txt` lists, from the same `built_crates` result and
  `engine_files` rows: no second resolution.
- The SBOM is the same bytes for the same build: no timestamp, no random
  serial number, components sorted by name and version.
- `Work` gains `origin`, Cargo's `source` of the crate (empty for a path
  crate). `Work` is `#[non_exhaustive]`, so the field is additive.
- `ExportOutput` gains `sbom: Option<FileSize>`, the SBOM's path from
  the export's folder and its size. A `Breaking:` line (no
  `#[non_exhaustive]` on it), the contract's next version, the schema
  regenerated.
- The disclosure check knows the SBOM as made by this export.
- Optional `cargo-auditable`: `shuyi.ron`'s `export.auditable: true`
  builds the desktop programs with `cargo auditable build`, which embeds
  the crate list in the program. Off by default. `ExportPart` is
  `#[non_exhaustive]`, so the field is additive.
- The template's `export.yml` uploads `*.cdx.json`, and its
  `release.yml` puts them in `release/`, so they are checksummed,
  attested and attached to the release with the packages.
- A page, `docs/cyber-resilience.md`: what the engine gives a game's
  maker for the CRA, and what stays theirs. Plainly not legal advice. A
  row for it in the docs map.
- **Either order with 2005:** if 2005 is done, the page links
  `SECURITY.md` for the supported versions and how to report a
  vulnerability; if not, the page names the Security tab's private
  reporting and says `SECURITY.md` comes with the first release, and
  2005 links the page from `SECURITY.md` when it writes it.
- **Either order with 3512:** if 3512 is done, the page's statement on
  how the engine handles vulnerabilities links 3512's ADR on security
  releases; if not, the page says the engine fixes only its latest
  release (2005's policy), and 3512 adds the link to its ADR on this page
  when it lands.
- **Either order with 3529:** if 3529 is done, the CRA docs page states
  the engine's own position (its section *The engine's own position*) as
  Nick answered question 1 in `docs/decisions.md`; if not, the page says
  the engine's own position is not decided yet, and 3529 adds that
  paragraph when it lands.
- Behaviour: every export writes one more file beside its package.

**Out (do not do):**
- `SECURITY.md` itself, the community files and the issue forms: 2005
  and 2849.
- How a security fix is released, advised and yanked: 3512.
- Deciding the engine's CRA position, or whether the game's own CRA
  duties are tracked on the game's side: 3529 asks Nick. This ticket
  takes no position.
- The game's third-party asset files (art, sound, fonts) as SBOM
  components. `NOTICES.txt` and 2223 list those; an SBOM here covers
  software components only.
- SPDX output, vulnerability data (VEX) in the SBOM, signing the SBOM
  apart from the release's attestation, and an SBOM from `shuyi web`
  outside `export`.
- `cargo-auditable` for the web and Playdate builds. Those exporters
  warn that `export.auditable` doesn't apply to them.
- Any new dependency. The JSON is written with `serde_json` and
  `Cargo.lock` read with `toml_edit`, both already in `shuyi-cli`.
- Any claim that a game using the engine is compliant.

**Estimated size:** about 1,100 lines (code 500, tests 400, docs 200).

**Split:** past 900 lines, split off steps 6 and 7 (`export.auditable`)
into a new ticket.

## Implementation steps

1. Read the CRA's text on EUR-Lex (Regulation (EU) 2024/2847): Articles
   13, 14, 24 and 71, Annex I Part II and Annex VII. Check each date and
   article this ticket names; correct any that is wrong, and cite them
   by article in the page. Read CycloneDX 1.6's JSON reference
   (`cyclonedx.org/docs/1.6/json/`).
2. `crates/cli/src/notices.rs`: add `pub origin: String` to `Work`, set
   from `Package::source` in `crate_work` (line 518); empty for a path
   crate and for the `engine_files` rows. `NOTICES.txt`'s text doesn't
   change: its tests stay as they are.
3. A new module `crates/cli/src/sbom.rs` (`pub mod sbom` in `lib.rs`,
   named in the crate doc):
   - `pub const EXTENSION: &str = "cdx.json";`
   - `pub fn document(game: &GameComponent, works: &[Work], lock: Option<&str>) -> String`:
     the SBOM as pretty JSON with a final newline.
   - `bomFormat: "CycloneDX"`, `specVersion: "1.6"`, `version: 1`;
     `metadata.component`: the game, type `application`, its package's
     name and version; `metadata.tools`: `shuyi` and its version. No
     `timestamp` and no `serialNumber`, so a build gives the same bytes.
   - Each crate a `library` component: `name`, `version`, `licenses` as
     one `expression` (the declared SPDX expression; none when the crate
     declares none), `externalReferences` with the `source` as a `vcs` or
     `distribution` reference, and a `purl`: `pkg:cargo/<name>@<version>`
     for a registry crate; with a `vcs_url` qualifier for a git one
     (from `origin`); none for a path crate. `bom-ref` is the purl, or
     `<name>@<version>` without one.
   - A registry crate's `hashes`: SHA-256 from `Cargo.lock`'s `checksum`
     of that name and version, when `lock` is given and has it.
   - Each `engine_files` row (the CLDR data, the controller database, the
     web shell's files) a `file` or `data` component with its version,
     license and source.
   - `dependencies`: the game's component depends on every crate. No
     tree below it: Annex I asks for the top level at least, and
     `cargo tree`'s flat list is what `built_crates` keeps.
   - Components sorted by `(name, version)`; the output is the same bytes
     on every system.
4. Write it in each exporter, beside `NOTICES.txt`'s own call:
   `ExportCx::native_notices` returns the works it wrote, and a new
   `ExportCx::write_sbom(&self, works, report)` writes
   `<folder>.cdx.json` into the export's folder, reading
   `<workspace>/Cargo.lock` if it is there. The desktop, web and
   Playdate exporters call it; `ExportOutput.sbom` names it with its
   size.
5. `crates/cli/src/export/disclosure.rs`: add the SBOM's path to `known`
   (beside `EXPORT_REPORT` and `REPORT`), so it is not a `Beside`
   finding. Read it as written by the export, so a canary in it is
   still found (it names crates, so it holds no game text).
6. `crates/cli/src/project.rs`: `ExportPart` gains
   `pub auditable: bool` (`export.auditable` in `shuyi.ron`, default
   false). When it is true, `ExportCx::build` runs
   `cargo auditable build …` with the same arguments, after checking
   `cargo auditable --version` starts; if not, it answers `SH1003` with
   "install it: `cargo install --locked cargo-auditable`". Check
   `cargo-auditable`'s license on crates.io (MIT OR Apache-2.0 expected)
   and say so in the completion notes. It is a tool on the developer's
   machine, never a dependency, so `deny.toml` doesn't change. The web
   and Playdate exporters add a warning to the report when it is set.
7. Document `export.auditable`'s cost on the page: it embeds every
   crate's name and version in the program, the game's own crates
   included, so the disclosure check's canaries may find a private
   crate's name there. A closed build may want it off.
8. `crates/cli/src/contract/export.rs`: `ExportOutput.sbom`, documented;
   bump `contract::VERSION`; regenerate `crates/cli/schemas/`. Add
   `export.auditable` to `SH1003.md`'s list of an export's tools.
9. The template: `templates/shuyi-default/base/.github/workflows/export.yml`
   uploads `*.cdx.json`; `release.yml` moves `*.cdx.json` into
   `release/` too (its `find … ! -name '*.json'` line), so they are in
   `SHA256SUMS` and attested. Update the comments there.
10. `docs/cyber-resilience.md`, short, plain words, headed "not legal
    advice":
    - *What the CRA asks of a game's maker*, with the dates and articles.
    - *What the engine gives you*: the SBOM and where it lands, the
      optional `export.auditable`, `NOTICES.txt`, the release's
      checksums and attestations, `cargo deny` and the weekly advisory
      check (ADR-0037), the supported-version policy and how to report
      a vulnerability (the 2005 and 3512 clauses).
    - *What stays yours*: your own support period, your reports to
      ENISA's single reporting platform, your conformity assessment and
      technical documentation, and an SBOM of anything you add outside
      Cargo.
    - *The engine's own position* (the 3529 clause).
11. Docs: a row for the page in `docs/README.md`'s first table; the SBOM
    file in `docs/targets/desktop.md`, `docs/targets/playdate.md`,
    `docs/contract.md`'s export section, `skills/shuyi-ship/SKILL.md` and
    the book's `site/content/learn/book/targets-and-export.md`, beside
    each mention of `NOTICES.txt`.

## Acceptance criteria

- [ ] `shuyi export linux` (and windows, macos, web, playdate) writes
      `<folder>.cdx.json` beside the package and never inside it, and
      `export-report.json`'s `sbom` names it with its size.
- [ ] The SBOM lists exactly the crates and engine files of that
      build's `NOTICES.txt`, no more and no fewer, and none of the game's
      own crates.
- [ ] A crate only a default feature brings is absent from a
      `--no-default-features` build's SBOM, as from its `NOTICES.txt`.
- [ ] A crates.io crate has `pkg:cargo/<name>@<version>`, its license
      expression and the SHA-256 that `Cargo.lock` records for it; a
      path crate has no purl.
- [ ] Two exports of the same build give the same SBOM bytes, on Windows
      and on Linux.
- [ ] The disclosure check passes an export with its SBOM, and still
      finds a canary planted in it.
- [ ] `export.auditable: true` with `cargo-auditable` missing answers
      `SH1003` naming the install command; with it installed, a linux
      release program holds the `.dep-v0` section; a web export warns
      and builds as before.
- [ ] A new game's `release.yml` puts the SBOMs in `release/` and in
      `SHA256SUMS` (the template's CI job).
- [ ] `docs/cyber-resilience.md` exists with the four sections, says it
      is not legal advice, cites each date by article, and follows the
      three either-order clauses; `cargo xtask check-docs` passes.
- [ ] The completion notes have a `Breaking:` line for
      `ExportOutput.sbom` with its `migration` block, and a `Behaviour:`
      line for the new file beside every package.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`sbom.rs`): a fixed list of `Work`s (a crates.io crate, a git
  crate, a path crate, a crate with no license, an engine file) and a
  small `Cargo.lock` give the expected JSON, field by field; the output
  is sorted and has no timestamp; a crate the lock lacks has no hash.
- Unit (`notices.rs`): `crate_work` sets `origin` from `source`; the
  `NOTICES.txt` snapshots don't change.
- Unit (`disclosure.rs`): an export folder holding the SBOM has no
  `Beside` finding; a canary written into it is found.
- Unit (`project.rs`): `export.auditable` reads, defaults to false, and
  an unknown field is still refused.
- Integration (`crates/cli/src/export/tests.rs`, where `NOTICES.txt` is
  read today): an export of the test game writes the SBOM, its crate
  names equal `NOTICES.txt`'s, and a second export gives the same bytes.
- Template CI: the generated game's release workflow lists the SBOMs.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
