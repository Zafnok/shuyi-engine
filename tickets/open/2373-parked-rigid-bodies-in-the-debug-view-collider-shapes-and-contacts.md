---
id: "2373"
title: "Parked: Rigid bodies in the debug view: each collider's shape and the last step's contacts on the collision channel"
type: feature
milestone: "Engine: more styles"
model: sonnet-5
effort: medium
status: blocked
blocked_by: ["1128", "2317"]
nick_input: none
completed:
---

# 2373 — Rigid bodies in the debug view: colliders and contacts

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; and 2026-10-20 for 1128, as its note quotes, `docs/decisions.md`): a follow-up of the parked rigid-body module (1128) and of the parked collision bounds in the debug view (2317), and a follow-up of parked work is parked. So it starts after both are unparked and done. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

2317 draws the collision kit on a `collision` channel of the debug
view: solid tiles, `Aabb` boxes, triggers and what each mover hit. It
leaves the rigid bodies of 1128 to this ticket.

1128 adds `shuyi-physics`, a module over `rapier2d`: a `PhysicsWorld`
of bodies and colliders, stepped from a `TickSim::step`. When a crate
falls through a slope, or a sensor fires in the wrong place, a
developer wants to see each collider and where it touches, drawn over
the running game. Godot's "Visible Collision Shapes" does this for its
physics bodies.

The helpers can't sit beside 2317's in `shuyi_ui::debug::collide`.
`shuyi-ui` is a default crate, and no default crate depends on a
module (ADR-0009 §4; `cargo xtask check-modules` fails on it). A
module brings its own pieces instead, and a drawing part sits behind
the module's own feature, so a headless server keeps the simulation
alone (ADR-0009 §4). So the helpers go in `shuyi_physics::view::debug`,
behind a `view` feature that adds `shuyi-ui`. The procedural animation
module's helpers sit the same way, in `shuyi_procanim::view::debug`.

The debug view's rules hold here too (its ADR, linked from
`shuyi_ui::debug`'s module docs):

- a gizmo is plain data, with a role the debug skin colours;
- gizmos are built from `&PhysicsWorld`, never `&mut`, so a replay
  with the channel on hashes the same as one with it off;
- nothing is built unless debug tools are on.

## Nick input

None.

## Scope

**In:**
- **`shuyi_physics::view::debug`**, behind the crate's new `view`
  feature, with two helpers:
  - `collider_gizmos(&PhysicsWorld, view, to_px, &mut Gizmos)`: each
    collider's outline:
    - a box: a `Rect` when it isn't rotated, the same one 2317's
      `box_gizmos` gives for an `Aabb` in the same place; four
      `Line`s between its corners when it is;
    - a ball: a `Circle` of its radius;
    - a capsule: a `Circle` at each end and a `Line` along each side;
    - a convex polygon, a polyline, a segment or a triangle: a `Line`
      per edge;
    - a compound shape: each of its parts, as above;
    - any other shape (a heightfield, a triangle mesh): its bounding
      box as a `Rect`, with the shape's kind as a `Label`.

    The role says what the collider is: `Solid` on a fixed body or on
    no body, `Bounds` on a dynamic or kinematic body, and `Trigger`
    for a sensor, whatever its body.
  - `contact_gizmos(&PhysicsWorld, view, to_px, &mut Gizmos)`: a
    `Cross` (`Note`) on each contact point of the last step, from the
    manifolds of each touching pair in rapier's narrow phase, in world
    space.
- Both push on 2317's `collision` channel, the same `Channel` value
  its helpers use, not a copy. Both check `Gizmos::channel_on` first
  and do no work when the channel is off.
- `view` and `to_px` are the same as in 2317's helpers, and every
  point is rounded through 2317's `debug::collide::px`. So a rigid box
  and a mover's box in the same place draw on the same pixels.
- Positions leave the world through the crate's own edge from `f32`
  to `Fx` (1128), never a second rounding rule. If 1128 scales metres
  to pixels, that scale is applied there, once.
- A stable order: colliders in the order `PhysicsWorld` keeps them,
  contacts sorted by their pair's collider handles. The gizmo list is
  the same on every run and every platform.
- A gizmo whose bounding box doesn't meet `view` is dropped, so a
  collider wholly outside the view gives none. The painter clips the
  rest, as it does every gizmo.
