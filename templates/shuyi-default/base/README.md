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
| `crates/game/build.rs` | Links the binary for the web: the page's JS plugins are left as imports (`cargo shuyi web`) |
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

## Art and licenses

The font is Terminus Font 4.49.1, under the SIL Open Font License 1.1
(`crates/game/files/font/Terminus-LICENSE.txt`); the atlas in
`crates/game/files/font/` was made from `files-src/ter-u16n.bdf` by the
engine's `font-atlas`. `cargo shuyi check` names a character of the text
the atlas lacks, and `cargo shuyi font-atlas files-src/ter-u16n.bdf
crates/game/files/font` adds it if the font has it.
