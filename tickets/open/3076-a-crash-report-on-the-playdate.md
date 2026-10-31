---
id: "3076"
title: "A crash report on the Playdate: the panic handler writes a CrashReport through the backend's storage, offered at the next start"
type: feature
milestone: After MVP
model: reasoning
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
SDK's error screen and console. 3010 says that once 3046 is done it also
writes a `shuyi_platform::crash::CrashReport`; 3046 wasn't done when 3010
landed, so this ticket holds that part.

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
