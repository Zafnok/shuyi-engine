---
id: "2250"
title: "A voice cast a voice tool can use: each speaker's voice, the tool's own settings, and a voice per clip variant"
type: feature
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-06
---

# 2250 — A voice cast a voice tool can use

## Context

1. **The game ticket:** `game#0760`
   ([github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0760-one-form-for-cast-ron.md](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0760-one-form-for-cast-ron.md)),
   and the game ticket that moves the game onto this one, `game#1242`.
2. **The job in one sentence:** say which voice reads each speaker's
   lines, with whatever the voice-making tool needs to make that voice
   (a sample to copy, which program reads, or which actor records), and
   a different voice for a clip variant (a hero's lines said of a man or
   of a woman).
3. **Friction numbers:** no friction review row of its own. The game's
   voice tool (`cargo xtask voice`, game ticket 0721) is about 1,000
   production lines, and its cast reader (`crates/xtask/src/voice/files.rs`,
   about 60 lines) is the generic part this ticket takes. The engine's
   `voice::Cast` (speaker to a voice id) was too thin for it, so the game
   wrote its own and, since `game#0760`, calls the engine's no more.
4. **The generic shape:** `shuyi_assets::voice` already has the manifest
   (`VoiceManifest<V>`, `from_source`), the clip variants a game names
   (`ClipVariant`) and staleness. Its `Cast` maps a speaker to a voice id
   and nothing else. A voice tool needs, per voice: the name the
   manifest's `voice` records (a reference clip's name for a generated
   voice, an actor's for a recorded one) and settings only that tool
   knows. And a speaker whose lines have variants needs one voice per
   variant. Make `Cast` generic over the tool's settings, the way
   `VoiceManifest` is generic over the game's variants.
5. **Examples from games that aren't tactics:** a visual novel whose
   voices are generated from samples names, for each character, the
   sample and the speech program; a narrative game that lets the player
   pick the hero's voice (deep or light) has two voices for the hero's
   lines, one per clip variant. Both write the same cast. The engine's
   own visual novel voices (2725, parked) would read it too.

## Nick input

None.

## Scope

