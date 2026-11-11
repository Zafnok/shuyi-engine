+++
title = "A game and its screens"
description = "Screens on a stack, the transitions between them, the context they share, and text by key."
weight = 60

[extra]
part = "chapters"
chapter = 6
+++

{% newcomer(to="shuyi", topic="the stack of screens") %}
Think of what a game shows you: a title, then the game itself, then perhaps a pause
menu on top of the game, then "Are you sure?" on top of the menu. When you back out,
each goes away and the one below is there again, as you left it.

That is exactly how a game on Shuyi Engine is built. Each of those is a **screen**,
and the screens sit on a **stack**, one on another, like a pile of cards. Only the
screen on top reads the player's input, and the screens below it wait. They all
share one **context**, `Ctx`, which holds the game's data and everything the engine
offers them.
{% end %}

Moon Garden has three screens. This is the second, the garden:

{{ try_it(name="book-garden") }}

## A screen

A screen is a Rust type with a few jobs. Here is Moon Garden's title, the whole of it.
Don't worry about reading all of it at once, as the notes below take it apart:

{{ snippet(file="src/screens.rs", anchor="title") }}

A screen has four jobs:

- `name` is a stable id in `snake_case` (lowercase, words joined by `_`), for tests
  and the tools. The comment above it tells the text check that this isn't text a
  player reads.
- `actions` declares every action the screen reads. An **action** is something a
  player can ask for, like Confirm. The tools check that they fit each target's
  buttons, and in tests a screen gets only the actions it declares
  ([chapter 4](@/learn/book/actions.md)).
- `update` runs once a frame while the screen is on top. It reads the frame's actions
  and answers with a `Transition`, what the stack should do next. Cancel quits only
  where `ctx.platform.can_quit()`: on the web, the player closes the tab instead
  ([chapter 9](@/learn/book/targets-and-export.md)).
- `draw` paints the screen into the frame. Here it paints the view, plain data built
  by `view`. [Chapter 2](@/learn/book/frame-and-skins.md) says why the two are apart.

{% newcomer(to="rust", topic="a `struct`, `impl` and a trait") %}
A `struct` groups named values into one type, and `TitleView` has three
`String`s, pieces of text. `pub struct Title;` is a struct with nothing in it, as
the title has nothing to remember. `#[derive(...)]` asks the compiler to write some
standard abilities for you, such as `Clone` (make a copy) and `PartialEq` (compare
with `==`).

`impl Title { ... }` adds functions to a type. `impl Screen<Data> for Title` is
different: `Screen` is a **trait**, a list of functions a type promises to have, and
this keeps that promise. `self` is the value a function was called on. `&self` means
it only looks, and `&mut self` means it may change it, and the compiler holds you to
that: `draw` takes `&self` and `&Ctx`, so a screen can't change anything while it
draws. The project's [step 1](@/learn/book/box-pusher/01-new.md#the-screen-s-jobs) goes through a
screen like this one line by line.
{% end %}

{% newcomer(to="rust", topic="`Option`, a closure and `i32`") %}
`Option<Vec<UiAction>>` is maybe a list of actions, and `Some(...)` means here it is.
`&&` means "and also". `return` hands back an answer early. `&'static str` is a piece
of text that's built into the program. `let c = |color| ...` is a small
nameless function, kept in a name for the next few lines. `i32` is a whole number
that may be negative, and `i32::from` converts to it.
{% end %}

## The stack

`update` answers with what the stack does next, a `Transition`.

{% newcomer(to="rust", topic="an `enum`") %}
A `Transition` is an **enum**, a type that's exactly one of a fixed list of
possibilities.
{% end %}

| `Transition` | Does |
| ------------ | ---- |
| `None` | stays |
| `Push(screen)` | puts a screen on top of this one |
| `Pop` | closes this one: the one below is on top again; the last one quits |
| `Transition::pop_with(value)` | closes this one and hands `value` to the one below |
| `Replace(screen)` | swaps this one for another |
| `Quit` | leaves the game |

`Push` holds the screen in a `Box`, written `Box::new(Title)`.

{% newcomer(to="rust", topic="a `Box`") %}
Screens are different types with different sizes, and a box lets the stack hold any
of them.
{% end %}

