// The system's languages for the web build: a miniquad plugin over the
// browser's navigator.languages (else navigator.language), most preferred
// first. This file is Shuyi Engine's own code, unlike the vendored plugins
// next to it.
//
// The Rust side is crates/app/src/languages/web.rs, which declares exactly
// the `shuyi_lang_*` functions below (a test of shuyi-cli compares the two
// lists) and hands the tags to the game, which picks its first language
// from them. A tag is ASCII, so it crosses one character code at a time.
(function () {
    "use strict";

    // Keep equal to PLUGIN_VERSION in crates/app/src/languages/web.rs.
    var VERSION = 1;

    // The languages read by the last shuyi_lang_count().
    var tags = [];

    // The browser's languages, most preferred first; none if it says none.
    function read() {
        var found = [];
        try {
            var all = navigator.languages;
            if (all && all.length) {
                for (var i = 0; i < all.length; i++) {
                    if (typeof all[i] === "string") {
                        found.push(all[i]);
                    }
                }
            } else if (typeof navigator.language === "string") {
                found.push(navigator.language);
            }
        } catch (e) {
            return [];
        }
        return found;
    }

    function register_plugin(importObject) {
        // Reads the browser's languages; returns how many.
        importObject.env.shuyi_lang_count = function () {
            tags = read();
            return tags.length;
        };
        // The character code at j of language i; 0 past its end.
        importObject.env.shuyi_lang_char = function (i, j) {
            var tag = tags[i];
            if (typeof tag !== "string" || j >= tag.length) {
                return 0;
            }
            return tag.charCodeAt(j);
        };
    }

    miniquad_add_plugin({
        register_plugin: register_plugin,
        name: "shuyi_languages",
        version: VERSION
    });
})();
