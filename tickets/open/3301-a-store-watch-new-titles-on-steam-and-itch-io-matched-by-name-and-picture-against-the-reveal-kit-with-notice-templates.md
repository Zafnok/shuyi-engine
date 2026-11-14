---
id: "3301"
title: "A store watch: new titles on Steam and itch.io matched by name and picture against the reveal kit, with an evidence bundle and notice templates"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: ["2816"]
nick_input: none
completed:
---

# 3301 — A store watch fed by the reveal kit

## Context

Research ticket 2917 ([report](../../docs/research/2917-reverse-engineering-angles-not-yet-covered.md),
section 11) found that detection and response speed is the one lever in
the public-demo-defenses track with case evidence behind it. In 2026 a
code-copying clone of *Needle In A Haystack Simulator* was gone from Steam
about nine days after it appeared, on the developer's copyright notice
with a file comparison as evidence; Wordle's clones left the App Store in
about two days. Valve removes a title on a facially valid notice within a
day and restores it on counter-notice if no suit follows. A daily watch
can plausibly see a Steam clone within a day. No technical measure the
track measured bought a comparable delay.

The reveal kit (2816) produces the inputs: approved captures, the
canonical store URL, the game's name, artifact hashes and dated receipts.
This ticket watches the stores against them and prepares what a notice
needs. It is advisory: false-positive rates for name and picture matching
are unmeasured, and the law in section 9 of the report says a notice over
mechanics alone fails and risks a counter-claim, so the tool never files
anything and never judges infringement.

Section 10 of the report says perceptual hashes find ripped or upscaled
reuse of art and not a redraw; that is this ticket's picture match.

**From 2816 (done):** the kit is a folder `shuyi reveal prepare` writes
(`dist/reveal` by default) from `shuyi.ron`'s `reveal` part
(`project::Reveal`), and `crates/cli/src/reveal/` holds it, private to
`shuyi-cli` ([ADR-0116](../../docs/adr/0116-the-reveal-kit-a-folder-sealed-by-its-manifests-sha-256-dated-where-the-digest-is-published.md)).
Its manifest, `reveal.ron` (`reveal::kit::Manifest`, version 1, read with
`kit::read_manifest`), gives `game`, `keywords`, `store_url`, `prepared`,
`build`, and `captures`, each with its `original` and `encodes` (`path`
in the kit, `sha256`, `bytes`) and `args`; `artifacts` (the builds by
hash), `receipts`, `commitments` and `index` (`EVIDENCE.md`). The kit's
digest, the manifest's SHA-256, is in `reveal.ron.sha256`, and `reveal
verify` fails any file the manifest doesn't name. So the `watch` section
goes in the `reveal` part of `shuyi.ron` and in a manifest of version 2
(a reader of version 1 refuses it by its version), not as a file added to
a kit. The pictures to hash are the captures' files in the kit; the
evidence bundle copies the kit's `EVIDENCE.md` section "A public clip".

## Nick input

None.

## Scope

**In:**
- `shuyi watch-stores` (a `shuyi-ship` workflow step, or the CLI
  subcommand the `document` skill's map puts it under): reads the reveal
  bundle's manifest (2816), polls Steam's `IStoreService/GetAppList`
  (the maintained one; `ISteamApps/GetAppList` is deprecated) and
  itch.io's new-releases feed, diffs against its last run, and for each
  new title: a fuzzy name match (normalized edit distance over the game's
  name and the manifest's keywords), and a perceptual-hash match of the
  title's capsule and screenshots against the kit's approved captures
  (pHash and dHash; a DINOv2 embedding is out).
- A report of candidates ranked by match, with the store URL, the date
  first seen and the matching picture pairs side by side, written to a
  dated folder; a quiet exit when nothing new matches.
- An evidence bundle per candidate: the kit's receipts, the matching
  hashes, the original build's hashes and a file-comparison template a
  person fills in.
- Pre-filled notice templates for Steam (Valve's DMCA agent form) and
  itch.io, with every field the person must check marked, and a line
  saying the template is not legal advice.
- Runs as a GitHub Actions workflow in a game's repository on a daily
  schedule, state kept in the repository, under the `shuyi-ship` skill.
- Thresholds in the manifest, with the defaults measured on a small
  fixture of true and false pairs in this repository's tests.

**Out (do not do):**
- Filing, posting or sending anything. The person does.
- Judging infringement, or any text that says a candidate is a clone.
- Google Play, the App Store, web portals, TikTok and YouTube (a later
  ticket when a target needs it).
- Paid brand-protection services.
- Scraping store pages past what their APIs and feeds allow; respect
  rate limits and robots rules, and say so in the docs.

## Implementation steps

1. Read 2816's bundle manifest and `shuyi-ship`; add a `watch` section to
   the manifest: name, keywords, approved capture paths, thresholds,
   stores on.
2. Store readers in `crates/cli/src/ship/watch/`: `steam.rs` over
   `IStoreService/GetAppList` (needs a Steam Web API key from the game's
   repository secrets; document how to get one) and the store page's
   capsule and screenshot URLs; `itch.rs` over its public new-releases
   feed. Each returns `(title, url, first_seen, pictures)`.
3. Matching in `match.rs`: name distance (`strsim`, MIT, check with
   `cargo deny`) and `image_hasher` or an in-house pHash/dHash over
   `image` (both MIT OR Apache-2.0; prefer in-house if the dependency is
   small to write, as ADR-0021's clean-room rules say).
4. The report and evidence bundle writers; the notice templates as
   `templates/ship/notices/*.md` with `{{field}}` placeholders.
5. The workflow file in `templates/shuyi-default/base/.github/workflows/`
   gated by the manifest's `watch` section; state in
   `.shuyi/watch/state.json`.
6. Fixture: ten true pairs (an approved capture and the same picture
   re-encoded, resized, cropped, with a store overlay) and ten false pairs
   (other pixel-art games' public domain or CC0 captures, licenses in
   `THIRD_PARTY_ASSETS.md`); the tests pin the default thresholds.
7. Docs per the `document` skill: the `shuyi-ship` skill, the book's
   release chapter, and the research report's section 11 gains a line
   pointing here.

## Acceptance criteria

- [ ] A daily run against recorded API responses reports a planted
      look-alike title and a planted re-encoded capture, each with its
      URL and matching pairs, and reports nothing on the unchanged
      fixture.
- [ ] The evidence bundle and both notice templates are written for the
      planted candidate, with every unchecked field marked.
- [ ] The tool sends nothing and its docs say it judges nothing.
- [ ] Default thresholds pass all ten true pairs and no false pair of the
      fixture.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: name distance over near names, unrelated names and the game's
  own name; pHash/dHash over the fixture pairs.
- Integration: a run over recorded store responses (no network in
  tests) producing the report, bundle and templates; a second run that
  reports nothing new.

## Completion notes

*(Filled in by the session that completes the ticket.)*
