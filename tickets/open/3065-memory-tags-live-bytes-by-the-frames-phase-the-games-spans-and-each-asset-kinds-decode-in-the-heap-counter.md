---
id: "3065"
title: "Memory tags: live bytes by the frame's phase, the game's spans and each asset kind's decode, counted by the heap counter in debug builds"
type: feature
milestone: MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["3064"]
nick_input: none
completed:
---

# 3065 — Memory tags

## Context

Nick, 2026-10-24 ([`docs/decisions.md`](../../docs/decisions.md),
"perf traces agents can read"):

> "we should also have easily agent-readable / streamable perf traces
> when building in debug -- both timings and heap snapshots"

He put them in the MVP. 3064 counts the heap: `shuyi-alloc-count`'s
`CountingAlloc` is the global allocator of every debug build of a game
on `shuyi-app` and of its tool program, and each frame's record has its
live bytes, its peak and its allocations. That says how much, not what.
3063's ADR defines a heap snapshot: live bytes and counts **by memory
tag**, a tag being the frame's phase, the game's span, or the asset kind
being decoded, the innermost one winning. This ticket counts by tag.
2554 then takes snapshots and diffs of the tags and puts them in the
trace.

An agent that sees the heap grow 50 KB a frame then reads where:
`update`, the game's `ai` span, or `image` decodes the game keeps. This
is how Unreal's Low-Level Memory Tracker counts (a tag per scope, set as
code enters it), with no call stacks.

**What Shuyi has today** (read on 2026-10-24):

- 3064's `CountingAlloc` counts bytes asked for, in total.
- 3063: `Ctx::span(name, |ctx| …)`, `Game::frame` reading the clock
  around the drawing (`self.redraw()` in `step`,
  `crates/ui/src/game.rs:426`), and `App` reading it around
  `present_game`, at a frame's start and end, and timing the load
  (`App::load`, `crates/platform/src/app.rs`). App moves the
  platform's clock into the game's context when the game runs.
- `crates/assets/src/format.rs`: every registry load goes through
  `Formats::decode` (line 604). `crates/assets/src/bundle.rs:453`:
  `bundle::watch` sets a process-wide `fn` in a `OnceBox` that hears
  every file read. `Formats` is `Send + Sync`: its kinds are
  `Box<dyn Kind>`, and `Kind: Any + Send + Sync`.
- 2368 (parked) planned `watch_decodes`, a watcher of each decode with
  its kind, path, bytes and result, for its tally of loads by kind. This
  ticket needs a watcher in the same place, before and after each
  decode.

## Nick input

None.

## Scope

**In:**
- **Tags in `shuyi-alloc-count`:**
  - `CountingAlloc::with_tags(inner)`: each block carries its tag in a
    header in front of it. The header takes `max(layout.align(), 8)`
    bytes, so the block keeps its alignment; the tag (a `u16`) sits in
    its last two bytes, just before the block. `dealloc` and `realloc`
    find it from the pointer and the layout's alignment. The inner
    allocator is asked for the size plus the header, at the same
    alignment. The counts stay the bytes asked for: the header's bytes
    are never counted. `CountingAlloc::new` (3064) has no header, for
    3008's bench and the Playdate;
  - **the names:** a process-wide table of 256 tags. Tag 0 is `other`
    (no tag entered); tag 255 is `overflow`, which every tag past the
    254th shares, with a count of how many names fell there.
    `tags::tag(class: u8, name: &'static str) -> u16` finds or adds a
    name (compared by its text, under a spin lock, outside the
    allocator's path);
  - **the current tag,** per thread: `tags::enter(tag: u16) -> u16`
    sets it and gives back the one before, and `tags::leave(before:
    u16)` puts that back. With `std` it is a `thread_local!` `Cell<u16>`
    with a `const` initializer (no destructor, so the allocator reads
    it without allocating); without `std`, a static (one thread);
  - **the counters, per allocator:** for each tag, live bytes, live
    blocks, bytes ever allocated, and the peak of its live bytes. An
    allocation adds to the current tag; a free takes from the tag in its
    header; a `realloc` moves the block's bytes to the current tag (it
    takes the old size from the old tag, adds the new size to the
    current one, and rewrites the header), so a `Vec` that grows in
    `update` counts in `update`;
  - `rows(&self) -> Vec<TagCounts>`: every tag with live bytes or bytes
    ever allocated, with its class, name and four counters. It
    allocates, so it is never called from inside the allocator;
  - the `global` static (3064) becomes `CountingAlloc::with_tags`.
- **`TagClass`** in `shuyi_ui::perf`: `Phase`, `Span`, `Asset`,
  `Other`, `#[repr(u8)]`, `#[non_exhaustive]`, with `from_u8`, serde,
  and `JsonSchema` under the `schema` feature.
- **`MemoryScopes { tag: fn(u8, &'static str) -> u16, enter: fn(u16) ->
  u16, leave: fn(u16) }`** in `shuyi_ui::perf`, `#[non_exhaustive]`
  with `new`: plain functions a backend hands over, so `shuyi-ui` and
  `shuyi-platform` never depend on the allocator.
  `Platform::memory_scopes(&self) -> Option<MemoryScopes>`, defaulted
  to `None`. `App` takes it with the clock and moves it into the
  context the same way (3063), only while debug tools are on.
