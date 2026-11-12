---
id: "3311"
title: "The storage's log lines print their error on the web, not a literal {e}"
type: bug
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3311 — The storage's log lines print their error on the web, not a literal {e}

## Context

Found by 3033's browser check (2026-11-12): with `localStorage` blocked,
the web build's console showed

```text
storage: smoke check failed: {e}
```

with the braces and the name as written, not the error. On the web,
macroquad's logging macros (`macroquad::prelude::error!` and the rest)
don't fill in an argument captured inside the string, so a line written
`error!("storage: smoke check failed: {e}")` prints `{e}`. Every other
log line in `shuyi-app` passes its arguments after the string
(`error!("{}", e)`), and prints them.

Two lines in `crates/app/src/storage/mod.rs` capture inside the string:
the smoke check's failure (`smoke_check`) and the storage that can't be
opened (`"storage: {e}; saves and settings will not persist"`).

## Nick input

None.

## Scope

**In:**
- Both lines pass `e` after the string, as the others in `shuyi-app` do.
- A test that keeps it so: no call of a macroquad logging macro in
  `shuyi-app`'s sources captures an argument inside its string (a scan of
  `crates/app/src`, as `shuyi-cli`'s tests scan the plugins).

**Out (do not do):**
- Changing which logger `shuyi-app` uses.

## Implementation steps

1. Find every macroquad log macro call in `crates/app/src` whose string
   holds `{name}`; fix them.
2. Write the scan test.
3. Check the web build's console with storage blocked (3033's completion
   notes say how).

## Acceptance criteria

- [ ] With `localStorage` blocked, the web build's console says why the
      smoke check failed.
- [ ] The scan test fails on a line written like the old one.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the scan.
- Manual: the browser check (step 3).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
