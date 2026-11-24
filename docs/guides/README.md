# Guides

Shuyi Engine is built for many kinds of game, and a part built for one
kind is easy to mistake for a part of another. These guides say which
parts go with which kind of game, and how a game moves from one kind to
another. Each is one page, and links to the crates' READMEs and the
examples rather than repeating them.

## Choosing a direction

A game answers a few questions, the **concerns**. A **direction** is one
answer to each, and each stage of a game picks its own: a game can walk
its town top-down and fight its battles on a grid.

- **Time.** Does the game wait for the player (turns), or run on its own,
  a fixed number of steps a second (fixed ticks)?
- **Perspective.** How is the world seen and walked: a top-down grid,
  top-down and free, isometric, from the side, on hexes, or in 3D from
  the eyes, from behind or from above?
- **Bodies and motion.** How do things move: the engine's movers and
  drawn clips, rigid-body physics, or procedural animation?
- **Online play.** None, or lockstep, rollback or a referee server.
- **World drawing.** Flat 2D, a diorama (2D sprites in a 3D world, HD-2D)
  or full 3D.
- **Kind of game.** Tactics, real-time strategy, a visual novel, a
  mystery, a roguelike, a dungeon crawler, a party RPG or an action RPG.
  Unlike the others, these can share a stage: a tactics game can tell its
  story as a visual novel.
- **Scripted rules.** Rules in Rust, or in scripts.

The answers, and the modules that give them, are listed in the catalog,
[`docs/modules.md`](../modules.md). Pick a perspective and a kind of game
first: the guide for each says what starts it, what it uses, what to
leave alone, and which example to read. Most of the answers aren't built
yet. Their guides are *planned*: a short page that says what is coming,
which tickets build it (the engine's plan of work, in
[`tickets/`](../../tickets/)), and what not to use for it meanwhile.

`cargo shuyi modules`, in a game's folder, lists the same catalog with
the modules the game has on.

## Each guide

A guide starts with the answers it covers, as `concern/answer` ids from
[`docs/modules.ron`](../modules.ron), its status (*ready* or *planned*)
and its tickets. Then a direction guide says, in this order:

1. which template starts it;
2. which kits and modules it uses;
3. which ones look related but aren't for it, and why;
4. which example to read first.

A move guide says what the screen that changes stages carries across,
what each stage builds from it, and which example does it.

## The guides

Built:

- [Top-down grid, with turns](top-down-grid.md): tactics, board and
  puzzle games, where the player moves pieces tile by tile.

Planned, by perspective and world drawing:

- [Top-down, free](top-down-free.md): walk anywhere, not tile by tile.
- [Isometric](isometric.md): a grid seen at an angle.
- [Side view](side-view.md): platformers.
- [Hex grid](hex-grid.md): a grid of hexagons.
- [Dungeon crawler](dungeon-crawler.md): a labyrinth seen from the eyes,
  a cell at a time.
- [First person](first-person.md), [third person](third-person.md) and
  [top-down 3D](top-down-3d.md): the world in 3D.
- [HD-2D](hd-2d.md): 2D sprites in a 3D diorama.

Planned, by kind of game:

- [Real-time strategy](rts.md)
- [Visual novel](visual-novel.md)
- [Mystery](mystery.md)
- [Roguelike](roguelike.md)
- [Party RPG](party-rpg.md)
- [Action RPG](action-rpg.md)

Moves between directions, planned:

- [Top-down free to top-down grid](top-down-free-to-top-down-grid.md): a
  town you walk that starts a battle on a grid.

A move is supported when its guide and its example exist.
