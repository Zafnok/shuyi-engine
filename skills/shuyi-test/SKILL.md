---
name: shuyi-test
description: Test a Shuyi Engine game at every layer, with unit and property tests of its rules, its screens played through the engine's headless Harness, the tool contract in tests, text snapshots of a look, and pictures of frames to review. Use when adding or changing rules, a screen or a stage, when a change needs a test that proves it, or when you want to see a frame without a window. Not for running the checks before a commit (shuyi-gates), for a bug only a record reproduces (shuyi-debug-replay), or for measuring how hard a stage is (shuyi-balance-bots).
license: MIT OR Apache-2.0
metadata:
  version: "1.1"
---

# Test a game

Tests are the review: a change is done when a test proves it. A game on
Shuyi Engine is tested without a window or a sound card. The rules are
plain functions of their state. The game runs headless in the engine's
Harness, frame by frame. A frame is text a test can read, and a PNG you
can look at.

## When to use it

- A rule changed or was added: test it with commands in, state and
  events out.
- A screen or a stage changed: play it through the Harness and read what
  happened.
- A look changed: keep a text snapshot, or look at a picture of the
  frame.

## When not to

- The checks to run before a commit: `shuyi-gates`.
- A bug a record reproduces, or two runs that differ:
  `shuyi-debug-replay`.
- How often bots win a stage: `shuyi-balance-bots`.

## Which test for what

| What changed | Its test | Reads |
| ------------ | -------- | ----- |
| A rule | a unit test in the rules crate (`crates/rules`) | the state and the events |
| Something true of every input | a property test (`proptest`) | the state |
| A screen, a stage, a flow between screens | a Harness test in `crates/game/tests/` | the screens, the screen's state or view |
| What the contract answers | a test calling `shuyi_cli::contract::*` | the answer |
| A look: layout, words on screen, colours | a text snapshot kept in the repository, or `frame-png` | the frame |

Tests of what happened read the state or the view, never the frame: a
new word or colour must not break them. Only tests of a look read cells,
colours or the frame's text.

## Steps

1. **The rules first.** In `crates/rules`, a test builds a state (its
   map from a string, its luck from a seed), applies commands with
   `TurnRules::apply`, and checks the events, the state and the
   `outcome`. Check a refused command too: `check` refuses it and
   nothing changes. The same seed and the same commands always give the
   same state, so a test never flakes.

2. **Properties.** Add `proptest = "1"` under `[dev-dependencies]` of the
   crate it tests (never `[dependencies]`: the rules may hold no outside
   luck). A property holds for any input: any seed and any steps give the
   same state twice, a piece stands only on open tiles, a score never
   goes down.

3. **The game through the Harness.** On the short path,
   `game::game().harness()` starts the game at its first screen with no
   window. A test drives it and reads it:
   - `keys("Enter Right")` presses and releases each key chord in turn,
     `pad("South")` each controller button, `act(UiAction::Confirm)`
     whatever key the player bound to an action, `wait(0.5)` lets time
     pass, `hold(chord, seconds)` holds a key;
   - `screens()` is the stack, `top_screen()` its top's name,
     `screen::<T>()` the screen of type `T` (one that gives `as_any`),
     `ctx()` the shared context, `sounds()` the sounds asked for,
     `quit_requested()` whether the game asked to quit, and `snapshot()`
     the frame as text.

   Key names in a test are the Harness's test-script names, pressing the
   game's default keys (`files/keymap.ron`): input to a test, not game
   code. A game with stages seeds them: a `grid` game's stage tools give
   `Tools::harness(seed)`.

4. **The contract in tests.** The template's tests call the operations
   as library functions with the game's tools: `check(&tools,
   &CheckInput::default())`, `run(&tools, &input)` with `input.script =
   keys("Enter Right")`, `bots`, `replay` and `import`, each answer's
   `ok`, `result` and `limits`. `crates/game/tests/` in a new game shows
   them.

5. **A look as text.** `h.snapshot()` is the frame as text. Compare it
   with a file kept in the repository, written only when asked (an
   environment variable the test reads), and read the new file before
   keeping it. A changed word or layout fails it; write it again only if
   the change was meant.

6. **A look as a picture.** Render a frame with no window, and look at
   it:

   ```bash
   cargo shuyi frame-png stage.png --keys "Enter Right"
   cargo shuyi frame-png stage.png --keys "Enter Right" --scale 2 --seed 3
   ```

   `--keys`, `--pad` and `--wait` are the steps, `--scale` the size,
   `--target <id>` a target's display (a one-bit one, such as the
   Playdate's, needs the game's one-bit skin), and the game's own
   options (`--seed`) follow (`cargo shuyi frame-png --help` lists
   them). The picture is for review: tests read state.

7. **Play it live, headless.** The engine's MCP server keeps one game
   running between calls: `start` (with the game's options), `step` (a
   script, then idle frames), `scene` (the scene and the log),
   `frame_png` (a picture of the frame now), `events`, `reset` and
   `stop`. Call `start` again after changing the game's code.
   `cargo shuyi run --script "Enter Right" --frames 30` gives the same
   answer once, for a script.

8. **A running build, in its window.** The headless runs above test a
   change. To look at what only the window shows (the real renderer, its
   size, a stutter), attach to a debug build that is running:

   ```bash
   SHUYI_LINK=0 cargo run --features link
   ```

   The game crate's `link` feature turns the live link on, in a debug
   build only; `SHUYI_LINK` is the port (`0` takes a free one). Its log
   says `shuyi link: listening on 127.0.0.1:<port>, token <token>`. Then:

   ```bash
   cargo shuyi attach 127.0.0.1:<port> --token <token> --act Confirm --screenshot shot.png
   ```

   plays the steps as a player's input (`--keys`, `--pad`, `--wait`, and
   `--act` presses an action by its key), then answers with the screens,
   the frame as text and the log (`--log-since` the last `log_next`
   reads on), and writes the window's picture to `shot.png`. Waits count
   the build's own frames. It changes nothing else in the build; start
   the build again after changing the code.

9. **Run them:** `cargo test --workspace`, or one file with `cargo test
   -p <crate> --test <name>`. A game's CI runs every test on every pull
   request.

## When it fails

A test panics with its message. The contract's commands answer with a
`code`: branch on it, and read its page with `cargo shuyi explain
<code>`.

- `SH0001`: the command is wrong, such as a key name the Harness
  doesn't know (`--keys: unknown key chord`); the answer's `help` lists
  the steps it takes.
- `SH3001`: the game's tools don't offer the operation (bots or a
  replay for a game with no rules).
- `SH3005`: the game's tool program didn't build: the answer ends with
  the compiler's output.
- `SH1005`: `attach` found no build listening: start it with the link
  first. `SH0004`: the token isn't the build's latest.

A `run` that is `ok` but whose `limits` say "no scene" ran on a screen
that shows none: open the stage first (`--script "Enter"`).

## Worked example

[`example.ron`](example.ron) runs on a new `grid` game. It adds
`proptest` to the rules' dev-dependencies, and tests of the rules (a
corridor walked to the goal, a wall that refuses a step, and a property
over any seed and steps). It adds Harness tests of the stage (a step,
an action, Cancel back to the title and out), and a text snapshot of the
stage. The snapshot fails until it is written on purpose, then passes,
then fails when the stage's words change while the Harness test still
passes. A `frame-png` picture of the stage follows, then a `run` whose
scene agrees with the tests' state, and `fmt`, clippy and `lint`.

[`example-attach.ron`](example-attach.ron) attaches to the game running
in its window, so CI can't run it (it is marked manual): start the game
with the link as its first comment says, then run it.
