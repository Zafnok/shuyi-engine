---
id: "2822"
title: "`tutorial-tries` on Windows can draw a try with another step's program"
type: bug
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] `cargo xtask tutorial-tries --check` passes on Windows with the pictures
      as they are.
- [ ] The job's time on Linux grows by no more than a minute.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `cargo xtask tutorial-tries --check` over the three tries above, then the list.
- Unit: anything new in `tries.rs` that doesn't need cargo.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
