---
id: "3047"
title: "Parked: Crash reports sent, if the player agrees, to an address the game sets"
type: feature
milestone: "Engine: crash reports"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["3046", "2601"]
nick_input: decision
completed:
---

# 3047 — Crash reports sent to an address the game sets

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; on 2026-10-20 he kept parked the gaps the game doesn't need, and left "sending crash reports" for later): the game doesn't ask for it, so this waits until after the MVP (`docs/ROADMAP.md`), after 3046 (the reports), after 2601 (HTTP requests, parked), which it sends through, after 2601's native and web backends, which 2601 writes as two tickets, and after Nick answers the question below. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo`, drops `Parked:` from the title, and adds the ids of 2601's two backend tickets to `blocked_by`.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6,
Unreal 5.8 and Defold 1.14:

- Unreal's Crash Report Client sends reports to the address a game sets
  (`DataRouterUrl` under `[CrashReportClient]`): Epic's service, or a
  paid one such as Sentry or BugSplat
  (<https://dev.epicgames.com/documentation/en-us/unreal-engine/crash-reporting-in-unreal-engine>).
- Unity sends crashes to its own Diagnostics service.
- Defold reads the last crash at the next start
  (`crash.load_previous()`) and leaves sending it to the game.
- Godot and Bevy send nothing.

What Shuyi has and plans (read 2026-10-21):

- 3046 writes a `CrashReport` (`shuyi_platform::crash`) through the
  computer's storage, keeps the newest five, and offers the newest one
  to the game once, at the next start (`Setup::last_crash`, a
  `LastCrash` with its `key`). Every field but `version` and `message`
  is `#[serde(default)]`, so fields can be added. Nothing leaves the
  machine. A player who wants to help finds the file in the game's data
  folder and sends it by hand.
- 2520 reads a report (`shuyi crash`) and replays its record
  (`replay --crash`).
- 2601 (parked) makes HTTP requests a platform capability. A screen, or
  `App`, queues an `HttpRequest` in `Ctx::http` (an `HttpQueue`);
  `App`'s `step` (`crates/platform/src/app.rs`) hands it to
  `Platform::http()` and brings the `HttpReply` back at a later frame.
  `HttpRequest::post(url, purpose, content_type, body)` builds a POST,
  and every request names its `purpose`. The URL rule allows `https://`,
  and `http://` to a loopback host for tests. `ScriptedHttp` scripts
  replies on `HeadlessPlatform`. Its ADR says the engine never sends
  anything by itself.
- The player's settings (`Settings`, `crates/ui/src/settings.rs`) are
  saved per profile, in a record whose form the game decides
  (`GameSettings::to_ron`). An options screen shows the rows a game
  gives (`OptionRow`, `crates/ui/src/screens/options.rs`); the engine
  offers ready-made ones such as `OptionRow::text_speed`.
- After the load, the computer's storage is the running game's
  `ctx.storage.computer_mut()` (`ProfileStore`,
  `crates/ui/src/profiles.rs`). `Platform::storage` hands its storage
  over once (`HeadlessPlatform` gives an empty one after that), so `App`
  reads and writes reports through `Ctx`, not through the platform.
- 3040 (MVP) gives `App` a log with levels and categories.
- No ticket sends data the player didn't start. 1148 (parked) is the
  only one that writes a privacy policy, for accounts.
- Nick listed "sending crash reports" among the decisions left for
  later (`docs/decisions.md`, 2026-10-20). Sending a player's data off
  their machine brings a privacy policy, the player's consent, and
  maybe a paid service, so it is his call (`AGENTS.md`, *Who decides
  what*).

Why it matters:

- **The game:** few Steam players find a file in a hidden folder and
  post it. Reports that arrive on their own show which crash is common
  after a patch. Visions of Shuyi gets this only if Nick says yes, and
  only if the game sets an address.
- **Other games** get the same, with the same rule: the engine sends
  nothing of its own, ever.

## Nick input

**Decision** (record the answers with the `ask-nick` skill in
`docs/decisions.md`):

1. Should the engine offer sending at all? **Recommended:** yes, off
   unless the game sets an address and the player agrees. The engine
   itself never sends anything (no telemetry, no analytics).
2. Where to? (a) An address the game's owner runs: the engine sends the
   report as JSON over HTTPS, and the owner keeps it, or forwards it
   to a service of their choice. (b) A hosted service with its own
   format: Sentry (a free tier with limits, then paid) or BugSplat
   (paid). Each has its own terms and a data processing agreement; the
   session checks prices and terms on the day. **Recommended:** (a),
   which costs the engine nothing and ties it to no vendor.
3. What may a sent report hold? The message, the place in the code, the
   backtrace, the versions, the OS, the frame number and the top screen
   hold nothing personal. The log lines and the record of the play can
   hold what the player typed, such as a name, and the log can hold file
   paths with the player's account name in them. **Recommended:** the
   first part only, and the log and the record only when the player
   agrees to send them too, report by report.
4. How does the player agree? **Recommended:** the game's notice at the
   next start asks ("Send this report?"), with a setting *Send crash
   reports*: *Ask* (the default), *Always*, *Never*. *Always* sends the
   first part only. Nothing is sent before the player answers.
5. The game's duty: its privacy policy names what is sent and where.
   The engine's docs give a paragraph to adapt. It is not legal advice.

## Scope

As recommended above. If Nick answers otherwise, change this section
in the PR that unparks it, before any code.

**In:**
- **The address:** `Start::send_crashes_to(url: &'static str)`, a
  builder beside 3046's `Start::crash_record`, stored as that one is.
  Without it nothing is ever sent. At the running game's first frame,
  `App` checks the URL with 2601's URL rule; a URL it refuses gives one
  `Warn` line in `App`'s log (3040), category `Platform`, and nothing is
  sent.
- **The player's answer** (`shuyi-ui`):
  - `CrashSending` in `crates/ui/src/settings.rs`: `Ask` (the default),
    `Always`, `Never`, with `ALL` and `key()` as `TextSpeed` has them
    (`options.value.ask`, `options.value.always`,
    `options.value.never`);
  - `Settings` gains `pub crash_reports: CrashSending`. `restored()`
    puts it back to `Ask` with the rest. It is saved only if the game's
    `GameSettings::to_ron` writes it, as every setting is; the toy's
    settings (`crates/ui/src/toy.rs`) write and read it;
  - `OptionRow::crash_reports()`, a `RowValue::Choice` row labelled
    `options.row.crash_reports`, made as `OptionRow::text_speed` is. A
    game that sets an address adds it to its rows. The toy's rows show
    it, and the four new keys go in `crates/ui/src/toy/en.ron`. Either
    order with 2351: if it is done, its built-in English gains the four
    keys; if not, 2351 adds them with the rest;
  - `Ctx::send_crash_report(&mut self, key: &str, with_log_and_record:
    bool)` queues the player's yes; `Ctx::take_crash_reports_to_send()`
    is the platform's side, as `SystemKeyboard::take_ask` is;
  - 3046's `LastCrash` gains `can_send: bool`: an address is set and
    passes the URL rule. The game's notice asks when `can_send` is true
    and `ctx.settings().crash_reports` is `Ask`, then calls
    `send_crash_report` on a yes. The words are the game's (`AGENTS.md`
    rule 4).
