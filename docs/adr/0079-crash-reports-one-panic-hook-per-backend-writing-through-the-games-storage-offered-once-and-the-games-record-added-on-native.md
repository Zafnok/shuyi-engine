# ADR-0079: Crash reports: one panic hook per backend, writing through the game's storage, offered once, and the game's record added on native

- **Status:** Accepted
- **Date:** 2026-11-05
- **Related tickets:** 3046; 2520, 3041, 3047, 3060 follow from it

## Context

A shipped game that panics leaves nothing today. No crate sets a panic
hook. The release profile strips the binary and doesn't set
`panic = "abort"`, so a panic unwinds; a Windows release build has no
console (`windows_subsystem = "windows"`), so std's message goes nowhere,
and the window closes. On the web miniquad's own hook writes the panic to
the browser's console, the canvas stops, and since wasm32 aborts on a
panic nothing runs after the hooks.

Other engines keep something: Unreal's Crash Reporter Client gathers the
call stack and the log; Unity's Diagnostics collect crashes with stack
traces; Godot 4.5 gives backtraces in release builds; Defold's
`crash.load_previous()` reads the last crash at the next start, so the game
can act on it. Bevy has no crash reporter. Nick's end goal (2026-10-19,
`docs/decisions.md`): an agent given a report reproduces the panic, fixes
it and checks the fix without asking the player anything.

What a report is built from already exists: the game's `Storage`
(`FileStorage`, `WebStorage`), `App`'s frame count and log (ADR-0030),
`Game::top_screen`, and a turn game's record of its play (hello-grid's
`StageRecord`, which `shuyi replay` re-runs). `shuyi-platform` already
has `serde` and `ron` (ADR-0072 added them to ADR-0024 §1's list, and
`tools/layers.ron` names them).

## Decision

### 1. The report is `shuyi-platform`'s; the hook is the backend's

`shuyi_platform::crash` (`no_std` + `alloc`) holds the report and its
rules, so every backend writes the same text and a game reads it the same
way:

