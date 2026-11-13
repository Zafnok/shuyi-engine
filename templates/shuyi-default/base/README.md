# {{name}}

A game on [Shuyi Engine](https://github.com/Zafnok/shuyi-engine), made
with `shuyi new`: a title with the game's name on it, its stages if it
has any, and every gate and tool wired, ready for the rest.

```
cargo run -p {{crate_name}}       # run it
cargo test --workspace       # its tests
cargo shuyi                  # the engine's tools on it (AGENTS.md)
cargo shuyi web              # its web page, in dist/web/ (serve it over HTTP)
```

## Layout

| Path | What |
| ---- | ---- |
| `crates/game/src/lib.rs` | The game: its name, its files, its data type and its screens |
| `crates/game/src/main.rs` | Runs it in a window |
| `crates/game/build.rs` | Names the folders the program embeds: `files/`, and the private files' `private/files/` when it is there; and links the binary for the web: the page's JS plugins are left as imports (`cargo shuyi web`) |
| `crates/game/src/title.rs` | The title, the first screen: Confirm starts the first stage |
| `crates/game/src/stages.rs` | The stages, in order, and each one's part of the data (`stages/`) |
| `crates/game/examples/tools.rs` | The program `cargo shuyi` runs the game's tools in |
| `crates/game/files/` | Embedded: the keys, the palette, the English text and the font; a release export (`cargo shuyi export <target> --release`) ships them baked into `<package>.pack` beside the program |
| `files-src/ter-u16n.bdf` | The font the atlas in `crates/game/files/font/` is made from (`cargo shuyi font-atlas`) |
| `crates/game/tests/` | The game played through the engine's Harness, and through its tools |
| `shuyi.ron` | The project file of the engine's tools |
| `AGENTS.md` | How to work on the game, for people and agents |
| `.claude/skills/` | The engine's skills for coding agents: how to do each kind of task on the game |
| `scripts/setup-linux.sh` | Sets up Linux to build the game: the C linker, `pkg-config`, the system libraries and the pinned Rust (`bash scripts/setup-linux.sh`) |
| `.devcontainer/` | A dev container (or a GitHub Codespace) set up by that script |

It is on the engine's short path; `AGENTS.md` says how it grows past it.

## Putting the game on GitHub

```
cargo shuyi repos --dry-run  # what it would make, changing nothing
cargo shuyi repos            # make it
```

`cargo shuyi repos` puts the game on GitHub with [`gh`](https://cli.github.com),
GitHub's command-line tool, signed in to your account (`gh auth login`). It
creates a repository on your account named from the game, public by default
(GitHub's CI is free there; `--visibility private` otherwise), and pushes the
game to it. A game with private files also gets the private repository
`shuyi.ron`'s `private_assets` names, with a README and its folders, the game
pinned to it and its files checked out, and the read-only deploy keys and the
secret `PRIVATE_ASSETS_KEY` its CI reads them with. What is there already is
left alone: run it again to finish what a failure left, or with `--new-keys`
to replace the keys. `cargo shuyi repos --help` says the rest.

## Releasing

A pushed tag `vX.Y.Z` releases the game (`.github/workflows/release.yml`):
the gates, then a package for each target CI builds, each on its own
system, then a GitHub Release of the tag with every package, its debug
file and `SHA256SUMS`.

1. Set the version: `app.version` in `shuyi.ron` when the game sets one,
   else `[workspace.package]`'s `version` in `Cargo.toml`. Then run
   `cargo shuyi app-info`, and commit.
2. `cargo shuyi export --plan` lists the packages and where CI builds
   them; `--tag vX.Y.Z` says whether a tag names the version.
3. Tag it, with the release's notes as the tag's message, and push the
   tag:

   ```
   git tag -a vX.Y.Z -m "What changed"
   git push origin vX.Y.Z
   ```

   A version with a pre-release part (`1.2.0-beta.1`) is a prerelease.

**A dry run:** in the repository's Actions tab, run the Release workflow
on a branch: the gates and the packages, with no release.

**itch.io:** set the repository variable `ITCH_GAME` to `user/game`, and
the repository secret `BUTLER_API_KEY` to a key of your itch.io account
(`butler login`, or the account's API keys). Each release but a
prerelease then pushes each package to its channel (`windows`, `linux`,
`macos`, `web`) with butler. On the game's itch.io page, set the `web`
channel's file to "This file will be played in the browser" once.

**macOS:** the `.app` is signed ad hoc, with no Developer ID, so macOS
stops it the first time: the player opens it from System Settings,
Privacy & Security (the package's `HOW-TO-OPEN.txt` says how).

**Checking a package:** on a public repository each file is attested:
`gh attestation verify <file> --repo <owner>/<repo>` says it was built
by this repository's release workflow.

## Art and licenses

The font is Terminus Font 4.49.1, under the SIL Open Font License 1.1
(`crates/game/files/font/Terminus-LICENSE.txt`); the atlas in
`crates/game/files/font/` was made from `files-src/ter-u16n.bdf` by the
engine's `font-atlas`. `cargo shuyi check` names a character of the text
the atlas lacks, and `cargo shuyi font-atlas files-src/ter-u16n.bdf
crates/game/files/font` adds it if the font has it.