A screen whose `is_overlay` is `true` lets the one below show through: the stack draws
the one below first, then it. That's one word in the code, and you can see it:

{{ try_it(name="book-overlay") }}

## Asking the screen below

Often a screen wants to ask something and act on the answer: "Leave the garden?". You
could put a yes-or-no mode in the garden itself, but then every screen that asks a
question grows a mode. Instead, a question is a screen of its own. The garden pushes
"Leave the garden?" over itself when the player presses Cancel:

{{ snippet(file="src/screens.rs", anchor="leave") }}

{% newcomer(to="rust", topic="`for` and `match`") %}
`for &action in &input.actions` runs once for each action this frame. A
`match` picks the arm that fits, and must handle every possibility, which is why
there's an arm of `_ => {}`: `_` is "anything else", and `{}` is doing nothing.
{% end %}

The question pops with the answer, and the stack hands it to the screen below in
`uncovered`. The garden reads it as the type the two agree on, here a `bool`.

{% newcomer(to="programming", topic="a `bool`") %}
A `bool` is true or false.
{% end %}

{{ snippet(file="src/screens.rs", anchor="garden") }}

The transition `uncovered` returns applies at once, so a screen acts on the answer in
the same frame. A screen also hears when one is pushed over it, in `covered`.

{% newcomer(to="rust", topic="`Self`, a match guard, `Result` and `dyn Any`") %}
There's a lot here, so one at a time.

`Self` is the type this `impl` is for, and `Self { garden: ... }` makes one.
`pub(crate)` means only code in this crate may reach the field. `_` as a parameter's
name, as in `_: &mut Ctx<Data>`, means a function takes the value and doesn't use it.

`UiAction::Confirm if self.garden.outcome().is_some() => ...` is a match arm with a
**guard**: it only fits if the `if` is true too, and otherwise the next arm gets its
chance. `continue` skips to the next time round the loop.

`self.garden.apply(&command)` gives back a `Result`, either a value (`Ok`) or what
went wrong (`Err`). Rust warns if you ignore one, and `let _ =` says you mean to.

`::<bool>` in `ScreenResult::take::<bool>` names a blank in that function, the type
to look for. `dyn Any` is Rust's "a value of some type, to be asked which".
{% end %}

`result.and_then(ScreenResult::take::<bool>)` opens the `Option`, if it's there, and
takes a `bool` out of it, if that's what it holds. If the question had popped with
some other type, this would be `None`, and the garden would stay.

`as_any` lets a test find this screen on the stack, with its real type. Without it, a
test's `h.screen::<GardenScreen>()` gets a `None`.

The garden's state isn't the screen's to change: it belongs to the rules, and the
screen changes it only by sending commands.
[Chapter 5](@/learn/book/rules-as-commands.md) writes them.

## The context

{% newcomer(to="shuyi", topic="why one context and no globals") %}
Every screen needs to reach things: the game's text, its colors, the player's saves.
Many engines keep these in **globals**, values anyone can reach from anywhere, which
makes it hard to tell what a piece of code depends on, and hard to run two games side
by side in a test. Here they all come in one value, handed to you.
{% end %}

Every `update` gets the context `&mut`, so it may change it, and every `draw` gets it
`&`, so it may only look. It is a plain struct, with no globals behind it:

- `ctx.game` is the game's data, the type the game names (`Data`); on the short path a
  `BasicData`, whose `own` is the game's own part;
- `ctx.text(key)` and `ctx.text_with(key, args)` give text by key in the player's
  language;
- `ctx.palette` gives colors by name, `ctx.keymap` the player's keys, `ctx.storage` the
  player's saves ([chapter 7](@/learn/book/saves-and-settings.md)), `ctx.settings()`
  the player's settings;
- `ctx.audio` takes the sounds a screen asks for, and `ctx.log(…)` writes a line to the
  engine's log.

A screen asks for music by its cue in the game's audio manifest:
`ctx.audio.play_music("garden")` switches to a track, and `ctx.audio.stop_music()` fades
it out, both with the manifest's default fade. A screen that wants a slower or quicker
fade names its own, for that switch or stop only:

{{ snippet(file="src/screens.rs", anchor="music") }}

