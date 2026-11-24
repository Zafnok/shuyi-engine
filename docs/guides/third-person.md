# Third person

- **Covers:** `perspective/third-person`, `world-drawing/full-3d`
- **Status:** planned
- **Tickets:** 3101, 3104, 3108

A 3D world seen from behind the player, the camera following. The engine
has no 3D yet: ticket 3101 builds 3D space in fixed point and the
`shuyi-3d` crate, ticket 3104 the GPU painter, and ticket 3108 the
first-person and third-person example and templates, which write this
guide in full.

## Until then

- Nothing in the default engine draws 3D, and nothing should be built
  toward it with floats: the rules' space will be fixed point.
- A 2D game drawn in a 3D diorama is [HD-2D](hd-2d.md), not this: its
  rules stay 2D.
