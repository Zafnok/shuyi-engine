# Third-party assets

Every file in this repository that isn't the engine's own and isn't a Rust
crate: fonts, vendored JavaScript, data compiled in. Rust crates are
checked by `cargo deny` (`deny.toml`). Only the permissive licenses of
ADR-0002 §8 are allowed (`AGENTS.md`, rule 7), and each item's license
text is committed next to it.

A game made with the engine ships some of these too. `shuyi web` writes
the web shell's JavaScript, with its license texts, into the
`NOTICES.txt` beside the page, with the crates compiled in and the
game's own texts (its `shuyi.ron`'s `notices`). `shuyi export` writes one
into every package, and a native one lists the controller database
`gilrs` compiles in. The files the engine's crates ship are data too,
`shuyi_cli::notices::FILES`, which a test keeps equal to the rows here
whose license file is under `crates/`.

| Item | Source URL | Version | License | License file | Used for | Added by |
| ---- | ---------- | ------- | ------- | ------------ | -------- | -------- |
| Terminus Font (`ter-u16n`, 8×16): the BDF at `examples/hello-grid/files-src/ter-u16n.bdf`, converted to `examples/hello-grid/files/font/atlas.png` (and its one-bit copy) by `cargo shuyi font-atlas`, and its glyphs stamped into `examples/hello-grid/files/tiles.png` by `cargo shuyi glyph-tiles` | https://terminus-font.sourceforge.net/ | 4.49.1 | OFL-1.1 | [`examples/hello-grid/files/font/Terminus-LICENSE.txt`](examples/hello-grid/files/font/Terminus-LICENSE.txt) | Hello, Grid's only font, and its tileset's glyphs (the rest of the tileset is the example's own palette colors). Unmodified | game 1118 |
| `crates/cli/web/mq_js_bundle.js` (macroquad's JS/WASM loader) | https://github.com/not-fl3/macroquad/blob/5e9b5ca912ac65962c05c0da842a4a70eaae34b9/js/mq_js_bundle.js | 0.4.16 (commit `5e9b5ca9`; no matching git tag, see `crates/cli/web/README.md`) | MIT | [`crates/cli/web/mq_js_bundle-LICENSE-MIT.txt`](crates/cli/web/mq_js_bundle-LICENSE-MIT.txt) | Loads and runs the WASM binary in the web build (`cargo shuyi web`) | game 0206 |
| `crates/cli/web/quad-storage.js` (miniquad `localStorage` JS plugin) | https://github.com/optozorax/quad-storage/blob/3760b953aec17d65cc4ca8edfa39c38e7337ec3a/js/quad-storage.js | 0.1.3 (commit `3760b953`, with a local `version` patch, see `crates/cli/web/README.md`) | MIT | [`crates/cli/web/quad-storage-LICENSE-MIT.txt`](crates/cli/web/quad-storage-LICENSE-MIT.txt) | Backs `shuyi-app`'s storage on the web | game 0207 |
| `crates/cli/web/sapp_jsutils.js` (JS↔Rust marshalling `quad-storage.js` needs) | https://github.com/not-fl3/sapp-jsutils, shipped inside the `sapp-jsutils` crate's `js/sapp_jsutils.js` | 0.1.7 | MIT (no upstream `LICENSE` file; standard MIT text reconstructed from the crate's declared license, see `crates/cli/web/README.md`) | [`crates/cli/web/sapp_jsutils-LICENSE-MIT.txt`](crates/cli/web/sapp_jsutils-LICENSE-MIT.txt) | Backs `shuyi-app`'s storage on the web | game 0207 |
| SDL_GameControllerDB (`gamecontrollerdb.txt`), compiled into native builds by the `gilrs` crate | https://github.com/mdqinc/SDL_GameControllerDB, shipped inside the `gilrs` crate (`SDL_GameControllerDB/`) | The copy in `gilrs` 0.11.2 | Zlib | [`crates/app/SDL_GameControllerDB-LICENSE.txt`](crates/app/SDL_GameControllerDB-LICENSE.txt), copied from the crate's `SDL_GameControllerDB/LICENSE` (the `gilrs` crate itself, MIT / Apache-2.0, is covered by `cargo deny`) | `shuyi-app`'s controllers: maps PlayStation, Switch and generic controllers to standard button positions (game ADR-0034) | game 0219 |
| Terminus Font (`ter-u16n`, 8×16): its glyphs in Hello, Grid's tileset drawn in black and white for the Playdate, `examples/hello-grid/files@playdate/tiles.png`, taken from `examples/hello-grid/files/tiles.png` | https://terminus-font.sourceforge.net/ | 4.49.1 | OFL-1.1 | [`examples/hello-grid/files/font/Terminus-LICENSE.txt`](examples/hello-grid/files/font/Terminus-LICENSE.txt) | The pieces' and the goal's glyphs on the Playdate's tileset; the floor and wall tiles are the example's own. The glyphs drawn black | 3007 |
| Terminus Font (`ter-u16n`, 8×16), the atlas made from it: a copy of Hello, Grid's `atlas.png` and `atlas.ron` at `templates/shuyi-default/base/crates/game/files/font/` | https://terminus-font.sourceforge.net/ | 4.49.1 | OFL-1.1 | [`templates/shuyi-default/base/crates/game/files/font/Terminus-LICENSE.txt`](templates/shuyi-default/base/crates/game/files/font/Terminus-LICENSE.txt) | The font of every game `shuyi new` makes, written into it with its license (its README names it). Unmodified | 1129 |
| Terminus Font (`ter-u16n`, 8×16), the atlas made from it: a copy of the template's `atlas.png` and `atlas.ron` at `site/snippets/files/font/` | https://terminus-font.sourceforge.net/ | 4.49.1 | OFL-1.1 | [`site/snippets/files/font/Terminus-LICENSE.txt`](site/snippets/files/font/Terminus-LICENSE.txt) | The font of the book's game (`site/snippets/`), which CI builds and tests; never shipped. Unmodified | 2802 |
