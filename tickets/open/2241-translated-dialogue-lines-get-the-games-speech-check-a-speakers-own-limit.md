---
id: "2241"
title: "Translated dialogue lines get the game's speech check: a speaker's own limit applies to every language"
type: bug
milestone: After MVP
model: implementation
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2241 — Translated dialogue lines get the game's speech check

## Context

Found by an analysis of the engine for areas that do the same job twice
(2026-10-28), beside 2239.

A game can limit what one character says beyond the text box's limit:
`SceneRules::speech_problem(&self, speaker: &CharacterId, cells: usize)
-> Option<String>` (`crates/assets/src/dialogue/check.rs`), "What is
wrong with `speaker` saying a line of `cells` cells (at most, tokens
filled in), if anything, beyond the box's limit". A game uses it for a
speaker whose portrait leaves less room, or one who only ever says a few
words. The `shuyi-dialogue` skill lists it among the hooks a game
writes (`skills/shuyi-dialogue/SKILL.md`).

Only English lines are checked against it. `Checker::step` calls it for
every `Step::Say`, with the line's length counted with every token at
its longest (`check.rs`, the `Step::Say` arm). A language pack's lines
go through `add_lines` (`crates/assets/src/lang/lines.rs`), which checks
the tokens and the box's limit (`limit(kind, limits)`) and nothing of
the game's: `PackRules` (`crates/assets/src/lang.rs`) carries the game's
`data_problem` for data text, its tokens, the limits and the variants,
but no speech rule. It couldn't use one anyway: `SourceLine`
(`crates/assets/src/lang/source.rs`), English's line as a pack sees it,
holds the scene, the text, the `LineKind` and the file, not who says
the line. `SourceText::add_dialogue` has the speaker (`line.speaker`)
and drops it.

So a translation can be as long as the box allows for a speaker the
game keeps to fewer cells, and loads. Translations run longer than
English (German and French often by a third), so this is where the
game's rule matters most.

## Nick input

None.

## Scope

**In:**
- **`SourceLine` knows its speaker:** a new field `speaker: String`,
  the line's speaker as the scene has it (`line.speaker` in
  `SourceText::add_dialogue`): a character id for speech,
  `NARRATION_SPEAKER` (`">"`) for narration, `REPLY_SPEAKER` for a
  reply, and empty for a caption (`add_captions`). Its doc says so.
- **`PackRules` carries the game's speech rule:** a new field
  `speech_problem: fn(speaker: &CharacterId, cells: usize) ->
  Option<String>`, a plain function as `data_problem` is, documented as
  the same rule as the game's `SceneRules::speech_problem`, run on a
  pack's speech lines. `LangPackRon` (`crates/assets/src/lang/formats.rs`)
  gets the same field, defaulting to `|_, _| None`, and a builder
  `speech_problem(self, f) -> Self` beside `data_problem`, and hands it
  to the `PackRules` it builds. Every other `PackRules { … }` in the
  workspace sets it to `|_, _| None` (grep `PackRules {`:
  `crates/assets/src/format/builtin_tests.rs`, `lang/formats.rs`,
  `lang/pack_tests.rs`, `lang.rs`'s tests, `crates/cli/src/lang_status.rs`,
  `crates/ui/src/toy.rs`).
- **`add_lines` runs it** on each text of an entry (`LineText::all`, so
  every variant) whose English line is `LineKind::Speech` and whose
  speaker is not `NARRATION_SPEAKER`, as English runs it on `Say` only.
  The cells are the `width` it already measures (`longest_width` with
  the pack's longest name), so a name token counts as the pack's
  longest name, as English counts its own. The message is the game's,
  after the entry's key, as the other errors there are (`at(&message)`).
- **The docs:** `SceneRules::speech_problem`'s doc says a game that
  checks packs passes the same rule to `LangPackRon::speech_problem`;
  the `shuyi-dialogue` skill's line on the hooks says the same (and
  `check-skills` passes).
- **Either order with 2239:** both change `add_lines`' loop over a
  text's variants. If 2239 is done, the box check there is a `match` on
  `box_fit` for a `Speech` line: run `speech_problem` after it, on the
  same `width`, whatever `box_fit` answered (English reports both);
  if not, run it after the `limit` check, and 2239 keeps it when it
  rewrites that check.
- **For the game:** two breaks, each with a `Breaking:` line (ADR-0038
  §6; CI's `api` job finds both): `PackRules` and `SourceLine` gain a
  field, so a struct literal of either needs it (`speech_problem: |_,
  _| None`, or the game's rule; `speaker`). A game that builds packs
  through `LangPackRon` and never writes either literal breaks nothing.
  The game's PR that moves the pin passes its speech rule to
  `LangPackRon::speech_problem`, and fixes any pack line it then
  refuses.

**Out (do not do):**
- The other `SceneRules` hooks (`expression_problem`, `music_problem`,
  `scene_problems`) for packs: a pack translates text only, so there is
  no expression or music of its own to check.
- Making `PackRules` or `SourceLine` `#[non_exhaustive]`: a larger
  break than this bug needs; a later ticket can if more fields come.
- The box's own limits on pack lines: 2239.

## Implementation steps

1. Read `Checker::step`'s `Step::Say` arm in `dialogue/check.rs`,
   `SourceText::add_dialogue` and `add_captions` in `lang/source.rs`,
   `add_lines` in `lang/lines.rs`, and `PackRules` and `LangPackRon`.
   Check whether 2239 is done (the clause above).
2. Add `SourceLine::speaker` and fill it in both places it is built.
3. Add `PackRules::speech_problem`, `LangPackRon`'s field and builder,
   and set it in every `PackRules { … }` the workspace writes.
4. Run it in `add_lines` as Scope says.
5. The docs, then the tests below; run the gates.

## Acceptance criteria

- [ ] With the tests' `RULES` and a `speech_problem` that refuses a
      `hero` line over 20 cells (as `crates/assets/src/dialogue/tests.rs`'s
      rules do), a pack's line said by `hero` of 25 cells fails with
      the game's message, in `text` and in a variant, and the same text
      said by another speaker, as narration or as a reply loads.
- [ ] `SourceText::add_dialogue` fills `speaker` with the line's
      speaker, `">"` for narration and `REPLY_SPEAKER` for a reply; a
      caption's is empty.
- [ ] `grep -rn "PackRules {" crates examples` finds no literal without
      `speech_problem`.
- [ ] The existing pack and dialogue tests pass unchanged.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/assets/src/lang/pack_tests.rs`): the acceptance
  criterion's cases, and a line that fits English's name for a name
  token but not the pack's longer name fails (the cells count the
  pack's longest name).
- Unit (`crates/assets/src/lang/source.rs`'s tests): the speakers of a
  scene's speech, narration, reply and caption.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
