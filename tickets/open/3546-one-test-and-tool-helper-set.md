---
id: "3546"
title: "One set of test and tool helpers in shuyi-cli and xtask: scratch folders, argv, git, cargo metadata and the module catalog's types"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3546 — One set of test and tool helpers in `shuyi-cli` and xtask

## Context

The redundancy sweep of 2026-11-28 found the same small helpers written
again in each module of `crates/cli` and `crates/xtask`. Each verifier
re-counted its lines.

- **Scratch folders.** 25 files define their own
  `fn scratch`/`fn temp_dir`. Most are the same six-line body:
  `temp_dir().join(format!("shuyi-<prefix>-{name}-{pid}"))`, then
  remove, then create. They are listed by
  `grep -rlnE "fn (scratch|temp_dir|tmp_dir)\(" crates`; 23 of them are
  in `crates/cli`. About 180 test lines end with a hand-written
  `let _ = fs::remove_dir_all(…)` that a panic skips. A folder that
  removes itself when dropped already exists, `TempDir` in
  `crates/cli/src/repos/host.rs` (about line 118).
- **argv.** 19 copies of `fn args(&[&str]) -> Vec<String>`
  (`grep -rnE "fn args\(.*&\[&str\]\) -> Vec<String>" crates`).
- **git.** The same `fn git(root, args) -> Result<String, String>` is in
  `crates/xtask/src/docs_rules.rs:321`, `pr_size.rs:397`, `gates.rs:1022`,
  `crates/cli/src/private_assets.rs:70` and `worktrees/trim.rs:677`.
  Variants are in `merges.rs`, `changelog.rs`, `ids.rs`, `paths.rs`,
  `book_pad.rs` and the tickets lint. `base_of` is byte-identical in
  `docs_rules.rs:365` and `pr_size.rs:452`.
- **cargo.** Three identical `cargo locate-project` calls:
  `catalog.rs`, `export/paths.rs` and `notices.rs`. The pipes-and-tail
  block in `mcp/child.rs` and `mcp/forward.rs` is about 20 lines,
  repeated.
- **The module catalog.** `crates/xtask/src/modules.rs:73-160`
  re-declares `Concerns`, `Concern`, `Pick`, `Choice`, `Info` and
  `Answered`, which `shuyi_cli::catalog` owns, and their own docs say so.

That is about 600 lines saved, almost all of it test code.

## Nick input

None.

## Scope

**In:**
- **`shuyi_cli::scratch::Scratch`**, promoted from `repos::host::TempDir`:
  it derefs to `Path`, takes an optional parent, and has `keep()` for a
  test that inspects the folder after a failure. The xtask uses it
  through its `shuyi-cli` dependency.
- **A `#[cfg(test)] pub(crate) mod testing` in `crates/cli`** with
  `scratch(prefix, name)` and `argv(&[&str])`. The variants (aside's
  debug and deps folders, attached's `shuyi.ron`, the ledger's counter)
  keep a wrapper of two or three lines. Do the same in `crates/xtask`.
- **`shuyi_cli::tools::git(dir, args)`**: untrimmed, with an option for
  `GIT_TERMINAL_PROMPT=0`. Add `default_base(root)` and
  `workspace_manifest(root)`, plus `take_pipes_and_tail(child)` for
  `mcp`. Each caller keeps its own error wording and its own trimming.
- **`crates/xtask/src/modules.rs`** reads
  `shuyi_cli::catalog::{Concerns, ModuleInfo}`. `deny_unknown_fields`
  replaces its `KEYS` check.
- **Behaviour:** none for a game. Only test and tool internals change.

**Out (do not do):**
- **The changed-files listings.** They differ on purpose: pr-size uses
  `numstat -z -M` with untracked files, docs-rules uses `--name-only
  --no-renames`, and the tickets lint leaves untracked files out. Keep
  each one.
- Sharing `argv` across crates: it would need a test-support feature.
- The flag parsers themselves: 3451.

**Estimated size:** about 1,200 lines changed (about 900 removed, 300
added).

**Split:** past 900 lines, split off steps 3–4 (the git, cargo and
catalog helpers) into a new ticket. Keep the scratch folders and argv
here.

## Implementation steps

1. Move `TempDir` to `crates/cli/src/scratch.rs` as `Scratch`. Keep a
   `pub use` in `repos::host` only if the API job asks for it.
2. Add the `testing` modules. Replace the scratch helpers and the `args`
   copies, and drop the trailing `remove_dir_all` teardowns where the
   guard now lives to the end of the test.
3. Add `crates/cli/src/tools.rs` (git, base, manifest, pipes). Route the
   copies through it, keeping each caller's error text.
4. Point `modules.rs` at the catalog types and delete its own.

## Acceptance criteria

- [ ] `grep -rlnE "fn (scratch|temp_dir)\(" crates` finds only the shared
      helpers and the wrappers this ticket keeps.
- [ ] `grep -rnE "^fn git\(" crates` finds one definition.
- [ ] Every cli and xtask test passes, and no test leaves a
      `shuyi-*` folder in the system temp folder after it passes.
- [ ] `cargo xtask check-modules` gives the same result on
      `docs/modules.ron`, and names an unknown key.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Scratch` removes its folder on drop and on panic, and `keep()`
  keeps it. `git` with a bad argument returns the error text.
- The existing tests are the regression suite.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
