---
id: "2911"
title: "Parked: Research: ray- and path-traced light and sound as an opt-in module for PCs and current consoles, behind a new ray_tracing capability that every other target lacks"
type: research
milestone: "Engine: ray tracing"
model: opus-5.5 / gpt-6.1-sol
effort: max
status: blocked
blocked_by: []
nick_input: none
completed:
---

# 2911 — Research: ray- and path-traced light and sound

> **Parked** (Nick, 2026-10-22: "obv the ray and path tracing would be for diff comp targets like PC, new gen consoles and wouldn't be shipped for i.e. GBA"): nothing in Visions of Shuyi needs it, and it builds on the renderer research (3034) and 3D (3101–3111), which are parked. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick asked on 2026-10-22 for ray-traced and path-traced visuals and
audio (`docs/decisions.md`, "weather, day and night, voice chat and ray
tracing get tickets"), as a feature for PCs and current consoles that
is never shipped to the GBA, the DS or the Playdate. The engine already
plans exactly that shape:

- **Modules a game opts into** (ADR-0009, 2003): a module is a crate
  with metadata, turned on by `shuyi add <module>`.
- **Target capabilities** (ADR-0012 §1, 3017, parked): each target's
  profile says what its hardware can do (`colour`, `screens`,
  `three_d`, `sprites`, `memory`, `audio`), each module says what it
  `needs`, and `shuyi check --target` and `shuyi export` refuse a module
  the target can't run. An `optional` part is left out and listed,
  never dropped silently.
- **Named effects with fallbacks** (ADR-0011 §7 and §8): HD-2D's
  effects are listed per target, and an optional effect a target can't
  run is left out there.

What stands in the way today:

- **The renderer.** `shuyi-app` draws through macroquad over miniquad
  (GL 3.3 and WebGL 2), which can't reach ray-tracing hardware. 3034
  (parked research) plans our own renderer over wgpu (ADR-0021 §1).
  wgpu's ray queries are, at the time of writing, an experimental
  feature on Vulkan (and in progress elsewhere), and browsers have no
  ray tracing in WebGPU.
- **The planned 3D painter** (3104, parked) stops at one shadow map;
  its **Out** leaves screen-space reflections, ambient occlusion and
  other effects not in ADR-0011 §7.
- **2D lights** (2328, parked): ambient and point lights with normal
  maps, no shadows cast by walls and no light that bounces.
- **Positional sound** (2363, parked): gain and pan by distance; its
  **Out** leaves occlusion, reverb zones and HRTF.
- **Consoles**: Switch (3014), PlayStation and Xbox (3016) are parked
  research; their backends are private (ADR-0008 §2). PS5 and Xbox
  Series X|S have ray-tracing hardware; the Switch 2's support must be
  checked on the day.

Three different things go by these names, and the report keeps them
apart:

1. **Ray-traced or path-traced 3D light** on ray-tracing hardware
   (DXR, Vulkan ray tracing, Metal ray tracing): shadows, reflections,
   global illumination, or a full path tracer with a denoiser.
2. **Traced 2D light**, which needs no ray-tracing hardware: rays
   marched through a 2D distance field on compute shaders (for example
   radiance cascades), so walls cast soft shadows and light bounces in
   a top-down or side-view world. It runs on any GPU with compute, so
   it could reach the web on WebGPU.
3. **Traced sound**: occlusion through walls, reverb shaped by the
   room, and sound that travels around corners, from rays cast through
   the level's geometry (Steam Audio, said to be Apache-2.0 since 2024;
   Resonance Audio, said to be Apache-2.0), or our own rays over the
   tile map and over 3105's 3D collision.

This ticket is research: a report, an ADR, an edit to 3017, and parked
tickets for the build. It promises no crate.

## Nick input

None. Which hardware and libraries to use is technical, and Nick has
set the product line: PCs and current consoles, never the small
targets. A library that costs money or brings a license with duties is
a question for Nick (the `ask-nick` skill), not a choice made here.

## Scope

**In:**
- **The needs**, written first: what a 2D game, an HD-2D game and a 3D
  game each get from traced light and from traced sound, with pictures
  or clips from shipped games named as examples; and what each costs a
  frame at 1080p and 4K on hardware of the day.
- **The capability** for 3017's `Capabilities`: a recommended shape,
  for example `ray_tracing: None | Compute | Hardware` (compute shaders
  only, or ray-tracing hardware), and which targets have which:
  desktop (by GPU, and what the check does when the player's GPU is
  older than the profile), the Steam Deck, the web, the Switch, PS5,
  Xbox Series, and `None` for the Playdate, the GBA and the DS. Whether
  traced sound needs a capability of its own (CPU cores and time, not
  a GPU). **Either order with 3017:** if it is done, write the new
  field as a build ticket; if not, write it as an edit to 3017's
  ticket.
- **The fallback rule**: a traced effect is `optional` by default, so a
  game that turns it on still runs everywhere with today's look
  (ADR-0012 §1); a game may mark it required, and then a target without
  it refuses the game. A player's setting turns it off on a capable
  machine. Say how a look test (`frame-png`) covers both paths.
