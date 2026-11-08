// The web sound plugin under node, with nothing of a browser but fakes: `sound.js`
// itself. The ring it hands to its worklet, its indices wrapping (ticket 3031), what
// of the mix reaches the worklet as the context starts (ticket 3088), and the lead that
// carries it over a late page as the device starts (ticket 2050).
// `node ring_test.js`.
"use strict";

const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const file = path.join(__dirname, "..", "..", "crates", "cli", "web", "sound.js");
const source = fs.readFileSync(file, "utf8");

// The plugin `sound.js` adds, run with `globals` as its page.
function load(globals) {
    let plugin = null;
    vm.runInNewContext(source, {
        miniquad_add_plugin: (added) => {
            plugin = added;
        },
        Int16Array,
        Math,
        ...globals,
    });
    return plugin;
}

const PcmRing = load({}).ring;

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

const RATE = 44100;
const BLOCK = 256;

// A page with the plugin's output open on it: a clock moved by hand, a context that
// runs when told, and a mix whose every block carries its number, so what reached the
// worklet says what was discarded. With `activation` false the browser doesn't say
// whether the page has had its gesture.
function page({ activation = true } = {}) {
    const state = {
        now: 0,
        made: 0,
        posted: [],
        activation: { hasBeenActive: false },
        gestures: [],
    };
    const memory = { buffer: new ArrayBuffer(4 + BLOCK * 4) };
    class Context {
        constructor(options) {
            this.sampleRate = options.sampleRate;
            this.state = "suspended";
            this.changes = [];
            this.audioWorklet = { addModule: () => ({ then: (loaded) => (state.loaded = loaded) }) };
            state.context = this;
        }
        addEventListener(type, listener) {
            this.changes.push(listener);
        }
        connect() {}
        suspend() {
            this.state = "suspended";
        }
        resume() {}
    }
    class Node {
        constructor() {
            this.port = {
                postMessage: (buffer) => state.posted.push(new Int16Array(buffer)[0]),
                onmessage: null,
            };
            state.node = this;
        }
        connect() {}
    }
    const plugin = load({
        window: {
            AudioContext: Context,
            isSecureContext: true,
            navigator: activation ? { userActivation: state.activation } : {},
            document: { addEventListener: (type, listener) => state.gestures.push(listener) },
        },
        AudioWorkletNode: Node,
        Blob: class {},
        URL: { createObjectURL: () => "blob:" },
        performance: { now: () => state.now },
        setInterval: (tick) => (state.tick = tick),
        wasm_exports: {
            shuyi_sound_fill: () => {
                new Int16Array(memory.buffer, 4, BLOCK * 2)[0] = state.made++;
                return 4;
            },
        },
        wasm_memory: memory,
        console,
    });
    const env = {};
    plugin.register_plugin({ env });
    assert.strictEqual(env.shuyi_sound_open(RATE, BLOCK, 0), RATE);
    state.loaded();
    // Time passing, the timer looking every 20 milliseconds.
    state.pass = (ms) => {
        for (let at = 0; at < ms; at += 20) {
            state.now += 20;
            state.tick();
        }
    };
    // The worklet saying it has played everything sent, as it does each block.
    state.played = () =>
        state.node.port.onmessage({ data: { played: state.posted.length * BLOCK, silent: 0 } });
    state.runs = () => {
        state.context.state = "running";
        state.context.changes.forEach((change) => change());
        state.played();
    };
    state.gesture = () => {
        state.activation.hasBeenActive = true;
        state.gestures.forEach((gesture) => gesture());
    };
    state.hold = (on) => env.shuyi_sound_hold(on ? 1 : 0);
    return state;
}

// Every block from `first` on reached the worklet, in order: none was discarded.
function heard_from(state, first) {
    const after = state.posted.filter((block) => block >= first);
    assert.ok(after.length > 0, "something was heard");
    after.forEach((block, i) => assert.strictEqual(block, first + i, "a block was discarded"));
}

// Before the gesture the mix runs on the clock and is discarded; from it, a sound is
// heard whole however long the context takes to start.
{
    const state = page();
    state.pass(200);
    const before = state.made;
    assert.ok(before >= 30, `the clock ran before the gesture: ${before} blocks`);
    assert.ok(!state.posted.includes(before - 1), "nothing before the gesture was heard");
    state.gesture();
    // What a sound asked for on the gesture starts in.
    const asked = state.made;
    state.pass(40);
    state.runs();
    heard_from(state, asked);
}

