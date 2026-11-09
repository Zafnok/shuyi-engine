+++
title = "Testing"
description = "The Harness plays a game with no window, bots play its rules, records replay with a hash per step, and a frame is text or a PNG."
weight = 80

[extra]
part = "chapters"
chapter = 8
+++

{% newcomer(to="programming", topic="what a test is") %}
Once a game is more than a few screens, you can't check it all by playing it after every
change. A **test** is a small program that plays part of your game and checks what
happened, and you run hundreds of them in seconds, every time, with no effort. When you
break something by accident, a test tells you right then, and names the line.
{% end %}

A game on Shuyi Engine is tested with no window and no sound card. Its rules are plain
functions of their state. The game runs headless in the engine's **Harness**, frame by
frame. A frame is text a test can read, and a PNG a person can look at. Tests are the
review: a change is done when a test proves it.

Which test to write depends on what changed:

| What changed | Its test | Reads |
| ------------ | -------- | ----- |
| A rule | a unit test of the rules | the state and the events |
| Something true of every input | a property test | the state |
| A screen, a flow between screens | a Harness test | the screens, a screen's view |
| What the tools answer | a test calling the contract | the answer |
| A look | a text snapshot of the frame, or a picture | the frame |

{% newcomer(to="rust", topic="`#[test]` and `assert_eq!`") %}
A test is a function with `#[test]` above it, which `cargo test` finds and
runs. A test passes if it reaches its end. `assert_eq!(a, b)` stops it, as failed, if `a`
and `b` differ, and `assert!(x)` does if `x` is false. Tests usually sit in a `mod tests`
at the end of a file, or in the game's `tests/` folder.
{% end %}

## The rules

