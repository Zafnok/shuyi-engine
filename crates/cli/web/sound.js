// Sound output for the web build: a miniquad plugin over the browser's Web Audio.
// This file is Shuyi Engine's own code, written from the W3C Web Audio specification
// and MDN, unlike the vendored plugins next to it.
//
// The Rust side is crates/sound-out/src/sys.rs, which declares exactly the
// `shuyi_sound_*` functions below (a test of shuyi-cli compares the two lists), and
// crates/sound-out/src/web.rs, which says the same contract from there:
//
// - Rust's `shuyi_sound_fill(frames)` mixes that many frames and returns where they
//   are, 16-bit stereo, interleaved. They are good until the next call, so they are
//   copied at once.
// - They go to an AudioWorklet in a transferred buffer, which it copies into a PcmRing
//   and sends back to be used again, so a block costs no garbage. Every block's worth
//   of frames it says how many it has played, and each time this tops its lead back up
//   to LEAD_BLOCKS blocks. It takes no timer and no animation frame, so a hidden tab
//   keeps its sound.
// - The worklet plays what it has and silence after that, and counts neither as lost:
//   the mix is only asked for what will be heard, so a late page delays the sound.
// - Nothing is shared but messages. So this needs a secure context, as AudioWorklet
//   does (HTTPS, or localhost), and no cross-origin isolation, no special headers and
//   no atomics. Where there is no worklet, `shuyi_sound_open` says 0 and Rust carries
//   on silently. The worklet's code loads from a blob: URL, which a page's own Content
//   Security Policy has to allow in `script-src`.
// - A timer asks for the frames that time passes and discards them until the context
//   runs, which takes the first gesture (index.html resumes every context on one), so
//   what plays keeps its place on the player's clock and its commands are heard.
(function () {
    "use strict";

    // Keep equal to PLUGIN_VERSION in crates/sound-out/src/sys.rs.
    var VERSION = 1;

    // How many blocks the worklet is kept ahead by, and how many its ring holds.
    var LEAD_BLOCKS = 4;
    var RING_BLOCKS = 8;
    // How often the timer looks, in milliseconds, before the context runs.
    var IDLE_MS = 20;

    // The worklet's code, run there as a string of its source and here for what the
    // ring is, which CI's ring test reads from this plugin's `ring`.
    function worklet_scope(in_worklet) {
        // A ring of frames for one writer and one reader, whose counters run on,
        // wrapping at 2^32, so the two are compared by their difference alone.
        function PcmRing(frames) {
            var capacity = 1;
            while (capacity < frames) {
                capacity *= 2;
            }
            this.capacity = capacity;
            this.samples = new Int16Array(capacity * 2);
            this.written = 0;
            this.taken = 0;
        }
        // How many frames it holds.
        PcmRing.prototype.held = function () {
            return (this.written - this.taken) >>> 0;
        };
        // Takes as many whole frames of `block`, interleaved, as there is room for.
        PcmRing.prototype.write = function (block) {
            var frames = Math.min(block.length >> 1, this.capacity - this.held());
            var mask = this.capacity - 1;
            for (var i = 0; i < frames; i++) {
                var at = ((this.written + i) & mask) * 2;
                this.samples[at] = block[i * 2];
                this.samples[at + 1] = block[i * 2 + 1];
            }
            this.written = (this.written + frames) >>> 0;
            return frames;
        };
        // Fills `left` and `right` as floats, silence past what it holds, and says
        // how many frames were real.
        PcmRing.prototype.read = function (left, right) {
            var frames = Math.min(left.length, this.held());
            var mask = this.capacity - 1;
            for (var i = 0; i < frames; i++) {
                var at = ((this.taken + i) & mask) * 2;
                left[i] = this.samples[at] / 32768;
                right[i] = this.samples[at + 1] / 32768;
            }
            for (var rest = frames; rest < left.length; rest++) {
                left[rest] = 0;
                right[rest] = 0;
            }
            this.taken = (this.taken + frames) >>> 0;
            return frames;
        };
        if (!in_worklet) {
            return PcmRing;
        }
        class Output extends AudioWorkletProcessor {
            constructor(options) {
                super();
                this.ring = new PcmRing(options.processorOptions.frames);
                this.report_every = options.processorOptions.report;
                this.played = 0;
                this.silent = 0;
                this.since = 0;
                var self = this;
                this.port.onmessage = function (event) {
                    self.ring.write(new Int16Array(event.data));
                    self.port.postMessage({ back: event.data }, [event.data]);
                };
            }
            process(inputs, outputs) {
                var left = outputs[0][0];
                var right = outputs[0][1] || new Float32Array(left.length);
                var real = this.ring.read(left, right);
                this.played = (this.played + real) >>> 0;
                this.silent = (this.silent + left.length - real) >>> 0;
                this.since += left.length;
                if (this.since >= this.report_every) {
                    this.since = 0;
                    this.port.postMessage({ played: this.played, silent: this.silent });
                }
                return true;
            }
        }
        registerProcessor("shuyi-sound", Output);
        return null;
    }

    var context = null;
    var node = null;
    var rate = 0;
    var block = 0;
    var sent = 0;
    var played = 0;
    var spare = [];
    var held = false;
    var last_idle = 0;
    var timer = null;
    var vets = [];

    // The next block of the mix, or null while Rust has no output open.
    function next_block() {
        var at = wasm_exports.shuyi_sound_fill(block);
        return at === 0 ? null : new Int16Array(wasm_memory.buffer, at, block * 2);
    }

    // Keeps the worklet its lead ahead.
    function top_up() {
        if (held) {
            return;
        }
        while (((sent - played) >>> 0) + block <= LEAD_BLOCKS * block) {
            var frames = next_block();
            if (frames === null) {
                return;
            }
            var buffer = spare.pop() || new ArrayBuffer(block * 4);
            new Int16Array(buffer).set(frames);
            node.port.postMessage(buffer, [buffer]);
            sent = (sent + block) >>> 0;
        }
    }

    function on_message(event) {
        if (event.data.back) {
            spare.push(event.data.back);
        } else {
            played = event.data.played;
            top_up();
        }
    }

    // Before the context runs, or with none: the frames time has passed, discarded.
    function idle() {
        var now = performance.now();
        if (held || (node && context.state === "running")) {
            last_idle = now;
            return;
        }
        var block_ms = (block * 1000) / rate;
        while (now - last_idle >= block_ms) {
            if (next_block() === null) {
                return;
            }
            last_idle += block_ms;
        }
    }

    function start_idle() {
        if (timer === null) {
            last_idle = performance.now();
            timer = setInterval(idle, IDLE_MS);
        }
    }

    function connect() {
        node = new AudioWorkletNode(context, "shuyi-sound", {
            numberOfInputs: 0,
            numberOfOutputs: 1,
            outputChannelCount: [2],
            processorOptions: { frames: RING_BLOCKS * block, report: block },
        });
        node.port.onmessage = on_message;
        node.connect(context.destination);
    }

    function register_plugin(importObject) {
        // Opens the output at about `wanted` frames a second, `frames` at a time, and
        // returns the rate it runs at, or 0 where this page can't. `discard` never
        // makes a sound: only the timer asks.
        importObject.env.shuyi_sound_open = function (wanted, frames, discard) {
            rate = wanted;
            block = frames;
            if (discard) {
                start_idle();
                return rate;
            }
            if (context) {
                return context.sampleRate;
            }
            var Base = window.AudioContext || window.webkitAudioContext;
            if (!Base || !window.isSecureContext) {
                return 0;
            }
            var opened;
            try {
                // Every browser with a worklet honours the rate, or refuses it here,
                // so the mix can be made for it before the context exists.
                opened = new Base({ sampleRate: wanted, latencyHint: "interactive" });
            } catch (error) {
                return 0;
            }
            if (!opened.audioWorklet || opened.sampleRate !== wanted) {
                opened.close();
                return 0;
            }
            context = opened;
            start_idle();
            var source = "(" + worklet_scope.toString() + ")(true);";
            var url = URL.createObjectURL(new Blob([source], { type: "application/javascript" }));
            opened.audioWorklet.addModule(url).then(connect, function (error) {
                console.warn("audio: the sound worklet didn't load: " + error);
            });
            return rate;
        };
        // Holds the output, or lets it go on if `on` is 0.
        importObject.env.shuyi_sound_hold = function (on) {
            held = on !== 0;
            if (context) {
                if (held) {
                    context.suspend();
                } else {
                    context.resume();
                }
            }
        };
        // Starts the browser's own decoder on a file, to learn whether it can read it.
        // A number to ask `shuyi_sound_vetted` about. It takes no gesture: an offline
        // context never reaches a device.
        importObject.env.shuyi_sound_vet = function (at, bytes) {
            var number = vets.length;
            vets.push(0);
            var copy = new Uint8Array(wasm_memory.buffer, at, bytes).slice().buffer;
            var Offline = window.OfflineAudioContext || window.webkitOfflineAudioContext;
            if (!Offline) {
                vets[number] = 2;
                return number;
            }
            new Offline(1, 1, 44100).decodeAudioData(copy).then(
                function () {
                    vets[number] = 1;
                },
                function () {
                    vets[number] = 2;
                }
            );
            return number;
        };
        // 0 while the browser decodes, 1 if it read the file, 2 if it refused it.
        importObject.env.shuyi_sound_vetted = function (number) {
            return vets[number] || 0;
        };
    }

    miniquad_add_plugin({
        register_plugin: register_plugin,
        name: "shuyi_sound",
        version: VERSION,
        ring: worklet_scope(false)
    });
})();
