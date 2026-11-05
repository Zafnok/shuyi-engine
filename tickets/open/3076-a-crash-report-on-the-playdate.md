---
id: "3076"
title: "A crash report on the Playdate: the panic handler writes a CrashReport through the backend's storage, offered at the next start"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: ["3010", "3046"]
nick_input: none
completed:
---

# 3076 — A crash report on the Playdate

## Context

**From 3010 (done):** `shuyi-playdate`'s panic handler
(`crates/playdate/src/device/panic.rs`) writes the panic's message to the
SDK's error screen and console. 3010 left writing a
`shuyi_platform::crash::CrashReport` there to this ticket, since 3046
wasn't done when 3010 landed. **From 3046 (done):** the report, its key
(`crash_key`), `to_text` and `keep_newest` are in
`shuyi_platform::crash`, which builds without `std`
([ADR-0079](../../docs/adr/0079-crash-reports-one-panic-hook-per-backend-writing-through-the-games-storage-offered-once-and-the-games-record-added-on-native.md)).

## Nick input

None.

## Scope

**In:**
- The panic handler writes a `CrashReport` (message and place; no backtrace
  and no record) through the backend's storage before it stops, without
  allocating where the heap is what ran out.
- The next start offers it as `Setup::last_crash`.
- A test on the fake SDK.

**Out (do not do):**
- Sending a report anywhere.

## Implementation steps

1. Read 3046's `crash` module and its section in
   `docs/platform-backends.md`.
2. Write the report from the handler, and read it at the next start.

## Acceptance criteria

- [ ] A panic on the fake SDK's files leaves a report the next start offers.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the report written and read back over the fake SDK.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
