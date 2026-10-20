# Hello, Grid

Shuyi Engine's first example: a small grid game made from the engine's
crates alone. A title menu; options with two settings (the text speed and
the map's look: glyphs or tiles) and the Key bindings screen; the
engine's player profiles screen; a short
scene; a 12 × 10 stage where you walk your piece (`@`) to the goal (`[]`)
up to four tiles a turn, while a foe (`&`) steps one tile toward you after
each of your moves, and sometimes trips; and a screen that says how it
ended.

```
cargo run -p hello-grid              # from the repository's root
cargo shuyi web --release            # from this folder: the page in dist/web/
cargo test -p hello-grid             # the tests
```

## Layout

| Path | What |
| ---- | ---- |
| `src/rules.rs` | The rules, over the engine's `TurnRules` and `Sides`, and the foe's mind (`Chase`, a `Bot`) |
| `src/screens.rs`, `src/screens/stage.rs` | The screens: title, scene, stage, end; the options' rows |
| `src/skin.rs`, `src/map.rs` | The look: each screen's view painted with glyphs; the map's terrains, pieces and both skins |
| `src/lib.rs` | The game's data (`GameData`), loaded from `files/` |
| `src/action.rs`, `src/settings.rs`, `src/sound.rs` | Its actions, its settings, its sounds (made by code) |
| `src/main.rs` | The binary: what the platform shell needs to know |
| `src/tools.rs`, `examples/tools.rs` | Its tools (`frame-png`, `lang-status`, and the tool contract's `check`, `run`, `scene`, `replay`, `bots`, `import`), and the tool program `shuyi` runs them in |
| `files/` | Embedded: the keymap, palette, English text, scene (`.dlg`), stage (`.map`), font atlas and tileset |
| `files-src/` | Not embedded: the font the atlas is made from |
| `shuyi.ron` | The project file of the engine's tools |
| `tests/it/` | A playthrough, bots, the tool contract, determinism, options and key rebinding, profiles, the files, and pictures as snapshots |

## Files made by the tools

Never edit these by hand; a test fails when one is stale.

- `files/font/` (the atlas of Terminus):
  `cargo shuyi font-atlas files-src/ter-u16n.bdf files/font`.
- `files/tiles.png` (the tileset, from the font and the palette's colors;
  `shuyi.ron` lists the tiles): `cargo shuyi glyph-tiles`.
- A picture of a frame, to look at: `cargo shuyi frame-png stage.png
  --stage --keys "Enter Up Right"` (`--tiles` for the other look).

## Its tools

From this folder, each with `--json` for an answer a program reads
([`docs/contract.md`](../../docs/contract.md)):

```
cargo shuyi check                                  # every error in files/
cargo shuyi run --stage --script "z Right z" --frames 60
cargo shuyi scene --stage                          # the stage's map and state
cargo shuyi bots 30                                # a random walker against the foe
cargo shuyi replay play.ron                        # a bot's record (bots --json), step by step
cargo shuyi import files/stage.map
```

## Art and licenses

Terminus Font 4.49.1 (`files-src/ter-u16n.bdf`, unmodified, and the atlas
made from it) is under the SIL Open Font License 1.1:
`files/font/Terminus-LICENSE.txt`. Everything else here is the engine's
own: the tileset is the font's glyphs on the palette's colors, and the
sounds are sine blips the game makes when it starts.