A quiet moment before the next track is the screen's to time: it asks for the next
track when the moment is over.

A game can also make a sound while it runs, from samples rather than a file: a synth, a
mod's sounds, sounds read from a file the player gives it. `ctx.audio.add_sound(cue,
samples)` makes the samples a cue, and from then on it plays as a cue of the manifest
does. `Samples::new` takes 16-bit samples, and `Samples::from_u8` 8-bit ones, which old
formats keep:

{{ snippet(file="src/screens.rs", anchor="chime") }}

A sound made this way plays a minute at most, and those made hold 32 MiB at most,
together; past that, or under the name of a cue the manifest has, it is refused and the
log says why. `ctx.audio.remove_sound(cue)` frees it, and a play of it after that plays
nothing. In a test, `h.added_plays()` gives each play's samples.

A line of dialogue may have a voice: `ctx.play_voice(line)` says it, if the game has a clip
for it. A screen that moves on by itself once the line is said, as a visual novel's auto
mode does, asks `ctx.voice_playing()`, which names the line whose clip is sounding, or
`None`. The platform reports it before each frame, so a screen sees its own `play_voice`
from the next frame on: `None` is "over" only once the line has been heard. A clip whose
file is missing, or that loads too late, never sounds, so the wait also ends when the line
hasn't started half a second after it was asked for:

{{ snippet(file="src/screens.rs", anchor="voice") }}

In a test, `h.voice_length(seconds)` gives every clip that length, and `h.wait` steps past
its end.

## Text by key

No screen writes its text in its code.

{% newcomer(to="games", topic="why text is kept out of code") %}
If it did, translating the game would mean hunting through all of it, and fixing a
typo would mean a rebuild.
{% end %}

The text is in the game's language file, `files/en.ron`, each line with a name,
called its **key**:

{{ snippet(file="files/en.ron", anchor="title", language="ron") }}

`ctx.text("title.name")` is a key's text. `ctx.text_with(key, args)` fills its
placeholders: each `{name}` that `args` names, as in `&[("seeds", &seeds)]`, and each
action's name, as `{Confirm}`, with the player's own key for it. In a `grid` game,
`cargo shuyi lint` finds text written in code (a `blank` game's `shuyi.ron` has no
`check_text` yet, and `lint` says so). The keys `keys.not_mapped`, `keys.arrows`, `keys.chord_join`
and `layout.arrows` are words the engine's own screens and help lines ask the game for.

{% newcomer(to="rust", topic="a tuple and `&[]`") %}
`&[("seeds", &seeds)]` lends a list of pairs. `("seeds", &seeds)` is a
**tuple**, values side by side: the placeholder's name, and what to put there. `&[]`
lends an empty list, for a line with only actions to fill in.
{% end %}

## Every screen, on a list

`game()` names every screen the game can show (its `.screens(…)`, in
[chapter 1](@/learn/book/quick-start.md#what-it-is-made-of)). `cargo shuyi check
--target` checks the controls of each screen on that list, so a new screen goes on it.

## The engine's own screens and widgets

Before writing a screen, look for it in the engine. The options screen, key rebinding,
the layout picker and player profiles are in `shuyi::ui::screens`, and a menu, a list's
focus, a question box, name entry and word wrap are in `shuyi::ui::widgets`.

{% newcomer(to="games", topic="a widget") %}
A **widget** is a part of a screen, as a menu is.
{% end %}

Each takes the game's rows or text, and says what it shows as a view.

Word wrap measures in cells of the screen, with `ctx.widths()`: a Japanese character
takes two. English breaks at spaces. Japanese has none, so it breaks between any two
characters, but never before `。` or `、` and never after `「`, as a reader of Japanese
expects. A key's name filled into a text, such as `Left Shift`, is never split across
two lines.

## A preview for your writer

A writer who doesn't program can't run your source. Hand them a **dialogue preview**
instead: a small program, built from your game's code, that lists the scenes of the
`.dlg` files in a folder and plays the one they pick in your own dialogue screen. They
save a file in their editor, and a second later it plays again from the same text box.
A switch sends away each character the scene's `@if` blocks test, so they read the
other part; the Lines tab jumps to any text box, or to the one a typed line number falls
in; a strip over the scene shows the file, the line and how the line is said; and the
Problems tab lists what the format's checks and yours find, with the file and line,
while the last good version keeps playing.

The engine has the program's screen, `DialoguePreview`. You hand it what only your game
knows, with the trait `PreviewGame`: the rules your scenes are checked against, and the
screen that plays one. Here is Moon Garden's dialogue screen:

{{ snippet(file="src/preview.rs", anchor="talk") }}

and what it hands the preview:

{{ snippet(file="src/preview.rs", anchor="preview") }}

`play.player(Plain)` starts the scene where the preview asks: from its text box, for
those there. `Showing::of` tells the preview which line is on screen, so a saved file
plays again from it. Your game's own lines (a `Directive` type), checks over every scene
(`problems`) and named switches (`switches`, a hero the player names or not, say) go
in the same trait.

Every control is one of the six moves of `UiAction`, so your keys work as they are:
the cursor moves and changes the tab, Confirm picks, and Cancel brings the tabs back
over the scene. The preview's words are text keys in your language files:

{{ snippet(file="files/en.ron", anchor="preview", language="ron") }}

The program is a second binary of your game, beside its own: `src/bin/preview.rs`, whose
`main` is one call.

{{ snippet(file="src/preview.rs", anchor="main") }}

`run_dialogue_preview` runs it as `run_game` runs the game: the same window, files and
settings, with the preview as its first screen, your game's modules, and no debug
screens. It reads the folder with `shuyi::app::read_watched_folder`, which watches it in
an optimized build too ([chapter 3](@/learn/book/assets-and-formats.md#reloading-while-the-game-runs)),
and a `DialogueFolder` module reads its `.dlg` files again when one changes. A folder
that can't be read shows why in the window, as a broken file of your game does.

A `.dlg` file dropped on the window joins the folder, as if it were saved there. On the
[web](@/learn/book/targets-and-export.md#the-web-build), which has no folders, that is how a writer
uses it: the preview starts with no scenes, its Scenes tab says to drop a file, and the
writer drops their `.dlg` files on the page. Changed, a file dropped again plays again
from the same text box. The tab's note there is the key `preview.note.no_scenes_drop`;
a language file without it shows `preview.note.no_scenes` instead.

Build the program you hand out as an
[edition](@/learn/book/targets-and-export.md#a-public-demo) that leaves your game's own
scripts out: its `bin` is the preview, and its `content` a folder of your files without
the scenes, so the program holds only the scenes in the writer's folder.

A test of it plays the preview through the Harness, which `preview_harness` starts on
your `BasicGame` over a folder you make in the test. `h.drop_file(name, bytes)`, or an
input script's `drop` step, drops a file on it:

{{ snippet(file="src/preview.rs", anchor="test") }}

## Modules

The default engine stays small, so a game doesn't pay for what it doesn't use.
Everything else is a **module**: a crate of the engine that a game turns on as a
**feature** of `shuyi`, such as physics or online play.

{% newcomer(to="rust", topic="a crate's features") %}
A feature is an optional part of a crate.
{% end %}

`cargo shuyi modules` lists them by concern (perspective, bodies and motion, online
play, kind of game, scripted rules), each with its status, and `cargo shuyi add <module>` turns one on
in the game's `Cargo.toml` and copies its skill into `.claude/skills/`.

A game installs the modules it turned on once, with `.modules(shuyi::enabled_modules)`
on its `BasicGame`, so turning one on needs no change to its code. A module joins the
running game with hooks of its own, functions the engine calls at set moments, and
keeps its state on the context, where `ctx.module_state::<T>()` finds it.

Some modules join nothing: they are libraries the game's rules call. `shuyi-script`
(feature `script`) is one. It runs rules a game's modders write as text, in the
language [Rhai](https://rhai.rs), on a view the rules hand it, and gives back plain
values the rules turn into commands. A script can't change the game, read a clock or
draw a random number, so a replay with scripted rules stays exact.

## A test of it

{% newcomer(to="programming", topic="a test") %}
A **test** is a small program that runs your code and checks what happened.
{% end %}

A test of screens plays the game with no window through the engine's **Harness**,
pressing the game's default keys by name, and reads the stack:

{{ snippet(file="src/screens.rs", anchor="harness") }}

{% newcomer(to="rust", topic="`#[test]`, `assert_eq!` and `let mut`") %}
`#[test]` marks a function for `cargo test` to run. `assert_eq!` stops the
test, as failed, if its two sides aren't equal. `h` needs `let mut`, as pressing keys
changes it: in Rust, a name can't change unless it says it may.
{% end %}

