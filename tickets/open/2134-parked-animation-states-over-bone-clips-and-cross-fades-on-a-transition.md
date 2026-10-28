---
id: "2134"
title: "Parked: Animation states over bone clips: the state machine picks the skeletal module's clips and cross-fades them on a transition"
type: feature
milestone: "Engine: animation"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2104", "2105", "2109", "2231"]
nick_input: none
completed:
---

# 2134 — Animation states over bone clips

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): a follow-up of the parked state machines over clips (2104) and the parked skeletal module (2105), and a follow-up of parked work is parked, so it starts after both are unparked and done (and after 2109 and 2231, which they wait for too). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

2104 builds animation state machines over clips: an `AnimGraph` whose
states each pick a clip, with transitions on parameters, exits when a
clip ends, direction sets and frame events. It is written for
frame-by-frame sprites, where two clips can't be mixed:
`AnimGraph::frame` gives a frame through 2231's `frame_at`. Its **Out**
leaves mixing two poses to bones.

2105 builds bones: `pose_at(&Rig, clip, elapsed_ms)` gives a pose, and
`Pose::mix(&a, &b, weight)` mixes two (weight 0 to 256). Its Scope says
a cross-fade is mixing two clips' poses over a few steps, but which
clip plays it leaves to 2104. Each ticket's **Out** points here.

Without this ticket, a game with a bone character writes its own state
machine, or drives 2104's graph and swaps bone clips by hand. Either
way the character pops from one pose to the next on every transition.

Other engines:

- Godot's `AnimationTree` state machine gives each transition an
  `xfade_time` and an `xfade_curve`.
- Spine's runtimes set a mix duration per pair of animations
  (`AnimationStateData.setMix`, with a `defaultMix`). An animation being
  mixed out fires no events (`TrackEntry.eventThreshold`, 0 by
  default). A mix cut short by a new animation keeps a chain of
  animations mixing out.
- Unity's Animator gives each transition a duration and an
  interruption source.

Here a fade is counted in the caller's steps, as 2104's runner counts
time, and its curve is a `shuyi_core::ease::Ease` (2109). The rule of
[ADR-0004](../../docs/adr/0004-assets-standard-files-in-the-engines-own-data-out-and-a-deterministic-movement-kit.md)
§3 holds: the pose shown is a pure function of the data, the stored
state and the step. A simulation stores one small `Copy` value per
thing, so replays are exact. A fade cut short keeps at most two states
mixing out, not Spine's chain, so that value has a fixed size.

## Nick input

None.

## Scope

**In:**
- **Clips from any source**, in `shuyi_assets::anim::graph` (2104's
  module):
  - a trait `ClipSource` with `has_clip(&self, name: &str) -> bool`,
    `loops(&self, name: &str) -> bool` and `ended(&self, name: &str,
    elapsed_ms: u64) -> bool`. 2231's `AnimationSet` implements it
    (through `AnimationClip::ended`), and so does 2105's `Rig`, in
    `shuyi-bones`, from a clip's duration and repeat rule;
  - `AnimGraph::step` reads `ended` through it, and
    `AnimGraph::check_clips(&self, clips: &impl ClipSource) ->
    Vec<GraphProblem>` runs 2104's clip checks (an unknown clip, `ended`
    on a looping clip) against any source;
  - how depends on what 2104 built (step 1). If `step` already takes
    its clips as a parameter, that parameter becomes `&impl ClipSource`.
    If the graph holds its clips, add `step_with(&self, clips: &impl
    ClipSource, …)` and make `step` call it with them. Either way a
    graph over sprite clips gives the same states and events as before.