- **What a report remembers** (`shuyi_platform::crash`):
  `CrashReport` gains `send: SendState`, `#[serde(default)]`:
  `NotAsked` (the default), `Agreed { log_and_record: bool, tries: u8
  }`, or `Sent`.
- **What is sent:** `SentReport`, built from a `CrashReport`: `version`
  (1, the sent shape's), `when`, `message`, `location`, `thread`,
  `backtrace`, `engine`, `game`, `target`, `frame` and `top_screen`;
  `log` and `record` only with `log_and_record` (`Option`s skipped when
  `None`). It is written as JSON with `serde_json`, added to
  `shuyi-platform` with `default-features = false` and `alloc`, so it
  builds without `std` (MIT OR Apache-2.0; already in `Cargo.lock`
  through `shuyi-cli`, so no new crate). A body over 256 KiB leaves out
  the record, then the log. No player id, no machine id, no header but
  the content type.
- **The sending, through 2601's `Ctx::http`, on native and in the web
  build alike:** a `Sender` in `shuyi_platform::crash` that `App` holds
  and `step` calls:
  - at the running game's first frame, after `step` has set
    `Ctx::http`'s presence: with `Always`, the report `Setup::last_crash`
    named becomes `Agreed { log_and_record: false, tries: 0 }`. Then
    every report in the computer's storage that is `Agreed` with
    `tries` under 3 is sent;
  - after the update, each report the screens named with
    `send_crash_report` becomes `Agreed` with the player's choice and is
    sent. A report already `Sent`, or already sent this start, is
    skipped;
  - to send: `ctx.http.send(HttpRequest::post(url, "crash report",
    "application/json", body))`, queued before `step` hands the
    screens' requests to the platform. Only when `send` returns `Ok`,
    `tries` goes up by one and the report is written back. So a quit
    while the request is in flight counts as a try, and a platform
    without HTTP (`Unsupported`) counts none;
  - the reply, taken by its `RequestId` in a later frame: a `2xx`
    makes the report `Sent`; anything else gives a `Warn` line,
    category `Platform`, with the status or the error, and the report
    waits for the next start. Each report gets one request per start
    and three in all;
  - with `Never`, nothing is sent, earlier yeses included;
  - a frame never waits: 2601's queue answers at a later frame.
- **The receiving side:** a *Sending reports* section in 3046's
  `docs/crash-reports.md`: the JSON fields and an example body; what a
  receiver does (accept the POST, answer `2xx`, and allow the game's
  web origin through CORS for the web build); the paragraph to adapt
  for a privacy policy; how to word the notice from `can_send` and the
  setting; and the rule that the engine sends nothing of its own. The
  docs sketch a receiver; the engine ships none.
- **An ADR** (the next free number, the `write-adr` skill), extending
  2601's and 3046's ADRs (accepted ADRs are not edited): a game may
  send crash reports, only to its own address, and only with the
  player's answer; what a sent report holds, and what only with a
  second yes; three tries at most.
- **Public types:** `CrashSending` and `SendState` are
  `#[non_exhaustive]`. Either order with 2013: if it is done, follow its
  rules for public types; if not, 2013 covers these with the rest.
- **Either order with 2015:** if it is done, `serde_json` joins
  `shuyi-platform`'s `outside` list in `tools/layers.ron`, with its
  comment; if not, 2015 lists it.
- **What changes for the game:** additions only. `Settings` gains a
  field (it has a private field, so no game builds it with a struct
  literal), `Start` a builder, `Ctx` two methods and `LastCrash` a
  field. Say so in the PR.
- **Size:** if the PR passes about 600 changed lines, the setting and
  its row (`CrashSending`, `Settings::crash_reports`,
  `OptionRow::crash_reports`, the toy's row and keys, and their
  criteria) go to a follow-up ticket (the `write-ticket` skill, blocked
  by this one). Until it lands, every report is asked about, as with
  *Ask*.

**Out (do not do):**
- An HTTP client of its own: 2601 and its backend tickets.
- A server or a service of the engine's own.
- A vendor's SDK (Sentry's crates and others), and formats of a
  service's own API. A follow-up ticket if Nick picks one.
