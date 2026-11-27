+++
title = "Coming from another engine"
description = "The words of Godot, Unity, Bevy, GameMaker and PICO-8, and what each is in Shuyi Engine."
weight = 200

[extra]
part = "appendices"
appendix = "A"
+++

This page is for you if you've used another engine, and you'd like to know what its
words are called here. If you haven't, you can skip it: nothing else in the book needs it.

Most of another engine's ideas have a counterpart here, under another name and often in a
plainer shape. The engine has no *scene tree* (a hierarchy of objects, as in Godot) and
no *entity system* (game objects as loose bundles of data, as in Bevy) by default, and a
game's state is plain structs it owns.

{% newcomer(to="rust", topic="a struct") %}
A **struct** is Rust's named group of values.
{% end %}

The table gives Shuyi's word first, with the chapter that explains it, then the words of
five engines for it. A dash means that engine has no such word; "none yet" means Shuyi has
no counterpart today.

{{ engine_terms() }}

## What has no counterpart elsewhere

A few of Shuyi's ideas are rarer in other engines, and the chapters
that explain them are worth reading first:

- **Determinism.** The same seed and commands give the same game on
  every machine, so a record of them is the whole play
  ([5](@/learn/book/rules-as-commands.md), [8](@/learn/book/testing.md#records-and-replays)).
- **Headless first.** Every game runs, and is tested and pictured,
  without a window ([8](@/learn/book/testing.md)).
- **A view between the rules and the look.** Tests read the view, so a
  new look breaks no test ([2](@/learn/book/frame-and-skins.md)).
- **One contract for people and agents.** The tools answer the same
  shape on the command line, over MCP and in tests
  ([10](@/learn/book/tools-for-agents.md)).
