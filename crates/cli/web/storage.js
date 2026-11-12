// Saves for the web build: a miniquad plugin over the browser's
// `localStorage`. This file is Shuyi Engine's own code, written from MDN's
// Web Storage API pages (Window.localStorage and Storage: getItem, setItem,
// removeItem, key, length), and from its TextEncoder and TextDecoder pages.
//
// The Rust side is crates/app/src/storage/web.rs, which declares exactly the
// `shuyi_storage_*` functions below (a test of shuyi-cli compares the two
// lists) and turns the numbers they return into its errors.
//
// Strings cross as UTF-8 bytes in the wasm memory. Rust passes a pointer and
// a length for each string it gives. For a string it asks for, it passes a
// buffer: the function writes the string there if it fits, and returns its
// length in bytes either way, so a length larger than the buffer means "ask
// again with a buffer this big" (nothing was written).
//
// `localStorage` can throw at every step: reading `window.localStorage` in a
// page whose storage is turned off (SecurityError), `setItem` when the quota
// is full (QuotaExceededError). Every function catches it and returns an
// error code, so the game warns and plays on.
//
// Writes stay synchronous: `set` and `remove` call `localStorage` and return
// after it, so a save made in a `pagehide` handler is kept.
(function () {
    "use strict";

    // Keep equal to PLUGIN_VERSION in crates/app/src/storage/web.rs.
    var VERSION = 1;

    // The error codes, kept equal to the ones in crates/app/src/storage/web.rs.
    // No value under the key (get), or no key at the index (key).
    var ABSENT = -1;
    // The page may not use `localStorage`: storage turned off, or a
    // sandboxed frame.
    var BLOCKED = -2;
    // `localStorage` is full.
    var FULL = -3;
    // `localStorage` threw something else.
    var FAILED = -4;

    var encoder = new TextEncoder();
    var decoder = new TextDecoder("utf-8");

    // The page's `localStorage`. Throws if the page may not use it.
    function store() {
        var storage = window.localStorage;
        if (storage === null || storage === undefined) {
            throw { name: "SecurityError" };
        }
        return storage;
    }

    // The error code for an exception `localStorage` threw.
    function code_of(error) {
        var name = error && error.name;
        // Firefox named its quota error NS_ERROR_DOM_QUOTA_REACHED once.
        if (name === "QuotaExceededError" || name === "NS_ERROR_DOM_QUOTA_REACHED") {
            return FULL;
        }
        if (name === "SecurityError") {
            return BLOCKED;
        }
        return FAILED;
    }

    // The string of `length` bytes of UTF-8 at `pointer`.
    function text_at(pointer, length) {
        return decoder.decode(new Uint8Array(wasm_memory.buffer, pointer, length));
    }

    // Writes `text` as UTF-8 into the `capacity` bytes at `pointer` if it
    // fits; returns its length in bytes either way.
    function give(text, pointer, capacity) {
        var bytes = encoder.encode(text);
        if (bytes.length <= capacity) {
            new Uint8Array(wasm_memory.buffer, pointer, bytes.length).set(bytes);
        }
        return bytes.length;
    }

    function register_plugin(importObject) {
        // The value under the key, given into the buffer; ABSENT if none.
        importObject.env.shuyi_storage_get = function (key, key_length, buffer, capacity) {
            try {
                var value = store().getItem(text_at(key, key_length));
                return value === null ? ABSENT : give(value, buffer, capacity);
            } catch (error) {
                return code_of(error);
            }
        };
        // Stores the value under the key; 0 once it is stored.
        importObject.env.shuyi_storage_set = function (key, key_length, value, value_length) {
            try {
                store().setItem(text_at(key, key_length), text_at(value, value_length));
                return 0;
            } catch (error) {
                return code_of(error);
            }
        };
        // Removes the key, if it is there; 0 once it is gone.
        importObject.env.shuyi_storage_remove = function (key, key_length) {
            try {
                store().removeItem(text_at(key, key_length));
                return 0;
            } catch (error) {
                return code_of(error);
            }
        };
        // How many keys the page's `localStorage` holds: those of every
        // page of its origin, other games' too.
        importObject.env.shuyi_storage_len = function () {
            try {
                return store().length;
            } catch (error) {
                return code_of(error);
            }
        };
        // The key at `index`, given into the buffer; ABSENT past the end.
        importObject.env.shuyi_storage_key = function (index, buffer, capacity) {
            try {
                var key = store().key(index);
                return key === null ? ABSENT : give(key, buffer, capacity);
            } catch (error) {
                return code_of(error);
            }
        };
    }

    miniquad_add_plugin({
        register_plugin: register_plugin,
        name: "shuyi_storage",
        version: VERSION
    });
})();