- The helpers read `PhysicsWorld`'s collider set and narrow phase
  from inside the crate (`pub(crate)` where needed). The only new
  public items are the two functions, the `view` module and the
  `view` feature. There is no new public type (ADR-0039), and nothing
  breaks, so no `Breaking:` line.
- **Wiring:**
  - `crates/physics/Cargo.toml`: `view = ["dep:shuyi-ui"]`, with
    `shuyi-ui` an optional dependency;
  - `tools/layers.ron`: `shuyi-ui` joins `shuyi-physics`'s `engine`
    list, with a comment saying the `view` feature brings it;
  - `crates/shuyi/Cargo.toml`: the `physics` feature turns on
    `shuyi-physics/view` too. The front door holds `shuyi-ui` already,
    so it costs a game nothing more.
- **Docs:** the `shuyi-physics` README gains a short section, "In the
  debug view": the feature, the two helpers, a `Screen::debug_draw`
  that calls them, and what each role means. Its "what it costs" part
  says the `view` feature adds `shuyi-ui`. The `shuyi-physics` skill
  (`skills/shuyi-physics/SKILL.md`, which 1128 writes) gains one line
  on the helpers.

**Out (do not do):**
- joints: 1128 builds only the joints its test scene needs;
- velocity arrows, forces, sleeping bodies and rapier's broad phase;
- anything in `shuyi_ui`: a default crate never depends on a module;
- calling the helpers from an example game: the tests use 1128's
  test scene;
- 3D colliders (2320);
- editing bodies or colliders from the view.

## Implementation steps

1. Read the debug view's ADR, `shuyi_ui::debug::gizmo`, 2317's
   `shuyi_ui::debug::collide` (its `px`, its channel and its
   `box_gizmos`), and 1128's `crates/physics`: `PhysicsWorld`, its
   edge from `f32` to `Fx`, its README and its test scene.
2. Add the `view` feature and its wiring (Scope, *Wiring*).
3. Write `crates/physics/src/view.rs` (module docs, `pub mod debug`)
   and `crates/physics/src/view/debug.rs` with the two helpers, behind
   `#[cfg(feature = "view")]` in `lib.rs`. Unit tests go in
   `view/debug/tests.rs`.
4. Make `cargo test --workspace` run them: give the crate a
   dev-dependency on itself with `view` on, as `crates/ui/Cargo.toml`
   does for its `toy` feature.
5. Write `crates/physics/tests/debug_view.rs`: a test game of one
   screen that holds 1128's test scene, steps it, and calls both
   helpers from its `debug_draw`. The Harness drives it
   (`shuyi-platform`'s `harness` feature, a dev-dependency).
6. Write the README section, and the skill line.
7. Run the gates, `cargo xtask check-layers` and `cargo xtask
   check-modules` among them.

## Acceptance criteria

- [ ] With `collision` on, the test game's frame snapshot lists the
      slope as `Solid`, each box as `Bounds`, and a `Note` cross where
      a resting box touches the slope.
- [ ] With the channel off, the helpers push nothing.
- [ ] An unrotated box collider and an `Aabb` of the same place give
      the same `Rect`.
- [ ] The test scene's hash per step matches its golden table
      (`crates/physics/tests/golden/scene.ron`) with the channel on
      and with it off.
- [ ] `cargo tree -p shuyi-physics -e normal` lists `shuyi-ui` only
      with `--features view`.
- [ ] `cargo xtask check-layers` and `cargo xtask check-modules` pass
      with the new edge.
- [ ] The `shuyi-physics` README has its "In the debug view" section.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, on small hand-made worlds:
  - a box at 0 and at 45 degrees (one `Rect`; four `Line`s);
  - a ball (one `Circle`), a capsule (two `Circle`s, two `Line`s), a
    triangle (three `Line`s) and a compound of two boxes;
  - the roles: a collider on a fixed, a dynamic and a kinematic body
    (`Solid`, `Bounds`, `Bounds`), and a sensor (`Trigger`);
  - the box against 2317's `box_gizmos` for an `Aabb` of the same
    place;
  - contacts: none before two colliders touch, at least one cross
    after, and the same list in the same order over two runs;
  - a collider outside `view` gives no gizmo.
- Property: every gizmo's bounding box meets `view`, for any view
  position and size and any step of the test scene.
- Golden: the test scene's table, with the channel on and off.
- Snapshot / integration: the test game with `collision` on, after
  the boxes come to rest. The snapshot (`insta`) is committed.

## Completion notes

*(Filled in by the session that completes the ticket.)*
