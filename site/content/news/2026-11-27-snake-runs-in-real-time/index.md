+++
title = "Snake runs in real time, and online play has started"
date = 2026-11-27
draft = false
description = "The engine's first real-time game, and the first pieces of networking for the next one."

[taxonomies]
tags = ["devlog"]

[extra]
author = "Nick"
image = "cover.png"
image_alt = "Snake: a run starts, and the snake turns down toward the bottom wall."
+++

Until now, every grid game on Shuyi Engine took turns. As of this week a
grid game can also run in real time, on fixed steps, and the first one is
**Snake**. You can [play it on the Examples page](@/examples/snake.md).

I picked Snake first because it's the natural next step from the book's
[box pusher](@/learn/book/box-pusher/_index.md): it's still a grid game,
so it adds only one new idea, real time. That's the plan for the
examples from here on. Each one builds on the ones before it and adds a
little more, until the engine can do something like a real-time 3D MMO.

In Snake, the snake moves one tile each time its timer fires, and speeds
up every five points. Because the game steps in fixed ticks and records
its inputs, any run can be replayed exactly. The same game builds for the
web and for the Playdate.

## Next: online play

The next example is **trails**, an original light-trail arena for up to
eight riders. I'm adding online play this early on purpose: it's hard to
bolt onto an engine later, so I'd rather build it in from the start.

Trails plays offline against bots first, then online. Online, a server
runs the game and players send it their inputs. Each room has eight
seats, and bots ride every empty one, so you can always jump straight in
and leave whenever you like. "Play online" picks a room for you. There
are no accounts, names or chat, and the server is meant to run on a free
hosting tier.

For a game made on the engine, it should look like this: the game's
rules stay the same pure, fixed-step rules Snake uses, and the engine
carries each seat's inputs between the players and the server. The
network itself can be swapped out. In tests it's an in-memory network
that can lose, duplicate and reorder messages, so a game's online play
can be checked without a real network.

Those first pieces are in: room inputs for up to eight seats, the
swappable network, and the in-memory one with its faults. Rooms, the
server, the browser and desktop clients, and trails itself come next.
Follow along on the [roadmap](https://github.com/Zafnok/shuyi-engine/blob/main/docs/ROADMAP.md),
or in the feed.
