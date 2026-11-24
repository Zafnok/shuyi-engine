# Dungeon crawler

- **Covers:** `kind/dungeon-crawler`, `perspective/first-person-grid`
- **Status:** planned
- **Tickets:** 2741, 2753, 2754

A labyrinth seen from the eyes of the party, one cell and one quarter
turn at a time, with walls, doors and traps between the cells. The rules
take turns, as on a grid; only the view is first person. A module,
`shuyi-crawl`, will hold the labyrinth and its moves (ticket 2741), a 3D
skin will draw it (ticket 2753), and an example and template, a small
dungeon crawler with a town and two floors (ticket 2754), writes this
guide in full.

## Until then

- The [grid kit](top-down-grid.md) has tiles, not walls between them, and
  no facing: a labyrinth built on it would be rebuilt for the module.
- Don't draw the view from a free 3D camera: a crawler steps a cell and
  turns a quarter at a time, and its view is the cell's, not a
  [first-person](first-person.md) game's.
