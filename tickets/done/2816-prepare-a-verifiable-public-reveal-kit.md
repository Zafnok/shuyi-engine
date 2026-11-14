---
id: "2816"
title: "Prepare a verifiable public reveal kit from game captures and releases"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: ["2811", "3050"]
nick_input: none
completed: 2026-11-13
---

# 2816 — Prepare a verifiable public reveal kit

## Context

A public gameplay hook can be imitated from footage without a binary. A developer benefits from a canonical store link, approved captures and dated evidence before the first reveal. Existing preview capture (2811) and tag release attestations (3050) supply the inputs. A timestamp helps attribution, not ownership of an idea.

## Nick input

None.

## Scope

**In:**
- A local, human-reviewed reveal bundle containing approved captures, canonical store URL, game name, artifact hashes and provenance receipts.
- Optional private salted commitments to unreleased source/content; opening them later requires the retained salt and source.
- A copyable incident evidence index tying a public clip to its source build.
- A checklist step before the first public footage: register the game's
  copyright (art, music, text; the US fee is about US$65 and statutory
  damages need registration before infringement or within three months of
  first publication, 17 U.S.C. 412), written as information with a
  "not legal advice" line, as research ticket 2917's report (section 9)
  says. Whether to file, and the fee, is Nick's: the kit only reminds.
- The bundle's manifest carries the game's name, keywords and approved
  captures in a form 3301 (the store watch) and 3302 (the recipient
  overlay) read; both are blocked on this ticket and add nothing here.

**Out (do not do):**
- Automatic Steam publication, legal conclusions or guaranteed takedowns.
- A requirement to ship a demo before a store page.

## Implementation steps

1. Compose existing frame capture and tagged-release outputs without duplicating their pipelines.
2. Define a stable manifest and hash/signature verification command; preserve originals and platform encodes.
3. Add a shuyi-ship workflow for checking the canonical page and approved media before reveal.

## Acceptance criteria

- [x] A fixture reveal bundle verifies after copying to another machine.
- [x] Changing a captured frame or manifest field invalidates verification.
- [x] A creator can prepare a store-only reveal with no demo artifact.
- [x] The book's chapter is updated (2802), or the completion notes have a Book: line saying why not.
- [x] All gates in the run-gates skill pass.

## Tests required

- Integration: produce, copy and verify a fixture reveal bundle; tampering fails.
- Unit: salted commitment opens with original bytes and salt, not changed bytes.

## Completion notes

**Done (2026-11-13):**