- Uploading debug files (2520's debug file) anywhere.
- Analytics or telemetry of any kind.
- Minidumps, and crashes 3046 can't catch (its Out).
- Sending a session record (2518, 2519): Nick's privacy decision there
  is theirs.
- Consent for other data: accounts (1148) and chat (1149) have their
  own.
- The Playdate and console backends.

## Implementation steps

1. Ask Nick the five questions above with the `ask-nick` skill, and
   record his answers in `docs/decisions.md`. If they differ from the
   recommendations, rewrite **Scope** to match before writing code.
2. Check that 3046, 2601 and 2601's two backend tickets are in
   `tickets/done/`, and whether 2013, 2015 and 2351 are. Read 3046's
   `crates/platform/src/crash.rs`, `Setup::last_crash` and
   `Start::crash_record`; 2601's `crates/ui/src/http.rs` and the HTTP
   part of `step` in `crates/platform/src/app.rs`; `TextSpeed` in
   `crates/ui/src/settings.rs`; `OptionRow::text_speed` in
   `crates/ui/src/screens/options.rs`; the toy's settings and rows in
   `crates/ui/src/toy.rs`.
3. In `shuyi-ui`: `CrashSending`, the `Settings` field,
   `OptionRow::crash_reports`, `Ctx::send_crash_report` and its take
   side, the toy's saving and row, and the keys in `toy/en.ron`, with
   their unit tests.
4. In `shuyi-platform`: `SendState`, `SentReport` and its JSON,
   `LastCrash::can_send`, `Start::send_crashes_to` and the `Sender`
   wired into `App` and `step`, with the `App` tests on
   `HeadlessPlatform` and `ScriptedHttp`. A second start in a test is a
   new `App` on a `HeadlessPlatform` given the first game's computer
   storage (`with_storage`).
5. Write the docs section and the ADR, add the ADR's row to
   `docs/adr/README.md`, and run the gates, the `no-std` build
   included.

## Acceptance criteria

- [ ] Nick's answers are in `docs/decisions.md`, and the scope matches
      them.
- [ ] `App` tests on `HeadlessPlatform` with `ScriptedHttp` and a stored
      report: no request without `Start::send_crashes_to`, with `Never`,
      or with `Ask` before `send_crash_report`.
- [ ] With `Always`: one request at the running game's first frame, a
      POST with `purpose` `"crash report"`, whose JSON has no `log` and
      no `record`.
- [ ] A yes with `with_log_and_record: true` sends both; with `false`,
      neither.
- [ ] A `2xx` makes the report `Sent`, and a second start sends nothing.
- [ ] A `500` reply, then a `Network` failure: one request per start,
      three over four starts, none after.
- [ ] Without HTTP on the platform: no request, and `tries` stays 0.
- [ ] An address that fails 2601's URL rule (`http://example.com/c`):
      one `Warn` line, category `Platform`, and no request.
- [ ] A unit test checks the JSON's keys against the list in
      `docs/crash-reports.md`; a report with a 1 MiB record gives a body
      under 256 KiB without `record`.
- [ ] A Harness test on the toy steps the crash reports row through
      *Ask*, *Always* and *Never*, keeps *Never*, and the saved settings
      read it back.
- [ ] `Cargo.lock` gains no crate; `cargo xtask check-no-std` and CI's
      `layers` job (`platform-deps` if 2015 isn't done) pass with
      `serde_json` in `shuyi-platform`.
- [ ] The ADR is written and listed in `docs/adr/README.md`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-ui`): `CrashSending`'s default, its keys and
  `restored()`; the toy's settings saved and read back with it;
  `send_crash_report` queued and taken once.
- Unit (`shuyi-platform`): `SentReport` from a report with and without
  `log_and_record`; the JSON's keys; the 256 KiB cut; an old report
  without `send` reads as `NotAsked`.
- Integration (`App` on `HeadlessPlatform` with `ScriptedHttp`): every
  case in the acceptance criteria; the Harness test of the row.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
