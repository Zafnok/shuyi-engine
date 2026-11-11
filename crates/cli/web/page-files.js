// Files from the page: a miniquad plugin that hands the game the text files
// its page holds, which it lays over its own, and sends the page what was
// wrong with them. This file is Shuyi Engine's own code, unlike the vendored
// plugins next to it.
//
// The Rust side is crates/app/src/page.rs, which declares exactly the
// `shuyi_page_*` functions below (a test of shuyi-cli compares the two
// lists). Strings cross as `sapp_jsutils.js`'s objects (`js_object`,
// `get_js_object`), so that file loads first.
//
// The page writes each file to `sessionStorage` under
// `shuyi-page-file:<path>`: only a page of the same origin can, it lives as
// long as the tab, it survives a reload (a page runs the game again by
// reloading its frame), and a read is synchronous, so the game has its files
// before its first frame. Problems go back with `postMessage` to the parent
// frame, at the page's own origin.
(function () {
    "use strict";

    // Keep equal to PLUGIN_VERSION in crates/app/src/page.rs.
    var VERSION = 1;

    // The start of every key that holds a file of the page.
    var PREFIX = "shuyi-page-file:";

    // The keys found by the last shuyi_page_file_count(), in key order.
    var keys = [];

    // `text` as UTF-16 that encodes to valid UTF-8: a lone surrogate, which
    // storage may hold, becomes U+FFFD, as Rust's strings need.
    function well_formed(text) {
        return new TextDecoder().decode(new TextEncoder().encode(text));
    }

    // Whether the page runs in a frame whose parent has its origin. Reading
    // another origin's location throws.
    function in_own_frame() {
        try {
            return window.parent !== window && window.parent.location.origin === window.location.origin;
        } catch (e) {
            return false;
        }
    }

    function register_plugin(importObject) {
        // Reads the keys of the page's files; returns how many. The other
        // functions index into that list until the next call. 0 where
        // sessionStorage throws (a page with storage turned off).
        importObject.env.shuyi_page_file_count = function () {
            keys = [];
            try {
                var storage = window.sessionStorage;
                for (var i = 0; i < storage.length; i++) {
                    var key = storage.key(i);
                    if (key !== null && key.startsWith(PREFIX)) {
                        keys.push(key);
                    }
                }
            } catch (e) {
                // Storage turned off, or gone mid-read: the page has no files
                // for the game, which is not an error for it. Drop any keys
                // read before the throw rather than hand over half a list.
                keys = [];
            }
            // Code-unit order, the same in every browser and locale, so the
            // game reads the files, and names the first one wrong, the same
            // way everywhere; `localeCompare` would make that the reader's.
            keys.sort(function (a, b) {
                if (a < b) {
                    return -1;
                }
                return a > b ? 1 : 0;
            });
            return keys.length;
        };
        // File i's path, after the prefix.
        importObject.env.shuyi_page_file_path = function (i) {
            var key = keys[i] === undefined ? PREFIX : keys[i];
            return js_object(well_formed(key.slice(PREFIX.length)));
        };
        // File i's text; "" if it went meanwhile.
        importObject.env.shuyi_page_file_text = function (i) {
            var text = null;
            try {
                text = keys[i] === undefined ? null : window.sessionStorage.getItem(keys[i]);
            } catch (e) {
                text = null;
            }
            return js_object(well_formed(text === null ? "" : text));
        };
        // Sends the page the text of the game's problems with its files, one
        // a line; "" for none. With no parent frame of the page's origin, the
        // console has it.
        importObject.env.shuyi_page_report = function (text) {
            var message = get_js_object(text);
            if (in_own_frame()) {
                window.parent.postMessage({ kind: "shuyi-report", text: message }, window.location.origin);
            } else {
                console.info(message);
            }
        };
    }

    miniquad_add_plugin({
        register_plugin: register_plugin,
        name: "shuyi_page_files",
        version: VERSION
    });
})();
