# AGENTS.md: the rules

`crates/rules` is the game's rules: pure and deterministic. No I/O, no
clock, no floats, no randomness but the seeded `SimRng` kept in the
state. The state changes only through commands (`TurnRules::apply`). The
same seed and the same commands always give the same game, which
replays, bots and tests rest on.

Clippy (`crates/rules/clippy.toml`) and the layers test
(`crates/rules/tests/layers.rs`) turn these rules into errors, each
saying what to use instead, and `cargo shuyi lint` reports them with
their file and line. Fix a failure in the code, never by loosening
`clippy.toml` or the test (a test that must use a float writes
`#[expect(clippy::disallowed_types, reason = "...")]`).

The stage that plays these rules is `crates/game/src/stages/top_down_grid.rs`,
and its tools (`top_down_grid/tools.rs`) give `cargo shuyi scene`,
`replay`, `bots` and `import` for it:

| Command | What it tells you |
| ------- | ----------------- |
| `cargo shuyi run --script "Enter Right Down" --frames 30` | the screens, the scene, what happened and the sounds |
| `cargo shuyi scene --script "Enter Right"` | the stage's map and state |
| `cargo shuyi bots 20` | how 20 games played by bots ended, with a record of each |
| `cargo shuyi replay record.ron` | a hash of the state at every step of a record |
| `cargo shuyi frame-png stage.png --script "Enter Right"` | a picture of a frame, with no window |
| `cargo shuyi import crates/game/files/stage.map` | what a file reads as, or its errors |
