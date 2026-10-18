// Controller support for the web build: a miniquad plugin over the
// browser's Gamepad API. This file is Shuyi Engine's own code, unlike the
// vendored plugins next to it.
//
// The Rust side is engine/crates/app/src/pads/web.rs, which declares exactly the
// `shuyi_pad_*` functions below (a test of shuyi-cli compares the two lists)
// and turns what they return into the game's buttons. All the rules (sticks as
// directions, the Switch-style swap, several pads as one) live in Rust.
//
// Only pads the browser reports with `mapping === "standard"` are listed, so
// button and axis numbers mean the same on every pad:
// https://w3c.github.io/gamepad/#remapping
// Browsers don't report a pad at all until one of its buttons is pressed
// while the page has focus.
(function () {
    "use strict";

    // Keep equal to PLUGIN_VERSION in engine/crates/app/src/pads/web.rs.
    var VERSION = 3;

    // The standard mapping's buttons before its optional extras: face
    // buttons, shoulders, triggers, the two centre buttons, stick presses,
    // D-pad.
    var STANDARD_BUTTONS = 16;

    // The pads found by the last shuyi_pad_poll().
    var pads = [];

    // The USB vendor id in a Gamepad.id, or 0. Chrome writes
    // "Wireless Controller (STANDARD GAMEPAD Vendor: 054c Product: 09cc)",
    // Firefox "054c-09cc-Wireless Controller"; XInput pads and Safari give
    // no id (they count as generic pads).
    function vendor_of(id) {
        var match = /Vendor:\s*([0-9a-f]{4})/i.exec(id) || /^([0-9a-f]{4})-[0-9a-f]{4}-/i.exec(id);
        return match ? parseInt(match[1], 16) : 0;
    }

    // The USB product id in a Gamepad.id, or 0, read like vendor_of.
    function product_of(id) {
        var match = /Product:\s*([0-9a-f]{4})/i.exec(id) || /^[0-9a-f]{4}-([0-9a-f]{4})-/i.exec(id);
        return match ? parseInt(match[1], 16) : 0;
    }

    function register_plugin(importObject) {
        // Reads the connected standard-mapping pads; returns how many. The
        // other functions index into this list until the next call.
        importObject.env.shuyi_pad_poll = function () {
            pads = [];
            var all;
            try {
                // Missing in old browsers; throws where a page's
                // permissions policy forbids gamepads (an iframe without
                // allow="gamepad").
                all = navigator.getGamepads();
            } catch (e) {
                return 0;
            }
            for (var i = 0; i < all.length; i++) {
                var pad = all[i];
                if (pad && pad.connected && pad.mapping === "standard") {
                    pads.push(pad);
                }
            }
            return pads.length;
        };
        // The browser's own index of the pad (stable while it's plugged in).
        importObject.env.shuyi_pad_index = function (pad) {
            return pads[pad] ? pads[pad].index : 0;
        };
        // The pad's USB vendor id, or 0 if the browser doesn't tell.
        importObject.env.shuyi_pad_vendor = function (pad) {
            return pads[pad] ? vendor_of(pads[pad].id) : 0;
        };
        // The pad's USB product id, or 0 if the browser doesn't tell.
        importObject.env.shuyi_pad_product = function (pad) {
            return pads[pad] ? product_of(pads[pad].id) : 0;
        };
        // Bit i is set while the pad's buttons[i] is pressed, i < 16.
        importObject.env.shuyi_pad_buttons = function (pad) {
            var buttons = pads[pad] ? pads[pad].buttons : [];
            var bits = 0;
            for (var i = 0; i < STANDARD_BUTTONS && i < buttons.length; i++) {
                if (buttons[i].pressed) {
                    bits |= 1 << i;
                }
            }
            return bits;
        };
        // The pad's axes[axis]: 0-3 are the left stick's x and y, then the
        // right stick's (down is positive).
        importObject.env.shuyi_pad_axis = function (pad, axis) {
            var value = pads[pad] ? pads[pad].axes[axis] : 0;
            return typeof value === "number" ? value : 0;
        };
    }

    miniquad_add_plugin({
        register_plugin: register_plugin,
        name: "shuyi_gamepad",
        version: VERSION
    });
})();
