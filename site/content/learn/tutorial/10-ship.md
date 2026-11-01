+++
title = "Shipping it"
description = "Check it against its targets, build it for the web, and put it on Pages."
weight = 100

[extra]
step = 10
+++

The game is finished. Now hand it out.

## Who it is

A new game's id is a placeholder, and two games with one id share their
saves. Set your own in `shuyi.ron`'s `app`, before you hand out a build and
never after:

{{ snippet(step="10-ship", file="shuyi.ron", anchor="id", language="ron") }}

Then write it into the code:

```bash
cargo shuyi app-info
```

## Check it

`check --target` holds the game to each target in `shuyi.ron`: that its
frame fits, that its text can be read, and that every screen's controls
exist there.

```bash
cargo shuyi check --target all
cargo shuyi lint
cargo test --workspace
```

## Build it

```bash
cargo shuyi web --release
```

`dist/web/` now holds everything: the page, the game and `NOTICES.txt`,
which must ship with it. Try it through any local web server, since a
browser won't load the game from a plain file:

```bash
python -m http.server --directory dist/web
```

## Put it on Pages

The folder is a static site, so any host works. On GitHub, publish
`dist/web/` from your repository with Pages (*Settings*, then *Pages*), by
a workflow that builds it or by pushing the folder to a branch. Everything
in it loads by a relative path, so it works from the address Pages gives
your repository.

[Chapter 9](@/learn/book/targets-and-export.md) covers the other targets,
and how `cargo shuyi export` packages a game for desktop.

## Where next

You used turn rules, a grid, actions, assets, history, menus, saves, the
Harness and the web build. To go further, change the levels, add a sixth,
or give a level a second kind of tile. The [book](@/learn/book/_index.md)
has the rest of what the engine can do.