- `shuyi reveal` ([ADR-0115](../../docs/adr/0115-the-reveal-kit-a-folder-sealed-by-its-manifests-sha-256-dated-where-the-digest-is-published.md)),
  a command of the CLI only, its module `crates/cli/src/reveal/` private
  to `shuyi-cli`, as `repos` is:
  - `prepare` writes the kit from `shuyi.ron`'s new `reveal` part
    (`project::Reveal`, `RevealCapture`) into `dist/reveal`: the approved
    captures, originals and platform re-encodes, copied whole; the
    builds by hash, never copied (`artifacts`, and each line of a
    release's `SHA256SUMS`, `sums`); receipts copied whole (3050's
    `SHA256SUMS` and Sigstore attestations, a registration's receipt);
    the commitments' digests; `reveal.ron`, the manifest, naming every
    file's SHA-256 and length; `reveal.ron.sha256`, its seal, which
    `sha256sum -c` checks; `EVIDENCE.md`, the copyable evidence index
    ending with a report's section for a public clip; `CHECKLIST.md`;
    and `.gitattributes` (`* -text`).
  - `verify [<dir>] [--digest <sha256>]` needs no project: hashes only,
    never a re-render, so a later engine still verifies an old kit.
  - `commit <file>... [--salts]` and `open <file> (--salts | --salt
    --digest)`: salted commitments, `SHA-256(label, 32 random bytes, the
    file)`, salts in a private file.
- The captures compose 2811: a capture of `preview.png` records the
  `preview` part's `frame-png` arguments. The releases compose 3050: its
  `SHA256SUMS` and attestations are read and kept, never rebuilt.
- `SH3015` (`reveal-kit`) with its page. SHA-256 moved from the exposure
  report's module to `crates/cli/src/sha256.rs`, fed in pieces too
  (`Hasher`, `file`), with a test that pieces give the same hash.
  `shuyi-cli` gains `getrandom` 0.4 (MIT OR Apache-2.0) for the salts; a
  new game's dependency count is 143, within the budget's 146.
- Tests (`crates/cli/src/reveal/tests.rs`, 18, and `sha256`'s): a kit
  copied to another folder with its project removed verifies, with the
  published digest; a changed frame, a changed manifest field (and the
  same re-sealed, against the published digest), an edited index, a file
  not listed, a missing receipt, paths outside the kit and a newer
  version each fail, naming the file; a store page alone, with no build,
  verifies; the commitment opens with the original bytes and salt and
  not with one byte changed or another salt, and a commitment made by
  Python's `hashlib` opens with this code; the CLI's exit codes. The
  committed fixture kit (`crates/cli/tests/fixtures/reveal/kit`, a
  store-only reveal of a real `frame-png` capture of Hello, Grid) was
  made on Windows and is verified against its digest on each of CI's
  systems: the copy to another machine. `SHUYI_BLESS=1` rewrites it.
- Docs: the `shuyi-ship` skill (step 9, `SH3015`, its worked example now
  prepares and verifies a kit on a new game), the book's chapter 9
  (*Before the reveal*, and a quick check), the docs map's row for the
  fixture, the README's `shuyi-cli` row, rustdoc, ADR-0115.

**Deviations:**

- Step 3's "shuyi-ship workflow" is the skill's step 9 and the kit's
  `CHECKLIST.md`, not a GitHub Actions workflow: checking the canonical
  page is a person opening it (the kit sends nothing, so it fetches
  nothing either).
- Step 2's "signature": the kit has no key of its own. Its digest is
  dated where it is published (a pushed commit or tag, a release's notes,
  3050's attestations), and `verify --digest` checks a copy against it
  (ADR-0115 §4 says why).
- **Outside the ticket, at Nick's word:** `cargo xtask check-book`
  allowed two or three quick checks a chapter, and refused the reveal
  kit's as a fourth in chapter 9. Nick said (2026-11-13) there is no
  need to limit them to three, and to change it here: `book.rs`'s
  `check_section` now asks for two or more, with a test that four pass;
  `site/README.md` and 2828's note say so, and `docs/decisions.md` has
  the decision. No ticket of its own: done in this PR on his word.
- Checked against the code (the ticket was 13 days old): 2811 and 3050
  are done and name what the steps use; nothing was renamed. No fix to
  the ticket.

**Hand-overs:** 3301 and 3302 now say what the kit holds and where
(`From 2816 (done)`), and both are `todo`: 3301 adds its `watch` section
in the `reveal` part and a version 2 of the manifest; 3302 keeps its
ledger beside the kit's folder and chooses its signing, since there is no
release signing key.

Book: chapter 9 (`targets-and-export.md`) has the new section *Before the reveal*.

Behaviour: shuyi-cli: `shuyi_cli::COMMANDS` and `usage` list `reveal`; `shuyi.ron` may have a `reveal` part, which an older engine refuses as an unknown field.
Behaviour: shuyi-cli: the problem code `SH3015` (`reveal-kit`) is new, listed by `shuyi explain`.

Searched the game for what changed: `gh search code --repo
Zafnok/visions-of-shuyi 'shuyi_cli::COMMANDS'`, `'reveal kit'` and
`'unknown command'` (its xtask and tickets only): nothing of the game
uses them.

Game: nothing to adopt, the kit is a new command a game uses when its owner reveals it.

Nick input: none. For Nick: before the game's first public footage, the
kit's checklist reminds of copyright registration (about US$65 in the US;
statutory damages need it before an infringement or within three months
of publication). Whether to register, and the fee, are his; the kit only
reminds.
