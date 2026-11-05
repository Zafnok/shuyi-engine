---
id: "2253"
title: "best_match: the game's language that fits the system's languages"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2226"]
nick_input: none
completed:
---

# 2253 — best_match: the game's language that fits the system's languages

## Context

Split from 2226 (language tags in BCP 47, a fallback chain per key and a
source language), whose diff ran past its size limit; its own words sent
this part here. 2226 gave `LangCode` as a BCP 47 tag
(`crates/assets/src/lang/tag.rs`): `LangCode::parse`, `language()`,
`script()`, `region()`, `parent()`, and the private
`tag::ancestry(tag: &str)` (the tag, then each parent, as `&str`).

2349 (the player's language) picks the language a game starts in from
the system's languages: `FirstLanguage::System` is `best_match` of the
system's languages among the source and the packs, else the source. Its
only user is 2349, which this ticket blocks.

**Outside tactics:** a puzzle game on the web that opens in the
visitor's browser language; a farming game on a Steam Deck set to
Brazilian Portuguese that opens in its `pt` pack.

## Nick input

None.

## Scope

**In:**

- A pure function in `shuyi_assets::lang`, in a new module
  `crates/assets/src/lang/matching.rs` (`no_std` with `alloc`, like the
  crate), re-exported from `lang.rs`:
  `best_match(preferred: &[LangCode], offered: &[LangCode]) ->
  Option<LangCode>`. `preferred` is the system's languages, most
  preferred first; `offered` is the source's tag and the packs'. For
  each preferred tag in order:
  1. the tag, then each of its `parent()`s, if offered (`en-GB` matches
     an `en` source);
  2. Chinese without a script takes one from its region (`CN` and `SG`
     give `Hans`; `TW`, `HK` and `MO` give `Hant`), then step 1 again
     with the script written after the language (`zh-CN` matches
     `zh-Hans`, through `zh-Hans-CN`);
  3. the only offered tag with the same language subtag, unless both
     have a script (written, or from step 2) and they differ (`pt-PT`
     matches a lone `pt-BR`; `zh-TW` doesn't match `zh-Hans`).

  The first preferred tag that matches wins. `None` if none does (the
  caller then uses the source).
- Its rustdoc lists the three steps; the module docs of `lang.rs` name
  it under **The fallback chain**.

**Out (do not do):**

- Reading the system's languages, the setting, the options row: 2349.
- CLDR data: likely subtags beyond step 2's five regions, aliases (`iw`
  for `he`), parent locales other than the shorter tag.

**For the game:** additive; nothing changes for a game that doesn't
call it. No `Breaking:` line.

## Implementation steps

1. Read `crates/assets/src/lang/tag.rs` (`ancestry`, `parent_of`, the
   accessors).
2. Write `matching.rs`: a private `chinese_script(&LangCode) ->
   Option<&'static str>`, a private `offered_along(tag: &str, offered)`
   over `tag::ancestry`, a private `match_one` for steps 1 to 3, and
   `best_match` over `preferred` with `find_map`.
3. `mod matching;` and `pub use matching::best_match;` in `lang.rs`; a
   line in its module docs.
4. Run the gates.

## Acceptance criteria

- [ ] `best_match` gives `pt` for `[pt-BR]` over `[en, pt]`, `zh-Hans`
      for `[zh-CN]` over `[en, zh-Hans]`, `None` for `[zh-TW]` over
      `[en, zh-Hans]`, and `fr` for `[de, fr-CA]` over `[en, fr]` (unit
      tests).
- [ ] Step 2 gives its script for each of its five regions, and step 3
      gives `None` with two offered tags of one language (unit tests).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: as in the criteria; `en-GB` matches `en`; `pt-PT` matches a lone
  `pt-BR`; `sr-Latn` doesn't match `sr-Cyrl` but matches `sr-RS`; an
  empty `preferred` is `None`; a later preferred tag wins when the first
  matches nothing.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
