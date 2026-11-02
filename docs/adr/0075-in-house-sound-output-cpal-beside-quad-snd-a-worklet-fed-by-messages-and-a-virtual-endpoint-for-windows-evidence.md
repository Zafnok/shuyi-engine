# ADR-0075: In-house sound output: CPAL beside quad-snd, a worklet fed by messages, and a virtual endpoint for Windows' evidence

- **Status:** Accepted
- **Date:** 2026-11-02
- **Related tickets:** 3031 (this), 3032, 3084, 3079, 2573, 3030
- **Extends:** ADR-0066, ADR-0067, ADR-0068, ADR-0073

## Context

ADR-0066 chose CPAL natively and our own Web Audio bridge, and made each port
establish a real reference capture before its output code. Working 3031 found three
things the plan hadn't settled:

- **CPAL and quad-snd can't resolve in one build.** CPAL's Linux path brings
  `alsa-sys` and quad-snd's brings `quad-alsa-sys`, and each says `links = "alsa"`.
  Cargo allows that once per build, on any target, even for a dependency only another
  target compiles, and a workspace's lock file resolves every feature. 2902 built CPAL
  alone, so it didn't show. ADR-0066 keeps both side by side until 3079.
- **A hosted Windows runner has no render endpoint**, and no sound service has a
  null sink to make one, as PulseAudio does on Linux.
- **The web build has no threads and no `wasm-bindgen`**, so nothing there can run a
  callback on an audio thread.

## Decision

**A held place for `alsa-sys`.** The root `Cargo.toml` patches `alsa-sys` to an empty
crate, `tools/sound-parity/alsa-sys-held`, with no `links`, and `shuyi-sound-out`
depends on CPAL only for Windows, so nothing is ever compiled against it. It costs one
thing a game can feel: a game that turns `in-house-sound` on needs the same `[patch]`,
as Cargo reads patches only from the root of a build. A game with it off resolves
neither. 3032 can't keep this, as Linux needs real ALSA there; its own ticket says so.
3079 removes it with quad-snd.

**A virtual endpoint for the evidence.** CI installs Scream, a virtual sound card, on
the Windows runner (`install-endpoint.ps1`, pinned by hash), and `capture.py windows`
records it through WASAPI loopback (`wasapi.py`: the documented COM interfaces over
`ctypes`, nothing to install), the same real path a player's sound takes as far as the
system's mixer. Its signature's certificate ended in 2023 with no timestamp, so the
clock goes inside its dates for the install alone; the runner has Secure Boot off,
so the kernel loads it. The endpoint is set to the corpus's 44,100 Hz, or a
sample-exact comparison would be measuring a resampler; that call (`IPolicyConfig`) is
undocumented and only runs on a runner, behind `--set-endpoint`. Elsewhere a different
rate is a refusal. A loopback client gets no packets while nothing renders, so a stream
of written zeros stays open for the whole session. Scream runs in CI and nothing of it
ships, as PulseAudio (ADR-0067). A Windows job records, and `sound-parity`, which has
the oracle, judges it with the rest.

**A new backend is a platform of the matrix.** `windows-in-house` and
`browser-in-house` are `NewBackend`s: every cell is mandatory, the scenarios are the
same expected signals, `old-new.jsonl` holds the same workload on old and new in turn
against limits of their own, and the decoders and the mixer stay measured on the
reference platform, being the same code. `linux-native`, `browser-chromium` and
`windows` keep recording quad-snd with every cell mandatory.

**The game's side names places, as it picks voices.** `Remote::reserve_sound` and
`reserve_stream` answer at once, and the object follows through a hand-over of boxes
(`shuyi-sound-out`'s `handover`, in `sys`, ADR-0021 §6) to `Mixer::put_sound` and
`put_stream`. So nothing waits on the audio thread, which may be held or absent. The
callback sends every box back, holding what is to be freed. `Inbox::apply_with` runs
the hand-over ahead of each command, so a play sent after its sound always finds it.
The mixer itself stays on the game's thread until the first play, so that a game's
sounds load at no cost, then crosses in one box.

**Device loss is watched, not assumed.** CPAL's documentation says a stream built on the
default device may follow it or may be lost, and not which on WASAPI. A thread owns
the stream, as it may not be `Send`, and rebuilds on a lost stream or a changed default,
looking five times a second. The callback lives in a mutex that only one stream at a
time locks, with `try_lock`, so the audio thread never waits. While there is no device
the mix goes on, discarded, so that what plays ends when it should.

**The web's queue is messages.** `sound.js` keeps an `AudioWorklet` a bounded number
of blocks ahead: the worklet reports frames played, and each report tops it up, calling
Rust's `shuyi_sound_fill` from that event, on the page's one thread. Nothing is shared,
so it needs only a secure context, not isolation, headers or atomics. It takes no timer
or animation frame, so a hidden tab keeps sounding. Until a gesture starts the context
a timer discards what time has passed, so the player's clock stays true.

**What the web hands the adapters.** A panic stops the program there. So a file goes to
the browser's `decodeAudioData` first, off the page's thread, and only one it reads goes
on to `hound` or `lewton`. The browser's decoder isn't the player: it is only asked
whether it can read the file, so a loop's samples still come from our own path. A file
it reads and a library panics on would still stop the program, which only a game's own
file can be.

## Consequences

- The Windows gate is real and needs no person, at the cost of a minute's install and
  a third party's driver on each sound PR's runner.
- A runner image or Scream that stops loading its driver stops sound PRs until fixed:
  fail closed, as ADR-0067 wants.
- `VorbisSource` and `WavSource` take any `AsRef<[u8]>` so a worker can own its bytes,
  which changes a public signature in a chain only its own tools use yet.
- A game's own `[patch]` is a line it adds to try the new sound, and every game's when
  3084 makes it the default. 3084 has to answer that before it flips.

## Alternatives considered

- **Replacing CPAL with the `wasapi` crate:** no `links` key, so no patch, but it
  undoes ADR-0066's choice for one target and leaves Linux and macOS to find another
  way. A patch is smaller and comes out cleanly at 3079.
- **Patching `quad-alsa-sys` instead:** changes the old path while it is the reference.
- **Recording on a developer's machine:** real speakers, a person's own settings, no
  CI, and a noise through the room each time.
- **Shared memory for the web's queue:** needs cross-origin isolation, which a game's
  host can't be counted on to set.
- **The browser's decoder as the web's only decoder:** whole-file decodes, resampled to
  the context's rate, with no bounded stream at all.
- **Answering each hand-over before a play:** every load would wait on a thread that may
  be suspended, and a browser's only runs after a gesture.
