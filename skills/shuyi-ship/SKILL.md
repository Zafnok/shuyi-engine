---
name: shuyi-ship
description: Get a game on Shuyi Engine ready to ship. A package per target (`cargo shuyi export <target>`); the web build (what a new game lacks for it, then `cargo shuyi web --release`); the targets it ships on and their controls (`check --target`); a baked pack; and the credits of every third-party work it uses. Use when building a game for players or the web, adding a target such as the Steam Deck, writing its credits, or making pictures for a store page. Not for the checks before a commit (shuyi-gates) or a new game (shuyi-new-game).
license: MIT OR Apache-2.0
metadata:
  version: "1.0"
---

# Ship a game

A game made with `shuyi new` builds for the desktop as it is. Its
`files/` folder is embedded in the binary, so a release build is one
program. The web build and each other target need a few lines more.
Ship only what `check --target` passes.

## When to use it

- Building the game for players: a release build, or the web.
- Adding a target to the ones it ships on (the Steam Deck, the web).
- Writing the credits of a font, a picture or vendored code.
- Pictures for a store page or a README.

## When not to

- The checks before every commit: `shuyi-gates`.
- Controls and key names: `shuyi-input` (this skill only adds a
  target's buttons).
- Starting a game: `shuyi-new-game`.

## Steps

1. **Pass the gates first** (`shuyi-gates`), then check every target
   `shuyi.ron` lists:

   ```bash
   cargo shuyi check --target all
   ```

   For each target it checks that the frame fits its display at the
   game's scale, the smallest text is tall enough, and every screen's
   required actions fit its buttons. A problem names the setting to
   change (`targets` in `shuyi.ron`, or the keymap's `targets`).

2. **A package per target**, from the game's folder:

   ```bash
   cargo shuyi export windows --release
   ```

   It runs the target's check first (`windows`, `linux` and `macos`
   check the `desktop` profile) and builds nothing on an error. Then it
   writes the package into `dist/export/<target>/`: `windows` a zip,
   `linux` a `tar.gz`, `macos` a folder with one `.app` for Apple
   silicon and Intel Macs, signed ad hoc, and `web` the web build's
   folder and its zip (step 3 first). Each desktop target builds on its
   own system, and a missing Rust target says its `rustup target add`.
   - Every package carries `NOTICES.txt` (step 3), and the `windows`
     and `macos` ones `HOW-TO-OPEN.txt`: the packages aren't signed by a
     developer, so players open them once by hand.
   - `--release` remaps the build's paths and refuses a program that
     still holds one of this machine's. On `windows` and `linux` it
     keeps `<package>.debug` beside the package: keep it with the
     release for crash reports, and never ship it.
   - `export-report.json` beside it gives the version, the sizes, the
     check's warnings and the signing; `--run` runs the package, on its
     own system.

   A release build on Windows opens no console window.

3. **The web build.** A new game lacks three things for it:
   - a `web` entry in `shuyi.ron`: the game's package and binary (its
     crate's name), the page's title and the folder it goes in:

     ```ron
     web: (
         package: "moon-garden",
         bin: "moon-garden",
         title: "Moon Garden",
         dist: "dist/web",
     ),
     ```

     `music` and `voice` name folders of `.ogg` tracks and voice clips
     copied beside the page, and `features` the package's features a
     `--<feature>` flag turns on;
   - `crates/game/build.rs`, so the binary links with the page's JS
     plugins (storage, controllers), which the page gives it when it
     runs:

     ```rust
     fn main() {
         if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
             println!("cargo:rustc-link-arg=--allow-undefined");
         }
     }
     ```

     Never in `.cargo/config.toml`'s `rustflags`: a `RUSTFLAGS` variable
     (CI sets one) replaces those, and the flag is lost.
   - `targets = ["wasm32-unknown-unknown"]` in `rust-toolchain.toml`.
     `cargo shuyi doctor` says whether the web target is installed.

   Then:

   ```bash
   cargo shuyi web --release
   ```

   It writes the page, its loaders and `<bin>.wasm` into `dist/web/`
   (which `.gitignore` leaves out), and prints the folder and the
   binary's size. With `wasm-opt` on `PATH` a release build is made
   smaller too. Serve the folder over HTTP to play it: a page opened as
   a file doesn't load the binary.

   Beside the page it writes `NOTICES.txt`, which the build owes the
   works in it: the crates compiled in, the page's scripts and the
   game's own license texts, then each license text once. Publish it
   with the page. The game names its own texts (its font's, any bought
   art's) in `shuyi.ron`, each file put in whole; a missing one fails
   the build:

   ```ron
   notices: (texts: ["crates/game/files/font/Terminus-LICENSE.txt"]),
   ```

   A crate whose license text can't be found is still listed, and the
   build prints a warning naming it.

4. **Another target.** Add it to `shuyi.ron`'s `targets`, with its frame
   in pixels, its scale (`integer` or `fit`) and the height of its
   smallest text. A target with no keyboard (the Steam Deck, the
   Playdate) needs its buttons in `files/keymap.ron`'s `targets`, by its
   id, an action to buttons as the `pad` table writes them:

   ```ron
   targets: {
       "steam-deck": {
           "Up": ["DpadUp", "LeftStickUp"],
           "Confirm": ["South"],
           "Cancel": ["East"],
       },
   },
   ```

   Every action a screen requires needs a button, each its own. Run
   `cargo shuyi check --target all` again. The engine's profiles name
   each target's display, buttons and status; a target whose platform
   backend the engine doesn't have yet is checked, never built.

5. **The credits.** Write every third-party work the game uses (a font,
   a picture, vendored code) in a credits file, `files/credits.ron`, and
   read it back:

   ```bash
   cargo shuyi import crates/game/files/credits.ron
   ```

   A game made with `shuyi new` uses one, the Terminus font (its license
   is beside it, in `files/font/`). The worked example's
   [`credits.ron`](example/credits.ron) credits it. Music and sound
   effects are credited in the game's audio manifest instead. A credits
   screen of the game's own reads the file (`shuyi::assets::credits`:
   `merge` makes one list of the file and the audio manifest).

6. **A pack**, if the game ships its files baked into one (`.shuyipak`):
   its tools give `GameTools::bake`, and

   ```bash
   cargo shuyi bake --target web --out dist/game.shuyipak
   cargo shuyi check --baked
   ```

   say what went in and prove the pack equal to its sources. A game made
   with `shuyi new` has no pack, and `bake` says it isn't offered
   (`SH3001`).

7. **Pictures for a store page** come from the game with no window:

   ```bash
   cargo shuyi frame-png shot.png --keys "Enter" --scale 3
   ```

   The store pages themselves, their text and their prices are the
   game's owner's.

## Licenses

Every work the game ships, code or not, must be under a license that
lets it be shipped. A credit's `license` is one of the engine's list
(`shuyi::assets::credits::LICENSES`: MIT, Apache-2.0, the BSD licenses,
CC0, the OFL and the like), or `Custom (<seller>)` for a bought work,
which names the files it covers. A credit the check refuses is never
made to pass by changing its license or by adding a license to the list:
whether the game may use a work is its owner's call. Ask the owner, or
take the work out.

## When it fails

Branch on the answer's `code`, never its message, and read its page
with `cargo shuyi explain <code>`:

- `check --target` answers `ok: false` with no code: each target's
  `problems` has its `check` (`fits`, `text`, `controls`, `one-bit`),
  its `setting` and its message. Change that setting; a `warning` doesn't
  fail it.
- `SH3003`: no such target in `shuyi.ron`'s `targets`, or the target
  profiles don't load. The message lists the ids there are.
- `SH1002`: `shuyi.ron` has no `web` entry, or lacks another part a
  command needs.
- `SH3001`: the game's tools don't offer the operation (`bake` without
  a pack).
- `export`: `SH3008`, the target's check found errors (listed); `SH3009`,
  the program holds a path of this machine (code writes one in as a
  value, such as `env!("CARGO_MANIFEST_DIR")`); `SH3010`, no exporter or
  no run step for that target yet; `SH3011`, the build or a packaging
  tool failed; `SH1003`, a Rust target or tool is missing.
- `import` answers `ok: false` with each error's file and line (and its
  code when its class has one): a credit with an empty field, a source
  that isn't a web link, a license not on the list.

## Worked example

[`example.ron`](example.ron) runs on a new `grid` game: `check --target
all` passes; the Steam Deck added to its targets fails it, with the
controls' problem; a button table in the keymap fixes it. Then the web
entry, `build.rs` and the web target, `cargo shuyi web --release`, the
page and the binary in `dist/web/`, a credits file that `import` reads,
and the gates still passing.