- **Graphics paths**, with the facts of the day: wgpu's ray-query
  feature (status per backend, the wgpu version, what 3034 picks;
  **either order with 3034:** if it is done, build on its renderer; if
  not, name what this needs from it and add a line to 3034's ticket);
  compute-only 2D tracing on wgpu and WebGPU; a denoiser with a
  permissive license (or none, and why); consoles through their private
  backends (one paragraph: what each SDK offers, never its contents).
- **Sound paths**: Steam Audio and Resonance Audio, each checked with
  `cargo deny check licenses` in a scratch crate (and by the bundled C
  or C++ code's own license file, since cargo-deny reads only the
  crate's declared one), Windows with the GNU toolchain, and what they
  add to a build; against our own rays over the tile map (2D) or 3105's
  ray casts (3D). **From 2902 (done):** fit the engine-owned integer
  PCM mixer (3030), with DSP/geometry policy added by the build tickets
  this research writes. CPAL native output and the current-loader
  bridge are below that seam. Replacement-path implementation observes
  2573's parity gates and retains quad-snd until 3079; tracing is not
  silently added to the initial mixer scope.
  **Either order with 2363:** if it is done, extend its falloff; if
  not, add a line to its ticket.
- **The layers** (`AGENTS.md` rules 5 and 9, ADR-0002): traced light
  and sound are a skin and a backend. The rules never read a traced
  result, so a replay is the same with it on or off; the geometry it
  traces is built from the scene, never from state the rules hide.
  Floats are allowed there, as in today's renderers, and never in a
  simulation.
- **Measure**, in a scratch folder never committed: one prototype of
  traced 2D light over a frame of `examples/hello-grid` (compute
  shaders on wgpu), its time per frame on the desktop at hand, and the
  same on the web if WebGPU runs it. Say which hardware was used; say
  so if no ray-tracing GPU was at hand, and leave the hardware numbers
  to the build tickets.
- **The module shape**, recommended, not built: for example
  `shuyi-light-trace` (2D and HD-2D), `shuyi-rt` (3D, beside 3104's
  painter) and `shuyi-acoustics` (sound), or fewer; each module's
  `needs`; its settings as skin data; and where its effects join
  ADR-0011 §7's named list.
- **Outputs**:
  - the report, `docs/research/2911-ray-and-path-tracing.md` (create
    `docs/research/` if no earlier research has), with the facts dated;
  - an ADR (the `write-adr` skill) with the capability, the fallback
    rule, the paths per target and the module shape;
  - the build tickets (the `write-ticket` skill), each one PR, each
    `Parked:` with this one's note and milestone "Engine: ray tracing",
    listed in `docs/ROADMAP.md`'s *Parked* section;
  - the edits the decision needs in 3017's, 3034's, 3104's, 2328's,
    2363's tickets and the selected audio build tickets (2210, 3030–3032),
    preserving 2573's parity gates. A ticket already in `tickets/done/` is
    never edited; the change becomes a new parked build ticket instead.
- **Size:** if the PR passes about 600 changed lines with the report and
  the ADR, the build tickets and the edits become a follow-up ticket
  blocked by this one; the PR says so.

**Out (do not do):**
- building any of it: the tickets it writes do;
- the renderer itself (3034 and the tickets it writes) and the 3D
  painter (3104);
- screen passes that trace nothing (2326) and 2D lights without
  shadows (2328);
- console SDK details in this public repository (ADR-0008 §2): the
  report says only what is public;
- HRTF and binaural sound without geometry (a follow-up of 2363 if a
  game asks);
- reading a library's source to copy it (ADR-0021 §2); reading its docs
  and examples to judge it is fine;
- anything in Visions of Shuyi.

## Implementation steps

1. Read ADR-0009, ADR-0011 §7–§8, ADR-0012 §1 and ADR-0021, and 3017,
   3034, 3104, 2328, 2363 and 2902 as they stand. Note which are done,
   for the either-order rules above.
2. Write the needs into the report.
3. Survey the graphics and sound paths; check the licenses with
   `cargo deny` in a scratch crate.
4. Build the 2D tracing prototype and measure it.
5. Write the capability, the fallback rule, the layers and the module
   shape, then the recommendation.
6. Write the ADR, the parked build tickets, the edits to the six
   tickets, and the roadmap lines.
7. Run the gates.

## Acceptance criteria

- [ ] `docs/research/2911-ray-and-path-tracing.md` keeps traced 3D
      light, traced 2D light and traced sound apart, each with a dated
      table of paths (license, targets, hardware it needs, size added,
      cost a frame).
- [ ] The report says, for each of desktop, the Steam Deck, the web,
      the Switch, PS5, Xbox Series, the Playdate, the GBA and the DS,
      which `ray_tracing` value it has and what a game with traced
      effects does there.
- [ ] The report has the 2D prototype's measured frame time and the
      hardware it ran on.
- [ ] The fallback rule says how a game with traced effects still runs,
      and is look-tested, on a target without them.
- [ ] The ADR is in `docs/adr/`, linked from the report.
- [ ] 3017's ticket (or a new parked one, if 3017 is done) carries the
      `ray_tracing` capability.
- [ ] The build tickets are in `tickets/open/`, each parked, and
      `docs/ROADMAP.md` lists them in order.
- [ ] No recommended library is outside ADR-0002 §8.
- [ ] `cargo xtask ticket-lint` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None shipped (research). The tickets it writes say their tests.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
