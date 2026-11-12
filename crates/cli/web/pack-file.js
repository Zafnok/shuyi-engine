// The pack beside the page: a miniquad plugin that hands the game the pack
// its release export baked (`<package>.pack`, beside `index.html`), fetched
// once before the game starts, so the game reads it at the same point as a
// desktop program reads the file beside it. This file is Shuyi Engine's own
// code, unlike the vendored plugins next to it.
//
// The Rust side is crates/app/src/pack_file.rs, which declares exactly the
// `shuyi_pack_file_*` functions below (a test of shuyi-cli compares the two
// lists). Bytes and strings cross as `sapp_jsutils.js`'s objects
// (`js_object`), so that file loads first.
//
// `index.html` calls `shuyi_fetch_pack(name, start)` in place of starting the
// game itself: with no name (a build that embeds its files) it starts at
// once; otherwise once the fetch is done, whatever came of it. A game that
// asks for a pack the page has none of hears why.
(function () {
    "use strict";

    // Keep equal to PLUGIN_VERSION in crates/app/src/pack_file.rs.
    var VERSION = 1;

    // The pack's bytes, once fetched.
    var bytes = new Uint8Array(0);

    // Why there are none: "" once the pack is fetched.
    var problem = "the page fetched no pack: this build embeds its files";

    // Fetches the pack `name` beside the page, then calls `start`.
    window.shuyi_fetch_pack = function (name, start) {
        if (!name) {
            start();
            return;
        }
        problem = name + " is still being fetched";
        fetch(name)
            .then(function (response) {
                if (!response.ok) {
                    throw new Error(name + ": HTTP " + response.status);
                }
                return response.arrayBuffer();
            })
            .then(function (buffer) {
                bytes = new Uint8Array(buffer);
                problem = "";
            }, function (error) {
                problem = String(error && error.message ? error.message : error);
            })
            .then(start);
    };

    function register_plugin(importObject) {
        // The pack's bytes; empty when the page has none.
        importObject.env.shuyi_pack_file_bytes = function () {
            return js_object(bytes);
        };
        // Why the page has no pack; "" when it has one.
        importObject.env.shuyi_pack_file_problem = function () {
            return js_object(problem);
        };
    }

    miniquad_add_plugin({
        register_plugin: register_plugin,
        name: "shuyi_pack_file",
        version: VERSION
    });
})();
