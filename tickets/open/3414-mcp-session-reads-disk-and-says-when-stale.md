---
id: "3414"
title: The MCP session reads the game's files from disk in debug builds, says when its build is stale, and answers on first contact in a new worktree
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3414 — The MCP session reads files from disk, says when its build is stale, and answers on first contact

## Context

The MCP server ([ADR-0031](../../docs/adr/0031-the-mcp-server-hand-written-json-rpc-the-game-in-its-tool-program-and-session-tools-as-ops.md),
[ADR-0035](../../docs/adr/0035-the-mcp-server-follows-the-sessions-folder-roots-a-forwarded-server-per-workspace-and-a-restart-when-the-engine-moves.md))
keeps one headless game running for a session. `start` launches the
game's tool program with `cargo run … mcp-host` (`Cargo::launch` in
`crates/cli/src/mcp/child.rs`), and the session tools (`step`, `scene`,
`frame_png`, `perf`, `events`) talk to that program until the next
`start`. Three things go wrong in an agent's edit, step and check loop.

1. **A data edit doesn't reach the session.** The tool program reads the
   game's files only as embedded bytes. The template's
   `crates/game/src/lib.rs` has `static FILES: Embedded =
   shuyi::files!("$SHUYI_FILES")`, and `BasicTools::session`
   (`crates/cli/src/basic.rs`) starts a `Headless` over that game. The
   disk layer exists only on the windowed path: `cargo shuyi run --watch`
   sets `SHUYI_WATCH` (`WATCH_VAR`, `crates/cli/src/watch.rs`), and
   `shuyi-app` lays the folder over the files (`watched_layers`,
   `BasicGame::over`). A grep of `crates/cli/src` for `read_folder`,
   `watched_layer` and `WATCH_VAR` finds only `watch.rs`. Yet the
   headless platform can already take a changed file:
   `Harness::change_file(layer, path, bytes)` (`crates/platform/src/harness.rs`)
   queues it, and `App` hands it to the modules that declared
   `Hook::Reloaded` at the next frame (`tell_changed`,
   `crates/platform/src/app.rs`), as 2220 built
   ([ADR-0071](../../docs/adr/0071-hot-reload-a-disk-layers-bytes-replaced-values-that-remember-what-they-read-and-a-module-hook.md)).
   Nothing feeds it from disk in a session.
2. **A code edit is answered by the old program, silently.** The server's
   only staleness check is `stale()` and `stale_diagnosis()`
   (`crates/cli/src/mcp/server.rs`, about line 738 and 759). They compare
   the host's contract number with the server's `VERSION`, never the
   game's sources nor the server's own. `docs/contract.md` (about line
   564) says only "call `start` again after changing the game's code". An
   agent that edits a rule, calls `step` and reads the scene draws its
   conclusion from code that no longer exists, and no answer says so.
