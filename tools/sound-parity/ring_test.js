// The web sound ring's indices, wrapping (ticket 3031): `sound.js` itself, the ring it
// hands to its worklet, run under node with nothing of a browser. `node ring_test.js`.
"use strict";

const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const file = path.join(__dirname, "..", "..", "crates", "cli", "web", "sound.js");
let plugin = null;
vm.runInNewContext(fs.readFileSync(file, "utf8"), {
    miniquad_add_plugin: (added) => {
        plugin = added;
    },
    Int16Array,
    Math,
});
const PcmRing = plugin.ring;

const stereo = (...samples) => Int16Array.from(samples);

// Rounded up to a power of two.
assert.strictEqual(new PcmRing(5).capacity, 8);
assert.strictEqual(new PcmRing(8).capacity, 8);

// What goes in comes out, in order, as a fraction of full scale, then silence.
{
    const ring = new PcmRing(4);
    assert.strictEqual(ring.write(stereo(16384, -16384, 8192, -8192)), 2);
    assert.strictEqual(ring.held(), 2);
    const [left, right] = [new Float32Array(3), new Float32Array(3)];
    left.fill(9);
    right.fill(9);
    assert.strictEqual(ring.read(left, right), 2);
    assert.deepStrictEqual(Array.from(left), [0.5, 0.25, 0]);
    assert.deepStrictEqual(Array.from(right), [-0.5, -0.25, 0]);
    assert.strictEqual(ring.held(), 0);
}

// A full ring takes what fits, and the rest isn't written over what is there.
{
    const ring = new PcmRing(2);
    assert.strictEqual(ring.write(stereo(1, 1, 2, 2, 3, 3)), 2);
    assert.strictEqual(ring.write(stereo(4, 4)), 0);
    const [left, right] = [new Float32Array(2), new Float32Array(2)];
    assert.strictEqual(ring.read(left, right), 2);
    assert.deepStrictEqual(Array.from(left), [1 / 32768, 2 / 32768]);
}

// Half a frame at the end of a block isn't a frame.
{
    const ring = new PcmRing(4);
    assert.strictEqual(ring.write(stereo(1, 2, 3)), 1);
}

// The counters wrap past 2^32, and the indices past the capacity, with nothing lost.
{
    const ring = new PcmRing(8);
    ring.written = 0xfffffffb;
    ring.taken = 0xfffffffb;
    let next = 0;
    let expected = 0;
    for (let pass = 0; pass < 64; pass++) {
        const size = 1 + (pass % 7);
        const block = new Int16Array(size * 2);
        for (let frame = 0; frame < size; frame++) {
            block[frame * 2] = next;
            block[frame * 2 + 1] = -next;
            next += 1;
        }
        assert.strictEqual(ring.write(block), size, `pass ${pass}`);
        assert.strictEqual(ring.held(), size, `pass ${pass}`);
        const [left, right] = [new Float32Array(size), new Float32Array(size)];
        assert.strictEqual(ring.read(left, right), size);
        for (let frame = 0; frame < size; frame++) {
            assert.strictEqual(left[frame], expected / 32768);
            assert.strictEqual(right[frame], -expected / 32768 + 0);
            expected += 1;
        }
    }
    assert.ok(ring.written < 0xfffffffb, "it did wrap");
}

console.log("the sound ring holds");
