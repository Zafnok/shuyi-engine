---
id: "3452"
title: "The web build's JS plugins are tested by one table: a row per plugin, so a new plugin adds a row and not three tests"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3452 — The web build's JS plugins tested by one table

## Context

The redundancy sweep of 2026-11-28 found that `crates/cli/src/web.rs`
(about 1689–2100) tests each JS plugin with the same three tests,
copied:

- that the plugin provides exactly the functions its Rust side imports;
- that its version matches the Rust constant;
- that `index.html` loads it in the right place (for example
  `index_html_loads_sound_js_between_the_bundle_and_the_game`).

The plugins are gamepad, languages, sound, player files, page files,
pack file and storage: about 330 lines. One table over (JS file, Rust
source, import prefix, plugin name, version constant, load position) is
about 100 lines. The per-plugin extra checks stay as small extra tests:
sound's `fill` export, and the import counts.

Four open tickets each add one more such plugin, and would each add
three more copies (about 40 lines each): 3068 (the clock), 3043 (the
lifecycle), 2377 (reduced motion) and 3229 (context loss). (3062 adds
no plugin.)

## Nick input

None.

## Scope

**In:**
- One table-driven test set in `web.rs` for every plugin, and the
  plugin's extra asserts kept as their own tests.
- **Either order with 3068, 3043, 2377 and 3229:** if one is done,
  its plugin becomes a row and its three copied tests go. If not,
  nothing to do (it adds a row, as its clause says).
- **Behaviour:** none. Tests only.

**Out (do not do):**
- Merging the plugins into one JS file: their versions would be
  coupled.
- Changing `index.html`'s load order.

## Implementation steps

1. Write the table and the three table tests.
2. Delete the per-plugin copies, keeping the extra asserts.
3. Check that a wrong version or a missing function in one row fails,
   naming the plugin.

## Acceptance criteria

- [ ] `grep -c "fn index_html_loads_" crates/cli/src/web.rs` is 1 (the
      table's test).
- [ ] Changing one plugin's version constant fails the table test with
      that plugin's name.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the table over every plugin, plus the kept extra asserts.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
