+++
title = "Fast compiles"
description = "How long a new game takes to rebuild, and the setup in its Cargo.toml behind it."
weight = 200
+++

What a game's developer waits for is the rebuild after a one-line
change in their own game. A game made with `shuyi new` rebuilds in
about a second, on the stable Rust it pins:

| A new `grid` game | Linux | Windows (MSVC) | Windows (GNU) |
| ----------------- | ----- | -------------- | ------------- |
| Rebuild after a one-line change in `main.rs` | 0.7–1.0 s | 1.8–1.9 s | 1.1 s |
| `cargo check` after the same change (what an editor runs) | 0.1 s | 0.2 s | 0.2 s |
| Clean build | 40–55 s | 105–120 s | 25 s |

Linux and Windows (MSVC) are GitHub's 4-core runners; Windows (GNU) is a
24-core desktop. The engine's CI measures a new game's rebuild on every
change to the engine, and fails one that goes over its budget.

## The setup

It is in a new game's root `Cargo.toml`, whose comments say the same:

- **No debug info in `dev` builds** (`debug = false`): it takes about a
  third off each rebuild. A panic still names its file and line; set
  `debug = "line-tables-only"` for lines in backtraces, or `true` for a
  debugger.
- **The engine and every other crate built optimized** (`opt-level = 2`
  for `"*"`), so the game runs at speed in a debug build. Level 1 makes
  no rebuild faster.
- **The linker Rust picks.** On Linux, Rust's own `rust-lld` has been the
  default since Rust 1.90, and `mold` is no faster. On Windows with the
  GNU toolchain, `rust-lld` can't read the import libraries a game
  links, so MSYS2's linker stays; with MSVC, `rust-lld` is no faster
  than `link.exe`.
- **Incremental builds** of the game's own crates, cargo's default for
  `dev`: without them a rebuild takes 10 to 50 % longer.

Linking the engine as one dynamic library, as Bevy's `dynamic_linking`
does, made no rebuild faster: the engine's crates are few and already
built, so linking them statically costs little.

## On a nightly toolchain

Two more options may help, for a developer to try in their own
`.cargo/config.toml`: the Cranelift backend
(`-Zcodegen-backend=cranelift`, with the `rustc-codegen-cranelift`
component) and shared generics (`-Zshare-generics=y`).