A rule's test builds a state, applies commands and reads what happened
([chapter 5](@/learn/book/rules-as-commands.md#turnrules)). The same seed and the same
commands always give the same state, so such a test never **flakes**, passing one run and
failing the next.

A test you write checks the cases you thought of. A **property** is something that should
hold for any input, and a property test tries hundreds of random ones for you. If one
fails, it shrinks it down to the smallest input that still fails. With `proptest` among
the dev-dependencies (crates only the tests use):

{{ snippet(file="src/testing.rs", anchor="property") }}

{% newcomer(to="rust", topic="`proptest!`, a range and a closure") %}
`proptest!` is a macro from that crate, so what's inside has its own form.
`seed in any::<u64>()` means any 64-bit number, and `commands in
prop::collection::vec(0..3_u8, 0..40)` means a list of up to 39 numbers, each 0, 1 or 2.
`0..3` is a **range**, including its start and not its end. `let run = || { ... };` keeps
a small nameless function that takes nothing, so the test can run the same play twice.
`&n` takes the number out of the reference the loop gives, and `[...][usize::from(n)]`
picks item `n` of a list, after turning `n` into the type used for counting.
{% end %}

## The Harness

On the short path, `game().harness()` starts the game at its first screen with no window,
in frames of 1/60 of a second. A test drives it and reads it:

- `keys("Enter Right")` presses and releases each key chord in turn, by the names of the
  game's default keys; `pad("South")` presses a controller button by its place,
  `act(action)` whatever key the player bound to an action, `hold(chord, seconds)` holds a
  key, and `wait(seconds)` lets time pass;
- `screens()` is the stack, `top_screen()` its top's name, `screen::<T>()` the screen of
  type `T` (one whose `as_any` gives it), `ctx()` the context, `sounds()` the sounds asked
  for, `quit_requested()` whether the game asked to quit, and `snapshot()` the frame as
  text.

[Chapter 6](@/learn/book/games-and-screens.md#a-test-of-it) has a test of the flow, and
[chapter 2](@/learn/book/frame-and-skins.md#why-tests-read-the-view) one that reads a view.

## A look

Most tests shouldn't care how the game looks, so that a new color breaks nothing. A test of
a look is the exception: it compares `h.snapshot()` with a file kept in the repository,
which it writes only when asked (an environment variable the test reads). Read the new file
before keeping it. For a picture:

```bash
cargo shuyi frame-png garden.png --keys "Enter Right Enter" --scale 2
```

That command makes this:

{{ try_it(name="book-frame-png") }}

`--keys`, `--pad`, `--act` (an action by name, pressed by its key) and `--wait` are the steps, `--scale` the size, and `--target <id>` a
target's display. The picture is for people; tests read the state.

That picture is the raster's, painted in software. To check that your window draws the same
pixels, give your game a `capture` feature (`capture = ["shuyi-app/capture"]`) and a plan, a
RON file of the same steps with captures between them:

```ron
(
    steps: [
        wait(1),
        capture("title"),
        keys("Enter"),
        wait(0.25),
        capture("garden"),
    ],
)
```

```bash
SHUYI_CAPTURE=capture/frames.ron cargo run --features capture
```

The game runs the plan frame by frame, with no saves read or written and no sound, compares
each capture's window with the raster, writes both pictures and `capture.txt` under
`target/capture`, and exits 0 when they match, 1 when one doesn't (with a picture of where)
and 2 when the plan can't run. A plan's `tolerance` (`(channel: 2, pixels: 0)` unless it
says) is how far the window may be from the raster. On Linux CI it runs under `xvfb-run`, as
this engine's own `render` job does.

## Bots

{% newcomer(to="games", topic="a bot") %}
A **bot** is a player made of code: it picks the next command from the state. Why would you
want one? It can play your game a thousand times while you watch, finding the rule that
never ends, or telling you how often a player can win.
{% end %}

`shuyi_core::bots::play` lets bots play a game to its end, or to a turn limit:

{{ snippet(file="src/testing.rs", anchor="bot") }}

`check` is what the rules offer to say whether a command would be allowed, without changing
anything.

{% newcomer(to="rust", topic="implementing a trait") %}
`impl Bot<Garden> for Gardener` keeps the promise of the trait `Bot`, for the
garden's rules, with one function: `choose`.
{% end %}

{% newcomer(to="rust", topic="a tuple, `dyn` and a panic") %}
`&mut [((), bot)]` lends a list of who plays: each a **tuple** of a side and a bot. The
garden has one player, whose side is `()`, a value holding nothing. `&mut dyn Bot<Garden>`
means "lend me some type with this trait". `.unwrap_or_else(|e| panic!("{e}"))` opens a
`Result`, and on an error stops the program with its message. A **panic** like this is for
what should never happen, and here that's a game that can't even start.
{% end %}

`RandomBot` picks one of the legal commands from a seed of its own, and the core has bots
that plan on copies of the game. A test then holds the game to how often a bot should win:

{{ snippet(file="src/testing.rs", anchor="test_bot") }}

{% newcomer(to="rust", topic="an iterator chain") %}
`(0..50).map(...).filter(...).count()` plays 50 seeds, keeps the wins, and counts
them. `(10..50).contains(&wins)` asks whether `wins` is in that range. In the text `"{wins}
wins in 50"`, `{wins}` is filled in with its value, printed if the test fails.
{% end %}

In a game whose tools give a playtest (a `grid` game's do), `cargo shuyi playtest --tries 20`
plays each of its named bots 20 times on every core: how each try ended, a record of each
with `--json` for `replay`, and, when the game gives checks, whether each bot wins as often
as the game wants. A `grid` game starts with one bot and no checks, a plain bot run.

## Records and replays

A player reports a bug, and you can't make it happen. If your game is deterministic, you
don't have to guess: the seed and the commands are the whole play, and they fit in a small
file. A **record** of them replays by the game's own rules, with the state's **hash** (a
short number worked out from the whole state) after every step:

{{ snippet(file="src/testing.rs", anchor="replay") }}

`cargo shuyi replay <record>` does the same from the game's tools. Given two records that
should play alike, it answers with the first step whose hashes differ, and that step's
command is where to look. A record that says what made it
([chapter 7](@/learn/book/saves-and-settings.md#a-save)) is refused by another build of the
game ([`SH3007`](@/learn/errors/sh3007.md)) rather than compared.

## Recording a session

A record of commands holds the rules and nothing else. It can't tell you why a menu got
stuck, or why the music and a screen fell out of step. A **session record** holds everything
the platform gave the whole game, frame by frame: the clock, the keys and buttons, what the
system said, the music's position, and the saved files as they were when the game loaded. No
code of your game takes part in making one.

In a test, `Harness::start_recording` starts the game the way a device does, showing its
loading card and then running its load, and records from the very first frame.
`Harness::from_session` makes a new Harness ready to play that record, and `play_all` or
`play_to(frame)` plays it, in the same game:

{{ snippet(file="src/testing.rs", anchor="session") }}

`first.session()` gives the record so far as an `Option`, because a Harness that isn't
recording has none.

{% newcomer(to="rust", topic="`.unwrap()` and `None`") %}
`.unwrap()` opens a `Result` or an `Option`, and stops the test if there's an
error or nothing inside, which is fine in a test: a game that can't start should fail it.
`None` in `assert_eq!(played.first_difference, None)` is that same
nothing: no frame differed.
{% end %}

Each frame of a record also holds a hash of what happened in it: the screens open, the sounds
and music asked for, and whether the game quit. It never covers the picture, so a new color
breaks no record. `played.first_difference` names the first frame that happened differently,
with both hashes, and the game stops on that frame, so a test can read `screens()` right
there. Frames count from 1, and the first two are the card and the load. Until the game has
loaded, `screens()` is empty, and what needs the game itself, such as `ctx()`, stops the test.

`played.limits` lists what the playing can't promise, such as a record made without a stamp
of its build, one that stopped early, or one that doesn't hold all of the saved files. A
record that another build of the game made is refused by `from_session`, unless you pass
`Stamps::Ignore` in place of `Stamps::Check`.

After the record's last frame, the same scripts as ever go on from where it ended: `keys`,
`pad` and `wait`. The music then comes from the Harness's own stand-in for a sound card, and
`limits` says so.

Three things differ from `game().harness()`:

- A key press reaches the screen even when the screen doesn't declare that action, as it does
  on a device.
- `start` and `start_recording` give the load's errors as their `Err`, rather than stopping
  the test. `Harness::start` is the same start with no recording.
- What a test changes behind the platform's back isn't in the record, so it won't play back:
  `act`, `ctx_mut` and `screen_mut`. Press the keys instead.

To keep a record, `record.to_ron()` is its text, one frame per line, so two records compare
frame by frame in any diff tool. `SessionRecord::from_ron` reads it back. The name to give the
file is `*.session.ron`.

### Sessions from your own builds

You don't need a test to get a record. A debug build (what `cargo run` makes) records every
run, and when you quit it writes the record to a folder called `sessions/`, inside the folder
where the game keeps its saves. The last ten stay. A build for players (`--release`) records
nothing, because a record holds what the player pressed and what they saved.

Something went wrong just now and you don't want to quit? If your game has a debug menu, it
lists **Save session**, which writes the record so far and carries on. In a browser, where a
page has no folders, that entry downloads the file instead.

Now play it, from the command line, with the path the game's log printed:

```bash
cargo shuyi run --session path/to/1730000000000.session.ron
```

This runs the game with no window, from its real start, through everything that happened, and
answers with the screens and the scene where the session ended. If this build plays it
differently, say because you changed a rule, the answer isn't `ok`, and its
`session.first_difference` names the frame where the two first parted. Everything else in the
answer is that frame's, so you read the game at the moment it went another way. A record made
by another build of the game is refused, since it may differ for that reason alone; add
`--any-stamp` to play it anyway.

Two things to know about your game:

- `shuyi run` needs to start your game the way your binary does. On the short path it already
  can. If you wrote your own `Start`, move it from `main.rs` into your library, and have your
  tools' `session_from` play the record from it.
- The comparison is made from a **hash** of each frame: a short number worked out from what
  happened in it, such as the screens open and the sounds asked for. To put your rules in it
  too, give the screen that holds them a `state_hash` method. Then a difference is found at the
  frame the rules moved, not only when a screen or a sound shows it.

### Keeping a session as a test

Once you have fixed the bug, the session that found it is a good test: it fails if the bug
comes back. Copy the file into your game's `tests/` folder and play it there:

{{ snippet(file="src/testing.rs", anchor="keep_session") }}

The record is stamped with the build that made it, so any change to your game's files makes
the test refuse it. If that change was meant, make the file again with this build, keeping the
same key presses:

```bash
SHUYI_BLESS=1 cargo test golden
```

`assert_session` does this when `SHUYI_BLESS` is `1`. The test's name has `golden` in it so that
this command finds it. Read what changed in the file before you commit it. If the game now reads
its keys differently, blessing fails and says so, and you record the session again.

## Golden values

A record kept in the repository pins what the game does: its hash at every step, in a
**golden table** beside the tests, the answers known to be right. With `shuyi-assets`'s
`golden` feature in its dev-dependencies, a test reads a table with
`shuyi_assets::golden!("tests/golden/stage.ron")`, checks each row (`check("step 007",
hash)`), and calls `assert`. A table starts as a file holding `{}`; a test that fails prints
the rows that moved and the new table, to copy into the file when the change was meant. A row
that changes is a play that changed: the same seed and commands no longer give the same game,
so old records of it no longer replay. Run the tests on every platform the game ships for, and
the table says whether they all play alike.

## Live, between calls

`cargo shuyi run --script "Enter Right" --frames 30` plays a script once and answers. The MCP
server keeps one game running between calls, for an agent
([chapter 10](@/learn/book/tools-for-agents.md#the-mcp-server)). And a debug build running in
its window can be attached to: built with the game's `link` feature, it listens, and `cargo
shuyi attach` plays steps on it and takes a picture of the real window.

## Running them

`cargo test --workspace` runs every test, and the game's CI runs them on every pull request
with the rest of its gates: formatting, clippy (Rust's checker of good style), `cargo shuyi
check` (with `--target all`) and `cargo shuyi lint`. The game's `AGENTS.md` lists them.

## When a test fails

A test that fails is doing its job. `cargo test` names the test, the file and line, and the
two sides. Here a test expected the title alone on the stack, and the game had pushed the
garden:

```text
---- screens::tests::confirm_opens_the_garden_and_leaving_asks_first stdout ----

thread 'screens::tests::confirm_opens_the_garden_and_leaving_asks_first' (74264) panicked at src\screens.rs:317:9:
assertion `left == right` failed
  left: ["title", "garden"]
 right: ["title"]
```

`left` is the first thing you gave `assert_eq!`, what the game did, and `right` is what you
said it should be. Decide which is wrong, then fix that one. Run just this test while you do:

```bash
cargo test confirm_opens
```

A few others you will meet:

**A key name the keymap doesn't know.** Key names in a test script come from the same list as
the keymap's, so a typo stops the test, and says what it would take:

```text
bad chord in test script: unknown key chord "Entr"; expected a key such as "h", "0", ";", "Left", "Enter", "Escape", "Space", "Tab", "Backspace" or "F1", optionally prefixed by "Shift+"
```

**`called Option::unwrap() on a None value`** on `h.screen::<GardenScreen>()`. Either that
screen isn't on the stack, so print `h.screens()`, or it has no `as_any` to be found by
([chapter 6](@/learn/book/games-and-screens.md#asking-the-screen-below)).

**A key that does nothing.** `game().harness()` hands a screen only the actions it declares, so
look at its `actions` ([chapter 4](@/learn/book/actions.md#when-it-goes-wrong)).

**A test that passes one run and fails the next.** In a game like this one, that isn't bad
luck: something outside the state got in, such as a clock or a hash map's order
([chapter 5](@/learn/book/rules-as-commands.md#when-it-goes-wrong)).
