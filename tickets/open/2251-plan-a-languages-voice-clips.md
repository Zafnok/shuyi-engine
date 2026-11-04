---
id: "2251"
title: "Plan a language's voice clips: what to make, make again, record again or remove, and each voice's recording script"
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: ["2250"]
nick_input: none
completed:
---

# 2251 — Plan a language's voice clips

## Context

1. **The game ticket:** `game#0760`
   ([github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0760-one-form-for-cast-ron.md](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0760-one-form-for-cast-ron.md)),
   and the game ticket that moves the game onto this one, `game#1242`.
2. **The job in one sentence:** given the scripts, the cast and the clips
   already made, say which clips are missing, which say old words or an
   old direction, which were made with a voice that changed, which a
   person must record again, which to delete, and which lines nobody is
   cast to say; and list a voice's lines in order for a recording session.
3. **Friction numbers:** no friction review row of its own. The game's
   voice tool (`cargo xtask voice`, game ticket 0721) is about 1,000
   production lines; its plan (`crates/xtask/src/voice/plan.rs`, about
   180 production lines) and its recording script are generic. Only the
   part that runs the speech programs is the game's.
4. **The generic shape:** a pure function of the dialogue table (the
   scenes to voice), the cast (2250's `Cast<S>`), the manifest's clips
   and three hooks from the tool:
   - which variants a line needs (`fn(speaker, text) -> &[V]`; none for a
     line nobody says aloud, such as a reply),
   - the words a variant says (`fn(text, V) -> Option<String>`, the same
     hook `is_stale` takes),
   - whether a generated clip's voice changed
     (`fn(&VoiceClip<V>, &CastVoice<S>) -> Option<String>`, a reason in
     the tool's words: "the voice changed", "the sample changed", "spelled
     differently"). The engine compares the manifest's `voice` with the
     cast's itself.

   It returns a `VoicePlan<V>`: every clip wanted (line, variant,
   speaker, cast key, words, direction, and the line before it for
   context), the ones to make with a `Why` (missing, words, direction,
   voice changed with the tool's reason), the recorded ones to record
   again (by words and direction only), the clips to remove (their line
   is gone from a voiced scene, or it needs other variants), and the
   uncast lines. `recording_script(plan, cast key)` gives one voice's
   wanted clips in script order.
5. **Examples from games that aren't tactics:** a visual novel with
   recorded voices needs, after each script change, the lines each actor
   must record again and the clips to delete; a narrative game with
   generated voices needs the clips to make again after a character's
   sample changes. Both are this plan. The engine's visual novel voices
   (2725, parked) would use it to report missing clips.

## Nick input

None.

## Scope

**In:**
- `voice::plan` (or a `voice::plan` module): `VoicePlan<V>`,
  `WantedClip<V>`, `Why`, the function above and `recording_script`, all
  in `shuyi_assets`, pure (no files, no clock).
- New public types follow ADR-0039: `#[non_exhaustive]`, with
  constructors where a game builds one.
- **API: additive.** The game's PR that moves the pin past this one needs
  no code; `game#1242` moves its tool onto it.
- The book's voice chapter: how a tool plans with it.

**Out (do not do):**
- Reading or writing the voice folder, the manifest file or any tool's
  own files: the tool's.
- Making or levelling clips, checking a take: the tool's.
- A `shuyi voice` command: a later ticket if a second game wants one.

## Implementation steps

1. Read the game's `crates/xtask/src/voice/plan.rs` (ticket 0721) as the
   worked example; take its logic, not its names (no `lead`, no
   `say.ron`).
2. Write the types and `plan` in `crates/assets/src/voice/plan.rs` over
   `Cast<S>`, `VoiceClip<V>` and the dialogue table, with the hooks above.
3. Write `recording_script`.
4. The book.

## Acceptance criteria

- [ ] Each `Why`, a recorded clip to record again, a removed clip (line
      gone; variants changed), an uncast line and a reply line with no
      clip each have a test.
- [ ] A clip in a scene that isn't voiced is never removed while its line
      exists.
- [ ] `recording_script` lists one voice's clips in script order, with the
      line before each.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the cases above, on a two-scene test script, a cast with a
  speaker-and-variant key, and the test `ClipVariant`.
- Property: planning a manifest made exactly as the plan's `wanted` gives
  nothing to make, record or remove.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
