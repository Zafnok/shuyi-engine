# ADR-0066: Sound uses library codecs and output, with an owned mixer and staged parity gates

- **Status:** Accepted
- **Date:** 2026-10-31
- **Related tickets:** 2902, 2573, 2210, 3030–3032, 2397, 3079
- **Resolves:** ADR-0021 §1's delegated sound choice; its other decisions remain accepted

## Context

The [research](../research/2902-sound-after-quad-snd.md) found permitted
decoder libraries and CPAL viable on the GNU host, while neither a native
library build nor a seek API proves browser streaming or gapless loops.
The present backend fully decodes music and cannot truly pause desktop
effects. Retaining those limitations is not the correctness oracle for
new features. Nick chose the hybrid only with automated correctness/speed
parity, a first gate ticket and a separate removal ticket
([decision](../decisions.md)).

## Decision

Use hound/lewton for WAV/Vorbis and optional claxon/nanomp3 adapters for
FLAC/MP3, with defaults off and licenses rechecked at implementation.
`shuyi-sound` owns engine-independent PCM/source types and the integer
mixer; its core builds with `no_std` and `alloc`. Codec adapters are
optional `std` features. Workers and I/O belong in the platform backend,
not the pure core or asset formats. The existing shared player retains
variants, clocks, gains, ducking and App's admission policy.

Use CPAL, defaults off, for desktop output after deployment/packaging
verification. Preserve an original Web Audio bridge for the current
macroquad loader. Playdate SDK players and GBA tracker/baked PCM remain
target adapters; this choice promises no live GBA Vorbis/MP3 decoder.
Native C FFI codecs remain replaceable adapters if their licenses, builds
and target support pass; they do not remove the current browser requirement.

**2573 comes first:** implement and run a real reference capture/oracle
and performance gate before replacement implementation. Unavailable capture
or an uncalibrated gate blocks progress. Keep quad-snd and candidate paths
side by side; each decoder, mixer, Windows/web, Linux/macOS and loop-region
stage extends and passes automated correctness and speed checks before
any consumer switches. The common old Linux/native and actual-loader
browser baselines enable portable work. Each output port must establish
its actual reference capture/performance baseline before its new output
implementation, first in 3031/3032 if necessary. Unproved Windows/Mac capture
blocks that port, not counted as passing evidence; final removal needs the
complete matrix. Use generated/public permissive fixtures, no game
assets. Existing behavior is compared as a black box; new requirements
use independent expected samples/state, rather than reproducing old bugs.

2210 replaces clean-room codec work; 2211–2214 are closed as dropped,
not implemented. 3030 owns mixing, 3031 Windows/web integration, 3032
Linux/macOS integration, and 2397 the loop-region contract. None removes
quad-snd. **3079 alone removes quad-snd and macroquad audio**, after every
consumer and required feature passes the comparative gates, deployment
audits and listening/device sign-offs. Existing backends remain until
their replacements qualify; the research does not unpark these builds.

## Consequences

Shuyi avoids owning a Vorbis implementation and three raw desktop APIs,
while retaining controllable integer mixing and constrained-target seams.
Lewton remains a quiet dependency, nanomp3 a new one, and adoption still
needs full dependency/license and compile-budget gates. CPAL's documented
macOS minimum is an adoption audit, not authorization to drop a target.

The first harness is real work and can block this plan if it cannot
establish reliable capture or performance evidence. Performance budgets
come from measured baseline variance on pinned runners, not the research's
single standalone build runs. Manual listening remains additional evidence.

## Alternatives considered

- **Rodio/Kira mixer:** viable libraries, but require policy adaptation and
  current-loader web streaming work; no uniform `no_std` target path.
- **Original owned codecs and raw output:** much more conformance, device
  testing and maintenance, without demonstrated desktop decode CPU need.
- **Immediate cutover or manual-only parity:** rejected by Nick; keep the
  working backend until automated stage gates and final removal qualify.
