# Shuyi Engine

> An open-source 2D game engine in Rust, built so that a person and their coding agent can make, run, see, test and replay a game without a window and without guessing.

A game's state changes only through commands (turn games) or inputs per fixed step (real-time games), so the same seed and the same inputs give the same game on every platform. Data, maps, dialogue and test snapshots are plain text. The `shuyi` tool contract makes a game, runs it headless, shows its screen and replays it, as JSON on the command line or as tools over MCP. Games depend on the `shuyi` crate alone. The engine is early and not yet released, and its API still moves; it is under MIT or Apache-2.0.

Start with *Quick start*, under *The book*. Each code block in the book is built and tested with the engine at the same commit.
