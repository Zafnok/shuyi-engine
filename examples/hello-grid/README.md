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
| `src/rules.rs` | The rules, over the engine's `TurnRules` and `Sides`, and a stage's `Tuning` (the foe's trip chance, and how many turns it waits); the foe's mind (`Chase`, a `Bot`); and what a rollout bot plans with: the stage's `Determinize` (the luck redrawn), its `score`, and `Walk`, a plain walker for your piece |
| `src/screens.rs`, `src/screens/stage.rs` | The screens: title, scene, stage, end; the options' rows |
| `src/skin.rs`, `src/map.rs` | The look: each screen's view painted with glyphs; the map's terrains, pieces and both skins |
| `src/lib.rs` | The game's data (`GameData`), loaded from `files/` |
| `src/action.rs`, `src/settings.rs`, `src/sound.rs` | Its actions, its settings, its sounds (made by code) |
| `src/main.rs` | The binary: what the platform shell needs to know |
| `src/tools.rs`, `examples/tools.rs` | Its tools (`frame-png`, `lang-status`, and the tool contract's `check`, `run`, `scene`, `replay`, `bots`, `playtest`, `autobalance`, `import`); the knobs an autobalance search turns (`Trip` and `HeadStart`, `shuyi_core::Knob`); and the tool program `shuyi` runs them in |
| `files/` | Embedded: the keymap, palette and its one-bit patterns, English text, scene (`.dlg`), stage (`.map`), font atlas and tileset |
| `files-src/` | Not embedded: the font the atlas is made from |
| `shuyi.ron` | The project file of the engine's tools |
| `playtest.ron` | The playtest's checks: how often each hero should reach the goal (`cargo shuyi playtest`) |
| `balance.ron` | The autobalance's stage: the tuning it starts from (a foe that never trips) and its band besides `playtest.ron`'s, that the walker sees the foe stumble in 50–70% of tries (`cargo shuyi autobalance`); `--apply` writes the tuning found back here. The game itself plays `rules::Tuning::default` |
| `tests/it/` | A playthrough, bots, a playtest and an autobalance search (`balance.rs`: a foe that never trips is tuned until the walker sees it stumble in 50–70% of tries; `autobalance.rs`: the same through the tool contract, on threads), the tool contract, determinism, options and key rebinding, profiles, the files, and pictures as snapshots |

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
cargo shuyi check --target all                     # it fits each target shuyi.ron lists
cargo shuyi run --stage --script "z Right z" --frames 60
cargo shuyi scene --stage                          # the stage's map and state
cargo shuyi bots 30                                # a random walker against the foe
cargo shuyi playtest --tries 50                    # each hero against playtest.ron's bands
cargo shuyi playtest --entrant walk --threads 1    # the walker alone, on one thread
cargo shuyi autobalance                            # tune balance.ron's stage into its bands
cargo shuyi autobalance --time 60 --apply          # at most a minute, then write the tuning found
cargo shuyi replay play.ron                        # a bot's record (bots --json), step by step
cargo shuyi import files/stage.map
```

`tests/it/bots.rs` also plays a rollout hero (`shuyi_core::Rollout`)
against the foe: it tries the tiles nearest the goal on copies of the
stage whose luck is redrawn, and wins far more often than the random
walker.

## Targets

`shuyi.ron` lists the desktop, the web and the Steam Deck, and `cargo
shuyi check --target all` passes on each. On the Deck the 384 × 384 frame
is shown at scale 2, so its 16 px text is 32 px. The Playdate is not
listed: the frame is taller than its 240 px screen even at scale 1.
Fitting it would take a smaller console for that target, which this
example doesn't have.

The controls fit every target, the Playdate too. Every screen declares
the actions it reacts to, and `files/keymap.ron` has a table of buttons
for the Playdate (the d-pad, A and B) and one for the Deck. `check
--target` checks that each screen's required actions (all six) bind to
distinct buttons of the target (ADR-0025). `screens::all` lists the
screens it checks.

Its look fits a black and white screen too. `files/palette@onebit.ron`
gives each color of the palette, by its name, a pattern of 8 × 8 pixels
(ADR-0008 §4), and keeps apart what the colors tell apart: your piece from
the foe, the floor from the walls and the goal. A test checks it as
`check --target` checks a one-bit target. `cargo shuyi frame-png
title.png --target playdate` paints the title as the Playdate would show
it, the top 240 pixels of the frame:
`tests/it/snapshots/it__onebit__title_playdate.snap.png`.

## Art and licenses

Terminus Font 4.49.1 (`files-src/ter-u16n.bdf`, unmodified, and the atlas
made from it) is under the SIL Open Font License 1.1:
`files/font/Terminus-LICENSE.txt`. Everything else here is the engine's
own: the tileset is the font's glyphs on the palette's colors, and the
sounds are sine blips the game makes when it starts.