- `CrashReport`, plain data, `#[non_exhaustive]`: `version` (1), `when`
  (milliseconds since 1970 on the backend's clock), `message` (cut at
  4 KiB), `location` (`file:line:column`), `thread`, `backtrace` (std's
  text, cut at 32 KiB), `engine`, `game` and `target` (versions and
  `os arch debug|release`), `frame` (as `App::frame_number` counts),
  `top_screen`, `log` (the last 200 lines, each cut at 1 KiB) and
  `record` (the game's record of its play, at most 1 MiB). Every field but
  `version` and `message` is `#[serde(default)]`, so a field added later
  (2520's) leaves older reports readable.
- Its text is RON with `\n` line endings (`to_text`); `parse` refuses
  text over 2 MiB, returns errors and never panics (ADR-0007 §5). It is a
  record in storage like the settings and the profiles, not an asset kind
  a game ships, so no `Format` is registered for it (ADR-0019).
- A report is stored at `crash-<when>` (`crash_key`), a key
  `is_valid_key` accepts. `report_keys` lists the keys that are `crash-`
  and digits only, newest first by their time; `keep_newest` deletes the
  rest.

Each backend installs one panic hook, which writes a report through the
storage it hands the game. `shuyi-app`'s is `shuyi_app::crash`; a backend
that can't (the Playdate's panic handler, the GBA's, the DS's) says why in
its docs (`docs/platform-backends.md`'s checklist).

### 2. The hook in `shuyi-app`

- `AppConfig::crash: CrashReports` (`game_version`, `keep`): 5 reports
  kept on native, 1 on the web, where `localStorage` holds about 5 MB for
  the whole site. `CrashReports::off()` installs nothing. `AppConfig::new`
  stays a `const fn`, so `CrashReports::new` and `off` are too.
  `run_game` names the short-path game's `AppInfo::version`.
- `run` calls `crash::install` before its first frame with a function
  that makes the game's storage (`storage::platform`, which reads no
  file). The hook takes the hook before it, writes the report through a
  new storage, keeps the newest `keep`, then calls the one before, so
  std's message in a terminal and miniquad's console line on the web
  stay. Two panics in one millisecond: `when` goes up by one until its key
  is free. A write that fails is dropped; the hook never panics.
- What it reads is in statics, not thread-locals, so a panic on the music
  worker thread gets it too: the frame about to run (an `AtomicU32`) and
  the top screen (a `Mutex` read with `try_lock`), set by `run` before
  each frame (`set_frame`), and a ring of the log's last 200 lines
  (`note_log`, `f{frame} {level} {category}: {text}`). A lock the
  panicking thread holds is skipped, never waited on.
- A panic caught on purpose is not a crash: `crash::quiet` turns reports
  off on its thread. The music decode's `catch_unwind` runs in it.
- The hook is installed once a process; `install` again changes what it
  writes.

### 3. Native catches the game's frame; the web can't

`run` calls `App::frame` inside `catch_unwind`. When it catches a panic,
the hook has written the report. `run` then stops the sound, asks the game
for its record (`App::crash_record`, inside `quiet` and a second
`catch_unwind`, so a panic there writes nothing more), adds it to the
report under the same key (`crash::add_record`), and ends the process with
exit code 101, as an uncaught panic does. The game's system hook is not
called: a game in a broken state never saves over a good save.

On the web a panic aborts, so a web report has no record.

`SHUYI_PANIC_AT_FRAME=<n>`, read once by `run` on native in any build,
panics at frame `n` inside the same `catch_unwind`, with a fixed message,
for the tests and CI's `render` job.

### 4. The game's record

`Start::crash_record(hook)`, with `CrashRecord<G> = fn(&Game<G>) ->
Option<String>`: the game's record of its play in the format its tools'
`replay` reads. A turn game's record keeps the command its rules are
applying until they return, so a panic inside the rules is in it.

### 5. What a report holds, and never holds

A report holds the panic, the versions, the frame, the screen, the log and
the game's record. It never holds the player's settings, saves or keymap,
nor anything the player typed unless the game's record holds it; the docs
ask a game to keep names out of its record until 3047 is decided. Reports
stay on the machine: sending one anywhere is 3047, Nick's decision.

### 6. Offered once, worded by the game

`App::load` fills `Setup::last_crash` (`LastCrash`: `when`, `message`,
`location`, `key`, `#[non_exhaustive]`) from the storage it hands the
game: the newest report newer than the one named at `crash-offered`, which
it then names (`offer_last_crash`). So each report is offered once, and a
game shows a line on its title, a screen of its own, or nothing; the words
are its own. The offer runs after a session record has read the storage,
so a playback offers it again and plays the same.

## Consequences

- A player whose game crashes has a file to send; a developer, or an
  agent with 2520's `replay --crash`, has the message, the place, the
  frame, the screen, the log and the play that led there.
- A report is only as good as its backtrace: a stripped release build
  gives `<unknown>` frames until 2520 names them from symbols kept per
  release.
- Stack overflows, aborts, running out of memory and faults in native code
  end the process without a panic hook: they leave no report. No
  minidumps.
- The web's report has no record, and on the web the hook's storage
  shares `localStorage`'s quota with the saves, hence one report kept.
- `run` installs a process-wide hook: a game that sets its own hook after
  `run` starts replaces it (and should call the one it took).
- 3041's log file and this hook are one hook: whichever lands second
  joins the other.

## Alternatives considered

- **`panic = "abort"` and a hook only** — simpler, but no record on native,
  and a report then can't be reproduced without asking the player.
- **A separate crash-reporter process** (Unreal's) — catches more than
  panics, but is a second binary per target and much more code; panics are
  where a Rust game fails.
- **Reports in a folder of their own, beside the storage** — but the web
  has no folders, and the game's storage is already where every backend
  keeps the player's files; one place on every backend wins.
- **The engine shows a notice** — the words, the look and whether to show
  anything at all are the game's (`AGENTS.md` rule 4).
- **Thread-locals for the frame and the screen** — a panic on the music
  worker would see none of them.
