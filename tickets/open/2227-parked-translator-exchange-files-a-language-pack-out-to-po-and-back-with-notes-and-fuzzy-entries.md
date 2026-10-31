---
id: "2227"
title: "Parked: Translator exchange files: a language pack out to PO and back, with notes and fuzzy entries"
type: feature
milestone: "Engine: localization"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2302"]
nick_input: decision
completed:
---

# 2227 — Translator exchange files: PO out and back

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; on 2026-10-20 he kept parked the gaps the game doesn't need): the game doesn't ask for it, so this waits until after the MVP (`docs/ROADMAP.md`), after 2302, whose `max_cells` it writes, and after Nick answers the question below. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. People who translate games work in a translation tool,
not in a game's own files:

- Godot reads translations as gettext PO files (and CSV), with a
  context per entry and plurals, and its editor writes the POT template
  (<https://docs.godotengine.org/en/stable/tutorials/i18n/localization_using_gettext.html>).
- Unity's Localization package exchanges its tables as CSV and XLIFF,
  and with Google Sheets.
- Unreal's Localization Dashboard exports PO files and imports them back.
- Bevy has nothing built in.
- Weblate, Crowdin, Poedit and Lokalise all read and write PO.

Shuyi today:

- A language pack is a folder of RON files: `lang.ron` (its name and
  `made_by`), `ui.ron`, `data.ron` and `dialogue/*.ron`. Each entry is a
  `key`, the English `source` it was made from and its `text`; a
  dialogue entry may have a `text_<variant>` per variant the game names
  instead (`crates/assets/src/lang.rs`, `lang/lines.rs`). A pack is read
  as the `lang-pack` format (`lang/formats.rs`).
- `LangPack`'s `ui`, `data` and `lines` maps are private, so only code
  inside `shuyi_assets::lang` can list a pack's entries or build a pack
  other than through `pack_from_files`.
- `lang-status` lists a pack's missing, stale and orphaned entries for
  whoever translates (`crates/cli/src/lang_status.rs`). Its order is
  `LangStatus`'s: screen text, then data, then dialogue, each in key
  order.
- Nothing reads or writes PO, XLIFF or CSV, and a key has nowhere to
  carry a note for its translator.
- 2218 writes the engine's simpler text formats and leaves language
  packs out: their files hold comments and layout a writer would lose,
  "a ticket of their own if wanted". This is that ticket.

Why it is parked: Visions of Shuyi's languages are translated by the agent,
"until a human translation replaces it" (game#0718), and the agent reads
the RON packs as they are. Game#0045 asks Nick who, if anyone, checks
the Chinese. Exchange files matter once a person translates or checks a
language in a translation tool, for this game or another one.

## Nick input

**Decision (asked when this is unparked, with the `ask-nick` skill):**
will a person translate or check the game's or the engine's text in a
translation tool (an agency, or volunteers on Weblate or Crowdin)? If
yes, unpark this ticket. Which tool they use decides whether XLIFF or
CSV is needed too (see *Out*).

## Scope

**In:**

- **`shuyi_assets::lang::po`** (`crates/assets/src/lang/po.rs`), written
  in-house, `no_std` with `alloc` like the rest of the crate. It sits in
  `lang` because it needs `LangPack`'s private maps. No new dependency:
  a reader and writer of the subset below is a few hundred lines.
  - `pub fn write_po(lang: &Lang, code: &LangCode) -> Result<String, String>`.
    An error for a code with no pack, and for an entry whose text is
    empty, naming its key (an empty `msgstr` means untranslated in PO,
    so it couldn't come back). One entry per key, in `lang-status`'s
    order, obsolete entries last:
    - `#.` notes: the key's `note` and its `max_cells` ("at most N
      cells") from 2302; for a dialogue line, its scene and kind
      ("scene hall, a reply") from `SourceLine`;
    - `#:` where it comes from: `en/ui.ron`, `data`, or the line's
      `<file>.dlg` (`SourceLine.file`; none when it is empty);
    - `msgctxt "<part>:<key>"`, the part `ui`, `data` or `dialogue`. A
      line the pack translates per variant (`LineText::Variants`) has
      one entry per variant, `msgctxt "dialogue:<key>|<variant>"`. A
      missing line goes out as one entry with no variant;
    - `msgid`: today's English; `msgstr`: the pack's text, or `""` for a
      missing entry;
    - a **stale** entry gets `#, fuzzy` and `#| msgid "<the English it
      was made from>"`;
    - an **orphan** (a dialogue entry whose line English no longer has)
      is written as an obsolete entry (`#~ msgctxt`, `#~ msgid`,
      `#~ msgstr`).
    - A header entry: `Language: <code>`, `MIME-Version: 1.0`,
      `Content-Type: text/plain; charset=UTF-8`,
      `Content-Transfer-Encoding: 8bit`, `X-Generator: shuyi
      lang-export`. No date, so two exports of the same pack are the
      same bytes. No `Plural-Forms`: packs have no plural forms (a 2348
      message goes out as it is written, its branches inside the
      string).
    - Strings escape `\\`, `\"`, `\n`, `\r` and `\t`. A string with a
      line break is split after each `\n` (`msgid ""` and one line per
      part); no other wrapping, so a diff of two exports shows only what
      changed.
  - `pub fn read_po(file: &str, source: &str, lang: &Lang, info: LangInfo, variants: &[&str]) -> Result<PoRead, Vec<ContentError>>`,
    where `pub struct PoRead { pub pack: LangPack, pub language:
    Option<String>, pub unconfirmed: usize, pub obsolete: usize }`
    (`language` is the header's `Language:`; the two counts are what it
    left out, below):
    - reads what `write_po` writes, and the same entries saved by other
      tools: comments in any order, strings split anywhere (in `#|`
      lines too), any other flags beside `fuzzy` (`#, fuzzy,
      c-format`), CRLF line ends;
    - an entry with an empty `msgstr` is no entry (missing);
    - not fuzzy: `source` is the `msgid`. Fuzzy with a `#|` previous
      `msgid`: `source` is that, so the entry stays stale. Fuzzy with no
      previous `msgid`: no entry (an unconfirmed suggestion), counted in
      `unconfirmed`;
    - a line's variant entries make one `LineEntry`. If they don't all
      give the same `source`, the line takes the one from `#|`: it stays
      stale until every variant is confirmed. A line with an unconfirmed
      variant is left out whole and counted;
    - an obsolete `dialogue` entry comes back as an entry (the orphan it
      was); an obsolete `ui` or `data` entry is dropped, counted in
      `obsolete`;
    - errors, each with the file and the line (`ContentError::at`):
      - an entry other than the header with no `msgctxt`, or one that
        isn't `<part>:<key>` with a known part;
      - a key used twice, or a line with both one text and variants;
      - a `ui` or `data` key English doesn't have, or a `dialogue` key
        that isn't a line id or a caption key;
      - a `ui` or `data` `msgstr` whose placeholders differ from its
        source's, by the same check `pack_from_files` makes (moved out
        of `entries` in `lang.rs` into a function both call);
      - a line with some of `variants` but not all;
      - `msgid_plural` or `msgstr[N]` (plurals aren't read);
      - an unknown escape, a string never closed;
    - never panics, and refuses a file over `TEXT_MAX_BYTES`
      (`crates/assets/src/format.rs`; ADR-0007 §5).
- **`shuyi_assets::lang::pack_files(pack: &LangPack, lang: &Lang, variants: &[&str]) -> Vec<(String, String)>`**
  (`crates/assets/src/lang/write.rs`): the pack's files as `lang-import`
  writes them, each a path inside the pack's folder and its RON text.
  - `ui.ron` always; `data.ron` and `dialogue/<stem>.ron` only when they
    have entries. No `lang.ron`.
  - Each a list in key order, one entry per line:
    `(key: "…", source: "…", text: "…")`, or `text_<variant>` fields in
    the order of `variants`. Every string is written with
    `ron::to_string`, so the escapes are RON's own. No comments.
  - A line goes to the file of its `.dlg` (`SourceLine.file`). An orphan
    goes to the file of another line of its scene. A line with no file,
    and an orphan whose scene English no longer has, go to
    `dialogue/other.ron`.
- **The commands**, beside `lang-status` in `crates/cli/src/lib.rs`
  (`crates/cli/src/lang_exchange.rs`), through the game's tools
  (`GameTools::lang`, `GameLang.variants`) as `lang-status` is: with no
  game, `game::run_in_game_program`.
  - `shuyi lang-export <code> [--out <file.po>]` writes the PO, to
    standard output without `--out`. Exit codes as `lang-status`'s: 1
    when the game's languages don't load, 2 for wrong arguments or a
    code with no pack.
  - `shuyi lang-import <file.po> <pack-dir> [--force]` reads the PO with
    the folder's own `lang.ron` (which must exist: the name and
    `made_by` are not in the PO). It refuses a PO whose `Language:` is
    not the folder's name. Then it writes `pack_files` into the folder.
    The PO is the whole pack: it also removes the folder's `data.ron`
    and `dialogue/*.ron` files that `pack_files` didn't give, so a key
    the PO leaves out has no entry after the import. It touches no other
    file (`lang.ron` included). Exit 0 when written, 1 for errors in the
    PO (each printed with its file and line), 2 for wrong arguments.
  - `lang-import` refuses to overwrite or remove a pack file that holds
    a comment (`//` or `/*` outside a string), naming it, unless
    `--force` (2218's reason: a writer loses comments).
  - It prints the files it wrote, with their entry counts, the files it
    removed, and the counts of unconfirmed and obsolete entries it left
    out. Then it says to run `shuyi check`, which checks the pack with
    the game's own rules (its tokens, its data text, its dialogue
    limits).
- **Notes.** 2302 adds an optional `note` beside `max_cells` in
  English's `ui.ron`. If 2302 was done without it, add it here the same
  way: optional, in the same value form as `max_cells`, kept by `Lang`
  (`Lang::note(key)`), never shown in the game, and a pack can't set
  it.
- **If 2226 is done** (language tags and a source language the game
  declares): `msgid` is the source language's text
  (`Lang::source_text`), `#:` names `<source tag>/ui.ron`, `Language:`
  is the pack's tag with `_` for `-` as gettext writes it (`pt_BR`), and
  both commands read a tag with `LangCode::parse`. If not, nothing
  here: 2226 does it (its clause says so).
- **If 2351 is done** (the engine's own English): a screen-text key
  whose English is the engine's (`Lang::is_engine`) goes out as a `ui`
  entry with `#: engine`. If not, nothing here (2351 does it, as its
  clause says).
- **Docs:**
  - a section "Translating with PO" in `docs/formats.md`: what goes
    out, what fuzzy and obsolete mean, why the RON pack stays the
    source, and how to start a new language (its folder with a
    `lang.ron` and an empty `ui.ron`, then `lang-export`);
  - `lang-export` and `lang-import` in the command list of
    `shuyi --help` (`usage` in `crates/cli/src/lib.rs`), in that file's
    module docs, in `crates/cli/Cargo.toml`'s description and in
    `README.md`'s `shuyi-cli` row.

**For the game:** new API and two new commands only. Nothing the game
calls changes, so the next pin bump needs no change in the game.

**Out (do not do):**

- **XLIFF and CSV.** PO is what the tools above all read. Write a ticket
  for XLIFF 2.0 or CSV when a translator's tool needs one (the Nick
  input decides when).
- A `.pot` template: a new language starts from an empty pack's export
  (the docs section says how).
- Loading a pack from PO at run time (a `Format` of `LangPack` in the
  registry): the RON pack stays what a game loads and what tools treat as
  the source (`AGENTS.md` rule 10). A follow-up if a game wants to ship
  PO.
- An `Encode` of `LangPack` for `import --print` (2218's writers): a
  pack is several files, and `lang-import` writes them itself.
- Plurals and grammatical gender in PO (`msgid_plural`): packs have
  none; 2348's messages travel inside the string.
- Notes on data text and dialogue lines (their English is the game's
  data and `.dlg` files, which have no note field).
- A generated pseudo-language (2353).
- Hosting, syncing with Weblate or Crowdin, Google Sheets, machine
  translation (the engine's own screens: 2352).
- PO for the engine's own packs (2352's `crates/ui/lang/<code>/`): a
  ticket of its own if a person checks them in a translation tool. A
  game's export writes a key its pack lacks with `msgstr ""`, even
  where an engine pack covers it: a follow-up if translators should
  see the engine's words.
- Changing what `lang-status` reports.

## Implementation steps

1. Read `crates/assets/src/lang.rs` (`Lang`, `LangPack`, `entries`,
   `pack_from_files`), `lang/lines.rs` (`RawLine`, `LineText`,
   `LineEntry`, `add_lines`, `orphans`), `lang/source.rs`
   (`SourceLine`, `LineKind`), `crates/cli/src/lang_status.rs` and the
   `lang-status` arm and `lang_status` function in
   `crates/cli/src/lib.rs`, and how 2302 reads `max_cells` (and `note`)
   in `english_from_source`.
2. If 2302 left `note` out, add it (*Notes* above) with a unit test.
3. Move the placeholder check out of `entries` into a function that
   `entries` and `read_po` share.
4. Write `write_po` in `lang/po.rs` with its unit tests, then the
   snapshot of a fixture pack.
5. Write `read_po` with a test per error and the other-tool fixture,
   then the round trip and the random-input property tests.
6. Write `pack_files` in `lang/write.rs` and its test that
   `pack_from_files` reads its files back to the same pack.
7. Write `crates/cli/src/lang_exchange.rs` (argument parsing, the
   comment scan, writing and removing files) and wire both commands
   into `main_with` and `usage`; then the integration test on the toy's
   tools (`crates/cli/src/toy.rs`) in a folder under
   `std::env::temp_dir()`, as `check_keys.rs`'s tests do.
8. Write the docs.
9. Run the gates.

If the diff passes about 600 changed lines (tests included, snapshots
aside) by the end of step 6, open the PR with steps 1 to 6 and the
`docs/formats.md` section. Write step 7, the commands' docs and their
acceptance criteria as a follow-up ticket with this ticket's milestone
(`write-ticket` skill).

## Acceptance criteria

- [ ] `read_po` of `write_po(lang, code)` gives back the pack
      (`PoRead::pack`), stale entries and orphans included, on the toy's
      `dbg` pack and on fixture packs with data, dialogue and variants
      (unit tests), and on random packs (property test).
- [ ] A stale entry goes out with `#, fuzzy` and its old English in
      `#|`. With the flag removed it comes back current; with the flag
      kept it comes back stale (unit tests).
- [ ] A key's `note` and `max_cells` appear as `#.` lines (unit test).
- [ ] A hand-written PO in the shape another tool saves (strings split
      mid-word, `#, fuzzy, c-format`, comments in another order, CRLF)
      reads to the same pack as `write_po`'s output (unit test).
- [ ] Each error of `read_po` names the file and the line (a unit test
      per error).
- [ ] The export of a fixture pack is committed as an `insta` snapshot
      and is the same bytes on every run.
- [ ] Random bytes and random edits of a valid PO never panic the
      reader, and a file over `TEXT_MAX_BYTES` is refused (property
      test, as ADR-0007 §5 asks of a decoder).
- [ ] `pack_files` gives files that `pack_from_files` reads back equal
      to the pack, variants and orphans included (unit test).
- [ ] `lang-import` writes the pack's files, removes the dialogue file
      the PO no longer fills, and refuses a pack file with a comment
      without `--force`; `lang-export` of a code with no pack exits 2
      (integration test on a temporary folder with the toy's tools).
- [ ] `shuyi-assets` still builds for `thumbv7em-none-eabihf` (3005's
      gate in `run-gates`).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: escapes; strings split over lines; the header; notes; variants;
  a missing line with variants in the game; obsolete entries; CRLF;
  each error with its line; `pack_files`' layout and file placement.
- Property: the round trip on random packs; the reader on random bytes
  and on random edits of a valid PO.
- Snapshot / integration: the fixture export; `lang-export` and
  `lang-import` on the toy's tools.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
