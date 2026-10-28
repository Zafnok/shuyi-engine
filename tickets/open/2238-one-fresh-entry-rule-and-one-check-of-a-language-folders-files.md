---
id: "2238"
title: "One fresh-entry rule for the language packs, and one check of a language folder's files for both readers"
type: infra
milestone: After MVP
model: sonnet-5
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2238 — One fresh-entry rule, and one check of a language folder's files

## Context

Found by an analysis of the engine for areas that do the same job twice
(2026-10-28). Two small copies in the language packs
(`crates/assets/src/lang.rs` and its submodules):

- **The fresh-entry rule is written six times.** A pack's entry is
  shown only when its `source` is today's English; otherwise English
  shows. Each lookup tests that itself:
  - `Lang::text` (`lang.rs:247`);
  - `Lang::status`'s stale check (`lang.rs:262`, the same test turned
    round);
  - the `name` closure in `pack_from_files` (`lang.rs:509-513`), which
    fills the names a pack's dialogue lines are checked with;
  - `Lang::data_text` (`lang/source.rs:160`), which `Lang::name` and
    `Lang::names` (`lang/source.rs:174-184`) go through;
  - `Lang::found` (`lang/source.rs:199`);
  - `Lang::line` (`lang/lines.rs:350`), which `Lang::scene` goes through.

  The closure in `pack_from_files` is `Lang::names` again, for one pack.
  2226 makes every lookup walk a fallback chain (`pt-BR`, then `pt`,
  then the source), and its list (`Lang::text`, `data_text`, `name`,
  `names`, `found`, `line` and `scene`) doesn't name this closure. So
  after 2226 the names a pack's lines are checked with could differ from
  the names play fills them with, and nothing would show it.
- **Both readers of a languages folder check its files.**
  `lang::from_files` (`lang.rs:572-587` and `600-607`) and
  `LangPackRon::decode` (`lang/formats.rs:111-118` and `139-149`) each
  decide which files a pack's folder may hold (`lang.ron`, `ui.ron`,
  `data.ron`, `dialogue/<file>.ron`) and whether its name is a language
  code, and each writes the same two messages: `formats.rs:18` has
  `NOT_A_LANGUAGE_FILE`, while `lang.rs:584` writes "not a file a
  language has" again, and "a language's directory must be named with 2
  to 8 lowercase letters" is written at `lang.rs:604` and
  `formats.rs:116`. Both already parse through the shared
  `pack_from_files`, so only this check is doubled. 2226, 2229 and 2349
  each change it in both places.

## Nick input

None.

## Scope

**In:**
- **One fresh-entry rule.** In `lang.rs`, a private trait `Sourced`
  (`fn source(&self) -> &str`), implemented by `Entry` and `LineEntry`,
  and a private
  `fn fresh<'e, E: Sourced>(entry: Option<&'e E>, english: &str) -> Option<&'e E>`:
  the entry when it was made from `english`, else `None`. Its doc
  comment states the rule once. Every test above calls it:
  `Lang::text`, `Lang::data_text`, `Lang::found`, `Lang::line`, and
  `Lang::status`, whose closure becomes a private generic function
  (`None` is missing, an entry `fresh` drops is stale).
- **One way to fill a pack's names.** A private
  `fn pack_names(data: &BTreeMap<String, Entry>, names: &Names) -> Names`
  in `lang/source.rs`: each name the pack's fresh `names.<id>` entry,
  else English. `Lang::names` calls it with the pack's `data` (and
  clones `names` when `code` has no pack, as today), and
  `pack_from_files` calls it in place of its `name` closure.
- **One check of a pack's folder.** In `lang.rs`:
  - the two messages as private constants, `NOT_A_LANGUAGE_FILE`
    (moved from `formats.rs`) and `BAD_FOLDER_NAME`;
  - `fn is_pack_file(name: &str) -> bool`: `name` (a path inside a
    pack's folder) is `lang.ron`, `ui.ron`, `data.ron` or a dialogue
    file (`is_dialogue_file`);
  - `fn pack_code(dir: &str) -> Result<LangCode, &'static str>`: the
    folder's code, or `BAD_FOLDER_NAME`.

  `from_files` and `LangPackRon::decode` call both, and write neither
  message themselves. Their results and messages don't change.
- **Either order with 2226:** if 2226 is done, its folder-name check
  and its two messages (a non-canonical tag, a `TagError`) go into
  `pack_code`, which returns the message as a `String`; its chain walk
  tests each pack along the chain with `fresh`; and `pack_names` takes
  the chain's `data` maps in order (the first fresh entry wins), with
  `Lang::names` passing the chain's packs and `pack_from_files` its own
  pack alone, since it reads one pack; if not, merge today's code, and
  2226 changes the check and the walk in the one place each (its
  clause).
- **Either order with 2229:** if 2229 is done, both readers skip a
  settings file: move that skip into the shared check, so a settings
  file is neither a pack file nor an error, and 2229's tests pass
  unchanged; if not, nothing to do (2229 adds the skip once, to the
  shared check).
- **Either order with 2349:** if 2349 is done, both readers accept a
  pack's `files/` folder: move that into `is_pack_file` (a path under
  `files/` is a pack file, never read as text), and 2349's tests pass
  unchanged; if not, nothing to do (2349 adds it once, to
  `is_pack_file`).
