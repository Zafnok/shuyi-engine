# {{name}}: private files

This repository holds what {{name}} may not hand out with its public
code: its writing, art and sounds, its story and design documents, and
the packs bought for it. **It must stay private.** The game is
{{game}}; its builds read this repository with a read-only deploy key.

## The folders

| Folder | Holds |
| ------ | ----- |
| `files/` | Files at the path they have in the game's files folder (`files/sprites/hero.png` is `crates/game/files/sprites/hero.png`). The game's build lays them over its own files: a file here replaces the public one at its path, and a file only here is added. Every file in it ships inside the game, so nothing else goes there: no notes, no README. |
{{kinds}}
| `docs/` | The story and design documents. No build reads them. |
| `library/` | The packs bought for the game, as they came, sorted by who made them. Only `{{command}} private-assets --library` brings it. |

An empty folder holds a `.gitkeep`, so that git keeps it: delete it once
the folder has files.

## Changing a file

1. In the game's folder, `{{command}} private-assets` checks this
   repository out into `private/`. Make the change there, on a branch.
2. Open a pull request here, and merge it into `main`.
3. In the game, run `{{command}} private-assets --pin`: it moves the
   game's `private.rev` to this repository's new commit. Open the
   game's pull request with that change.

The game builds with the commit its `private.rev` names, so a change
here reaches the game only through that second pull request.
