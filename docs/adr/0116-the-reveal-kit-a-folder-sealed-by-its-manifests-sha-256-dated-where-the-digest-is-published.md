# ADR-0116: The reveal kit: a folder sealed by its manifest's SHA-256, dated where the digest is published

- **Status:** Accepted
- **Date:** 2026-11-13
- **Related tickets:** 2816, 3301, 3302; research 2917 (sections 9 to 11)

## Context

A gameplay idea shown in public can be copied from the footage alone,
with no build of the game. Nothing technical stops that. What helps the
developer afterwards is evidence: which captures were theirs, which build
made them, which store page is the real one, and when each existed. Research
2917 found that a takedown notice over art works where one over mechanics
fails (section 9), that copyright registration before publication decides
statutory damages in the US (17 U.S.C. 412), and that detection speed is
the lever with case evidence behind it (section 11). The store watch (3301)
and the per-recipient overlay of closed builds (3302) both need a fixed,
machine-readable record of the game to work from.

The engine already makes the inputs: `frame-png` makes captures and the
`preview` part keeps their arguments (2811), and a tagged release writes
`SHA256SUMS` and attests each package with Sigstore (3050, ADR-0114).

## Decision

1. **A CLI command, not an operation.** `shuyi reveal` with four
   subcommands: `prepare`, `verify`, `commit`, `open`. It acts on the
   developer's own evidence and never on the game, so it is not part of the
   tool contract or the MCP server, and its module is private to
   `shuyi-cli` (as `repos` is, ADR-0112). Its settings are a `reveal` part
   of `shuyi.ron` (`project::Reveal`, `RevealCapture`). `verify` and `open`
   need no project: a kit is checked wherever it was copied.

2. **The kit is a folder, sealed by one digest.**
   - `reveal.ron`, the manifest (version 1): the game's name, keywords and
     store page, the day it was prepared, the source build (version, git
     commit, whether its folder had changes not committed), the captures
     (each original with its platform re-encodes and the `frame-png`
     arguments that made it), the builds by hash, the receipts, the
     commitments' digests, and the evidence index. Every file it names has
     its path, SHA-256 and length.
   - `reveal.ron.sha256`, the seal: the manifest's SHA-256, written as
     `sha256sum` writes it, so `sha256sum -c` checks it with no engine.
     That SHA-256 is the kit's digest.
   - `captures/` and `receipts/`: files copied in whole, byte for byte.
     Builds are recorded by hash and never copied: a store page alone is
     a kit with no build.
   - `EVIDENCE.md`: an index of all of it with a section to copy into a
     report of a public clip. It is written before the manifest, which
     names its hash, so it is sealed like a capture.
   - `CHECKLIST.md`, the steps before the reveal, and `.gitattributes`
     (`* -text`, so a checkout on Windows changes no byte) are the only
     files not sealed. Any other file in the folder fails `verify`.

3. **`verify` checks hashes only, never renders.** It compares the seal
   with the manifest, `--digest` (the digest published before) with the
   manifest, and each file named with its hash and length; it refuses a
   path outside `captures/`, `receipts/` and `EVIDENCE.md`, and a file the
   manifest doesn't name. Because it never re-renders the index or
   re-serializes the manifest, a later engine that writes them differently
   still verifies an old kit. A manifest of a newer version is refused by
   its version, never misread. The fixture kit in
   `crates/cli/tests/fixtures/reveal/kit` is committed and verified on each
   of CI's systems; a change that rewrites it must keep the old one
   verifying.

4. **No signature of our own; the date comes from where the digest is
   published.** The kit proves the folder unchanged since its digest was
   computed. Its `prepared` date is the preparer's clock and is said to be
   a claim. What dates it is a third party's record of the digest: a commit
   or a tag pushed to the game's repository, a release's notes, or the
   Sigstore attestation 3050's release already makes of each package
   (kept as a receipt, checked with `gh attestation verify`). The
   `CHECKLIST.md` says to publish the digest, and `verify --digest` checks
   a copy against the published one.

5. **Salted commitments for what isn't shown yet.** `reveal commit <file>`
   draws 32 bytes from the system's random source (`getrandom`), and records
   `SHA-256("shuyi reveal commitment v1\0" ‖ salt ‖ bytes)` with its label
   and salt in a salts file the developer keeps private (the `reveal`
   part's `salts`). `prepare` copies only the labels and digests into the
   manifest. `reveal open` shows that a file and its salt give a digest:
   the same bytes and salt open it, a changed byte or another salt don't.
   Without the salt nobody can test a guess at a short design note against
   a published digest. A label is committed to once: a commitment is never
   replaced.

6. **SHA-256 is the engine's own** (`crates/cli/src/sha256.rs`, moved from
   the exposure report's), now also fed in pieces so a trailer is hashed
   without being held whole. No crate is added for it.

7. **Information, never advice.** The checklist reminds of copyright
   registration (the US fee, about US$65, and 17 U.S.C. 412) and says the
   decision and the fee are the game owner's. Every text the kit writes
   ends saying it is not legal advice and judges nothing. The kit never
   publishes, posts or sends anything.

## Consequences

- 3301 reads the manifest's `game`, `keywords` and `captures` (each
  capture's path and SHA-256) and adds its own `watch` section in a later
  manifest version. 3302's ledger sits beside the kit and can record the
  kit's digest.
- A game's kit lives in `dist/reveal` by default, which a new game's
  `.gitignore` leaves out: captures can be large. The developer keeps a
  copy (an archive) and publishes the digest, not the folder.
- A kit changed by one byte fails: the remedy is a copy that verifies or a
  new kit with a new digest, never an edit.
- `shuyi-cli` gains `getrandom` 0.4 (MIT OR Apache-2.0); a new game's
  dependency count stays within its budget.
- A new problem code, `SH3015` (`reveal-kit`).

## Alternatives considered

- **An Ed25519 signature with a key the developer keeps** — a key a solo
  developer must make, back up and never lose, for a signature that dates
  nothing: a self-signed claim of a date is still the developer's claim. A
  third party's record of the digest (git host, Sigstore's log) dates it,
  and costs nothing to keep.
- **An RFC 3161 timestamp or OpenTimestamps in `prepare`** — a network
  call to an outside service from a command that otherwise sends nothing.
  Their receipts can still be added as `receipts` by hand.
- **Re-rendering `EVIDENCE.md` in `verify`** — ties every old kit to the
  current renderer: a wording change would fail kits already published.
- **A `SHA256SUMS` over the whole folder as the seal** — standard, but the
  index must name the kit's digest and would then be inside what it names.
  The manifest carries every hash for tools; the seal covers the manifest.
- **An operation of the contract** — it would put the developer's evidence
  and private salts within an agent's MCP session over the game; the CLI
  alone is enough, as for `repos`.
