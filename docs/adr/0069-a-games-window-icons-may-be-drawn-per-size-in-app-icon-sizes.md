# ADR-0069: A game's window icons may be drawn per size, in `app.icon_sizes`

- **Status:** Accepted; extends ADR-0061
- **Date:** 2026-11-01
- **Related tickets:** 3113 (this), 3072 (the icon), 3049 (the packages'
  sizes)

## Context

- **ADR-0061** makes every window icon by box-filtering one square picture
  of 512 pixels or more, and says pictures drawn per size wait for a target
  that needs them.
- **A pixel-art game needs them.** A line one source pixel wide at the small
  size turns to mud. Measured on one game's icon, a font glyph on a square:
  the 64 pixel picture came out identical, while 64 of the 256 pixels differ
  at 16 and 133 of the 1024 at 32, and the glyph can't be read at 16. A
  farming game whose icon is its 16 pixel sprite wants that sprite in the
  title bar.
- **No one source gives a drawn set.** The box filter's 16 pixel picture is
  the mean of its 32 pixel one, and pictures drawn per size aren't means of
  each other.

## Decision

1. **The field.** `shuyi.ron`'s `app.icon_sizes`
   (`shuyi_cli::project::AppPart`) is a map from a side in pixels to a PNG
   file, empty by default:
   `icon_sizes: { 16: "art/icon-16.png", 32: "art/icon-32.png" }`.
2. **A drawn picture replaces the downscale at its side.**
   `app_info::files` writes that side's window icon as the file's pixels,
   re-encoded with `codec::encode_png`, and downscales the other sides from
   `icon` as ADR-0061 says. The output is still a generated file in
   `window_icons`, so `stale`, `--check` and `check --target` cover it, and
   the generated Rust doesn't change. A drawn picture is never scaled to
   another size.
3. **The rules**, each an error `app.icon_sizes`, in `app_info::problems`
   and so in `check --target`, and stopping `app-info` before it writes:
   - a file that doesn't read or decode, or isn't square at its key's side;
   - a key that isn't one of `WINDOW_ICON_SIDES`: nothing would read it;
   - a non-empty map without `icon` or without `window_icons`: the sizes it
     doesn't draw come from `icon`, and without the folder nothing is
     written, so the pictures would be silently ignored.
4. **Only the window.** The packages' sizes (3049) keep downscaling from
   `icon`. `WindowIcon` and `AppInfo` don't change.

## Consequences

- A pixel-art game's window shows its icon crisp at 16 and 32 pixels.
- A project without the field writes the same bytes as before, and `AppPart`
  is `#[non_exhaustive]`, so the API doesn't break (ADR-0039).
- A game with drawn pictures still needs a large `icon`, even when it draws
  every window size, because a package asks for sizes the map can't hold.
- Re-encoding means the generated file isn't the drawn file's bytes, only its
  pixels; an optimized source PNG gives the same output as a plain one.

## Alternatives considered

- **Copying the drawn file's bytes** — its size and chunks depend on the tool
  that saved it, and every other window icon is `encode_png`'s output.
- **Dropping `icon` when all three sizes are drawn** — the packages would have
  no source, and the rule would depend on which keys are set.
- **Ignoring a key that isn't a window size** — a game that draws 128 would
  believe it is used. If a later target reads more sizes, that key stops being
  an error, which breaks no game.
- **A sharper or nearest-neighbour filter** — the box filter's bytes must stay
  the same everywhere (ADR-0061), and no filter recovers a picture drawn for
  its size.
