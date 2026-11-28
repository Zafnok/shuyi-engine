---
id: "3444"
title: "An edition's cut list in shuyi_embed: leave out files and folders, keep some back, keep only named entries of a RON file, swap a text, strip RON comments"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3444 — An edition's cut list in `shuyi_embed`

## Context

- **The game ticket:** game#1401
  (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1401-the-demo-cut-on-the-engines-cut-list.md`),
  from the game's third engine-friction sweep, game#1397 (row C2). It is
  not a blocker.
- **The job:** build a smaller edition of a game (a demo, a trial, a
  console build) from the same files, leaving out what it doesn't ship
  and trimming the data files it ships to the entries it uses.
- **The friction numbers:** sweep game#1397.
  - What the game would keep: about 80 lines, its language-pack rules
    and its list files.
  - What it has today: about 470 production lines,
    `crates/content/src/demo_bundle.rs` (game tickets 0914 and 0934,
    game ADR-0119).
  - Ratio: about 6×. Outcome 1, an engine feature.
  - Game#1288 merges the game's second parser, in `build.rs`, into this
    one first.
- **What the engine has:**
  - Editions with a content root (`Edition::content`, handed to the
    build as `SHUYI_CONTENT`, ADR-0100, `crates/cli/src/project.rs`).
  - `shuyi_embed::Table` with `skip` patterns.
  - Nothing that trims what an edition embeds.
- **The generic shape:** a cut list, a small text file read by
  `shuyi_embed::Table::cut(path)` in a build script. It holds these
  lines:
  - `folder/` and `file.ext` leave out (a folder and everything below
    it);
  - `+ path` keeps a file a folder's line left out;
  - `keep file.ron: a b` keeps only these entries of the file's
    top-level map or list. An entry is kept by its key: a map's key, a
    list's string, or a struct's `id`. Keys made from it with a `.` are
    kept too (`a` keeps `a.first`).
  - `keep file.ron field: a` does the same for its field `field`;
  - `swap file: A => B` replaces text found exactly once.

  Every embedded `.ron` file loses its comments. A line that matches
  nothing fails the build, so the list can't go stale. The rules are
  pure and take the texts handed in, so a crate's tests check the same
  code the build runs.
- **An example from a game that isn't tactics:** a farming game's demo
  keeps the first season's crops in `crops.ron`, swaps the calendar's
  `last_season` and leaves out the later maps.

## Nick input

None.

## Scope

**In:**
- `shuyi_embed::cut`: `Plan::parse(text) -> Result<Plan, Vec<LineError>>`
  and `Plan::left_out(path)`.
- `Plan::rewrite(path, text) -> Result<Cow<str>, Error>`: entries, swaps
  and stripped comments.
- `Plan::unused() -> Vec<LineError>`, the lines that matched nothing.
- `Table::cut(list_path)`, which applies the plan while writing the
  table, plus `rerun-if-changed` for the list.
- A RON comment stripper that leaves strings alone.
- A hook for a game's own rules on top: `Plan::with(rule)`, a trait
  with `left_out(path, &Plan) -> Option<bool>` and
  `rewrite(path, text, &Plan)`. The game's language-pack and dialogue
  rules stay in the game behind it.
- **The API is additive.** New types are `#[non_exhaustive]`.
- **What changes for the game:** game#1401 replaces its `demo_bundle.rs`
  with this, in the PR that moves the pin.

**Out (do not do):**
- Language packs, dialogue line ids, or anything a game names; those go
  through the hook.
- Cutting art by path (the game's `art` lines are its own).
- Changing `shuyi export --edition`'s flags.

**Estimated size:** about 1,100 lines (code 450, tests 550, docs 100).

**Split:** past 900 lines, split off `keep` for a field of a struct
(the `keep file.ron field:` form) into a new ticket.

## Implementation steps

1. Read the game's `crates/content/src/demo_bundle.rs` and its tests
   (public repository) as the reference for the rules and errors.
2. Add `crates/embed/src/cut.rs`: the parser, the matcher (reusing
   `matches`), the RON entry filter and the comment stripper.
3. Add `Table::cut` and its rebuild lines.
4. Add the hook trait.
5. Add an example in the module docs, and a page in the book's export
   chapter on editions.

## Acceptance criteria

- [ ] `keep data/names.ron: a b` keeps exactly `a`, `b`, `a.x` and
      `b.y` of a map, and the rest of the file's text is byte-for-byte
      the same.
- [ ] A `keep` or `swap` that matches nothing fails `Table::write` with
      the list's line number.
- [ ] Comments inside a RON string survive stripping. `//` and `/* */`
      comments elsewhere are gone.
- [ ] `+ path` keeps a file whose folder is left out.
- [ ] An embedded table built with a cut list differs from one without
      it only in what the list names.
- [ ] The book's chapter is updated (2802), or the completion notes
      have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated (`shuyi-ship`, editions),
      or the completion notes have a `Skills:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: every line kind, and errors with line numbers.
- Property: stripping comments then parsing gives the same RON value as
  parsing the original.
- Integration: a build-script fixture (`write_to`) with a list.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
