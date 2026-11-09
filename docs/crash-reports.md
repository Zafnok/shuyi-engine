# Crash reports

When a game on `shuyi-app` panics, the engine writes a crash report to the
player's machine, beside the game's saves. At the game's next start the game
hears of it once, and decides what its player sees. Reports stay on the machine:
nothing is sent anywhere. The design is
[ADR-0079](adr/0079-crash-reports-one-panic-hook-per-backend-writing-through-the-games-storage-offered-once-and-the-games-record-added-on-native.md);
a backend's side is [`platform-backends.md`](platform-backends.md), *Panics*.

## Turning it on

It is on for every game. Give it the game's version:

- **On the short path** (`shuyi_app::run_game`), the report names the game's
  `AppInfo::version` (`shuyi.ron`'s `app.version`). Nothing to do.
- **With an `AppConfig` of its own**, a game assigns its field:

  ```rust
  let mut config = AppConfig::new(WINDOW, APP.storage, files, FONT_PNG, sounds);
  config.crash = CrashReports::new(APP.version);
  ```

  `CrashReports::off()` writes no report. `keep` is how many are kept, the
  newest: 5 on desktop, 1 on the web.

## What a report holds

A report is RON (`shuyi_platform::crash::CrashReport`):

| Field | What it is |
| ----- | ---------- |
| `version` | The report's version: 1 |
| `when` | Milliseconds since 1970 |
| `message` | The panic's message, up to 4 KiB |
| `fatal` | Whether the panic ended the game: `true` on the thread that runs it, `false` on another (below). A report without it reads as `true` |
| `location` | Where it panicked: `file:line:column` |
| `thread` | The thread's name: `main`, or the music's worker |
| `backtrace` | The standard library's backtrace, up to 32 KiB. A release build is stripped, so its frames are `<unknown>` |
| `engine`, `game` | The engine's version and the game's |
| `target` | The OS, the processor and the build: `windows x86_64 release` |
| `frame` | The frame that was running, counted from 1 |
| `top_screen` | The name of the screen on top |
| `log` | The log's last 200 lines, `f{frame} {level} {category}: {text}`: in a build without debug tools, its warnings and errors only |
| `record` | The game's record of its play, if it gives one, in a build with debug tools or one the game asks for it in (below), up to 1 MiB |

It never holds the player's settings, saves or keymap. It holds what the player
typed only if the game's record does.

`CrashReport::parse` reads one back.

## Where it lands

Each report is the storage key `crash-<when>`:

- **Windows:** `%APPDATA%\<storage name>\crash-<when>.ron`;
- **macOS:** `~/Library/Application Support/<storage name>/crash-<when>.ron`;
- **Linux and the Steam Deck:** `$XDG_DATA_HOME/<storage name>/crash-<when>.ron`,
  or `~/.local/share/<storage name>/` without it;
- **the web:** `localStorage`, under the key `<storage name>/crash-<when>`.

The storage name is `AppConfig::storage_name` (`shuyi.ron`'s `app.storage`), and
`shuyi_app::storage::data_dir` gives the folder. A player who reports a crash
sends the newest `crash-*.ron` there.

## Telling the player

The game's load gets `Setup::last_crash`: the newest report of a panic that ended
the game (`fatal: true`) it hasn't been offered, once (the key `crash-offered`
remembers which). It holds `when`,
`message`, `location` and the report's `key`. What the player sees is the
game's: a line on its title screen, a screen of its own, or nothing. A line in
the game's own words might read: *The game closed unexpectedly last time. A
report was saved in the game's data folder.* The message itself is for the
developer, not the player.

```rust
let start = Start::new(card, look, |setup| {
    let crashed = setup.last_crash.is_some();
    let mut ctx = setup.ctx(my_ctx()?);
    ctx.game.crashed_last_time = crashed;
    Ok(Game::new(ctx, Box::new(Title), GameHooks::default()))
});
```

## The game's record

A report can hold the game's record of its play, so a crash can be replayed
without asking the player anything. Give `Start` a function that writes it, as
text in the format the game's tools' `replay` reads:

```rust
pub fn crash_record(game: &Game<MyGame>) -> Option<String> {
    ron::to_string(&game.screen::<StageScreen>()?.record()?).ok()
}

let start = Start::new(card, look, load).crash_record(crash_record);
```

A game on the short path (ADR-0036) has no `Start` of its own: it gives the
same function to its `BasicGame` (`.crash_record(crash_record)`), which passes
it on in `BasicGame::start`. A game made with `shuyi new --template grid` does,
through its generated `stages::crash_record` and its stage's
`top_down_grid::crash_record`; the blank template gives none.

It returns `None` when there is nothing to record (no stage open). A turn game
that keeps the command its rules are applying until they return (hello-grid's
and the grid template's `StageScreen::pending`) has it in the record, so a panic inside its rules is
replayed to the panic. Keep what the player typed (a name) out of the record.

### In a release build

A build with debug tools (a debug build, or one with the `debug-tools` feature)
adds the record. A release build without them doesn't, and doesn't read
`SHUYI_PANIC_AT_FRAME` either: together they would write a shipped game's state
on demand, for anyone who runs it. Its report keeps the rest, the backtrace and
the log's warnings and errors among it. A game that wants its players' records
asks for them:

```rust
config.crash = CrashReports::new(APP.version).with_record_in_release();
```

On the short path, `BasicGame::crash_record_in_release()` does the same.

On desktop the engine catches the panic, asks for the record, adds it to the
report, and closes the game with exit code 101. The game's system hook is not
called, so a game in a broken state never saves over a good save. On the web a
panic stops the page at once, so a web report has no record.

## What makes no report

Stack overflows, aborts, running out of memory and faults in native code end
the game without a panic, and write nothing. A panic the engine catches on
purpose (a music file that fails to decode) is not a crash.

A panic the game survives still writes a report, but it isn't offered as a
crash. On a Linux machine with no sound card the sound library's own threads
panic as the game starts, and the game runs on without sound: their reports,
`fatal: false` and naming their `thread`, say why, and the next start's
`last_crash` skips them
([ADR-0083](adr/0083-a-crash-report-says-whether-the-panic-ended-the-game-and-only-those-are-offered.md)).
A panic on the thread that runs the game ends it, so its report is fatal. A game
that catches a panic of its own on purpose runs it in `shuyi_app::crash::quiet`,
which writes no report.

## Trying it

`SHUYI_PANIC_AT_FRAME=<n>` panics at frame `n` on purpose, on desktop, in a
build with debug tools (and a release build whose game asked for its record):

```text
SHUYI_PANIC_AT_FRAME=3 cargo run -p hello-grid
```

It exits with 101 and writes a report whose message is `a panic on purpose
(SHUYI_PANIC_AT_FRAME)`, with `fatal: true`.
