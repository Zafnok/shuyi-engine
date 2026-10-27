# The tool contract

The operations an agent, CI or a person runs on a game, each with a
typed input and a typed answer
([ADR-0005](adr/0005-ai-native-tooling-one-contract-three-surfaces-and-visual-scripting-as-a-view-over-text.md),
[ADR-0022](adr/0022-the-tool-contract-answers-with-limits-schemas-by-version-and-a-session-over-the-harness.md)).
They are library functions in `shuyi_cli::contract`
(`crates/cli/src/contract.rs`), and `shuyi <op>` runs them on the
command line. The MCP server, `shuyi mcp` ([below](#the-mcp-server)),
is another way to reach them.

## The operations

| Op | Does | Needs |
| -- | ---- | ----- |
| `new <dir>` | A new game in `dir` from the engine's template ([`templates.md`](templates.md)), its parts picked by the answers: `--template grid` or `--template blank` (a short name), `--answers <file.ron>`, `--set <question>=<answer[,answer]>`; it lists the files made, the parts, the modules turned on, the stages and the engine's skills it copied into `.claude/skills/`, and `--force` writes into a folder that isn't empty, listing what it wrote over | nothing |
| `check [--baked]` | Loads the game's content through every validator: every error, with file, line and message. With `--baked`, bakes the game's pack too, and every asset read from it must equal the one read from its source | the game |
| `check --target <id \| all>` | Whether the game's frame at its scale fits a target's display inside its safe area, and its smallest text is as tall as the target asks (ADR-0008 §1): each problem with its target, its setting in `shuyi.ron`'s `targets` and the numbers; with the game's tools, that each screen's required actions fit the target's inputs (ADR-0025), and on a one-bit target that every palette role has patterns and the roles of a `distinct` group stay apart (ADR-0026, each problem with its `roles`); a warning doesn't fail it | the project file; the game for the controls and the patterns |
| `lint` | Everything wrong with the shape of the game's code, each finding with its file, line, problem code and what to do: `check-keys`' findings (`SH2101`), `check-text`'s (`SH2102`: every literal when there are more than `check_text.max_literals`, and every data name a screen reads), and the rules crate's fence ([ADR-0032](adr/0032-the-layers-as-data-checked-over-cargo-metadata-a-float-ban-in-the-simulations-and-a-new-games-rules-crate-kept-pure.md) §4) on the package `shuyi.ron`'s `rules` names: `cargo clippy` read for `disallowed_types`, `disallowed_methods` and `float_arithmetic`, with clippy's reason (`SH2103`, with the column), and `cargo tree` for a screen, device or tool crate or outside luck under it (`SH2104`, with the `cargo tree -i` that shows the way in). A part `shuyi.ron` lacks is a limit | the project file and cargo |
| `modules` | The catalog of the engine the game builds with ([`modules.md`](modules.md)), by concern: each answer, who gives it (`default` or a module's crate), its status and its tickets; every module the engine builds, with its feature of `shuyi` and its metadata (what it answers, serves and sits beside); and the modules the game has on, from its `Cargo.toml` | the game's `Cargo.toml` and its engine |
| `add <module>` | Turns a module of the engine on (`physics` or `shuyi-physics`): its feature of `shuyi` in the game's `Cargo.toml`, its layout kept; its skill copied into `.claude/skills/` if the game keeps one; its README's first paragraph. It warns of an alternative already on, with the README's section on using both; gives the line to add if the game's code never installs `shuyi::enabled_modules`, and the line for its `formats()` if the module adds formats. A second run changes nothing (`changed: false`); a name that isn't built fails with `SH0003` and the names that are | the game's `Cargo.toml` and its engine |
| `run [steps] [--frames N] [--perf] [--perf-out <file.jsonl>]` | Starts the game headless, plays the steps, runs N frames more: the top screen, the screens, the scene, the frame as text, what happened, the sounds, the [log](#the-log), and the modules the game installed with the hooks each declared (`before_update`, `draw_over`), its state's types and its debug menu entries. `--perf` measures each frame and answers a summary of the times, `perf`; `--perf-out` also writes every kept frame as a JSON line, from the project's folder, and implies `--perf` ([Perf traces](#perf-traces)) | the game |
| `scene [steps]` | The same, answering with the scene or stage on screen and the log | the game |
| `replay <record> [<other>]` | Re-runs a record by the game's rules: a state hash per step, and the first step where two runs differ | the game |
| `bots <games>` | The game's bots play: how each play ended, how long it took, the game's measures, a record of each | the game |
| `playtest [--stage <id> \| --every-stage] [--entrant <name>]... [--tries N] [--seed N] [--turns N] [--threads N] [--report <file>] [--history <dir> \| --no-history] [--brief]` | Each of the game's named bots (its entrants) plays N tries from the same seeds; the game's checks say per bot whether it succeeded as often as its band wants (✓, ✗ too hard, ⚠ too easy) and compare medians between bots. The tries run on every core (`--threads`), each timed: every entrant's answer has its seconds per try, the one number that changes with the machine. The report never depends on the thread count. `--stage` picks one of the game's stages (needed when it has several), `--every-stage` plays each in turn, a report each; `--entrant` picks entrants or a group of them (the game's choice by default, all of them if it has none); a number not given is the game's default (100 tries from seed 0, to turn 200, unless it says). `--report` also writes the JSON answer to a file. Each entrant's line carries the plan's summary items (`won 83/100 · turns median 12`, its `summary`). Every run is kept as a JSON line in a history file per stage, entrants and game options (`<stage>-<entrants joined by +>[-<options>].jsonl` in `target/playtest-history/` under the project, or `--history`'s folder; `--no-history` keeps none, and a folder that can't be written fails the answer with `SH3004`), and the next run says what the last one found: after each entrant's line `· was 47/100 succeed` (its first rate check, else its first summary item; `was` in JSON) and a line `previous <date> · commit <hash>` (`previous`). `--brief` leaves out the line per try. It fails only when a bot falls under a check's floor | the game |
| `autobalance [--stage <id>] [--entrant <name>]... [--tries N] [--quick N] [--budget N] [--time <seconds>] [--seed N] [--turns N] [--threads N] [--apply]` | The game's knobs change one of its stages, never its bots, until each of its entrants succeeds as often as its checks' bands want (`shuyi_core::autobalance_by`): the smallest change the search finds, each in the knob's own words, why it stopped, and the reports before and after. The stage as authored and the one found play `--tries` tries; the search's own runs play `--quick` (a fifth of them). Each setting's tries run on every core and are judged once in seed order, so what it finds never depends on `--threads`. It stops at `--budget` evaluations (100) or, with `--time`, after that many seconds, with the best setting seen: a limit says so, since that depends on the machine. `--apply` has the game write the changed stage to its files, and the answer lists them. It fails while a rate is outside its band | the game |
| `import <file> [--kind K] [--print]` | Reads a file through the format registry as every kind that knows it, or gives its errors | the game's kinds, or the engine's |
| `bake [--target <id>] [--out <file>]` | Bakes the game's sources into one pack ([`formats.md`](formats.md#the-pack)): what went in, the sizes, how long baking and loading took; refuses sources with errors | the game |
| `doctor` | Checks Rust, Cargo, rustup's targets, MSYS2's linker on the Windows GNU toolchain, on Linux the C linker, `pkg-config` and the system libraries the backend links and opens, and the web target, and says how to fix each | nothing |
| `explain [<code>]` | Every problem code, or one code's page: what it means and how to fix it ([below](#problem-codes)) | nothing |
| `attach <host:port> --token <t> [steps] [--frames N] [--screenshot <file.png>] [--log-since N] [--perf-since N] [--perf-follow <file.jsonl> [--for S]] [--perf-snapshot]` | Reaches a debug build already running in its window ([below](#attaching-to-a-running-build)): plays the steps as a player's input, lets N frames pass, then answers with its frames, phase, screens, top screen, whether it is suspended, focused or quit, its frame as text, its [log](#the-log) lines numbered N or more, each with its `seq`, and `log_next`, the next `--log-since`; `--screenshot` writes the window's picture; `--perf-since` adds its frames' records numbered N or more, `perf`, with `perf_next` and `perf_summary`; `--perf-follow` writes each new frame's record to the file as the build runs, for S seconds (10), and answers their summary ([Perf traces](#perf-traces)) | a running build |
| `schema [<name> \| --out <dir>]` | The contract's JSON schemas | nothing |

A step is `--script "Enter Up"` (or `--keys`: each key chord pressed and
released in turn), `--pad "South"` (controller buttons, by position) or
`--wait 0.5` (seconds). Key and button names are the Harness's test-script
names. `attach` also takes `--act <Action>`: the game's action by name,
pressed by the player's primary key for it, or its primary button when it
has no key. The game's own start options (`frame-png --help` lists them, such
as Hello, Grid's `--stage` and `--seed`) go to `run`, `scene`, `bots`,
`playtest` and `autobalance` too.

From a game's folder, `cargo shuyi <op>` runs them (the alias the
templates write in `.cargo/config.toml`, or the engine's for its own
examples).

## The answer

Every operation answers with the same shape, `Answer<T>`:

```json
{
  "contract": 25,
  "op": "scene",
  "ok": false,
  "result": { "frames": 0, "top_screen": "title", "scene": null, "log": [] },
  "error": null,
  "code": null,
  "help": null,
  "limits": ["no scene: no stage is open (the top screen is `title`)"]
}
```

- `contract` is the contract's version.
- `ok` is the verdict: nothing found by `check`, two runs that agree, a
  game made, a toolchain complete, a scene found. `run` passes when it
  ran.
- `result` is what the operation found or made. It is absent when the
  operation could not run, and `error` says why ("the game's tools don't
  offer bots (GameTools::bots)").
- `code` is the error's problem code (`SH3001`), and `help` how to fix
  it ([Problem codes](#problem-codes)). An answer that ran and failed on
  the setup has them too (`doctor` with a failed check: `SH1003`). A
  problem in a file (`check`'s and `import`'s `errors`) has its own
  `code` and `help` when its class has one.
- `limits` says what the operation could not determine: frames after the
  game quit, a stage that isn't open, a record that ran twice in one
  process only, a kind printed without a writer of our text format.

`shuyi <op>` prints the answer for people; `--json` prints it as one JSON
document. The exit code is 0 when `ok`, else its code's class's: 1 when
it ran and failed, 2 for wrong arguments, 3 for a setup to fix (with
`--json`, a failed answer with the error, its code and its help).

## The log

`run` and `scene` answer with the game's log, `log`: what the engine
and the game logged while it ran (settings it couldn't read, a voice
manifest it refused, a game's own lines), oldest first. Each line is:

```json
{ "frame": 0, "level": "warn", "category": "storage", "text": "settings: unreadable: …, using the defaults" }
```

- `frame` is the frame it was logged in. Frames count from 1; 0 is
  before the first (what the game logged as it started).
- `level` is `error`, `warn`, `info` or `debug`; the log keeps `info`
  and more serious.
- `category` is what it is about: `assets`, `audio`, `input`,
  `screens`, `storage`, `platform`, or `game` for the game's own lines.
- `text` is English, never translated.

The log keeps the last 500 lines. When it dropped older ones, `limits`
says how many. For people, `shuyi run` and `shuyi scene` print the
warnings and errors.

## Perf traces

A debug build measures each frame: how long its parts took, the spans
the game marked around its own code with `ctx.span("ai", |ctx| …)`, and
the heap. The last 600 frames are kept. Agents read them four ways, as JSON
lines: `run --perf`, MCP's `perf`, a running build's frames over the
live link (`attach`, and MCP's session attached to it), and a running
build's trace file.

Each line of a trace is one object, tagged by `kind`: a `frame`, a heap
`snapshot` or a `diff` ([What holds the heap](#what-holds-the-heap)).
Most lines are frames:

```json
{"kind":"frame","frame":12,"times":{"update_us":310,"draw_us":420,"paint_us":610,"present_us":15800,"frame_us":17200},"spans":[{"name":"ai","start_us":40,"us":250,"depth":0}],"spans_dropped":0,"heap":{"live_bytes":4194304,"peak_bytes":4521984,"allocs":12}}
```

- `frame` is the frame's number: frames count from 1.
- `times` are whole microseconds: `update_us` the input, the modules'
  `before_update` and the screens' update; `draw_us` the drawing;
  `paint_us` painting the frame into the window; `present_us` the time
  from the end of the last frame to the start of this one (the buffer
  swap and the wait for vsync, 0 on the first frame measured);
  `frame_us` all of it, never less than the four.
- `spans` are the game's `ctx.span` calls that ended in the frame, in
  the order they ended (an inner one before its outer one): `name`,
  `start_us` from the frame's start, `us` long, and `depth`, how many
  spans it was inside. A frame keeps 64; `spans_dropped` counts the rest.
- `heap` is the heap at the frame's end, in bytes the program asked for
  (never the allocator's own overhead): `live_bytes` allocated and not
  yet freed, `peak_bytes` the most at once since the program started,
  and `allocs` the allocations and reallocations in this frame. It is
  left out where the heap isn't counted.

**The heap is counted in every debug build** of a game and of its tool
program, by a counting allocator the engine turns on, and in no release
build. It counts the whole program, every thread's allocations; the
limits say so when the tool program ran another thread beside the game.
A release tool program, or a game that opted out, has no `heap`, and
the limits say "the heap isn't counted". A game with a
`#[global_allocator]` of its own opts out (a program has one) in its
`.cargo/config.toml`:

```toml
[build]
rustflags = ["--cfg", "shuyi_no_heap_count"]
```

**Times are this machine's**, and from an unoptimized build unless it
says otherwise: compare two runs on the same machine, never numbers
across machines. A build without debug tools measures nothing, and the
answer's `limits` say so. A time never changes what the game does: a run
plays the same with perf on and off.

**`run --perf`** answers `perf`, a summary of the kept frames: `frames`,
`dropped` (older frames no longer kept), a spread each (`p50`, `p99`,
`worst` and `mean`, in microseconds) for `frame`, `update`, `draw`,
`paint` and `present`, and the five `spans` with the most time (`name`,
`count`, `total_us`, `mean_us`, `worst_us`), and `heap`, where it was
counted: `peak_bytes` (the most over the kept frames), `live_bytes` (at
the last frame's end) and `allocs`, a spread of the allocations per
frame. With `--perf-out perf.jsonl` it writes every kept frame as a
line, oldest first, and `file` is the file's absolute path. For people
it prints one line: `perf: 600 frames, frame p50 1.2 ms p99 3.4 ms worst
8.0 ms; span ai 0.8 ms mean; heap peak 4.1 MB`.

**A session, as a stream.** MCP's `start` measures every session. After
each `step`, `perf` with the last answer's `next` as `since` answers the
frames since the last read. After `reset` or a new `start` the numbers
start again: a `since` past the session's last frame answers no records,
`next` 0 and the limit "read from 0".

**A running build, over the link.** A build listening for `attach`
([Attaching to a running build](#attaching-to-a-running-build)) answers
its frames' records by number too:

- `attach … --perf-since 0` adds `perf`, the kept records (at most
  600), `perf_next`, the `--perf-since` of the next read, and
  `perf_summary`, their summary as `run --perf` answers it; when older
  frames were no longer kept, the limits say how many.
- `attach … --perf-follow perf.jsonl --for 30` follows the build
  instead: every 100 ms it reads the frames since its last read (from
  `--perf-since`, or else from the build's next frame) and appends each
  to the file, written afresh, as a `frame` line, flushed after each
  read. It stops after `--for` seconds (10 by default, 1 to 3,600) or
  when the build quits, and only then answers: `perf_summary` over every
  frame written (its `file` the file's absolute path), and `follow`,
  with `file`, `lines`, `dropped` and `closed` (the build quit). Run it
  in the background and read the file as it grows. A build that runs
  more than 600 frames between two reads loses the frames between, and
  the limits say "N frame(s) were dropped between reads". A relative
  path is from the folder `shuyi` runs in.
- `--perf-snapshot` takes a heap snapshot after the frame with the
  records: with `--perf-since`, `perf_snapshot`; with `--perf-follow`,
  one as it starts and one as it ends, written as the file's last three
  lines (the two `snapshot`s and their `diff`), and the summary's
  `heap_tags` and `heap_growth`.

MCP's session attached to a build answers `perf` as a headless session
does, over the link; it keeps the snapshots it takes, for `diff_from`.

**A running build's own file.** A running build (desktop, debug) writes
its own trace while it runs, with no client attached, when `SHUYI_PERF`
names a file: `SHUYI_PERF=perf.jsonl cargo run`. It
logs the file's name, and after each frame appends that frame's line
and flushes, so `tail -f perf.jsonl`, or reading the file again from
the last line read, follows it. It also writes a heap snapshot on its
first frame and every 600 frames after, each after the first followed
by its diff from the one before, so the file shows what grows as a
person plays. A build without debug tools logs why and writes nothing.

### What holds the heap

Where the heap is counted, it is counted by **memory tag** too: each
block of memory counts under the tag current when it was allocated, or
last grown (a list that grows in `update` counts in `update`). A tag has
a class and a name:

- `phase`, a part of the frame: `load` (the game's load; in a tool
  program, the session's start), `update`, `draw`, `paint` (painting
  the frame), `perf` (the frame's record), and `app` for the rest of
  the engine's frame;
- `span`, a span the game marked: `ctx.span("ai", …)` counts what the
  closure allocates under `ai`;
- `asset`, an asset kind being decoded: `image`, `palette`, …;
- `other`: `other`, what is allocated outside any tag (before the
  game starts, or by another thread), and `overflow`, which every name
  past the first 254 shares.

A **snapshot** is what holds the heap at a frame:

```json
{"kind":"snapshot","frame":600,"live_bytes":1146880,"peak_bytes":1523712,"tags":[{"class":"phase","name":"load","live_bytes":524288,"live_blocks":40,"allocated_bytes":524288,"peak_bytes":524288},{"class":"phase","name":"update","live_bytes":98304,"live_blocks":6,"allocated_bytes":475136,"peak_bytes":475136}],"overflowed":0}
```

- `frame` is the frame it was taken after (0 is before the first);
  `live_bytes` and `peak_bytes` are the whole heap's.
- `tags` has a row for each tag with bytes live or ever allocated
  under it, the most live bytes first (then by class, then by name):
  `live_bytes` and `live_blocks` live now, `allocated_bytes` ever
  allocated under it (a block grown adds its new size), and
  `peak_bytes` the most live at once.
- `overflowed` counts the names that shared `overflow`.

A **diff** is what grew and shrank between two snapshots:

```json
{"kind":"diff","from":1,"to":600,"live_bytes":126976,"tags":[{"class":"phase","name":"update","live_bytes":98304,"live_blocks":6},{"class":"asset","name":"image","live_bytes":24576,"live_blocks":3},{"class":"span","name":"ai","live_bytes":4096,"live_blocks":1}]}
```

It has the tags whose live bytes or blocks changed (a tag missing from
one snapshot counts as 0 there), the largest change first, less than 0
for fewer.

**To find what grows,** take two snapshots and read their diff:

- `run --perf` takes one once the game has started, before the steps,
  and one after the last frame. Its `perf` adds `heap_tags`, the last
  snapshot's first 10 rows, and `heap_growth`, the diff with its first
  10 rows; `--perf-out` ends the file with the two `snapshot` lines and
  their `diff`. For people it prints a line: `heap grew 124.0 KB from
  frame 0 to 600: update +96.0 KB, image +24.0 KB, ai +4.0 KB`.
- In an MCP session, `perf` with `snapshot` takes one now; the session
  keeps the last 8, by frame, and every answer lists their frames in
  `kept`. `perf` with `diff_from` (a kept frame) answers the `diff`
  from it to the snapshot taken now, or to the newest kept without
  `snapshot`. A frame that isn't kept is wrong arguments (`SH0001`), and
  the error lists the kept frames.
- In a test, the Harness takes them: `h.heap_snapshot()` twice, and
  `first.diff(&later)`.

A tag says where the bytes were allocated, not who holds them. To give
suspect code's bytes a tag of their own, mark it with a span. Where the
heap isn't counted there is no snapshot, and the limits say why.

## Attaching to a running build

`run` and `scene` start the game headless. `attach` reaches a build
that is already running in its real window: the renderer players see,
its keys pressed the way a player presses them, and its log. It is for
checking what only the window shows; the headless operations stay the
way to test a change.

- **The build.** A debug build (or one with the `debug-tools` feature)
  of a game whose backend has its `link` feature on, started with
  `SHUYI_LINK=<port>` (`0` takes a free port): `SHUYI_LINK=0 cargo run
  --features link` for a game made by `shuyi new`. A release build, an
  export, or a build without the feature never listens.
- **The token.** It listens on `127.0.0.1` only, and logs one line:
  `shuyi link: listening on 127.0.0.1:<port>, token <token>`. The
  token is 128 random bits, new each start, unless `SHUYI_LINK_TOKEN`
  sets it (16 characters or more). A connection that doesn't start with
  the right token is closed (`SH0004`), so a web page can't use it.
- **One client at a time.** A second one is told the link is busy
  (`SH3006`).
- **What it reads:** the screens, the frame as text, the window's
  picture, the log by number (`--log-since` the last `log_next` reads
  on from there), and the frames' records by number, read once or
  followed into a file as the build runs ([Perf
  traces](#perf-traces)). Waits are counted in the build's own frames, at
  60 a second; the steps wait while the build loads or is suspended,
  and `limits` says so.
- **What it never does:** change the game any way but a player's input
  (no settings, no files, no commands), or start, stop or reset it.
  Nothing is built or run: start the build yourself, then attach.

`attach` with no build listening answers `SH1005` (exit code 3), and a
build of another engine `SH1006`.

MCP's session attaches to a running build too, and keeps the link
between calls: `start` with `link` ([The MCP server](#the-mcp-server)).

## Problem codes

Every error the contract gives has a stable code, `SH` and four digits
([ADR-0028](adr/0028-problem-codes-a-stable-code-by-class-a-help-line-a-page-and-exit-code-3-for-the-setup.md)).
Its first digit is its class, and the class sets the exit code:

| Class | Codes | Exit code | Means |
| ----- | ----- | --------- | ----- |
| usage | `SH0nnn` | 2 | the arguments or the input are wrong: change the command |
| setup | `SH1nnn` | 3 | the project file or the toolchain needs fixing: fix it, then run the same command again |
| content | `SH2nnn` | 1 | a file of the game's content has a problem |
| operation | `SH3nnn` | 1 | the operation failed on something else: the game's tools, a target, a file it reads or writes |

An answer without a code that isn't `ok` exits 1: it ran and found
something (`check`'s errors, two runs that differ). No exit code means
"retry as it is".

For people, an error reads:

```text
check: error[SH1002]: shuyi.ron names no game tool program (`game`), and check runs the game
  help: fix shuyi.ron as the message says, then run the same command again
  see: shuyi explain SH1002
```

and a problem `files/stage.ron:3:4: error[SH2001]: Expected comma`, with
one `help[SH2001]: …` line after the list per code used.

`shuyi explain` lists every code; `shuyi explain SH2003` (or its name,
`missing-file`) prints its page, from
[`crates/cli/errors/`](../crates/cli/errors/). The table is
`shuyi_cli::contract::codes::TABLE`; the content codes of the game's
files are `shuyi_assets::error::codes`, and those of its code, `SH21nn`
(`lint`), are the table's.

A code is never reused or renumbered: a retired one keeps its row and
its page. **A new error gets a new code, its row and its page in the
same PR**; a test fails on a page with no row, a code twice, or a row no
source uses.

## The schemas

Each operation's input and answer has a JSON schema, generated from the
types and checked in under
[`crates/cli/schemas/`](../crates/cli/schemas/): `run.input.json`,
`run.output.json` and so on. `shuyi schema run.output` prints one.

They change only with the contract's version. A test fails when the
checked-in schemas are not what the types generate, and when their
fingerprint is not the one
[`versions.json`](../crates/cli/schemas/versions.json) lists for
`contract::VERSION`. To change an input or an answer:

1. change the type;
2. `cargo shuyi schema --out crates/cli/schemas` from the repository's
   root;
3. raise `contract::VERSION` by one;
4. add the line the test asks for to `versions.json`.

## The MCP server

`shuyi mcp` serves the contract over MCP, on stdin and stdout. Each
operation is a tool: its input is the operation's input schema
(`run.input.json`), and its result is the operation's answer, as
structured content and as JSON text. An answer that isn't `ok` is an
error result, with its `code` and `help` as they are: branch on the
`code`. The tools that run the game, and `lint`, take `project`, the
game's folder from the session's folder (the session folder's own
project if it is left out). `lint` runs in the server, not the game's
tool program.

The **session's folder** is the client's first root: a client that
declares `roots` in `initialize` gets a `roots/list` request after
`notifications/initialized` (Claude Code lists the session's working
folder, a worktree's in a worktree session). Without roots it is the
server's working folder. When the client sends
`notifications/roots/list_changed` the server asks again and takes the
new first root as the folder, following that folder's server's tools (or
its own, back in its own workspace); a session game keeps running where
it runs.

A **session** keeps one game running headless between calls:

| Tool | Input | Does |
| ---- | ----- | ---- |
| `start` | `project`, `start` (the game's start options) | builds the game's tool program and starts its game; the answer lists the options it offers. A game already running is stopped |
| `step` | `script`, `frames` | plays the script, then idle frames; answers as `run` does, for everything since the start |
| `scene` | nothing | the scene on screen now, and the log, as `scene` answers |
| `frame_png` | `scale` (1 to 8) | a picture of the frame now, as an image, with its size |
| `events` | nothing | what happened since the start, and the sounds |
| `perf` | `since` (0), `max` (120, 1 to 600), `snapshot` (false), `diff_from` | the frames' records numbered `since` or more, at most `max`: `records`, `next` (the `since` of the next read), `dropped`, and the `summary` of the records answered; with `snapshot`, a heap snapshot taken now, `snapshot`; with `diff_from`, a kept snapshot's frame, the `diff` from it; and `kept`, the frames of the snapshots the session keeps ([Perf traces](#perf-traces)) |
| `reset` | nothing | the game started again as `start` started it |
| `stop` | nothing | the game stopped |

**A session over the link.** `start` with `link` (`{ "address":
"127.0.0.1:7357", "token": "…" }`, from the build's log line) attaches
the session to a build already running in its window, as `attach`
reaches one, instead of building and starting the game headless.
`project` isn't needed. It answers as `start` does, with the build's
`game` (its window's title), no `project` and no options; a game
already running in the session is stopped first. A wrong token, no
build and another link version answer `attach`'s codes (`SH0004`,
`SH1005`, `SH1006`). Then:

| Tool | Over the link |
| ---- | ------------- |
| `step` | plays the script as a player's input (waits in the build's frames, at 60 a second), then `frames` as a wait; answers as `run` does from the build's screens, frame and log, with no scene, events or sounds |
| `scene` | the frame's text in place of a scene (`kind` `frame`), and the log |
| `frame_png` | the window's picture, at the window's size; `scale` is refused |
| `events` | none: the limit says so |
| `perf` | the build's frames' records, as a headless session answers them: the same input and answer; the heap snapshots it takes are kept by the session, for `diff_from` |
| `reset` | refused (`SH0001`): the link never changes a running build |
| `stop` | detaches, and leaves the build running |

The answers' `limits` say what the link doesn't carry (`no scene over
the link`, `no events over the link`). A link that is lost answers
`SH3006` and ends the session.

Their schemas are `session.<tool>.input.json` and
`session.<tool>.output.json`. The one-off `scene` is the session's here:
`run` answers with a one-off run's scene. A session tool with no game
running answers `SH0002`. The game runs in the game's tool program
(`cargo run`, as `shuyi mcp-host`), which `start` builds, so call
`start` again after changing the game's code. A program that doesn't
build, or stops, answers `SH3005` with the last of its output, and ends
the session. A game paints the session's frame with
`Headless::with_picture`.

`shuyi mcp install` registers the server with the agents it finds, in
the project's folder: Claude Code's `.mcp.json` always, and Cursor's,
VS Code's and Gemini CLI's files where it finds them. It shows the
changes and asks before writing, and keeps each file it changes as
`<file>.bak`. `--yes` doesn't ask; `--dry-run` writes nothing and prints
each file as it would write it (Claude Code's is also what `claude
--mcp-config <file>` reads). The entry runs `--command`'s words and
`mcp`: the project's `command` (`cargo shuyi` in a new game) unless
given. For Claude Code it also sets `MCP_TIMEOUT` to 5 minutes in the
project's `.claude/settings.json`, so Claude Code waits for the server's
first build to finish. After writing, it runs that entry with `--ready`
and says how long the build took.

On Windows the server, and the game's tool program it starts, move their
own program into `.shuyi-running/` beside it as they start, and leave a
copy in its place, so a build while they run writes a fresh program
instead of waiting for them to stop.

The entry builds the server from the project's own engine (a game's
moves with its pin). `<command> mcp --ready` builds the server and the
game's tool program and prints `mcp: ready (contract N)`, so a first
connection doesn't wait for a build.

**Each project gets its own server.** A client may start the server in
another checkout than the session's: Claude Code's desktop app starts a
worktree session's server in the repository's main checkout. So each
call's project is looked up to its cargo workspace (`cargo
locate-project --workspace`, once per project root), and a project in
another workspace than the server's goes to that workspace's
**forwarded server**: a `shuyi mcp` built from that workspace's engine,
one per workspace, kept for the session. It is the project's `command`
and `mcp`, run in its root; or, with no `command` in its `shuyi.ron` (or
no `shuyi.ron`, as at the engine repository's root), the `shuyi` entry
of the workspace's `.mcp.json`, run there; or `shuyi mcp` in the
project's root. A folder outside any workspace, or with neither a
project nor an entry, is the server's own. The call goes on with its
`project` (and `new`'s `dir`) made absolute, and its answer comes back
as it is. When the session's folder is in another workspace, its server
starts at the roots' answer, so its build runs while the agent works; a
call waits for it. One that doesn't build, or stops, answers `SH3005`
with the last of its output, and the next call starts it again. A
forwarded server forwards nothing itself (it runs with
`SHUYI_MCP_FORWARDED` set), and it ends when its stdin closes, as the
server does, so nothing runs from a worktree's `target` once its session
ends.

**The tool list follows.** The server declares `tools: { listChanged:
true }`. Once the session folder's forwarded server is ready, the server
fetches its `tools/list`; if it differs from its own, it sends
`notifications/tools/list_changed` and from then lists those tools, and
hands a tool it doesn't know itself to that server.

**A stale server restarts.** When the game's engine speaks another
contract than the server (the checkout moved under a running server), a
call in the server's own workspace goes to a fresh forwarded server of
that workspace, and so does every later one. Only if that one doesn't
start does the answer get a limit naming both contracts.

## What a game gives

The `shuyi` binary knows no game. A game gives its tools in a program of
its own that calls `shuyi_cli::main_with` with its `GameTools`, and
`shuyi.ron`'s `game` names that program. Each operation that runs the
game is a method of `GameTools`, with a default that says the game
doesn't offer it:

| Method | For | What a game writes |
| ------ | --- | ------------------ |
| `check` | `check` | its loader's errors (`ContentError`s) |
| `session` | `run`, `scene`, the MCP session | `Headless::new(harness)`, with `with_scene` (its stage's `MapScene` as text and its state as JSON), `with_events` (its rules' events) and `with_picture` (its `frame-png` painter, for `frame_png`) |
| `replay` | `replay` | its record parsed, and `replay::turns(file, first_state, &commands)` |
| `bots` | `bots` | each play by `shuyi_core::play`, summed up by `bots::tally` |
| `playtest_plan`, `playtest_stages`, `playtest_defaults` | `playtest` | a `PlaytestPlan` for a stage: a title, its entrants by name (each with a label, a mode, if it has one), named groups of them (`"all"`), the default ones, its checks over the entrants that play (a data file of `shuyi_core::bots::playtest::Check`s, or a `playtest::Targets` file by tier whose `checks` gives a stage's, keeping those whose `Check::entrants` all play), a `Sync` function playing one try of one entrant from one seed, most simply `playtest::play_tries` over that seed, and each entrant's `summary` items (`Sum`, `Spread`, `Most`, `Median`, `Max` over its measures, serde so they may live in a data file); its stages' ids, if it has several; and its own default tries, seed and turn limit, if not the engine's |
| `autobalance_plan` | `autobalance` | an `AutobalancePlan` for a stage: a title, its entrants, groups, default entrants and checks as its `PlaytestPlan` has them, and the stage it may change, a boxed `Tunable` over the game's own stage type `S`: the stage as authored, its knobs (`shuyi_core::Knob<S>`, in the order the search tries them), a `Sync` function playing one try of one entrant from one seed on a given stage (most simply `playtest::play_tries` over that seed), and one writing a changed stage to the game's files, giving the files it wrote. Its stages and tries are `playtest`'s |
| `importers` | `import` | `Importers::standard()` and its own formats and kinds |
| `bake` | `bake`, `check --baked` | a `GameBake`: its `Bake` (the assets it bakes and its stamp), the bundle of its sources for the target, and its formats |

A game on the short path (a `shuyi_platform::BasicGame`) gives
`shuyi_cli::BasicTools::new(game)`: `frame` and `frame-png`, `lang`,
`check` (its files' errors), `session` (`Headless` over its Harness, with
no scene) and `controls` (its keymap and every screen it can show). A
game that adds a scene, a record, bots, importers or a bake writes its
own `GameTools` and calls `BasicTools`' methods for the rest.

Hello, Grid's `src/tools.rs` writes every one. A game made with `shuyi
new --template grid` writes every one but `bake`, which a new game
leaves out until it chooses a pack, and `autobalance_plan`, which needs
knobs of its own stage: its stage's `Tools`
(`crates/game/src/stages/top_down_grid/tools.rs`) adds a scene, records,
bots and the map's importer, and calls `BasicTools` for the rest.

A state hash is `shuyi_core::state_hash`, over any `Serialize` state: the
same on every platform and every run, if the state's maps are ordered.

## The playtest in CI

A game's CI plays its bots with the engine's reusable workflow,
`.github/workflows/playtest.yml` (ticket 2021). On every pull request
or push that changes one of the game's levers it runs `cargo shuyi
playtest --every-stage --brief`, puts each stage's report in the job's
summary, a fenced block a stage, and fails only when the command does:
an `::error::` names each stage blocked (a bot under its floor), or
says the playtest itself broke, with its code and error (`SH3001`,
`SH3002`, `SH3004`, `SH3005`) or that no report was written. A missed
band is reported, never a failure. A game calls it from a workflow of
its own (the grid template's game has one):

```yaml
concurrency:
  group: playtest-${{ github.event_name == 'push' && github.sha || github.ref }}
  cancel-in-progress: ${{ github.event_name != 'push' }}

jobs:
  playtest:
    uses: Zafnok/shuyi-engine/.github/workflows/playtest.yml@<engine commit> # the engine's commit
    with:
      levers: bash .github/playtest-levers.sh
```

| Input | Default | Means |
| ----- | ------- | ----- |
| `paths` | every file but `*.md`, `docs/*`, `tickets/*` | The changed files that make a play: shell patterns, one a line, from the repository's root. The first that matches a file decides: `pattern` plays, `!pattern` doesn't; a file none matches doesn't |
| `levers` | none | A command in the game's repository, run from its root, that reads the changed paths on stdin, one a line, and prints `play=true` or `play=false`. When given, `paths` is not used: for rules shell patterns can't say (tests never play, wherever they are) |
| `working-directory` | `.` | The game's folder, with its `shuyi.ron` |
| `system-packages` | the macroquad libraries the templates' CI installs | The apt packages the game builds with, space-separated |
| `build` | none | A command run before the playtest, to build what it runs (a release build of the tool program) |
| `command` | `cargo shuyi playtest --every-stage --brief` | The playtest; the workflow adds `--report <file>` and reads the file to say why it failed |
| `allow-no-plan` | `false` | Pass, with a notice, while the game gives no playtest plan (`SH3001`). The grid template's game sets it, since it has no plan until it writes one (the `shuyi-balance-bots` skill); drop it then, so a plan that stops working fails |
| `timeout-minutes` | 120 | The playtest job's timeout |

The changed files are a pull request's (its merge commit against the
base) or a push's (`before..HEAD`, every commit pushed). A push with no
earlier commit to compare with (a new branch, a force push) plays, and
so does any other event (`workflow_dispatch`). The workflow always runs
and reports: `detect` decides, and the `playtest` job is skipped by its
`if:`, never by a workflow's `paths` filter, so a check that branch
protection requires reports on every pull request (game ADR-0014, as
the engine's own `changes.yml`). `concurrency` is the caller's: the one
above cancels a pull request's earlier runs, and never a push's, whose
levers would otherwise go unplayed when the next push lands. The
history `playtest` keeps is per checkout: CI starts each run without
one.

## The templates

`shuyi new <dir>` makes a game from the engine's template,
`templates/shuyi-default`, made of parts that the answers pick
([`templates.md`](templates.md), ADR-0013 §3). It builds and passes its
own gates as it is. Two short names stand for sets of answers:

- `--template blank` (or no answers): the `base` part alone. One screen,
  the title, with the game's name, on the short path: its `lib.rs` names
  its files, its data type (`BasicData<UiAction, stages::Own>`), its
  title and its screens in a `BasicGame`; its `main` is one call (the
  prelude's `run_game`); its tool program gives `game::stages::tools()`,
  the engine's `BasicTools`;
- `--template grid` (`--set perspective=top-down-grid`): `base` and the
  part `stage-top-down-grid`. The title's Confirm starts a top-down grid
  stage: a rules crate over `TurnRules` (walk to the goal while a
  wanderer takes a random step after each of yours), the stage painted
  by the engine's glyph skin, its map, lights and seed as `BasicData`'s
  own part, and a `GameTools` of its own with a bot, records and every
  operation of the contract wired.

Each has its crates, `shuyi.ron`, `AGENTS.md` (and `CLAUDE.md`), the
engine's skill pack in `.claude/skills/` (all but the modules' skills,
which `add` copies), CI with the gates (`fmt`, `clippy`, the tests, `check`, `check --target all`,
`check-keys` and `lint`), and the `cargo shuyi` alias. The grid game also
has `.github/workflows/playtest.yml`, which calls the engine's playtest
workflow ([The playtest in CI](#the-playtest-in-ci)) at the engine's
commit it builds with: a 40-digit `--engine-rev`, else `main`. It has
no playtest plan yet, so it passes `allow-no-plan` until it writes one. The game crate
depends on the engine through the `shuyi` crate, its front door, with no
module on: the template's own parts need none. It installs
`shuyi::enabled_modules` once, in its `BasicGame::modules`, so `add`
needs no change to the game's code. The grid stage's rules crate depends
on `shuyi-core` and `shuyi-assets` alone, which keeps it pure. The game
depends on the engine's repository at `main`, at `--engine-rev
<commit>`, or on the checkout at `--engine-path <dir>`. CI makes a game
with each short name on every pull request and runs its gates.