[Chapter 8](@/learn/book/testing.md) has the rest of the Harness.

## When it goes wrong

**A screen forgets one of its jobs.** Leave out `draw`, and the compiler says so, and
says what to write:

```text
error[E0046]: not all trait items implemented, missing: `draw`
   --> src\screens.rs:102:1
    |
102 | impl Screen<Data> for GardenScreen {
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ missing `draw` in implementation
```

**`update` doesn't answer.** Leave out `Transition::None` at the end, and `update`
ends on its loop, which gives back nothing, `()`:

```text
error[E0308]: mismatched types
    | |_________^ expected `Transition<BasicData<UiAction, Plan>>`, found `()`
```

{% newcomer(to="rust", topic="a function's last line") %}
The last line of a function is what it gives back.
{% end %}

**A `match` misses a possibility.** Remove the `_ => continue` arm, and the compiler
lists the actions you left out:

```text
error[E0004]: non-exhaustive patterns: `shuyi::prelude::UiAction::Up`, `shuyi::prelude::UiAction::Down` and `shuyi::prelude::UiAction::Debug` not covered
```

{% newcomer(to="rust", topic="why a missed possibility is an error") %}
It's one of Rust's most helpful errors, and it means that adding a new possibility to
an enum shows you every place that must handle it.
{% end %}

