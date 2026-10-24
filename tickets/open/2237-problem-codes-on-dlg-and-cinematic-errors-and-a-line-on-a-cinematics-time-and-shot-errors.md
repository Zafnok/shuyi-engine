---
id: "2237"
title: "Problem codes on .dlg and cinematic errors, and a line on a cinematic's time and shot errors"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2237 — Problem codes on `.dlg` and cinematic errors

## Context

Found by the skill pack's authors (1134), writing the worked examples of
`shuyi-dialogue` and `shuyi-cinematic`. ADR-0028 gives every error the
contract shows a stable code, so an agent branches on the code, never
the message. 2530 (done) gave codes to the keymap, language and map
validators (`codes::UNKNOWN_NAME`, `DUPLICATE`, `OUT_OF_RANGE`, with
`ContentError::with_code`). Two of the engine's text formats were left
out:

- **`.dlg` files** (`crates/assets/src/dialogue/parse.rs` and
  `check.rs`): every problem has its line (`Parser::err` and the
  checker's `err` call `ContentError::at`), but none has a code. `cargo
  shuyi check` and `import` answer `code: null` for them. Only the file
  that isn't UTF-8 has one (`codes::NOT_TEXT`, `dialogue.rs`).
- **Cinematics** (`crates/assets/src/cinematic.rs`, `check`): a time, a
  length, an id or a shot that is wrong gives a `ContentError::new(file,
  message)` with neither a line nor a code. Only the file's syntax gets
  `SH2001` and a line, from the RON reader.

So `skills/shuyi-dialogue/SKILL.md` (*When it fails*: "A `.dlg` problem
carries no `code` of its own") and `skills/shuyi-cinematic/SKILL.md`
(*When it fails*: a problem of the times or the shots names the shot)
teach messages, where every other format's skill teaches codes.

## Nick input

None.

## Scope

**In:**
- **Codes for `.dlg` problems,** each from the existing content codes, at
  every `err` site of `parse.rs` and `check.rs` (the `err` helpers take a
  `Code`):

  | Problem | Code |
  | ------- | ---- |
  | a character that isn't plain ASCII, a bad id, a scene with no `@end`, an `@else` twice, a block not closed (`@endchoice`, `@endif`), a directive's arguments, `"name:"` with no text | `SH2001` parse |
  | an unknown directive, character, name id or `{token}` | `SH2008` unknown name |
  | a scene id or a line id given twice (`check_duplicates`) | `SH2009` duplicate |
  | text longer than the box's limit, a reply too long, too few or too many replies or reaction texts, a scene with no speech or narration | `SH2010` out of range |

  A problem a game's own `Directive` reports (`Parsed::error`) keeps no
  code: its words are the game's.
- **Codes and lines for cinematic problems:**
  - `cinematic::check` gives each problem its code: an `id` that isn't
    the file's name and a music cue the manifest lacks `SH2008`; a
    length out of range, no shots, a time that isn't one, a first shot
    not at 0, shots out of order and a shot past the end `SH2010`. It
    keeps its signature.
  - A new `cinematic::check_in(file, stem, source, raw, audio, shots)`
    calls `check` and puts each problem on its line in `source` (the
    file's text): the `id:` line, the `length:` line, the `shots:`
    line, and for shot *n* the line of its `at:`. It uses
    `error::line_of` or a helper like the keymap's `line_of_quoted`. A
    problem the game's `ShotCheck` reports for shot *n* goes on shot
    *n*'s line too, with no code. `from_source` calls `check_in`.
- **The docs:** `docs/formats.md`, *Checks after loading*, names the
  codes of the `.dlg` and cinematic checks, and `check_in` beside
  `cinematic::check`. Each code's page in `crates/cli/errors/` already
  says what a problem of its class means. Add the formats to a page's
  list of examples only where it has one. The keymap's
  `line_of_quoted` is private to `keymap.rs`: move it to `error.rs`
  beside `line_of` if `check_in` uses it.
- **The skills:**
  - `skills/shuyi-dialogue/SKILL.md`, *When it fails*, branches on
    `SH2001`, `SH2008`, `SH2009` and `SH2010`. Its worked example
    (`example.ron`) expects `SH2001` on the unclosed `@choice` and
    `SH2010` on the line too long.
  - `skills/shuyi-cinematic/SKILL.md` reads the file with `check_in`,
    and *When it fails* branches on the codes, with the problem's line.
    Its example (`example/intro.rs`, `example.ron`) expects `SH2010`
    and the line on the first shot at 0.5.
  - `check-skills` passes, and both examples pass in CI's `skills` job.
- **Either order with 2364:** if 2364 is done, `check_with`'s track
  problems get codes too (`SH2008` for a track the game didn't declare,
  `SH2010` for the rest), and `check_in` places them on their track's
  line. If not, nothing to do (2364 gives its problems codes from this
  ticket's table, by its own clause).

**Out (do not do):**
- new problem codes: every problem above fits a content code that
  exists. A new one would need its row and its page (ADR-0028);
- codes for problems a game's own code reports (its `Directive`s, its
  `ShotCheck`);
- Yarn Spinner and ink (2202), which bring their own readers.

## Implementation steps

1. Read 2530's completion notes and `crates/assets/src/keymap.rs`
   (`with_code`, `line_of_quoted`), then the `.dlg` parser and checker,
   and `cinematic.rs`.
2. Give the `.dlg` `err` helpers a `Code`, and each call its code from
   the table.
3. Give `cinematic::check`'s problems codes; write `check_in` and have
   `from_source` call it.
4. Update `docs/formats.md`, the two skills and their worked examples.
5. Run `cargo xtask skill-examples shuyi-dialogue shuyi-cinematic`, and
   the gates.

## Acceptance criteria

- [ ] Every `.dlg` problem the engine's parser and checker give has a
      code and a line (a test over one file per row of the table).
- [ ] Every cinematic problem `check` gives has a code, and `check_in`
      gives each its line (tests).
- [ ] `cargo shuyi check` on a short-path game answers the codes (the
      two skills' worked examples).
- [ ] `check-skills` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/assets/src/dialogue/tests.rs`): one broken scene per row
  of the table, its code and line.
- Unit (`crates/assets/src/cinematic/tests.rs`): each of `check`'s
  problems with its code; `check_in` places the id, length and shot
  problems on their lines, and a `ShotCheck` problem on its shot's line.
- Integration: the two worked examples in CI's `skills` job.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