- **A fade on a transition**, in the same module:
  - `Fade { steps: u16, ease: Ease }` (2109's `Ease`), serde, defaulting
    to 0 steps (a cut) and `Linear`;
  - a transition gains `fade`, and a `Dirs` state gains `fade` for a
    change of pick. In RON: `fade: (steps: 6, ease: InOut(Sine))`. Both
    may be left out (`#[serde(default)]`). Steps are the caller's: the
    step numbers `step` takes;
  - `Stepped` gains `fired: Option<usize>`, the index of the transition
    that fired, so a runner reads its fade;
  - the sprite runner (`AnimGraph::frame`) ignores fades, since a frame
    can't be mixed. The docs say so.
- **The runner**, in `shuyi_bones::graph` (`crates/bones/src/graph.rs`,
  tests in `crates/bones/src/graph/tests.rs`), pure and `no_std`:
  - `BoneGraph<'a>` over `&'a AnimGraph` and `&'a Rig`, made by
    `BoneGraph::new(graph, rig) -> Result<Self, Vec<GraphProblem>>`,
    which runs the checks below;
  - `BoneAnim { now: AnimState, fade: Option<Fading> }`, `Copy` and
    serde: all a simulation stores per thing, 2104's `AnimState` and
    the fade in progress;
  - `Fading { from: FadeFrom, began: u32, fade: Fade }`, where
    `FadeFrom` is `Playing(AnimState)` or `Mix { a: AnimState, b:
    AnimState, weight: u16 }` (a fade cut short, below);
  - `BoneGraph::step(&self, anim: BoneAnim, params: &Params, step: u32)
    -> BoneStepped { anim, events, used_triggers }`.
    It calls the graph's step. When a transition fires whose fade has
    steps, or a `Dirs` state's pick changes and its fade has steps, it
    starts a fade from what was shown, with `began` the current step;
  - `BoneGraph::pose(&self, anim: &BoneAnim, step: u32) -> Pose`. A
    state's pose is `pose_at` of its clip (for a `Dirs` state, its pick)
    at `shuyi_core::timer::ms_from_steps(step - entered_step)` ms (the
    subtraction saturating), the one rule from steps to ms that 2104's
    `step` uses too; no `ms_per_step` is passed. With no fade, that
    pose. During a fade, `Pose::mix(from, to, w)`, where `from` for a
    `Mix` is `Pose::mix(a, b, weight)`. Drawing needs nothing new:
    2105's `RigView::from` takes the `Pose`.
  - **Either order with 2115:** if 2115 is done, use its
    `ms_from_steps`; if not, use the one 2104 (a blocker) wrote in
    `shuyi_core::timer`. Either way nothing is written here.
- **The weight.** At `k = step - began` steps into a fade of `n` steps,
  `p = k * ONE / n` (2109's `ONE`), `e = fade.ease.ease(p)`, and
  `w = e * 256 / ONE`, in `i64`, rounded to nearest with halves up and
  clamped to `0..=256`. So an overshooting curve (`Back`, `Elastic`)
  holds at the ends. `w` is 0 on the step the fade began and 256 at
  `k = n`, where the fade ends (`fade` becomes `None`).
- **A fade cut short** starts from the pose shown. When a new fade
  starts (a transition, or a pick change) during a fade at weight `w`,
  `y` being the state the old fade was fading to:
  - if the fade was from `Playing(x)`, the new fade is from `Mix { a:
    x, b: y, weight: w }`. Both keep playing and the weight between them
    holds, so on the cut step the pose is exactly the old fade's;
  - if it was from `Mix { a, b, weight }`, the pose shown mixes three
    states, and only two are kept. Their shares are `a`: `(256 - w) *
    (256 - weight)`, `b`: `(256 - w) * weight`, `y`: `w * 256`. The two
    largest are kept (on a tie, the newer), as a `Mix` of the older and
    the newer, whose weight is the newer's part of the two, out of 256,
    rounded. The pose jumps by the dropped share, at most a third. The
    docs say so;
  - a transition whose fade has 0 steps cuts to its state, and the fade
    ends. A pick change with no fade only changes the clip `y` plays,
    and the fade goes on.
- **Events.** During a fade only the incoming state fires events, from
  its clip's event keys, by 2105's `events_between` (the rule of
  `shuyi_core::timer::crossed`): a clip being mixed out fires none, as
  in Spine. Entering a state is the first call, so an event at 0 ms
  fires then, as 2104's frame events do. A `Dirs` pick change keeps the
  elapsed time, so the new pick's clip fires the keys crossed since the
  step before.
- **The checks**, in `BoneGraph::new`, each naming the state and the
  clip (and the graph's file and line, if 2104's graph keeps them after
  loading):
  - a state names a clip the rig doesn't have;
  - `ended` on a state whose bone clip loops;
  - frame events (`(frame index, name)`) written in the graph for a bone
    clip: a bone clip's events are its own keys, so this is an error.
- **Fixtures** (ours, CC0): a graph over 2105's fixture arm,
  `arm.anim_graph.ron` beside the rig, with a looping `walk` state, a
  `Once` state `wave` that returns to `walk` when it ends, a `stop`
  trigger that cuts `wave` short, and fades of 4 steps. If the fixture
  arm has one clip only, add a short `wave` clip to it.
- **An ADR** (the `write-adr` skill): a graph drives bone clips through
  `ClipSource`; a fade is counted in steps with an `Ease`; a fade cut
  short starts from the pose shown and keeps at most two states.
- **Docs** (the `document` skill): the module docs of
  `shuyi_bones::graph`, with an example; the `anim_graph` format's
  description in `docs/formats.md`, where 2104 wrote it, gains `fade`.
- **Public types (ADR-0039):** `Fade` is config a game fills in, so it
  is `#[non_exhaustive]` with `Fade::new(steps)` and `with_ease`.
  `BoneAnim` gets `BoneAnim::start(AnimState)`, and `FadeFrom` is
  `#[non_exhaustive]`. The fields added to 2104's transition, `Dirs`
  state and `Stepped` are additive where 2104 made those
  `#[non_exhaustive]`. The PR says which.
- **For the game:** no API break, unless a type of 2104's that gains a
  field isn't `#[non_exhaustive]`. Then the completion notes get a
  `Breaking:` line for it (ADR-0038 §6).

**Out (do not do):**
- layers (an upper body on one graph over legs on another) and additive
  poses: a follow-up if a game needs one;
- blends of more than two clips at once by a parameter (Godot's
  `BlendSpace2D` over poses): a follow-up if a game needs one;
- fades between frame clips: 2104 draws frames, which can't be mixed;
- a fade written once per pair of states (Spine's `setMix`): here each
  transition has its own; a follow-up if graphs repeat themselves;
- showing fades in a graph panel or a debug readout;
- blending clips on 3D skeletons (3110, parked);
- IK, physics on bones and mesh deformation (2105's **Out**).

## Implementation steps

1. Read 2104's `anim::graph` as built: how `step` reaches its clips and
   `ended`, which of its types are `#[non_exhaustive]`, and whether the
   module sits behind a feature of `shuyi-assets`. Read 2105's ADR,
   `Rig`, `pose_at`, `Pose::mix` and `events_between`, and 2109's
   `Ease` and `ONE`. Pick the shape of `ClipSource` (Scope) and write it
   in the PR.
2. In `shuyi-assets`: `ClipSource` and its impl for `AnimationSet`,
   `check_clips`, `Fade`, the two `fade` fields and `Stepped::fired`,
   with RON tests for `fade`. 2104's tests pass unchanged.
3. In `shuyi-bones`: `ClipSource` for `Rig`; `BoneGraph`, `BoneAnim`,
   `Fading`, `FadeFrom`, `step`, `pose` and the checks. If 2104 put
   `anim::graph` behind a feature of `shuyi-assets`, turn it on in
   `crates/bones/Cargo.toml`. No new crate edge: `shuyi-bones` already
   depends on `shuyi-assets` for `Rig` (its line in `tools/layers.ron`).
4. Write the fixture graph, the tests below and the snapshot.
5. Write the ADR and the docs.
6. Run the gates, including the crate's `no_std` build. Expect about
   600 changed lines, not counting fixtures and snapshots.

## Acceptance criteria

- [ ] A fade of `n` steps, for `n` from 1 to 16 and each `Ease` the test
      lists, gives exactly the outgoing pose on its first step and
      exactly the incoming pose from step `n` on.
- [ ] A fade of 0 steps cuts: the incoming pose on the step the
      transition fires.
- [ ] With `Linear` the weight never goes down during a fade; with
      `Out(Back)` it stays in `0..=256`.
- [ ] A fade cut short once gives, on the cut step, the pose the old
      fade gave on that step.
- [ ] A fade cut short twice keeps the two largest shares, as the Scope
      says, and `BoneAnim` keeps its size.
- [ ] A `Dirs` state with a fade turns with no pop: on the step its pick
      changes, the pose is the old pick's.
- [ ] During a fade only the incoming clip fires events, each once.
- [ ] A recorded list of parameters played twice gives the same
      `BoneAnim`s, events and poses (property test).
- [ ] A clip the rig lacks, `ended` on a looping bone clip, and frame
      events on a bone clip each give an error naming the state and the
      clip.
- [ ] 2104's tests over sprite clips pass unchanged.
- [ ] CI's `no_std` job builds `shuyi-bones` with the runner.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the weight at `k = 0`, `n / 2` and `n`; a fade of 0 steps; a
  transition from a state to itself; a cut during a fade from
  `Playing` and from `Mix`; a `Dirs` pick change with and without a
  fade; events during a fade; each check; `fade` read from RON and left
  out.
- Property: replay of random parameter lists; `pose` is pure; random
  fade lengths and eases give a weight in `0..=256` with exact ends; a
  single cut at a random step gives the old fade's pose on that step.
- Snapshot / integration: the fixture arm through a scripted run
  (`walk`, the `wave` trigger with its fade, cut short by `stop`, back
  to `walk`): each bone's position and angle (2105's `world`) per step,
  as `insta` text. The tests read the pose and the bones, not sprites
  (`AGENTS.md` rule 9).

## Completion notes

*(Filled in by the session that completes the ticket.)*