- **Where tags are entered,** each only when the context has scopes:
  - `Ctx::span` enters a `Span` tag named for the span around its
    closure;
  - `Game::frame` enters the `Phase` tags `update` (from its start to
    the drawing) and `draw` (the drawing);
  - `App` enters `load` around the game's load, `paint` around
    `present_game`, `perf` around building and keeping the frame's
    record (so the trace's own bytes are seen apart), and `app` for the
    rest of its frame;
  - **each decode:** `shuyi_assets::format::watch_decodes(watch:
    DecodeWatch) -> Result<(), DecodeWatch>`, set once in a `OnceBox`
    like `bundle::watch`, with `DecodeWatch { before: fn(kind: &'static
    str) -> u32, after: fn(token: u32, &Decoded<'_>) }` and `Decoded<'a>
    { kind: &'static str, path: &'a str, bytes: usize, ok: bool }`.
    `Formats::decode` calls `before` with `K::NAME` and `after` once,
    after it decides, with the token `before` gave. Nothing else in
    `shuyi-assets` changes, and `Formats` stays `Send + Sync`. A backend
    with scopes sets it when it starts, before the game's load:
    `before` enters an `Asset` tag named for the kind and `after` leaves
    it. So the bytes a decode makes, and the game keeps, count under
    their kind: what the game holds of each kind of asset, with no asset
    store (2906 may decide one).
  - **Either order with 2368:** if 2368 is done, it wrote
    `watch_decodes` with an `after` alone, for its tally of loads by
    kind: add `before` and the token, and keep its tally working in the
    same watcher; if not, 2368 hangs its tally on this `after` (its own
    clause says so).
- **The backends:** `shuyi-app` and `shuyi-cli`'s `Headless` give
  `MemoryScopes` from `shuyi_alloc_count::tags` and set the decode
  watcher, when `global::reading()` is `Some`. The headless backend has
  `HeadlessPlatform::with_scopes(MemoryScopes)` for tests.
- **Read-only:** entering a tag changes no state a game reads. The toy
  plays a script with scopes on and off and ends with the same screens
  and frame text.
- **Docs** (the `document` skill): `docs/platform-backends.md`
  (`Platform::memory_scopes`, and setting the decode watcher); the
  crate's docs (the header, the table, `other` and `overflow`, and the
  cost: a header per block in debug builds).
- The new public types follow ADR-0039.

**Out (do not do):**
- heap snapshots, diffs, `Platform::heap_snapshot` and the tags in the
  trace (2554);
- loads by kind as a tally, and the debug view's `memory` channel
  (2368, parked);
- call stacks, single blocks, and the largest blocks: native tools do
  those (3063's ADR);
- tags for each screen or each module hook, made by the engine itself: a
  follow-up if agents ask, as for spans;
- a tag that follows a block to the code that keeps it (the tag is
  where it was allocated or last grown).

## Implementation steps

1. Read 3063's ADR, 3064's crate, `shuyi_ui::perf`, `Ctx::span`,
   `Game::frame`, App's measuring, `crates/assets/src/format.rs`
   (`Formats::decode`) and `bundle.rs` (`watch`), and 2368's ticket.
2. Write the header, the names table, the current tag and the counters,
   with their tests on an instance called directly.
3. Add `TagClass`, `MemoryScopes`, `Platform::memory_scopes`, and the
   tags entered in `Ctx::span`, `Game::frame` and `App`.
4. Add `watch_decodes` and its call in `Formats::decode`.
5. Hand the scopes and the watcher over in `shuyi-app` and `shuyi-cli`;
   write the docs; run the gates.

## Acceptance criteria

- [ ] On an instance with tags, blocks of random sizes and alignments
      (1 to 4,096, alignments 1 to 64) keep their bytes through
      `realloc` and free cleanly, and every tag's live bytes equal a
      model's (the property test below).
- [ ] A `Vec` made under one tag and grown under another counts under
      the second.
- [ ] The 255th distinct name counts under `overflow`, and the overflow
      count says 1.
- [ ] A test loads a bundle of two kinds through the registry with a
      watcher set: `before` and `after` are heard once per decode, with
      the kind, path, bytes and result, the broken file's `ok` false.
- [ ] `Formats` is still `Send + Sync` (a compile-time test).
- [ ] With scopes on the toy (fake scopes that record what was entered),
      a frame enters `app`, `update`, `draw`, `paint` and `perf`, and a
      test screen's span enters its name inside `update`; with scopes
      off, nothing is entered; the screens and frame text are the same.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the header's place for each alignment; `tag` finding a name
  again by its text; `enter` and `leave` nesting; `realloc` moving bytes
  between tags; `rows` leaving out tags never used; the decode watcher.
- Property: random sequences of `alloc`, `alloc_zeroed`, `realloc` and
  `dealloc` with random sizes, alignments and current tags, on an
  instance: each block's bytes survive, and each tag's live bytes and
  blocks equal a model's.
- Integration: the toy with fake scopes; a registry load with the
  watcher.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