- **Either order with 2348:** if 2348 is done, `Lang::text_in` and
  `Lang::data_text_in` hold the lookups that `text` and `data_text` are
  built on: they test an entry with `fresh`; if not, nothing to do
  (2348 builds them on `fresh`).
- **Either order with 2352:** if 2352 is done, `Lang::text`'s test of
  an engine pack's entry goes through `fresh` too; if not, nothing to
  do (2352 tests the engine's entries with `fresh`).
- **For the game:** no API change. Everything added is private;
  `Lang::names`, `pack_from_files`, `from_files` and `LangPackRon` keep
  their signatures and results.

**Out (do not do):**
- Checking a pack's lines with its parents' names along 2226's chain
  (a `pt-BR` pack's lines with `pt`'s names): `LangPackRon` reads one
  pack without its parents, so it would need them handed in. Not this
  ticket; `pack_from_files` keeps using its own pack's names.
- The English folder's own rule (only `ui.ron`, no `lang.ron`), and the
  `en/ui.ron` path in both readers: 2226 makes them the source
  language's.
- Merging the two readers into one: they read files differently (a
  list handed in, against `DecodeCx`), and both already parse through
  `pack_from_files`.
- Changing what any lookup returns, or any message's words.

## Implementation steps

1. Read `crates/assets/src/lang.rs` (`Entry`, `Lang::text`,
   `Lang::status`, `pack_from_files`, `from_files`), `lang/source.rs`
   (`data_text`, `name`, `names`, `found`), `lang/lines.rs`
   (`LineEntry`, `Lang::line`) and `lang/formats.rs`
   (`LangPackRon::decode`). Settle the clauses above with what is done.
2. Add `Sourced`, its two impls and `fresh` to `lang.rs`, with unit
   tests. Move the six tests onto it (`lang.rs`, `lang/source.rs`,
   `lang/lines.rs`).
3. Add `pack_names` to `lang/source.rs`; build `Lang::names` on it and
   replace the closure in `pack_from_files`.
4. Add the two constants, `is_pack_file` and `pack_code` to `lang.rs`;
   call them from `from_files` and `LangPackRon::decode`, and delete
   `formats.rs`'s `NOT_A_LANGUAGE_FILE` and its literal message.
5. Run the gates.

## Acceptance criteria

- [ ] `grep -rnE "source(\(\))? *(==|!=)" crates/assets/src/lang.rs crates/assets/src/lang/`
      finds one line, in `fresh`.
- [ ] `pack_from_files` has no closure that looks up `names_key`; it
      and `Lang::names` both call `pack_names`.
- [ ] Outside `#[cfg(test)]` code, "not a file a language has" and
      "must be named with" are each written once, in `lang.rs`'s
      constants (`grep -rn` over `crates/assets/src/lang.rs` and
      `crates/assets/src/lang/formats.rs`, ignoring test modules).
- [ ] `LangPackRon::decode`'s loop and `from_files`' loop call
      `is_pack_file`, and both folder-name checks call `pack_code`.
- [ ] Every existing test of `lang.rs`, `lang/pack_tests.rs` and the
      language cases of `crates/assets/src/format/builtin_tests.rs`
      passes unchanged.
- [ ] `shuyi-assets` builds for `thumbv7em-none-eabihf` (the no-std
      gate).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`lang.rs`): `fresh` keeps an `Entry` made from today's English,
  drops one made from older English, and gives `None` for `None`; the
  same for a `LineEntry`. `is_pack_file` takes `lang.ron`, `ui.ron`,
  `data.ron` and `dialogue/a.ron`, and refuses `notes.txt`,
  `dialogue/a/b.ron` and `dialogue/.ron`. `pack_code` takes `ja` and
  refuses `JA` and `x1` with `BAD_FOLDER_NAME`.
- Unit (`lang/pack_tests.rs`): `pack_names` over a pack with a fresh
  name, a stale name and no entry gives the translation, English and
  English. `a_name_token_counts_as_the_packs_longest_name` already
  covers the closure's replacement and passes unchanged.
- The existing tests pass unchanged.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
