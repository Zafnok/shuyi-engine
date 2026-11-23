# ADR-0141: The hosted preview: a worker process per visitor, actions in, a picture and its text out

- **Status:** Accepted
- **Date:** 2026-11-23
- **Related tickets:** 2602 (this: the worker); 2609 (the action API),
  2610 (the quotas), 2611 (the page, the load test and the cost), which
  2602's split made; 3083 and 2603 (parked: marks, and content held back,
  on top of it); 2322 (parked: an accessibility tree)

## Context

Whatever a game decodes on a player's machine can be taken from it
([2917's report](../research/2917-reverse-engineering-angles-not-yet-covered.md),
§3 and §5). A preview played on the developer's machine keeps the program,
its pack, its source art and its structured scene there, and still lets a
visitor play. Nick unparked 2602 on 2026-11-23 to build and measure one
locally and in CI, with no paid hosting until he approves its measured
cost ([decisions](../decisions.md#2026-11-23-the-hosted-preview-2602-unparked-built-and-tested-locally-only)).

What the engine had, and why it can't face a visitor:

- `Session` and `Headless` (`shuyi_cli::contract::session`) run a game
  headless and paint its frame, but the MCP host serves scripts, frame
  counts, scenes, snapshots, logs and heap records over them (ADR-0031),
  and the live link (ADR-0047) is a debug build's.
- A debug build has the developer's debug tools on (`DEBUG_TOOLS`), and a
  game's Debug action (`UiAction::Debug`, a key no player can rebind)
  opens the debug menu: the scene camera and the debug view draw what the
  rules see.
- A snapshot's overlays name each sprite's file.

Measured on Hello, Grid's stage in a debug build (2602): its grid cursor
pulses every frame, so waiting for the whole snapshot to hold still never
ends, and each action ran its 300 frames, about 200 ms. Waiting for its
glyphs and its overlays' places takes 35 to 75 ms an action, about 6 KB a
picture at scale 1.

## Decision

### 1. A worker process per visitor, in the game's tool program

`preview-worker [--frames N] [--scale N] [the game's options]` is a
command of the game's tool program (`shuyi_cli::preview::worker`), as
`mcp-host` is (ADR-0031 §2): one process a visitor's session, so a panic
or a hang ends that session alone, and the server can kill it and read
its memory. Its options are the server's: the game's start options (its
seed among them), the most frames an action runs and the pictures' scale.
A visitor never chooses them.

It reads JSON lines on stdin, each `{"action": "<name>"}` and nothing more
(`deny_unknown_fields`: no script, no frame count), and answers each on
stdout after the mark `shuyi-preview-reply `: a `frame` (`png`, the
picture in base64; `text`; `actions`, a visitor's; `ended`, whether the
game quit), a `refused` (the session goes on) or a `failed` (it doesn't).
Its first answer is the frame it starts on. It ends when stdin closes.

### 2. A visitor's session: the player's actions, no debug tools

`Session::for_visitor` turns the developer's debug tools off for good and
gives the actions a visitor may press: those with a key or a button that a
player can rebind (`ActionSet::is_rebindable`), since an action no player
can rebind is a developer's key. Its default is `None`: a game whose
session doesn't say has no preview. `Headless` implements it, so every
game whose tools give a `Headless` has one.

### 3. One bounded frame an action

An action is pressed as a player presses it (`Step::Act`: its primary key
or button). Then frames run in sixes until the frame's still part has
held for 30 frames, or `--frames` have run (300 by default, at most the
Harness's 2,000). The still part is the snapshot's glyph rows and each
overlay's layer, kind and rectangle: a colour that pulses, a sprite's
brightness or its picture don't keep it running, and a piece that walks
does. The picture is the session's (`Session::picture`).

### 4. The text beside the picture is what the frame shows

The text is the snapshot's glyph rows, trailing spaces and blank rows
trimmed: what a person reads off the picture. The colours, the legend, the
overlays (with their files), the glyph styles and the scene's data never
leave the worker. It reads glyph art as it is drawn (a map as rows of
glyphs) and says nothing of a sprite: an accessibility tree (2322) would
say more. A game that draws hidden information into cells and covers it
with a sprite gives it away in the text.

### 5. The server and its limits (2609, 2610)

`shuyi preview`, in the game's tool program, serves HTTP/1.1 of its own
(no framework): one request a connection, a bounded head and body, no
chunked body, on this machine's own address by default. It answers `POST
/session` (a worker started, its first frame and a session id of 128
random bits), `POST /session/<id>` with an action, and `DELETE
/session/<id>`; every other path is `404`. It kills a worker past a
deadline and ends idle and old sessions (2609). It bounds the sessions at
once, per address and started a minute, the actions and their rate, the
game's time a session uses (the sum of its calls, the CPU it can cost),
a worker's memory where the system says it (Linux's `/proc`), and the
bytes a session's answers send (2610).

### 6. The page, the load test and the cost (2611)

A page of its own, with no game program or file in it, plays it with the
keyboard and a screen reader. A load test of concurrent sessions measures
the action-to-picture time, a session's CPU and memory and its bytes; the
estimate of a month's hosting is made from them, for Nick to decide on.
Nothing is deployed or paid for before he does.

## Consequences

- What leaves the machine is pictures and the text they show. What a
  visitor sees, they can record, copy and imitate: the docs and the book
  say so, and no limit here changes it.
- A process per session costs a program's memory each; 2611 measures it.
- The pressed actions are a player's, so a visitor can do what a player
  can, the options and their key bindings included, in their own session.
  Nothing persists past it: the headless platform's store is the
  session's.
- A game whose session isn't a `Headless` implements `for_visitor` to
  have a preview.
- The memory limit holds on Linux only: reading another process's memory
  on Windows or macOS needs platform calls the engine's `forbid(unsafe)`
  rules out here.

## Alternatives considered

- **The MCP host or the live link, behind a filter** — both exist to read
  a game's state; a filter in front of them is one bug away from serving
  it, and 2602 rules them out.
- **Sessions as threads of one server process** — cheaper in memory, but a
  panic or a hung frame in one game reaches the others, and no session's
  memory can be told apart or bounded.
- **A web framework (tokio with hyper or axum, or `tiny_http`)** — a stack
  for three routes. A reader of our own bounds every byte it reads, as the
  MCP server's JSON-RPC of its own does (ADR-0031 §1).
- **Video** — real time, and the web build's networking waits for 1147
  (ADR-0006 §6); a turn game's actions are seconds apart.
- **A fixed number of frames after each action** — too few for a walk and a
  foe's turn, or too many for a cursor's move.
- **Waiting for the whole snapshot to hold still** — a pulsing cursor
  never does (measured above).