**In:**
- `Cast<S>`: voices by key, each a `CastVoice<S>` with `voice: String`
  (what the manifest's `voice` records) and the tool's `settings: S`
  (`S: DeserializeOwned`; `()` for a tool with none). Pick the RON shape
  (`(voice: "narrator", settings: (reader: Qwen3))` or a flattened one;
  `serde(flatten)` is unreliable in RON, so test the one you pick) and
  show it in the module doc.
- Keys: a speaker (a character id, or `NARRATION_SPEAKER`), or a speaker
  and a variant, written as the speaker followed by the variant's
  `ClipVariant::suffix()` (`"hero.f"`), the same suffix a clip's file
  name has.
- `Cast::voice_for(speaker, variant) -> Option<&CastVoice<S>>`: the
  speaker-and-variant entry if there is one, else the speaker's.
- `cast_from_source::<S, V>(file, source, can_speak)`: today's checks (no
  empty `voice`, every speaker narration or one that `can_speak`, errors
  at the entry's line), plus: a key with a suffix names a speaker that
  can speak and a suffix one of `V`'s variants has. If `ClipVariant` has
  no way to list its variants, add one (an associated `ALL` slice or
  function).
- **API: breaking.** `Cast`'s field and `cast_from_source`'s signature
  change. Each gets a `Breaking:` line in the completion notes. The game
  doesn't call either today (`game#0760`), so the game's PR that moves the
  pin past this one needs no code; `game#1242` moves its tool onto it.
- `docs/formats.md`'s line on `voice::cast_from_source`, and the book's
  voice chapter if it shows a cast.

**Out (do not do):**
- Planning which clips to make, remake or record: 2251.
- Making clips (calling a speech program, levelling, checking a take):
  each tool's own.
- Any change to the manifest, its staleness rule or playback.

## Implementation steps

1. In `crates/assets/src/voice.rs`, replace `Cast` and `RawCast` with
   `Cast<S>` and `CastVoice<S>` (both `#[non_exhaustive]` with a
   constructor, ADR-0039), and `Cast::voice_for`.
2. Rewrite `cast_from_source` as above, generic over `S` and `V`.
3. Add the variant list to `ClipVariant` if needed, with the existing
   impls (`()`, the test variants) updated.
4. Update the module doc (with the RON example), `docs/formats.md` and
   the book.

## Acceptance criteria

- [x] A cast with tool settings and a speaker-and-variant key parses, and
      `voice_for` returns the variant's voice for that variant and the
      speaker's for another.
- [x] Every error has its own test: an empty `voice`, a speaker who can't
      speak, an unknown suffix, a suffix on a speaker who can't speak, and
      settings the tool's `S` rejects.
- [x] The completion notes have a `Breaking:` line for each changed item.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the cases above, with a test `S` (an enum of two readers) and the
  test `ClipVariant`.
- Property: `voice_for` never returns a voice for a speaker that has no
  entry of its own or for its variant.

## Completion notes

- `shuyi_assets::voice::Cast<S>` holds `CastVoice<S>` entries (`voice`,
  `settings`) by key; both are `#[non_exhaustive]` with a `new`
  (ADR-0039 kind 2), and `Cast<S>` has a `Default` for any `S`.
  `Cast::voice_for(speaker, variant)` reads the speaker-and-suffix entry,
  else the speaker's.
- The RON shape is nested, not flattened:
  `"hero.light": (voice: "hero_light", settings: (reader: Piper))`. A tool
  with no settings uses `S = ()` and writes `settings: ()`: serde can't
  default a missing field without `S: Default`, and the ticket asks for
  `S: DeserializeOwned` only. The module doc shows the shape.
- `cast_from_source::<S, V>` keeps the old checks and adds the variant
  keys. A key that is itself a speaker is the speaker's, whatever it ends
  with. Otherwise a key ending in a variant's suffix names that speaker,
  which must speak. A key with a speaking prefix and no known suffix gets
  its own message listing the suffixes. Narration may have variants too
  (`">.sun"`).
- `ClipVariant::variants()`: a new provided method (the default plus the
  split ones), so no implementor changes. A game with variants outside
  both overrides it.
- `docs/formats.md`'s *Checks after loading* line names the variants and
  the tool's settings.
- No follow-up tickets. 2251 already builds on `Cast<S>`. The game moves
  its voice tool onto this in `game#1242`. The game doesn't call `Cast`
  or `cast_from_source` today (`game#0760`), so the pin bump past this PR
  needs no game code.

Breaking: shuyi-assets: voice::Cast: it takes the tool's settings type, `Cast<S>`; `voices` holds `CastVoice<S>` (`.voice` is the old string), and it is `#[non_exhaustive]`, built with `Cast::new` or `default()`.

```migration
since: 0.1.0
what: A cast's voices carry the voice tool's settings, so Cast takes the settings' type and each voice is a CastVoice.
old: "Cast { voices: BTreeMap<String, String> }"
new: "Cast<S> { voices: BTreeMap<String, CastVoice<S>> }"
before: |
  let voice: &String = &cast.voices["guide"];
after: |
  let voice: &String = &cast.voice_for("guide", variant).unwrap().voice;
find: ["cast.voices["]
```

Breaking: shuyi-assets: voice::cast_from_source: it is generic over the tool's settings `S` and the game's `ClipVariant` `V`, and each entry in the file is `(voice: "..", settings: ..)` instead of a bare string.

```migration
since: 0.1.0
what: A cast file gives each speaker a voice and the tool's settings, and its keys may name a clip variant.
old: "cast_from_source(file, source, can_speak) -> Result<Cast, _>"
new: "cast_from_source::<S, V>(file, source, can_speak) -> Result<Cast<S>, _>"
before: |
  let cast = voice::cast_from_source("cast.ron", &text, can_speak)?;
  // cast.ron: (voices: { "guide": "guide_a" })
after: |
  let cast: Cast<()> = voice::cast_from_source::<(), Voice>("cast.ron", &text, can_speak)?;
  // cast.ron: (voices: { "guide": (voice: "guide_a", settings: ()) })
find: ["cast_from_source(\""]
```

Book: none, the book shows no voice cast; the cast is read by a game's own voice tool, not by a game's code the book teaches.
