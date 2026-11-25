---
id: "2278"
title: "A language's own copies of files: a pack's files/ folder, localized_path and Ctx::localized"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2349"]
nick_input: none
completed:
---

# 2278 — A language's own copies of files

## Context

Split from 2349 (the player's language) by its **Size** clause: that PR
passed 600 changed lines, so its step 6, *A language's own copies of
files*, is this ticket.

- **The game ticket.** game#0831
  ([`0831-game-over-words-per-language.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0831-game-over-words-per-language.md)):
  a language brings its own picture of words. It waits for this ticket
  (the game's PR adds `engine#2278` to its `blocked_by`).
- **The job:** a picture with words in it can differ by language, and
  a change of language shows the right picture at once.
- **Friction:** no friction review: a missing feature.
- **The benchmarks:** Unreal replaces an asset with the copy at the
  same path under `L10N/<culture>/`
  (https://dev.epicgames.com/documentation/en-us/unreal-engine/asset-localization-in-unreal-engine);
  Godot's Remaps swap a resource per locale.
- **Shuyi today:**
  - A pack's folder may hold only `lang.ron`, `ui.ron`, `data.ron` and
    `dialogue/*.ron` (`from_files_in` in `crates/assets/src/lang.rs`,
    `LangPackRon::decode` in `crates/assets/src/lang/formats.rs`). A
    language has no copy of any other file. Both readers already skip
    an import settings file (`is_settings_file`, 2229).
  - **From 2349 (done):** the language in use is `Ctx::lang`, the
    player's pick (`Settings::language`) or the game's pick at start;
    `Ctx::effective_lang` hides the debug language without debug tools,
    and `Lang::chain` walks a tag's packs to the source.
  - `ImageTable` already lists every PNG of the bundle, copies included;
    the pack (`crates/assets/src/pack.rs`) holds every file of the
    bundle it was baked from, so copies are baked with no change.
- **Outside tactics:** a farming game's shop sign drawn as a picture of
  words; a visual novel's title logo per language.

## Nick input

None.

## Scope

**In:**

- **A pack's `files/`.** A pack's folder may hold `files/<path>`: that
  language's copy of the game's file at `<path>`, as in Unreal's
  `L10N/<culture>/<path>`. Both readers (`from_files_in`,
  `LangPackRon::decode`) accept `files/` and never read it as text. Any
  other unknown file is still an error. Add the exception beside the
  settings-file skip (`is_settings_file`) in the same two loops, and
  test a pack holding `files/x.png` and `files/x.png.meta` in both.
- **Either order with 2238:** if 2238 is done, accept `files/` once,
  in `is_pack_file` (`crates/assets/src/lang.rs`), which both readers
  call; if not, accept it in both readers, and 2238 moves it into
  `is_pack_file`.
- `shuyi_assets::lang::localized_path(languages: &str, chain:
  &[LangCode], path: &str, has: impl Fn(&str) -> bool) -> String`, pure:
  the first `<languages>/<tag>/files/<path>` along the chain that `has`
  (the source's folder has no copies), else `path`.
- `Ctx::localized(&self, languages: &str, path: &str, has: impl
  Fn(&str) -> bool) -> String`: the same, along `Lang::chain` of
  `effective_lang`. A screen asks each time it draws, so a change of
  language shows at once:
  `images.id(&ctx.localized("lang", "title/end.png", |p| images.id(p).is_some()))`.
- **With per-target layers (3007, done):** the language's copy is
  looked for first, then the plain path, each through the bundle's
  layers in order. A target that needs its own copy of a language's
  file puts it at that same path in its layer
  (`assets@playdate/lang/ja/files/title/end.png`). The words in a
  picture matter more than the device's version of another language's
  picture. `localized_path` asks `has`, which a bundle answers through
  its layers, so nothing more is needed; a test shows it.
- A one-language build ships a layer without the other packs' folders
  and starts in its language with `FirstLanguage::Fixed` (2349). Both
  readers need no pack and `localized_path` asks `has`, so nothing is
  named with `Layer::leaving_out` (which names files a layer lacks; it
  removes none).
- **Docs:** `docs/formats.md` (a pack's `files/`), the module docs of
  `lang.rs`, and the book's chapter on languages if the docs map names
  one.

**Out (do not do):**

- Making a one-language build's layer: 3059.
- Voices per language: parked 2725, from `VoiceFiles`' folder per
  language, not from a pack's `files/`.
- Reloading a picture the game read at start: the game asks
  `Ctx::localized` each time it draws.

**For the game:** additive: two new functions, and a pack folder that
accepts more. game#0831 builds on `Ctx::localized`.

**Estimated size:** about 350 lines (code 120, tests 160, docs 70).

## Implementation steps

1. Read `from_files_in` and `is_settings_file` in
   `crates/assets/src/lang.rs`, `LangPackRon::decode` in
   `lang/formats.rs`, `Lang::chain`, and `Ctx::effective_lang`
   (`crates/ui/src/ctx.rs`). Check whether 2238 is done, for its
   clause.
2. Accept `files/` in both readers, with the tests.
3. Add `localized_path` beside `Lang::chain`, with its tests.
4. Add `Ctx::localized`, with a test that switches the language.
5. Write the docs; run the gates.

## Acceptance criteria

- [ ] `localized_path` finds `lang/pt/files/x.png` for `pt-BR`, prefers
      `lang/pt-BR/files/x.png` when that exists, and gives `x.png`
      otherwise. With `lang/pt/` left out of the bundle, it gives
      `x.png`. A copy read back from a pack equals the loose one (unit
      tests).
- [ ] A pack with `files/x.png` and `files/x.png.meta` passes both
      readers; a pack with another unknown file still fails both (unit
      tests).
- [ ] `Ctx::localized` gives another path as soon as the language
      changes, and the plain path for the debug language without debug
      tools (unit test).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `localized_path` along a chain and with a pack's folder left
  out; both readers with `files/`; `Ctx::localized` across a change of
  language.
- Integration: a copy read back from a baked pack equals the loose one.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
