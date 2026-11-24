# First person

- **Covers:** `perspective/first-person`, `world-drawing/full-3d`
- **Status:** planned
- **Tickets:** 3101, 3104, 3108

A 3D world seen from the player's eyes, walked and looked around freely.
The engine has no 3D yet: ticket 3101 builds 3D space in fixed point and
the `shuyi-3d` crate, ticket 3104 the GPU painter that draws it, and
ticket 3108 the first-person and third-person example and templates,
which write this guide in full.

## Until then

- Nothing in the default engine draws 3D, and nothing should be built
  toward it with floats: the rules' space will be fixed point, as every
  simulation's is.
- A labyrinth walked a cell at a time is a
  [dungeon crawler](dungeon-crawler.md), not this.
