// The pack beside the page: a miniquad plugin that streams the game the pack
// its release export baked (`<package>.pack`, beside `index.html`), in the
// order the bake laid it (ADR-0128 §2, §3 and §6). The page fetches the
// pack and the `.wasm` together, starts the game once the pack's header and
// index have arrived, and hands the game the rest as it comes, keeping none
// of it once handed over. This file is Shuyi Engine's own code, unlike the
// vendored plugins next to it.
//
// The Rust side is crates/app/src/pack_file.rs, which declares exactly the
// `shuyi_pack_file_*` functions below (a test of shuyi-cli compares the two
// lists). Strings cross as `sapp_jsutils.js`'s objects (`js_object`), so
// that file loads first; bytes are written straight into the game's memory.
//
// `index.html` calls `shuyi_fetch_pack(name, start)` in place of starting the
// game itself: with no name (a build that embeds its files) it starts at
// once; otherwise once the pack's header and index have arrived, or the
// fetch has ended or failed, whatever came of it. A GET streamed with
// `response.body.getReader()` needs no `Range`, so it works on every host,
// Python's `http.server` included; one that answers without a body stream
// gives the whole pack at once. A game that asks for a pack the page has none
// of, or whose stream breaks, hears why.
(function () {
    "use strict";

    // Keep equal to PLUGIN_VERSION in crates/app/src/pack_file.rs.
    var VERSION = 2;

    // A pack's header: its bytes 20 to 24 are the index's length, which
    // follows it (crates/assets/src/pack.rs).
    var HEADER = 24;

    // The chunks the page received and hasn't handed over, oldest first,
    // and how many bytes they hold.
    var queue = [];
    var queued = 0;

    // Whether the whole body has been received (the queue may still hold
    // some of it).
    var ended = false;

    // Why the page has no pack, or why its stream broke; "" while it is fine.
    var problem = "the page fetched no pack: this build embeds its files";

    // Copies the queue's first bytes into `into`, as many as it holds, and
    // drops them from the queue if `drop`: how many it copied.
    function copy(into, drop) {
        var done = 0;
        var at = 0;
        while (done < into.length && at < queue.length) {
            var chunk = queue[at];
            var count = Math.min(chunk.length, into.length - done);
            into.set(count === chunk.length ? chunk : chunk.subarray(0, count), done);
            done += count;
            if (!drop) {
                at += 1;
            } else if (count === chunk.length) {
                queue.shift();
            } else {
                // A copy of the rest, so the chunk's buffer goes.
                queue[0] = chunk.slice(count);
            }
        }
        if (drop) {
            queued -= done;
        }
        return done;
    }

    // How many bytes the pack's header and index take, once its header has
    // arrived.
    function needs() {
        if (queued < HEADER) {
            return Infinity;
        }
        var head = new Uint8Array(HEADER);
        copy(head, false);
        var length = head[20] + head[21] * 256 + head[22] * 65536 + head[23] * 16777216;
        return HEADER + length;
    }

    // Fetches the pack `name` beside the page, and calls `start` once its
    // header and index have arrived, or the fetch has ended or failed.
    window.shuyi_fetch_pack = function (name, start) {
        if (!name) {
            start();
            return;
        }
        problem = "";
        var started = false;
        function begin() {
            if (!started) {
                started = true;
                start();
            }
        }
        function arrive(bytes) {
            if (bytes.length > 0) {
                queue.push(bytes);
                queued += bytes.length;
            }
            if (queued >= needs()) {
                begin();
            }
        }
        fetch(name)
            .then(function (response) {
                if (!response.ok) {
                    throw new Error("HTTP " + response.status);
                }
                if (!response.body || !response.body.getReader) {
                    return response.arrayBuffer().then(function (buffer) {
                        arrive(new Uint8Array(buffer));
                    });
                }
                var reader = response.body.getReader();
                function next() {
                    return reader.read().then(function (part) {
                        if (part.done) {
                            return undefined;
                        }
                        arrive(part.value);
                        return next();
                    });
                }
                return next();
            })
            .then(function () {
                ended = true;
            }, function (error) {
                problem = name + ": " + String(error && error.message ? error.message : error);
            })
            .then(begin);
    };

    function register_plugin(importObject) {
        // Writes the next bytes of the pack the page received, up to
        // `capacity`, at `buffer` in the game's memory, and drops them: how
        // many it wrote.
        importObject.env.shuyi_pack_file_take = function (buffer, capacity) {
            if (queued === 0 || capacity === 0) {
                return 0;
            }
            return copy(new Uint8Array(wasm_memory.buffer, buffer, capacity), true);
        };
        // 1 once the whole body has been received, else 0.
        importObject.env.shuyi_pack_file_ended = function () {
            return ended ? 1 : 0;
        };
        // Why the page has no pack, or why its stream broke; "" while it is
        // fine.
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
