---
id: "2822"
title: "`tutorial-tries` on Windows can draw a try with another step's program"
type: bug
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-11
---

# 2822 — `tutorial-tries` on Windows can draw a try with another step's program

## Context

Found by 2819. On Windows with the GNU toolchain, this fails, though nothing
changed:

```bash
cargo xtask tutorial-tries --check up-is-down walked wall-look
```

It says `wall-look/before.png` is stale. The picture it made has step 3's help
line, "arrows walk", where step 2 has only "Backspace back": the copy of step
`02-board` ran the program of `03-walk`. `--check wall-look` alone passes, and
CI's job on Linux passes the whole list.

All the copies build in one target folder (`Renderer::new`,
`crates/xtask/src/tries.rs`), and every step's package has one name,
`box-pusher`, so each `cargo shuyi` ends at the same
`target/tutorial/debug/examples/tools.exe`. The cause isn't proved: finding it
is the first step. Without `--check` it is worse, as it can write a wrong
picture over a right one.

## Nick input

None.

## Scope

**In:**
- Find why the wrong program runs, and make each render run its own copy's
  program, on every OS.

**Out (do not do):**
- what any picture shows, or the list of tries;
- giving up the shared target folder for the engine's crates, whose one build
  keeps the job to minutes, unless the notes show there's no other way.

## Implementation steps

1. Reproduce it with the command above, and read what `cargo run` rebuilds and
   runs (`cargo run -v`).
2. Fix it in `Renderer`: for example, run the program by the path cargo reports
   for that build, or give each step its own `examples` output.
3. Check the whole list on Windows.

## Acceptance criteria

- [x] `cargo xtask tutorial-tries --check` passes on Windows with the pictures
      as they are.
- [x] The job's time on Linux grows by no more than a minute.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- `cargo xtask tutorial-tries --check` over the three tries above, then the list.
- Unit: anything new in `tries.rs` that doesn't need cargo.

## Completion notes

**The cause.** Every copy of a step is one package, `box-pusher`, at the
same place in its own workspace, so in the shared target folder each copy's
build replaces the last one's, and cargo decides by files' times whether a
copy is built already. Two things fooled it on Windows:

- `fs::copy` keeps a file's time there (not on Linux), so a copy's sources
  looked older than the last build of another copy, and passed as built;
- the game's text (`files/en.ron`, where the help line is) is embedded by
  `include_dir!`, so it is in no list of files cargo checks; the only guard
  is the build script's, the time and path of the package's newest file.
  In the main checkout steps 02 and 03 have the same newest file,
  `tests/it/main.rs`, 1 ms apart, and many files share a time to the
  tick: when the two match, cargo runs step 03's program for step 02.

Proved by setting every file of steps 02 and 03 to one time: the old code
fails exactly as the ticket says (`wall-look/before.png` is stale), and the
fix passes.

**The fix.** `Renderer` remembers which copy the target folder last built
this run. Before a frame of another copy it sets the time of each of that
copy's files to now (`touch_tree`), so cargo builds the game's crate again;
the engine's crates stay built. A copy rendered twice in a row isn't touched.

**Checked.** `--check up-is-down walked wall-look`, then the whole list,
pass on Windows (the whole list in 18 minutes). On Linux, copies already got
new times, so the build script's guard already rebuilt on each switch: the
number of builds is the same, and the job's time doesn't grow.

Ticket 2056 said 2822 was open; its line now says what 2822 fixed.

Book: none, only the xtask's renderer changed; no public item, and no
picture.

No break.