// Where the browser doesn't say, the page's own gestures count.
{
    const state = page({ activation: false });
    state.pass(100);
    state.gesture();
    const asked = state.made;
    state.pass(40);
    state.runs();
    heard_from(state, asked);
}

// A context that never starts doesn't stop the clock: the mix catches up on it.
{
    const state = page();
    state.gesture();
    const asked = state.made;
    state.pass(1000);
    assert.strictEqual(state.made, asked, "the mix waits while the context starts");
    state.pass(1100);
    assert.ok(state.made - asked >= 340, `the mix caught up: ${state.made - asked} blocks`);
}

// A hold let go is heard whole too, while the context starts again.
{
    const state = page();
    state.gesture();
    state.runs();
    state.hold(true);
    state.pass(100);
    state.hold(false);
    const asked = state.made;
    state.pass(40);
    state.runs();
    heard_from(state, asked);
}

// sound.js's leads: at least, while it starts, and once it has settled.
const START_LEAD = 8;
const FEWEST_LEAD = 4;

// The device as CI's Chromium drives it (ticket 2050's trace): 512 frames a callback,
// about every 11.6 ms, four quanta of 128 at once, the worklet saying what it played
// each block. The page answers after each callback, unless it is busy then (`busy`,
// from and for how long, in milliseconds). What the worklet played as silence, and how
// far ahead the page is left. The worklet's counters run on from call to call.
function device(state, { ms, busy = [0, 0] }) {
    const QUANTUM = 128;
    const CALLBACK = 512;
    const worklet = state.worklet || (state.worklet = { taken: 0, silent: 0, since: 0 });
    const before = worklet.silent;
    const words = [];
    for (let at = 0; at < ms; at += (CALLBACK * 1000) / RATE) {
        for (let quantum = 0; quantum < CALLBACK / QUANTUM; quantum++) {
            const real = Math.min(QUANTUM, state.posted.length * BLOCK - worklet.taken);
            worklet.taken += real;
            worklet.silent += QUANTUM - real;
            worklet.since += QUANTUM;
            if (worklet.since >= BLOCK) {
                worklet.since = 0;
                words.push({ played: worklet.taken, silent: worklet.silent });
            }
        }
        if (at < busy[0] || at >= busy[0] + busy[1]) {
            words.splice(0).forEach((data) => state.node.port.onmessage({ data }));
        }
    }
    return { silent: worklet.silent - before, ahead: state.posted.length * BLOCK - worklet.taken };
}

// The context running, as the device starts.
function starts(state) {
    state.context.state = "running";
    state.context.changes.forEach((change) => change());
}

// The worklet is the start's lead ahead before the gesture, so a sound asked for on it
// waits as long as the next ones do.
{
    const state = page();
    state.pass(100);
    assert.strictEqual(state.posted.length, START_LEAD, "ahead from the first moment");
}

// A page busy for 35 ms just after the device starts, as CI's was for 25 around the
// click's release: the worklet never runs dry. With the old lead of 4 blocks it did.
{
    const state = page();
    state.pass(100);
    state.gesture();
    starts(state);
    const { silent } = device(state, { ms: 1000, busy: [100, 35] });
    assert.strictEqual(silent, 0, "the worklet ran dry while the page was busy");
}

// Once the context has played a second, the lead comes back down, and the latency of
// a sound with it.
{
    const state = page();
    state.gesture();
    starts(state);
    const settling = device(state, { ms: 900 });
    assert.strictEqual(settling.ahead, START_LEAD * BLOCK, "the start's lead while it settles");
    const settled = device(state, { ms: 1200 });
    assert.strictEqual(settled.silent, 0, "nothing ran dry as the lead came down");
    assert.strictEqual(settled.ahead, FEWEST_LEAD * BLOCK, "the lead came down");
}

// A hold let go starts the device again, with the start's lead again.
{
    const state = page();
    state.gesture();
    starts(state);
    device(state, { ms: 1500 });
    state.hold(true);
    state.hold(false);
    starts(state);
    const again = device(state, { ms: 200 });
    assert.strictEqual(again.ahead, START_LEAD * BLOCK, "the start's lead after a hold");
}

console.log("the sound ring holds, and the context's start loses nothing");
