+++
title = "The tutorial"
description = "Make a whole game from nothing: a box-pushing puzzle, step by step. No programming experience needed."
sort_by = "weight"
template = "tutorial.html"
page_template = "tutorial-step.html"
+++

This tutorial takes you from nothing to a finished small game: a box-pushing
puzzle on a grid. You walk, you push a box, you put every box on a target,
you undo a mistake, and the levels come from files you can edit.

**You don't need to know how to program.** If you have never written a line
of code or made a game, you are who this was written for. Every word is
explained the first time it comes up, and the [glossary](@/learn/tutorial/glossary.md)
keeps them all in one place. If you already program, you can skim the parts
about Rust and read the parts about Shuyi.

Along the way you will learn three things at once:

- **How games work:** frames, screens, input, rules, and why a game keeps what
  it draws apart from what is true.
- **How Shuyi works:** the parts the engine gives you, and what each is for.
- **As much Rust as a Shuyi game needs,** the language the game is written in,
  each piece explained on the line of the game that uses it.

Each step ends with a game that runs, and you can play it in your browser from
that step's page, to see where you are going before you get there. Each step
also has pictures of what you should see, and things to try: make this color
blue, make this room twice as big, and see the picture of what happens. Trying
them is the fastest way to learn what each line does.

Every piece of code on these pages comes from that step's real files, and those
are built and tested each time the engine changes, as are the things to try. So
what you read here works.

Take your time. The whole tutorial is a few evenings of reading and typing.
[Start here](@/learn/tutorial/00-start.md).
