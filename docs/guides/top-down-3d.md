# Top-down 3D

- **Covers:** `perspective/top-down-3d`, `world-drawing/full-3d`
- **Status:** planned
- **Tickets:** 3101, 3104, 3109

A 3D world seen from above, for games whose rules need height. The
engine has no 3D yet: ticket 3101 builds 3D space in fixed point and the
`shuyi-3d` crate, ticket 3104 the GPU painter, and ticket 3109 the
top-down 3D and HD-2D example and templates, which write this guide in
full.

## Until then

- A game whose rules are flat and only its look is 3D is
  [HD-2D](hd-2d.md): it keeps the 2D kits and gets a 3D skin.
- A flat world seen from above is [top-down free](top-down-free.md) or
  the [top-down grid](top-down-grid.md). Don't reach for 3D to draw one.