3. **The server fails on first contact after a cold or stale build.** The
   repository's `.mcp.json` runs `cargo run -q --bin shuyi -- mcp`, so
   cargo builds the whole engine before the server can answer
   `initialize`. `.claude/settings.json` gives it 300 s (`MCP_TIMEOUT`).
   In a new worktree, or after the checkout moved, the build is longer,
   and the client gives up: the session that audited the engine on
   2026-11-23 started with `shuyi (CONNECTION_CLOSED)`, and had no tool
   surface all session. 2535 (done) added `mcp --ready`, which builds
   ahead, and 2078 (done) added `seed-target`. Both are run by hand:
   `scripts/git-hooks/post-checkout` only compresses `target/`, and
   seeding still rebuilds every workspace crate with its own paths
   (2078's study). The handshake waits on the build either way.

There is a fourth, smaller piece. 3412 embeds a docs search corpus in
the `shuyi` binary, and its **Out** leaves "reading the
corpus from disk in a debug build instead of the embedded copy" to this
ticket. The server already embeds the template, the skill pack and the
targets through `shuyi_embed::Table` (`crates/cli/build.rs`), so an
edit to a skill in the engine repository reaches the server only after
a rebuild and a reconnect.

Related: 2905 (research: code hot reload by patching, or a rebuild that
replays to the same moment), 2269 (BasicGame reloads its own files under
`run --watch`), 2270 (a `disk-files` development build that embeds
nothing), 3415 (build failures as structured diagnostics). From the
engine blind-spot audit of 2026-11-23 (agent experience, items 65 and
69).

The game (pinned at engine revision `50472bbc`) writes its own
`GameTools` (`gh search code --repo Zafnok/visions-of-shuyi 'GameTools
for'` finds `crates/tools/src/game_tools.rs` and
`crates/tools/src/skirmish.rs`), so it builds its sessions itself. The
disk layer reaches it only if it opts in (step 3). Nothing in the game
must change.

## Nick input

None.

## Scope

**In:**
- **The session's disk layer.** In a build with debug tools
  (`shuyi_ui::DEBUG_TOOLS`), the server starts the host with
  `SHUYI_WATCH` and `SHUYI_WATCH_PRIVATE` set as `run --watch` sets them
  (the project's `assets` folder, and its private files when the checkout
  has them). `BasicTools` lays that folder over the embedded files with
  `BasicGame::over`, and its sessions watch it. A game with its own
  `GameTools` opts in with one call (`DiskLayer::from_env` and
  `Headless::with_disk`, step 3). Hello, Grid opts in.
- **Reloads in the session.** Before each `step`, the session scans the
  folder (each file's length and modified time). It queues each changed
  file with `Harness::change_file`, so the game's `Hook::Reloaded`
  modules reload it in the first frame played. `step`'s answer lists what
  changed: each path, and for each the modules that reloaded it, the
  ones that failed with their errors, or none. A changed file that no
  module took gets a limit saying the game keeps the old value until
  `start`. A file added or removed gets a limit saying it needs `start`.
  `scene`, `frame_png`, `perf` and `events` play no frame: a limit says
  that N files changed and reach the game at the next `step`.
- **The game's build says when it is stale.** At `start`, the server
  records a fingerprint of the game's code: the path, length and
  modified time of each `*.rs`, `Cargo.toml`, `Cargo.lock` and
  `.cargo/config.toml` under the project's cargo workspace, leaving out
  `target/`, `.git/`, `.shuyi-running/` and the disk layer's folders (a
  data edit reloads; it isn't stale). Each later session tool call
  checks it again. If it differs, the answer gets a limit naming the
  first changed files ("src/rules.rs and 2 more changed after `start`:
  this answer is from the program `start` built; call `start` again"),
  and a new `stale` field (`true`). The answer stays `ok`: it is right
  for the old program.
- **The server says when it is stale.** A server whose `shuyi-cli`
  sources are on disk (the folder at `env!("CARGO_MANIFEST_DIR")`
  exists and is not under `CARGO_HOME`) records the same fingerprint of
  `crates/cli/` (its `src/`, `errors/` and `build.rs`) as it starts. A
  tool call after a change gets a limit: "the server's sources changed
  after it started; reconnect it (in Claude Code: /mcp)". The check runs
  at most once every 2 s.
- **The server's embedded tables read from disk in a debug build.**
  `shuyi_embed::Table` also writes the absolute folder it embedded, as a
  constant beside the table. In a debug build, the `shuyi` binary reads
  a file of the template, the skill pack and the targets from that
  folder when the folder exists, and falls back to the embedded bytes
  when it doesn't (an installed or moved binary). A release build never
  reads it.
- **First contact.** A new crate, `crates/mcp-launch` (`shuyi-mcp-launch`,
  a binary, `publish = false`), depends only on `serde` and
  `serde_json`, so it builds in seconds from a cold `target/`. Its
  arguments are the real server's command. It:
  - answers `initialize` at once, declaring `tools: { listChanged: true }`;
  - answers `tools/list` from the list the last real server wrote to
    `target/shuyi/mcp/tools.json`, or an empty list;
  - starts the real server and forwards every message once it has
    answered `initialize`, then sends `notifications/tools/list_changed`
    if its list differs from what was served;
  - holds a tool call made while the server builds for up to 240 s, then
    answers an error result with a new problem code (the next free
    `SH30xx`, `SH3018` today): "the MCP server is still building", with
    the seconds so far and the last line cargo printed;
  - answers a build that failed with that build's last output as
    `SH3005` does, and starts the build again at the next call.
  The repository's `.mcp.json` runs `cargo run -q -p shuyi-mcp-launch --
  cargo run -q --bin shuyi -- mcp`. `mcp --ready` (2535) builds the
  launcher too.
- **The template's games get the launcher.** The template gains a
  workspace member that runs `shuyi_mcp_launch::main`, and `shuyi mcp
  install` writes it as the game's entry, with `cargo shuyi mcp` as the
  real server. This is the part step 11 splits off first.
- New public items follow ADR-0039: `DiskLayer`, the reload list's entry
  (`Reloaded`) and the launcher's options are `#[non_exhaustive]` with
  constructors; the new fields of `RunOutput` and the session tools'
  answers (`reloads`, `stale`) are additive, as those types are already
  `#[non_exhaustive]`. `Session` gains provided methods only.
- A new engine crate adds its line to `tools/layers.ron` (a tool: no
  engine crate, `outside: Only(["serde", "serde_json"])`, no devices)
  and its row to `docs/modules.ron`/`docs/modules.md` if
  `check-modules` asks for one. Licences: `serde` and `serde_json` are
  MIT OR Apache-2.0 and already in the workspace (`cargo deny check`
  checks them).
- An ADR (the `write-adr` skill): the session's disk layer and the
  staleness fingerprint, and the launcher that answers before the build.
  It amends ADR-0031 (the host reads embedded files only) and ADR-0035
  (who answers `initialize`).
- **Either order with 2269:** if 2269 is done, `BasicGame`'s
  `basic_reload` module reloads text, palette, keymap, maps and dialogue
  in a session too: add a test that a text edit shows in the next
  `step`'s frame. If not, nothing to do (a `BasicTools` session then
  reloads pictures only, and the "no module took it" limit says so for
  the rest; 2269 adds the test when it lands).
- **Either order with 2270:** if 2270 is done, a game built with
  `disk-files` already reads its files layer from disk, so lay no second
  disk overlay for it on the headless path; read that layer for reloads
  and stale answers as you read the overlay. If not, nothing to do (2270
  adds a test that a `disk-files` tool program reports an edited file as
  reloaded, once this ticket is done).
- **The search corpus from disk.** 3412 embeds the Learn pages, the
  book's snippets and the examples for its `docs` index. Reading that
  corpus from disk in a debug build is this ticket's, as step 6 reads
  the other embedded tables. **From 3412 (done):** read its corpus
  tables (`learn`, `snippets`, `tutorial` and `examples` in
  `crates/cli/build.rs`, read as `LEARN`, `SNIPPETS`, `TUTORIAL` and
  `EXAMPLES` in `crates/cli/src/contract/docs/corpus.rs`) through step
  6's helper, with a test that an edited Learn page is found without a
  rebuild. The index is built once per process (`Index::embedded`, a
  `OnceLock` in `docs/index.rs`): from disk it is built again when a
  file changes, or per call.

**Out (do not do):**
- Rebuilding and replaying the session to the same moment after a code
  edit. 2905 researches it (its option 1); this ticket only reports that
  the build is stale, and keeps the steps an agent would replay
  (`start`'s own record).
- A disk layer for the one-off `run`, `scene`, `frame_png` and
  `playtest`. Each runs `cargo run`, which rebuilds after any edit of
  `files/` (the template's `build.rs` watches it); 2270's `disk-files`
  build is what avoids that rebuild.
- Reload on the web, and in a release build.
- Structured compiler diagnostics for a failed build, and the `build`
  operation: 3415. Saying that a session's build is older than its
  sources, and answering on first contact, stay here.
- Running `seed-target` or `mcp --ready` from the post-checkout hook.
  Seeding still rebuilds every workspace crate (2078), so the handshake
  would wait anyway; the launcher fixes the handshake.
- Changing any other harness's settings, or Claude Code's
  `MCP_TIMEOUT`.
- Keeping a last good server binary to answer tool calls while a new one
  builds. If the launcher's wait proves too long in use, write a ticket
  with the measured times.

**Estimated size:** about 2,000 lines (code 1,000, tests 750, docs 250).

**Split:** past 900 lines, split off steps 9–11 (the launcher, the
`.mcp.json` entry and the template's member) into a new ticket, blocked
by nothing; then past 900 again, split off steps 5–6 (the server's own
staleness and the embedded tables read from disk).

## Implementation steps

1. Read ADR-0031, ADR-0035, ADR-0071, `crates/cli/src/mcp/server.rs`
   (`stale`, `answer_of`, the `start` path near `self.game.take()`),
   `crates/cli/src/mcp/child.rs`, `crates/cli/src/mcp/host.rs` (`start`,
   `step`), `crates/cli/src/mcp/forward.rs` (how a forwarded server's
   `initialize` and `tools/list` are handled), `crates/cli/src/watch.rs`
   (`plan`), `crates/app/src/folder.rs` (`read_folder`, `watch::scan`)
   and `crates/platform/src/headless.rs` (`change_file`,
   `changed_files`).
2. **The folder reader.** `shuyi-cli` never depends on `shuyi-app`
   (`tools/layers.ron`). Read the folder with `std::fs` into a
   `Layer::owned`, as `contract::import` does (about line 304). If that
   would copy `shuyi_app::read_folder`'s walk and `watch::scan`, move
   both into one function in `shuyi-platform`'s `std` part that both
   call, and keep `check-layers` passing.
3. **`DiskLayer`** (`crates/cli/src/contract/session/disk.rs`, new):
   `DiskLayer::from_env() -> Option<Result<DiskLayer, String>>` reads
   `WATCH_VAR` and `WATCH_PRIVATE_VAR` when `DEBUG_TOOLS` is on, and
   holds each folder, its `Layer` and its last scan.
   `DiskLayer::changed(&mut self) -> Changes` scans again and returns
   the changed, added and removed paths, with the new bytes of each
   changed one. `Headless::with_disk(disk)` keeps it. `BasicTools::new`
   lays the layers with `BasicGame::over` (and `private`) when
   `from_env` gives them, and a read error becomes the
   unreadable-files problem (`BasicGame::files_unreadable`). Hello,
   Grid's `session_of` (`examples/hello-grid/src/tools.rs`) opts in.
4. **Reload before `step`.** Add a provided `Session::take_disk_changes(&mut
   self) -> Vec<Reloaded>` (none by default). `Headless` queues each
   changed file with `Harness::change_file`, plays the step, then reads
   which modules noted which path. For that, `Ctx` keeps the last
   frame's `note_reload` calls (id and outcome) beside its errors
   (`crates/ui/src/ctx.rs`, `module::ReloadErrors`), readable with a new
   `Ctx::last_reloads`. `host::step` puts the list in the answer
   (`RunOutput::reloads`, skipped when empty) and the limits from the
   scope. `scene`, `frame_png`, `perf` and `events` only count the
   pending changes for their limit.
5. **The game's fingerprint.** `crates/cli/src/mcp/fingerprint.rs`
   (new): `Fingerprint::of(root, leave_out) -> Fingerprint` (a sorted
   list of path, length, modified time) and `Fingerprint::changed(&self,
   now) -> Vec<String>`. Over 20,000 files it stops and says so in a
   limit. The server takes it after a good `start`, with the workspace
   root forward.rs already finds (`cargo locate-project --workspace`),
   and checks it in each session tool's answer (`answer_of`): the limit
   and `stale: true`. The server's own (the scope's `crates/cli/` rule)
   is taken in `Server::new` and checked at most every 2 s.
6. **Tables from disk.** `crates/embed/src/lib.rs`: `Table::write_to`
   also writes `pub const <NAME>_ROOT: &str` with the folder it walked.
   `crates/cli/src/embedded.rs` (new): `fn bytes(root, path, embedded)
   -> Cow<[u8]>` reads `root/path` in a debug build when `root` exists,
   else the embedded bytes. Use it where the template, the skill pack
   (`Pack::embedded`, `crates/cli/src/skills.rs`) and the targets are
   read. A test: a debug build reads an edited file from a temp copy.
7. Contract and schemas: the new `reloads` and `stale` fields in
   `session.step.output.json` and the other session tools' outputs;
   regenerate the schemas; the new code's page
   `crates/cli/errors/SH3018.md` (what happened, how to fix it).
8. Docs: `docs/contract.md`, *The MCP server* (the disk layer, the
   reload list, the stale limit, the launcher); `docs/formats.md`, *Hot
   reload* (the session's layer); the book's
   `site/content/learn/book/tools-for-agents.md`; the shipped skill
   `skills/shuyi-new-game/SKILL.md`, which names the MCP server; the
   ADR.
9. **The launcher.** `crates/mcp-launch/` (new; `Cargo.toml` with
   `publish = false`, `src/main.rs` over a `src/lib.rs` with
   `pub fn main()`): read JSON-RPC lines on stdin; answer `initialize`,
   `notifications/initialized` and `tools/list` as the scope says; start
   the real server from its arguments with piped stdio; forward each
   message and answer once it has answered `initialize`; hold tool
   calls with a deadline; send `list_changed`. The real server writes
   its `tools/list` to `target/shuyi/mcp/tools.json` as it starts
   (`crates/cli/src/mcp/server.rs`). Check that the outer `cargo run`
   holds no build lock while the launcher runs; if it does, the
   launcher runs the built `shuyi` binary through a second `cargo build`
   first and says so in the ADR.
10. `.mcp.json`: the launcher's entry. `mcp --ready` (`crates/cli/src/mcp.rs`,
    `crates/cli/src/mcp/install.rs`) also builds `shuyi-mcp-launch`.
    `tools/layers.ron` gets its line.
11. The template: a member crate `crates/mcp/` whose `main` calls
    `shuyi_mcp_launch::main()`, depending on `shuyi-mcp-launch` by the
    same source as the other engine crates; `shuyi mcp install`
    (`crates/cli/src/mcp/install.rs`) writes `cargo run -q -p mcp --
    <command> mcp` when the project has that member, else as now. The
    template is checked by CI's template job, not `cargo fmt --all`.

## Acceptance criteria

- [ ] In a session on a game made by `shuyi new grid`, a picture changed
      in `files/` after `start` shows in the frame the next `step` paints,
      with no rebuild, and `step`'s `reloads` lists its path.
- [ ] In a Hello, Grid session, `palette.ron` changed after `start` is
      reloaded by its palette module at the next `step`; a broken
      `palette.ron` gives that module's error in `reloads` and keeps the
      old colours.
- [ ] A changed file no module reloads gives the limit saying the game
      keeps the old value until `start`; an added file gives the limit
      saying it needs `start`.
- [ ] A `.rs` file of the game touched after `start` makes the next
      `step`, `scene` and `frame_png` answers carry `stale: true` and the
      limit naming that file; after `start` again, neither appears.
- [ ] A data file changed under the disk layer's folder never makes an
      answer stale.
- [ ] A release build of the tool program lays no disk layer, and its
      `step` lists no reloads.
- [ ] A server whose `crates/cli/src` file changed after it started puts
      the reconnect limit on its next tool answer.
- [ ] A debug `shuyi` reads an edited skill from the engine checkout
      without a rebuild; with the checkout's folder renamed it uses the
      embedded copy.
- [ ] With the real server's build faked to take 60 s, the launcher
      answers `initialize` in under 2 s, a `tools/call` made then waits
      and is answered by the real server, and `list_changed` is sent when
      the lists differ.
- [ ] With the build faked to take longer than the wait, the call is
      answered with `SH3018`, and `shuyi explain SH3018` prints its page.
- [ ] With the build faked to fail, the call is answered with `SH3005` and
      the build's last output, and the next call starts the build again.
- [ ] From a fresh worktree with no `target/`, Claude Code connects to
      `shuyi` without `CONNECTION_CLOSED` (checked by hand once on Nick's
      Windows machine, the time to `initialize` in the completion notes).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `DiskLayer::changed` for a changed, added and removed file, and
  for a file over the size limit. `Fingerprint::changed` leaves out the
  disk layer's folders, `target/` and `.git/`, and stops past its file
  limit. `Ctx::last_reloads` holds one frame's notes and empties the
  next. `embedded::bytes` reads disk in a debug build and falls back.
  The launcher's message handling with a fake child (a pipe pair): the
  early `initialize`, the cached list, forwarding, the deadline, a
  failed build and its retry.
- Property: for random edit sequences over a small folder, the paths
  `DiskLayer::changed` reports equal the files whose bytes differ from
  the last scan.
- Snapshot / integration: an MCP session test (`crates/cli/src/mcp/tests.rs`,
  with the `InProcess` host) that edits a file between two `step`s and
  checks `reloads`; one that touches a `.rs` file and checks `stale`; a
  test in `crates/mcp-launch/tests/` that runs the built launcher over a
  fake server script; the schema snapshots hold the new fields.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
