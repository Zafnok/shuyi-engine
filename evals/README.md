# The agent eval

Nick set the engine's end goal on 2026-10-19
([`docs/decisions.md`](../docs/decisions.md), *the end goal*):

> The measure: an agent asked "make me a snake game with Shuyi Engine"
> makes it, with a web build, a Playdate build and the other targets, in
> under an hour and in few lines. Every new API is judged against it.

Real time waits for Visions of Shuyi's launch, so until then the eval
measures turn-based games. This folder holds what it measures with:
tasks, the words an agent gets and the checks its game must pass, and
reference solutions, graded the same way. A change to the engine that
breaks a reference shows. Each reference's lines over its template are
the first numbers of how few lines a game takes. Running a real agent
on the tasks, with its minutes and tokens, is ticket 2525.

## A task

`tasks/<id>.ron`, read by `crates/xtask/src/eval/task.rs` (its types
document every field; an unknown field is refused):

- `prompt`: the words the agent gets, verbatim, and nothing else;
- `template` and `name`: the game it starts from, `shuyi new --template
  <template> --name <name>`, which is also the base its lines are
  counted over;
- `targets`: the target ids the `Targets` check checks;
- `checks`: `Gates`, `Check`, `Targets`, `Web`, `Export(<target>)`,
  `Responds(steps)`, `Replay` and `Scene(steps, has)`. Their steps are
  `Act("<action>")` and `Wait(<seconds>)`, passed to the contract as
  `--act` and `--wait`; never keys;
- `reference`: `Overlay("<folder>")`, only the files that differ from a
  fresh `shuyi new`, laid over one; `Project("<folder>")`, a whole
  project graded where it is; or `None`;
- `waits_for`: tickets it waits for. A task with any is listed, never
  graded;
- `unmeasured`: what of the end goal no check covers yet, with the
  tickets that would.

**A check reads only the contract.** Each verdict comes from a contract
answer or a cargo command's exit, and from what the prompt asks the game
to expose: its action names and its scene data. Never a reference's
files, types or screens, so any agent's game can pass. A prompt names
every fact a check reads (the actions, the screen it opens on, the
scene's data and its values).

To add a task, write its file here with its prompt and checks, and a
reference, or a `waits_for` naming the ticket that gives it one.

## Grading

`eval-grade` grades any game, an agent's among them; `--base` names a
fresh `shuyi new --template <template> --name <name>` to count its lines
over. The box-pushing reference, the tutorial's last step:

```powershell
cargo run -q -p shuyi-cli -- new target/eval/base/box-pushing --template blank --name "Box Pusher" --engine-path .
```

```powershell
cargo xtask eval-grade evals/tasks/box-pushing.ron site/tutorial/10-ship --base target/eval/base/box-pushing
```

It exits 0 when every check passed, 1 when one failed and 2 when it
couldn't grade, and writes `eval-result.json` (`--out`): each check's
verdict, seconds and why it failed, the lines added and removed and the
files changed over the base (builds, `dist/` and `Cargo.lock` left out),
and what is unmeasured. `Export("linux")` builds only on Linux: elsewhere
it fails with the contract's `SH1003`. A game made with `--engine-path`
depends on this checkout's crates by path, so its `cargo fmt --all
--check` checks theirs too. Grading every reference on fresh games, and
in CI, is ticket 3421.
