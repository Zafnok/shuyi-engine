---
id: "2352"
title: "Parked: The engine's screens in other languages, machine-translated and labelled as such"
type: feature
milestone: "Engine: localization"
model: implementation
effort: medium
status: blocked
blocked_by: ["2351", "2350"]
nick_input: decision
completed:
---

# 2352 — The engine's screens in other languages

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; on 2026-10-20 he kept parked the gaps the game doesn't need): the game doesn't ask for it, so this waits until after the MVP (`docs/ROADMAP.md`), after 2351, which it builds on, after 2350, whose text scan it extends, and after Nick answers the question below. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Unreal ships the engine's own text already translated:
its `Engine` localization target comes with many cultures
(`Engine/Content/Localization/Engine/<culture>/`), so a game's
translators never translate the engine's dialogs. Shuyi's equivalent
is small: the engine's screens (options, Key bindings, the layout
picker, profiles, name entry, the text box's help) have about 90 short
lines (87 keys in 2351's count).

What Shuyi has and plans:

- 2351 gives those lines built-in English (`crates/ui/lang/en/ui.ron`,
  read by `shuyi_ui::text::engine_english()`), beneath the game's
  English, with `Lang::with_engine_english` in `shuyi_assets::lang`.
  English only.
- A language pack is `<code>/lang.ron` and `<code>/ui.ron`
  (`docs/formats.md`, *A language pack*), checked by
  `pack_from_sources` (`crates/assets/src/lang.rs:445`). Its
  `lang.ron` already says who made it: `LangInfo::made_by`, with
  `MadeBy::Machine` for a machine-made pack
  (`crates/assets/src/lang.rs:100` to `117`).
- `Lang::text` (`crates/assets/src/lang.rs:229`) shows a pack's entry
  only when it was made from today's English, else the English.
- 2350 scans every text a `Lang` holds (`Lang::texts`), puts the
  characters the text uses in a game's font atlas, and makes
  `shuyi check` fail on a character the atlas lacks.
- So today every game with a French pack translates the same engine
  lines again, or shows the engine's screens in English.

Nick listed "machine-translated engine screens" among the decisions
left for later (`docs/decisions.md`, 2026-10-20). Machine-made words in
a game change what a player sees and what a game's credits say, so this
is his call (`AGENTS.md`, *Who decides what*).

Why it matters:

- **The game first:** Visions of Shuyi translates its own text and has
  every engine key in its English, so nothing changes for it unless its
  own ticket turns this on (below).
- **Other games:** a small game that translates its own few lines into
  French gets French options, Key bindings and profiles screens without
  translating them.

## Nick input

**Decision:**

1. Should the engine ship the words of its own screens (about 90 short
   lines) in other languages, translated by machine in a the agent session
   and labelled machine-translated (`made_by: Machine`)? A game that
   turns them on and has a pack in one of those languages shows the
   engine's screens translated. Its own pack's words still win.
   Visions of Shuyi gets them only if a game ticket of its own turns
   them on.
2. If yes, which languages first? **Recommended:** French, German,
   Spanish, Italian and Russian. Their codes are plain two letters, and
   their letters take one cell each. The first four use Latin-1
   letters, which every font atlas the engine's `font-atlas` makes
   holds (`ATLAS_RANGES`, `crates/cli/src/font_atlas.rs:19`). Russian
   draws only in a game whose font has Cyrillic, as Hello, Grid's
   Terminus font does: 2350 puts those letters in the atlas when the
   text uses them, and `shuyi check` names them when the font lacks
   them. Brazilian Portuguese and Simplified Chinese need region and
   script tags (2226). Japanese, Chinese and Korean also need wide
   glyphs (2301, 2303). Arabic and Hebrew wait for research 2908.
3. The cost to know: once this is in, a PR that changes an engine
   screen's English also updates each language, by machine, in the same
   PR. Each language adds a little to every such PR.

4. May people later review and correct these packs (their `made_by`
   would then name them)? That is part of the contribution policy, so
   the answer also goes in `CONTRIBUTING.md` (1132's file; see the
   either-order list below).

Record the answers with the `ask-nick` skill in `docs/decisions.md`.

## Scope

**In:**
- **The engine's packs:** for each language Nick picks,
  `crates/ui/lang/<code>/lang.ron` (its own name for itself and
  `made_by: Machine`) and `crates/ui/lang/<code>/ui.ron` (one entry,
  `(key, source, text)`, for every key of `crates/ui/lang/en/ui.ron`).
  These are the files a game's pack has, so no new format.
  Placeholders stay as they are, `{Confirm}` and `{Cursor}` included
  (`pack_from_sources` refuses an entry whose placeholders differ from
  its source's).
- **`shuyi_ui::text::engine_packs() -> BTreeMap<LangCode, LangPack>`**,
  beside 2351's `engine_english()` in `crates/ui/src/text.rs`:
  - a private array of `(code, lang.ron, ui.ron)`, each file read with
    `include_str!`;
  - each pack checked by `pack_from_sources` against
    `engine_english()`, with the `PackRules` `Toy::new` uses
    (`crates/ui/src/toy.rs:455`: no data rules, `NoTokens`); the
    dialogue limits don't apply to screen text;
  - a pack that fails panics, naming its file. The unit test loads
    every pack, so a broken one fails the tests, not a game;
  - it works without `std`.
- **In `shuyi_assets::lang`:**
  - `Lang::with_engine_packs(self, packs: BTreeMap<LangCode,
    LangPack>) -> Self`, opt-in like 2351's `with_engine_english`;
  - `Lang::text(code, key)`: the game's pack entry made from today's
    English; else the engine's pack of the same code, its entry made
    from today's English; else English;
  - an engine pack is used only under a game's pack of the same code:
    it never adds a language, and `Lang::codes` doesn't change. A key
    whose English the game overrides makes the engine's entry stale,
    so the game's English shows, as it should;
  - `Lang::status`: a key a fresh engine entry covers isn't missing.
    `Lang::from_engine_pack(&self, code, key) -> bool` says whether the
    text comes from the engine's pack, and
    `Lang::engine_pack_codes(&self) -> impl Iterator<Item = &LangCode>`
    lists the game's pack codes that have an engine pack beneath them.
- **2350's scan and check see the engine's packs:** `Lang::texts`
  lists every engine pack entry `Lang::text` would show (under a game
  pack's code, where that pack has no fresh entry of its own) as a new
  `TextPart::EngineScreen`. `font::undrawn` names its file
  `shuyi-ui lang/<code>/ui.ron`. So `font-atlas` puts an engine pack's
  letters in a game's atlas, and `shuyi check` names one the game's
  font lacks.
- **Labelled:** `lang-status` (2351's "engine screen text" part) says
  how many keys come from the engine's machine-translated pack, and
  lists the engine keys it doesn't cover.
- **Kept current:** a test fails when an engine pack lacks a key of the
  engine's English, has a key it lacks, or has a stale entry. The
  `work-ticket` skill (`.claude/skills/work-ticket/SKILL.md`, section
  3, *Implement*) gets a line: a PR that changes
  `crates/ui/lang/en/ui.ron` updates every engine pack in the same PR,
  by machine, still `made_by: Machine`.
- **The toy** (`crates/ui/src/toy.rs`) gets a pack of its own for each
  engine pack's code and builds its `Lang` with
  `with_engine_packs(shuyi_ui::text::engine_packs())`. The first
  language's pack translates `toy.speed` and
  `options.row.text_speed`; the others have an empty `ui.ron` (`[]`),
  so the engine's pack shows under each. Its debug pack (`dbg`) stays
  as it is.
- **A Harness test** (`crates/ui/tests/it/engine_packs.rs`) opens the
  options screen with `h.ctx_mut().lang` set to the first language.
  Through `h.ctx().text`: the title (`options::TITLE`) is the engine
  pack's text, the `toy.speed` row is the toy pack's, and
  `options.row.text_speed`, which both translate, is the toy pack's.
  `h.snapshot()` shows the translated title.
- **Snapshots in each language:** beside the existing snapshot tests
  of the options screen (`crates/ui/src/screens/options/glyph.rs`), the
  Key bindings screen (`crates/ui/src/screens/key_bindings/glyph/tests.rs`)
  and the profiles screen (`crates/ui/src/screens/profiles/tests.rs`),
  one test per screen loops over `engine_packs()`'s codes, sets the
  toy `Ctx`'s `lang`, and takes an `insta` snapshot named by the code
  (`buf.to_snapshot(&c.palette)`, as the tests there do). The snapshots
  are committed and looked at for text cut short. A test finds each
  screen's whole translated title, and every options row label, in
  that screen's snapshot text.
- An ADR (the next free number, `write-adr` skill) that builds on
  2351's: engine packs, machine-made and labelled, beneath a game's
  pack of the same language, opt-in per game with one call.
- `docs/formats.md`, *A language pack*: one sentence on
  `Lang::with_engine_packs`.
- **Either order:**
  - **2226** (language tags and fallback chains): if it is done, the
    engine's packs take part in its chain (a `pt-BR` game pack falls
    back to the engine's `pt` pack before the source text); if not,
    2226 adds them to it, as its clause says.
  - **2223** (credits and notices): if it is done, the credits section
    of a game's `NOTICES.txt` says which languages' engine screens the
    game shows machine-translated (`Lang::engine_pack_codes` of the
    game's `Lang`, read through `GameTools::lang` as `lang-status`
    reads it); if not, 2223 adds that, as its clause says.
  - **2348** (plurals): if it is done, `profile.message.too_many` is a
    plural message in each engine pack, with the categories of the
    pack's language; if not, 2348 makes it one in each engine pack, as
    its clause says (the upkeep test holds its PR to that).
  - **2302** (`max_cells`): if it is done, give the engine's English
    keys a `max_cells` where their place on the engine's screens has a
    fixed width, and the upkeep test also fails on an engine pack entry
    wider than its key's `max_cells`; if not, the snapshots are the
    check, and 2302 adds both (its clause says so).
  - **1132** (community files): if it is done, add Nick's answer to
    question 4 to its `CONTRIBUTING.md`; if not, 1132 writes it there
    from `docs/decisions.md`, as its clause says.
  - **Either order with 2238:** if 2238 is done, test an engine pack's
    entry with `lang`'s private `fresh`, as the game pack's entry is
    tested; if not, nothing to do (2238 moves every test onto `fresh`,
    this one included).

About 550 changed lines, the packs and the snapshots aside.

**Out (do not do):**
- Translating a game's own text.
- A language picker or the player's saved language (2349).
- Languages that need tags with a region or script (2226) or wide
  glyphs (2301, 2303). **Either order with 2226, 2301 and 2303:** if
  they are done, Nick may pick such a language (question 2); if not,
  it is left out, and nothing is left to them (an engine pack in it
  later is a ticket of its own, if Nick asks). Right-to-left and
  shaped scripts (2908).
- Translation by people, or community packs: question 4 asks Nick
  only whether people may correct these packs; more is a ticket of
  its own.
- Translator exchange files for the engine's packs: 2227 (parked)
  exports a game's packs only, and says so. A ticket of its own if a
  person checks the engine's packs in a translation tool.
- Translation at run time, an online service or a paid API: the packs
  are made once in a session and checked in.
- A walk over every screen for cut text: 2353's pseudo-language walk,
  which lengthens every key by a third, the engine's included.
- Turning the packs on in Hello, Grid, the templates or 2511's
  `BasicData`: none has a pack in another language, so the packs
  would never show. A game turns them on where it builds its `Lang`.
- Turning the packs on in Visions of Shuyi (a game ticket, if Nick
  wants it).

## Implementation steps

1. Read Nick's answer in `docs/decisions.md`, 2351's ADR,
   `crates/assets/src/lang.rs` (`Lang`, `pack_from_sources`), 2350's
   `Lang::texts` and `font::undrawn`, and
   `crates/cli/src/lang_status.rs`.
2. Write `Lang::with_engine_packs`, the lookup in `Lang::text`,
   `Lang::status`, `Lang::from_engine_pack` and
   `Lang::engine_pack_codes`, with unit tests on a `Lang` built from
   small packs in the test.
3. Add `TextPart::EngineScreen` to `Lang::texts` and its file name to
   `font::undrawn`, with unit tests.
4. Write the packs, one language at a time, from
   `crates/ui/lang/en/ui.ron`, and `engine_packs()` with its test.
5. Add the upkeep test and the `work-ticket` line.
6. Add the `lang-status` lines and their test.
7. Add the toy's packs, the Harness test and the snapshot tests. Run
   `cargo insta review`, look at each snapshot, and shorten any
   translation that is cut.
8. Write the ADR and the line in `docs/formats.md`; do the either-order
   items that apply.
9. Run the gates.

## Acceptance criteria

- [ ] Every engine pack Nick picked loads through `pack_from_sources`
      against the engine's English, with `made_by: Machine`.
- [ ] A game's pack entry wins over the engine's; the engine's entry
      shows where the game's pack has none; English shows where the
      game overrides the key's English.
- [ ] An engine pack with no game pack of its code adds no language.
- [ ] `lang-status` says which keys come from the engine's
      machine-translated pack.
- [ ] Changing a line of `crates/ui/lang/en/ui.ron` without updating
      the packs fails a test.
- [ ] `Lang::texts` lists the engine pack entries a game shows, and
      `font::undrawn` reports a character of one that the atlas lacks,
      naming `shuyi-ui lang/<code>/ui.ron` and the key.
- [ ] The snapshots of the three screens in each language are
      committed, and a test finds each screen's whole title and every
      options row label in them.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] The ADR is merged.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-assets`): the lookup order of `Lang::text`; no new
  language from an engine pack; `status`, `from_engine_pack` and
  `engine_pack_codes`; `Lang::texts` with an engine pack beneath a
  game pack; `font::undrawn` on an engine pack's character.
- Unit (`shuyi-ui`): `engine_packs()` loads each pack; the upkeep test.
- Unit (`shuyi-cli`): a `lang-status` report on a pack with an engine
  pack beneath it.
- Integration: `crates/ui/tests/it/engine_packs.rs`, the toy's Harness
  test in the first engine language.
- Snapshot: the toy's options, Key bindings and profiles screens in each
  language, with the test that finds their titles and row labels.

## Completion notes

*(Filled in by the session that completes the ticket.)*
