# {{name}}

A turn-based game on [Shuyi Engine](https://github.com/Zafnok/shuyi-engine):
walk your piece (`@`) to the goal (`[]`) on a small grid, one tile a
turn, while a wanderer (`&`) takes a step of its own after each of
yours, wherever its luck sends it. Bump into it, or let it bump into
you, and the game is over.

```
cargo run -p {{crate}}       # play it
cargo test --workspace       # its tests
cargo shuyi                  # the engine's tools on it (AGENTS.md)
```

## Layout

| Path | What |
| ---- | ---- |
| `crates/rules/` | The rules, over the engine's `TurnRules`: pure and deterministic |
| `crates/game/src/lib.rs` | The game's data (`GameData`), loaded from `files/` |
| `crates/game/src/stage.rs` | The one screen: the stage, its map painted by the engine's glyph skin |
| `crates/game/src/action.rs`, `settings.rs` | What a player can ask for, and the settings kept |
| `crates/game/src/tools.rs`, `examples/tools.rs` | The game's tools, and the program `cargo shuyi` runs them in |
| `crates/game/files/` | Embedded: the keys, the palette, the English text, the stage (`.map`) and the font |
| `crates/game/tests/` | The game played through the engine's Harness, and through its tools |
| `shuyi.ron` | The project file of the engine's tools |
| `AGENTS.md` | How to work on the game, for people and agents |

## Art and licenses

The font is Terminus Font 4.49.1, under the SIL Open Font License 1.1
(`crates/game/files/font/Terminus-LICENSE.txt`); the atlas in
`crates/game/files/font/` was made from it by the engine's `font-atlas`.
