// Files the player keeps, on the web, which has no files of its own: a
// miniquad plugin that offers the browser a download, and opens its file
// picker for a file the game asks for. This file is Shuyi Engine's own code,
// unlike the vendored plugins next to it.
//
// The Rust side is crates/app/src/player_files/web.rs, which declares exactly
// the `shuyi_*` functions below (a test of shuyi-cli compares the two lists).
// Strings and bytes cross as `sapp_jsutils.js`'s objects (`js_object`,
// `get_js_object`), so that file loads first.
//
// A browser opens a picker only shortly after the player pressed something
// (transient user activation, about 5 seconds): the game asks in the frame of
// the press. The answer is kept until the game polls for it.
(function () {
    "use strict";

    // Keep equal to PLUGIN_VERSION in crates/app/src/player_files/web.rs.
    var VERSION = 2;

    // The answer to the open ask, as `shuyi_file_answer` gives it: `kind` 0
    // while the player chooses, 1 picked (`name`, `bytes`), 2 cancelled, 3
    // refused (`reason`).
    var answer = { kind: 0 };

    // Each ask's number: an answer to an earlier one is dropped.
    var asks = 0;

    function refuse(reason) {
        answer = { kind: 3, reason: reason };
    }

    // Opens a hidden file input for `accept`, answering once the player has
    // chosen or closed it. A file over `max_bytes` is refused unread.
    function ask(accept, max_bytes) {
        var number = ++asks;
        var input = document.createElement("input");
        input.type = "file";
        input.accept = accept;
        input.style.display = "none";
        var done = function () {
            input.remove();
            return number === asks;
        };
        input.addEventListener("cancel", function () {
            if (done()) {
                answer = { kind: 2 };
            }
        });
        input.addEventListener("change", function () {
            var file = input.files && input.files[0];
            if (!done()) {
                return;
            }
            if (!file) {
                answer = { kind: 2 };
            } else if (file.size > max_bytes) {
                refuse(file.name + " was refused: " + file.size + " bytes, more than " + max_bytes);
            } else {
                file.arrayBuffer().then(function (buffer) {
                    if (number === asks) {
                        answer = { kind: 1, name: file.name, bytes: new Uint8Array(buffer) };
                    }
                }, function (error) {
                    if (number === asks) {
                        refuse(file.name + " couldn't be read: " + error);
                    }
                });
            }
        });
        document.body.appendChild(input);
        input.click();
    }

    function register_plugin(importObject) {
        // Offers a file called `name` holding `text`, as a download: 1 once
        // the browser was handed it, 0 if it threw.
        importObject.env.shuyi_offer_file = function (name, text) {
            try {
                var blob = new Blob([get_js_object(text)], { type: "text/plain" });
                var url = URL.createObjectURL(blob);
                var link = document.createElement("a");
                link.href = url;
                link.download = get_js_object(name);
                document.body.appendChild(link);
                link.click();
                link.remove();
                setTimeout(function () {
                    URL.revokeObjectURL(url);
                }, 0);
                return 1;
            } catch (e) {
                console.warn("shuyi_offer_file: " + e);
                return 0;
            }
        };
        // Opens the file picker for the extensions in `accept` (".ron,.txt";
        // any when ""): 1 once it opened, 0 when the browser won't (no recent
        // press, where it says so).
        importObject.env.shuyi_ask_for_file = function (accept, max_bytes) {
            var activation = navigator.userActivation;
            if (activation && !activation.isActive) {
                return 0;
            }
            answer = { kind: 0 };
            try {
                ask(get_js_object(accept), max_bytes);
                return 1;
            } catch (e) {
                console.warn("shuyi_ask_for_file: " + e);
                return 0;
            }
        };
        // The answer to the open ask; once given, the next poll waits again.
        importObject.env.shuyi_file_answer = function () {
            var given = answer;
            if (given.kind !== 0) {
                answer = { kind: 0 };
            }
            return js_object(given);
        };
    }

    miniquad_add_plugin({
        register_plugin: register_plugin,
        name: "shuyi_player_files",
        version: VERSION
    });
})();
