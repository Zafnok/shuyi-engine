+++
title = "Fast compiles"
description = "How long a new game takes to rebuild, and the setup in its Cargo.toml behind it."
weight = 200
+++

You change one line, you want to see it, and you wait. What you wait for is a
**rebuild**: Cargo compiling what changed and joining it to the rest, a job called
**linking**. It's the wait you'll meet a hundred times a day, so it's the one that
matters. A game made with `shuyi new` rebuilds in about a second, on the stable Rust it
pins:

| A new `grid` game | Linux | Windows (MSVC) | Windows (GNU) |
| ----------------- | ----- | -------------- | ------------- |
| Rebuild after a one-line change in `main.rs` | 0.7–1.0 s | 1.8–1.9 s | 1.1 s |
| `cargo check` after the same change (what an editor runs) | 0.1 s | 0.2 s | 0.2 s |
| Clean build | 40–55 s | 105–120 s | 25 s |

A **clean build** compiles everything, the engine included, as your first build does.
`cargo check` looks for errors and doesn't make a program, which is why it's so quick.
MSVC and GNU are Rust's two toolchains on Windows
([chapter 1](@/learn/book/quick-start.md#install-rust)).

Linux and Windows (MSVC) are GitHub's 4-core runners; Windows (GNU) is a 24-core desktop.
The engine's CI measures a new game's rebuild on every change to the engine, and fails one
that goes over its budget.

## The setup

You don't need to do anything for this: it comes with a new game. It is in the root
`Cargo.toml`, whose comments say the same, and you can read on if you want to change it.

- **No debug info in `dev` builds** (`debug = false`). `dev` is the build `cargo run`
  makes, and **debug info** is the extra data a debugger reads. Leaving it out takes about
  a third off each rebuild. A panic still names its file and line; set `debug =
  "line-tables-only"` for lines in backtraces, the list of what was running when it
  stopped, or `true` for a debugger.
- **The engine and every other crate built optimized** (`opt-level = 2` for `"*"`), so the
  game runs at speed in a debug build. They rarely change, so you pay for that once. Level
  1 makes no rebuild faster.
- **The linker Rust picks.** On Linux, Rust's own `rust-lld` has been the default since
  Rust 1.90, and `mold` is no faster. On Windows with the GNU toolchain, `rust-lld` can't
  read the import libraries a game links, so MSYS2's linker stays; with MSVC, `rust-lld` is
  no faster than `link.exe`.
- **Incremental builds** of the game's own crates, cargo's default for `dev`, which reuse
  what didn't change: without them a rebuild takes 10 to 50 % longer.

Linking the engine as one dynamic library, a file loaded when the program starts, as Bevy's
`dynamic_linking` does, made no rebuild faster: the engine's crates are few and already
built, so linking them statically, into the program, costs little.

## When it's slow

- **The first build takes minutes.** That's the clean build, and it happens once per
  folder, and again when you change the Rust version, or run `cargo clean`.
- **A rebuild takes much longer than the table.** See whether you added a dependency, which
  has to compile too, and whether you changed `debug` or an `opt-level`. `cargo build
  --timings` writes a page saying where the time went.
- **`cargo run --release` is slow to build.** A release build is the one you ship: it
  optimizes everything, so it takes a lot longer. Use it for shipping, and for measuring
  speed.

## On a nightly toolchain

**Nightly** is the Rust that's built each night, with features not yet finished. Two more
options may help, for a developer to try in their own `.cargo/config.toml`: the Cranelift
backend (`-Zcodegen-backend=cranelift`, with the `rustc-codegen-cranelift` component) and
shared generics (`-Zshare-generics=y`).
