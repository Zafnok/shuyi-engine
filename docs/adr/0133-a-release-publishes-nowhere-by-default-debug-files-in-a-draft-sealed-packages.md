# ADR-0133: A game's release publishes nowhere by default, keeps its debug files in a draft of their own, and hands a public game's packages between jobs sealed

- **Status:** Accepted
- **Date:** 2026-11-18
- **Related tickets:** 3406, 3050, 3407, 2520, 3321, 3322, 3323, 3410
- **Amends:** ADR-0114 §2 and §3 (where a tag's release publishes, and
  why a package with the private files may be an artifact), and through
  it ADR-0111 §5

## Context

ADR-0114 §3 let a release's packages of a game with private files be
uploaded as artifacts, reasoning that "a release's own packages are the
one artifact that may hold the private files, since the release makes
them public anyway". That was circular: the release made them public
only because `release.yml` said so, and nobody had decided it should.
3050's `release.yml` put every package **and every `.debug` file** on a
published GitHub Release, and `export.yml` uploaded the same files as
plain artifacts for seven days. A public repository's artifacts are
anyone's to download who is signed in to GitHub.

`shuyi new` makes a game's repository public and its writing, art and
sounds private by default (Nick's decision of 2026-11-12), "to limit AI
copying capability". A packed release build holds the private files in
its pack, an unpacked one in its program (ADR-0111 §3). So a tag of a
new game gave its private files away. 3050's `release`, attestation and
`itch` jobs had never run.

Nick decided on 2026-11-18 (`docs/decisions.md`, *a release publishes
nowhere until the developer picks where*): no public GitHub page by
default; the developer picks each place to publish, the major
storefronts and a public GitHub Release among them. The skill
`shuyi-ship` and 2520 already said a debug file is never shipped.

Checked on 2026-11-18:

- GitHub's REST docs (*List releases*): "Only users with push access will
  receive listings for draft releases". A draft is seen by the people who
  can change the repository.
- GitHub's billing docs: standard runners are free in public
  repositories.
- The engine's scratch run on `Zafnok/scratch-release-3406` (3406's
  completion notes) sealed and opened the packages with each runner's
  own `openssl`: `ubuntu-latest`, `windows-latest`'s Git Bash,
  `macos-latest` and Valve's sniper image (`export::SNIPER`).

## Decision

### 1. The tag's release is a draft unless the developer asks

`release.yml`'s `release` job sets `PUBLISH` from
`vars.PUBLISH_ON_GITHUB == 'true' || github.event.repository.private`.
Without it, `gh release create "$TAG"` takes `--draft`: the packages,
`SHA256SUMS` and the tag's notes wait in a draft only people who can
change the game see, until the developer presses Publish or downloads
the files for a store. With the repository variable `PUBLISH_ON_GITHUB`
set to `true`, the release is published, a free download for anyone.
A private repository's release is private already and is published as
before. The rule holds with or without private files: Nick's second
answer makes a public GitHub download an option for every game.

The other places are options beside it: itch.io (`ITCH_GAME` and
`BUTLER_API_KEY`, unchanged, never a prerelease), and Steam, GOG and the
Epic Games Store (3321, 3322, 3323), each turned on by its own variable
and secret, each its own job after `release`. `shuyi repos` asking which
ones a developer wants is 3410. A re-run of the job uploads into the
release that exists (`gh release upload --clobber`), rather than making
a second draft of the same tag.

Attestations stay on a public repository: they name each package and
its digest, never its bytes.

### 2. Debug files: a draft of their own, `<tag>-debug`

*The files and their checksums* moves every `*.debug` into `debug/`
before the rest goes into `release/`, the folder the release publishes.
`debug/SHA256SUMS` lists them. The step *The debug files' draft* makes
the draft release `<tag>-debug` (`--draft --target "$GITHUB_SHA"`,
titled "Debug files of `<tag>`: never publish"), or uploads into it with
`--clobber` on a re-run. Nothing publishes it, and a test holds that no
step of the job runs `gh release edit` or `--draft=false`. A draft's tag
is made only when it is published, so no `<tag>-debug` tag exists.

`gh release download <tag>-debug` fetches them for 2520's `shuyi crash
--symbols` (`docs/crash-reports.md`). The itch.io job never pushes one.

### 3. A public game's package leaves its runner sealed

A package of a public game with private files is never a plain artifact.
In `export.yml`, when the caller asks for the package
(`inputs.upload-private`), the repository is public and the private
files' action is there, the step *Seal the package* encrypts each
`*.zip`, `*.tar.gz` and `*.debug` into
`<dir>/dist/sealed/<target>/<name>.sealed`:

```bash
openssl enc -aes-256-cbc -pbkdf2 -iter 100000 -md sha256 -salt \
  -pass env:PRIVATE_ASSETS_KEY -in <file> -out <file>.sealed
```

It copies the two reports there as they are, and *Upload the sealed
package* uploads that folder only, under the same `if:`, as the same
artifact name `package-<target>`. The release and itch.io jobs open each
`*.sealed` with `openssl enc -d` and the same options, given the secret
by name. The plain upload keeps two cases: a game without private files
(`hashFiles(…) == ''`) and a private repository's release
(`inputs.upload-private && github.event.repository.private`).

The key is `PRIVATE_ASSETS_KEY`, the private half of the read-only deploy
key of the private repository: whoever holds it reads the private files
already, so sealing with it opens nothing new and needs no new secret. A
dry run of a public repository still uploads no package and seals none.

3407's test over every workflow
(`every_job_with_the_private_files_caches_no_build_and_uploads_none`)
allows an upload in a job with the private files when its `if:` is
`hashFiles(…) == ''` or a private repository's
(`UPLOAD_INPUTS`), or when it takes only the sealed folder after the
seal step under the same `if:` (`uploads_sealed`). The old
`inputs.upload-private` alone is now refused.

## Consequences

- A tag of a new game publishes nothing a stranger can download: its
  packages are in a draft, sealed artifacts, or the stores the developer
  turned on. The developer presses Publish, or sets `PUBLISH_ON_GITHUB`,
  for a free download.
- A crash report from a CI-built release can always be symbolized: its
  debug files are in `<tag>-debug`, read by whoever can change the game.
- AES-CBC with no authentication: an artifact is written only by the
  run's own jobs, so tampering is not the threat; secrecy is.
- An `openssl` that lacks `-pbkdf2` would fail the seal step, not
  upload in the clear: the step stops at the first error.
- The game's own workflows are its own (game ADR-0127 already keeps its
  packages off GitHub): nothing for it to adopt.

## Alternatives considered

- **Keep publishing on GitHub (3050).** Gives a paid game's private
  writing and art away; Nick ruled it out as a default.
- **The private repository's release page.** Needs a fine-grained token
  made by hand (GitHub has no API that makes one), against the
  2026-11-12 rule that the tools make every key; Nick didn't choose it.
- **A write token in each export job** (`gh release upload`, or butler
  from the export job, as the game does). That job runs every
  dependency's build script with the token: ADR-0114 rejected it for that.
- **A key of its own for sealing.** One more secret for `shuyi repos` to
  make and the developer to keep, protecting nothing the deploy key
  doesn't.
- **`age` or `gpg` for the seal.** Not on every runner; `openssl` is,
  and the scratch run checked all four.
- **The debug files in the tag's draft.** A developer who presses
  Publish would publish them too.
