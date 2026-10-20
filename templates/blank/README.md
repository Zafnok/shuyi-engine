# {{name}}

A game on [Shuyi Engine](https://github.com/Zafnok/shuyi-engine), made
with `shuyi new blank`: one screen with the game's name on it, and every
gate and tool wired, ready for the rest.

```
cargo run -p {{crate}}       # run it
cargo test --workspace       # its tests
cargo shuyi                  # the engine's tools on it (AGENTS.md)
```

## Layout

| Path | What |
| ---- | ---- |
| `crates/game/src/lib.rs` | The game's data (`GameData`), loaded from `files/` |
| `crates/game/src/title.rs` | The one screen |
| `crates/game/src/action.rs`, `settings.rs` | What a player can ask for, and the settings kept |
| `crates/game/src/tools.rs`, `examples/tools.rs` | The game's tools, and the program `cargo shuyi` runs them in |
| `crates/game/files/` | Embedded: the keys, the palette, the English text and the font |
| `crates/game/tests/` | The game played through the engine's Harness, and through its tools |
| `shuyi.ron` | The project file of the engine's tools |
| `AGENTS.md` | How to work on the game, for people and agents |

## Art and licenses

The font is Terminus Font 4.49.1, under the SIL Open Font License 1.1
(`crates/game/files/font/Terminus-LICENSE.txt`); the atlas in
`crates/game/files/font/` was made from it by the engine's `font-atlas`.