**A screen tries to change something while drawing.** Rust stops it, because `draw`
only has `&self`:

```text
error[E0596]: cannot borrow `self.garden` as mutable, as it is behind a `&` reference
```

**A text key isn't in the file.** This one compiles, and `cargo shuyi check` passes.
Running the game stops it in a debug build, with a **panic** that names the key:

```text
text key not in the English text: "title.name"
```

{% newcomer(to="rust", topic="a panic") %}
A panic is Rust's way of stopping when something that should never happen did.
{% end %}

So run the screens you change, or better, test them.

**An action's name is misspelled in the text.** `{Confirn}` names no action, and
`cargo shuyi check` fails on it, with its line:

```text
check: 1 error(s)
  files/en.ron:15: error[SH2008]: "title.help": unknown action "{Confirn}"; known actions: Up, Down, Left, Right, Confirm, Cancel, and {Cursor} for the cursor keys
help[SH2008]: use one of the names the message lists, or fix the spelling
```

A capitalized name in braces is a key, and lowercase is a value the screen passes
([chapter 4](@/learn/book/actions.md#naming-a-key-in-text)).

## When a game outgrows the short path

The short path's parts grow one at a time, and the game's start stays as it is:

- **actions of its own**: an action set of the game's in place of `UiAction`
  ([chapter 4](@/learn/book/actions.md#actions-of-a-game-s-own));
- **a part of its own**: a map, a table of rules, read by the function
  `BasicGame::with_own` takes ([chapter 3](@/learn/book/assets-and-formats.md));
- **tools of its own**: a scene, records and bots, through a `GameTools` of its own
  ([chapter 10](@/learn/book/tools-for-agents.md)).

You can leave this section until you need it. A game whose data needs more than
`BasicData` has (sounds, voices, color themes, settings of its own) writes its own
`GameData`. Each method hands the engine a part it reads:

{{ snippet(file="src/screens.rs", anchor="game_data") }}

{% newcomer(to="rust", topic="a trait's `type` lines") %}
The `type` lines fill in blanks of the trait. `GameData` needs to know this
game's actions, colors, voices and settings, and `()`, a value holding nothing, says
this game has none of its own.
{% end %}

Such a game makes its context with `Ctx::new(data)`, its game with `Game::new(ctx,
first_screen, GameHooks::default())`, and its binary calls `shuyi::app::run` with its
own window settings. The API docs of `shuyi_ui` and `shuyi_app` have the details.
