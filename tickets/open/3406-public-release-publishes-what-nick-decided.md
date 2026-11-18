---
id: "3406"
title: "A public game's tag release publishes what Nick decided, and never a debug file"
type: bug
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 3406 — A public game's tag release publishes what Nick decided, and never a debug file

## Context

Found by the retro of PRs #379–#484 (2026-11-17), its problem 1.

Nick decided on 2026-11-12 that `shuyi new` makes a game's repositories:
the game's **public** by default and its writing, art and sounds in a
**private** one beside it, "to limit AI copying capability"
([decisions](../../docs/decisions.md), *`shuyi new` sets up a game's
repositories*). [ADR-0111](../../docs/adr/0111-a-games-private-files-a-second-repository-laid-over-its-files-whenever-checked-out.md)
§5 built it so: "none may upload one that embeds the private files".

3050 (done, `nick_input: none`) then wrote the template's release, and
[ADR-0114](../../docs/adr/0114-a-games-release-from-a-tag-one-plan-one-export-workflow.md)
§3 amended ADR-0111 §5 with this reasoning:

> a release's own packages are the one artifact that may hold the
> private files, since the release makes them public anyway.

That is circular. The release makes them public only because the
workflow says so, and nobody decided it should. It is a product call
(a paid game's writing and art given away on GitHub) that Nick never
made. What the template does today, checked in this checkout:

- `templates/shuyi-default/base/.github/workflows/release.yml`, job
  `release`, step *The files and their checksums*: `find artifacts -type
  f ! -name '*.json' -exec mv -n {} release/ \;`, then *Release*: `gh
  release create "$TAG" --verify-tag --notes-file notes.md … release/*`.
  So every package and every `.debug` file goes on a **published**
  GitHub Release. Its comment reasons: "a private repository's release
  is private, and a public one's source is public already". A
  private-files game's packages hold the private files (ADR-0111 §3: the
  packed build's pack holds them, an unpacked build embeds them). Those
  are exactly what is *not* public.
- `export.yml`, step *Upload the package*: on a tag
  (`upload-private: ${{ github.event_name == 'push' || … }}` in
  `release.yml`) it uploads the package **and** `*.debug` as the
  artifact `package-<target>`, `retention-days: 7`. A public
  repository's artifacts are anyone's to download, so the same files are
  public for a week whatever the release does (ADR-0114's
  *Consequences* says so).
- The `itch` job downloads those same artifacts.

It contradicts three places:

- **`skills/shuyi-ship/SKILL.md`, step 2:** "keeps `<package>.debug`
  beside the package: keep it with the release for crash reports, and
  **never ship it**". Its step 8 says the opposite: "a GitHub Release of
  the tag with them, their debug files and `SHA256SUMS`".
- **2520** (open): the debug file is "kept beside the package as
  `<package>.debug` and never shipped". 2520's `shuyi crash --symbols
  <file>` is the user of a release's debug files: whatever this ticket
  does, a developer must still be able to get the debug file of each
  CI-built package of a release.
- **The game's own choice.** Visions of Shuyi's `release.yml` (read on
  2026-11-17 with `gh api
  repos/Zafnok/visions-of-shuyi/contents/.github/workflows/release.yml`;
  the game pins engine `e738ffc2fc37662d9b24335ab5b1d4d8743d8a6f`)
  makes **no** GitHub Release (game ticket 0920, game ADR-0127 "No
  GitHub Pages site and no GitHub Releases", from Nick's request of
  2026-11-16). No package leaves its job through GitHub (game ticket
  0158). Its debug files go straight onto a release of the private
  assets repository with a token Nick made by hand (`DEBUG_FILES_TOKEN`,
  game ticket 0917). Builds people play go to password-protected itch.io
  pages, and the store page comes with game ticket 0901.

A game's debug file is the program before stripping
(`crates/cli/src/export/desktop.rs`). A new game's release export is
built with `packed` (`templates/shuyi-default/base/crates/game/src/lib.rs`:
`PRIVATE` is `#[cfg(not(feature = "packed"))]`), so its debug file holds
no private file, only the public code's symbols and line tables. A game
of its own shape without `packed` embeds them in the program and its
debug file both, as the game's does ("content and all").

3050's `release`, attestation and `itch` jobs have **never run**. The
engine's CI runs `export.yml` for Hello, Grid, never `release.yml`
(ADR-0114, *Consequences*), and no game has used the template's release
yet.

**From 3407 (done):** its test over every workflow
(`every_job_with_the_private_files_caches_no_build_and_uploads_none` in
`crates/cli/src/contract/new/tests.rs`) already holds `export.yml`'s
upload guard, so change the inputs it allows to open an upload (its
constant `UPLOAD_INPUTS`, today `inputs.upload-private`) to match what
this ticket does, in the same PR.

## Nick input

**Decision**, asked with the `ask-nick` skill before any code. Re-check
each fact named here on the day.

### When a game keeps its writing and art private but its code public, where do its downloads go when the developer releases a version?

This is what `shuyi new` sets up by default. Today, pushing a version
puts the whole game, private writing and art included, on the public
GitHub page as a free download for anyone.

**A. A hidden draft the developer publishes by hand.** GitHub has drafts
of a release for this. CI builds everything and leaves it as a draft on
the game's GitHub page, which only people who can change the game see.
Nothing is public until the developer presses Publish, or downloads the
files and sells them elsewhere. Cost: one click a release; no money.

**B. itch.io (or a store) only; nothing to download on GitHub.** This
is what Visions of Shuyi does since 2026-11-16 (no GitHub releases;
builds on a password-protected itch.io page). GitHub shows the version,
its notes and the checksums, and the downloads are on the developer's
itch.io page, where they set the price, a password or none. Cost: an
itch.io account per developer (free; check itch.io's share of a sale
on the day). A game without itch.io set up gets A's draft instead.

**C. The private repository's own release page.** Only people given
access to the private repository can download. Cost: the developer
makes one token by hand on GitHub's settings page, because the setup
can't make that kind (as Visions of Shuyi did for its debug files). The
2026-11-12 decision said every key is made by the tools. Only invited
people can play.

**D. As today: a free public download for anyone.** For example, a game
sold on Steam that also gives free builds on its GitHub page (Mindustry
does; check on the day). Right for a free game. For a paid game, it
gives the private writing and art away.

**My recommendation:** B, with A when itch.io isn't set up. It is what
Nick chose for Visions of Shuyi on 2026-11-16, and it keeps the point of
the 2026-11-12 decision: the private files stay out of anyone's reach
until the developer chooses who gets them.

**Or describe your own.**

Ask in the same message, as a yes or no: may this session make a
scratch game repository and its private repository on Nick's GitHub
account with `shuyi repos` (both free: Actions minutes on a public
repository cost nothing, per GitHub's pricing page, check on the day),
push a tag there to run the release for real, and delete both
repositories afterwards? itch.io stays off in that run: it would need an
itch.io account and a key, which are Nick's to make.

Record both answers in `docs/decisions.md` (`ask-nick`, *Recording the
answer*). If the answer changes what the game should do, say so in the
completion notes; the game's own workflows are its own (Out).

## Scope

**In:**
- `docs/decisions.md`: Nick's answer, verbatim, and the rule.
- **Debug files, whatever the answer** (technical, no question):
  - no `*.debug` is attached to a release a player can see, or pushed to
    itch.io (already true there);
  - each release's debug files are kept where only people who can change
    the game read them, and a developer can fetch them for 2520's
    `shuyi crash --symbols`: a **draft** GitHub Release of their own,
    named `<tag>-debug`, which nothing publishes. Check on the day that
    GitHub's docs still say a draft is seen only by people with write
    access to the repository. If not, pick another place where only they
    read it, and say why in the ADR. With answer C, they go on the
    private repository's release beside the packages instead.
- **The packages, as Nick answers** (`release.yml`, `export.yml`):
  - A: the release is `gh release create --draft`, packages included;
  - B: a published release with the notes, `SHA256SUMS` and the
    attestations, no package; the `itch` job pushes the packages; with
    no `ITCH_GAME`, A's draft;
  - C: packages and debug files on a release of the private repository,
    with a token the developer makes (the README's *Releasing* and the
    private part's `AGENTS.md` say how, with the exact clicks); the
    public release holds the notes and `SHA256SUMS`;
  - D: the published release as today, without the debug files.
  - The rule covers a **public** repository with private files. A
    private repository's release is private, and keeps today's
    behaviour, but for the debug files. A game without private files:
    the same as with them unless Nick's answer says otherwise (ask it as
    part of the question if he doesn't say).
- **The handoff between jobs.** Under A, B or C, no package of a
  public private-files game may be a plain artifact either, even for a
  minute. The recommended way: `export.yml` encrypts the package (and
  its debug file) before `upload-artifact` with a key from the secret
  `PRIVATE_ASSETS_KEY` (whoever holds it can already read the private
  repository, so it opens nothing new), and the job that publishes
  decrypts it. For example `openssl enc -aes-256-cbc -pbkdf2 -pass
  env:…`: check that every runner of the plan has an `openssl` that
  takes `-pbkdf2`, namely `ubuntu-latest`, `windows-latest`'s Git Bash,
  `macos-latest` (LibreSSL), and Valve's sniper image `export::SNIPER`.
  Not chosen: a write token in each export job (`gh release upload`, or
  butler from the job as the game does), because that job runs every
  dependency's build script with it; ADR-0114 rejected it for that.
  Under D the plain artifact stays.
- A new ADR (the `write-adr` skill) that **amends ADR-0114 §3**: it
  replaces the reasoning quoted above with Nick's decision, the debug
  files' place and the handoff. The status lines of ADR-0114 and
  ADR-0111 (§5) name it, and both rows of the index change.
- The docs that say a release carries its debug file, made to agree:
  - `skills/shuyi-ship/SKILL.md` step 8 (and step 2's "keep it with the
    release" names the `<tag>-debug` draft);
  - the base's `README.md` *Releasing* and `AGENTS.md` *Releasing*;
  - the private part's `README.md` and `AGENTS.md` (its "which
    publishes them inside the program anyway");
  - `docs/templates.md` *The private files*;
  - `docs/contract.md` (line ~773);
  - `docs/crash-reports.md` (where a CI release's debug file is, and the
    `gh release download <tag>-debug` that fetches it);
  - the book's `site/content/learn/book/targets-and-export.md`
    *Releasing*.
- Tests in `crates/cli/src/contract/new/tests.rs` (below).
- A real release on a scratch repository, if Nick says yes.

**Out (do not do):**
- The game's own workflows and releases: they are the game's (game ADR-0127
  already keeps its packages off GitHub). If Nick's answer asks the game
  to change, write the game's ticket (`ask-nick`, step 3).
- Caches and uploads of jobs that check out private files, the test over
  every workflow, and the Dependabot secret: 3407.
- The `itch` job's real run: it needs an itch.io account and an API key,
  Nick's setup. Without his yes it stays untested, as today.
- Signing: 3051 (parked) adds its `sign` input to the same workflows.
- `shuyi crash` itself and how it reads a debug file: 2520.
- `shuyi repos` making a token for answer C: GitHub has no API that
  makes a fine-grained token, so the developer makes it by hand, and
  the docs give the clicks.

**Estimated size:** about 1,400 lines (code 450, tests 500, docs 450).

**Split:** past 900 lines, split off the docs outside the template
(the book, `docs/contract.md`, `docs/crash-reports.md`) into a new
ticket, MVP, blocked by this one. The workflows, the ADR, the skill and
the tests stay here: they are what stops the leak.

## Implementation steps

1. Read ADR-0111 §3 and §5, ADR-0112 §2 step 6, ADR-0114, 3050's
   completion notes, the base's `release.yml` and `export.yml`, the
   private part's `action.yml`, and the tests named below.
2. Ask Nick (`ask-nick`), the question above, and record his answer in
   `docs/decisions.md`. Wait for it before step 4.
3. Check on the day: GitHub's docs on who sees a draft release; on
   artifacts of a public repository; on the price of public
   repositories' Actions minutes; and, under A, B or C, each runner's
   `openssl` (step 5).
4. **`release.yml`, job `release`:**
   - *The files and their checksums* moves `*.debug` into `debug/`, not
     `release/`. `SHA256SUMS` lists the packages; a second
     `debug/SHA256SUMS` lists the debug files.
   - A step makes the draft `<tag>-debug` (`gh release create
     "$TAG-debug" --draft --title "Debug files of $TAG: never publish"
     --notes …` with `debug/*`). It runs again safely on a re-run: an
     existing draft gets `gh release upload --clobber`.
   - *Release* follows Nick's answer (Scope). Its comment says why, and
     the old "a public one's source is public already" goes.
   - The `itch` job reads the same handoff as `release`.
5. **`export.yml`**, under A, B or C: a step before *Upload the package*
   that, on a public repository with the private files' action, encrypts
   each file the upload takes but the two reports. The upload's `if:`
   keeps `hashFiles(...) == ''` as one alternative (3407's test reads
   it). Copy the file byte for byte to the engine's
   `.github/workflows/export.yml`
   (`every_games_workflows_are_the_same_and_the_engines_export_is_theirs`).
   The release and itch jobs decrypt with the same secret, passed by
   name.
6. Under C: the private repository's release step, with the token's
   secret by name, and the README's and `AGENTS.md`'s clicks to make it
   (fine-grained, *Contents: Read and write*, that one repository only).
7. The new ADR (step in Scope), and the status lines and the index.
8. The docs listed in Scope.
9. Bless `plans.golden` (`SHUYI_BLESS=1 cargo test -p shuyi-cli --lib
   template`), run the gates.
10. If Nick said yes: make the scratch game with `shuyi new` (private
    files on), run `shuyi repos` on it, push, run the Release workflow's
    dry run from the Actions tab, then push a tag `v0.1.0`. Check that
    the release (or draft) holds what the decision says, that no
    artifact of the public repository holds a plain package (under A, B
    or C), and that the `<tag>-debug` draft holds the debug files and
    is not visible signed out. Delete both repositories after. Put the
    run's links and what was seen in the completion notes. If he said
    no, write a ticket for the first real run (`nick_input: setup`) and
    name it in the notes.

## Acceptance criteria

- [ ] `docs/decisions.md` holds Nick's answer verbatim, dated, and the rule.
- [ ] On a public repository no `*.debug` file is in the published
      release, an itch.io push or a plain artifact a stranger can read
      that holds private files. A tag's run puts each CI-built debug file
      in the draft `<tag>-debug`, or in the private repository's release
      under C.
- [ ] A public private-files game's packages go where the decision says,
      and nowhere else: under A, B or C no artifact of the run holds one
      unencrypted.
- [ ] A private repository's release, and a game without private files,
      do what the Scope says.
- [ ] A dry run (`workflow_dispatch`) still publishes nothing and uploads
      no package of a public private-files game.
- [ ] `skills/shuyi-ship/SKILL.md` steps 2 and 8 agree, and so do the
      README, `AGENTS.md`, `docs/templates.md`, `docs/contract.md` and
      `docs/crash-reports.md`.
- [ ] A new ADR amends ADR-0114 §3, the circular reasoning is gone, and
      the status lines and the index name it.
- [ ] Either a real tag release ran on a scratch repository (its links in
      the completion notes) or a ticket for it exists and is named.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `crates/cli/src/contract/new/tests.rs`, a test that reads the grid
  game with private files' `release.yml` and `export.yml` with
  `serde_norway` (not by text):
  - no step of the `release` job moves a `*.debug` into the folder
    `gh release create "$TAG"` publishes, and a step makes the draft
    `"$TAG-debug"` with `--draft`;
  - the publishing step matches the decision (`--draft` under A, no
    package under B, and so on);
  - under A, B or C: `export.yml`'s encryption step runs before
    `upload-artifact` when the action is there on a public repository,
    and the upload never takes an unencrypted package then.
- Update `the_private_part_writes_the_repository_its_ignore_line_and_its_ci_step`'s
  helper `assert_the_ci_runs_the_part_and_keeps_its_files` and
  `no_job_of_a_tag_reads_a_cache_and_only_the_release_writes` (its list
  of jobs and of which may write) to the new workflows.
- `every_games_workflows_are_the_same_and_the_engines_export_is_theirs`
  passes with the engine's copy of `export.yml`.
- actionlint and zizmor (CI) pass on the changed workflows.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
