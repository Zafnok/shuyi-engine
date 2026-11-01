---
id: "3032"
title: "Gated CPAL sound integration on Linux and macOS, retaining quad-snd until final removal"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3031", "2513", "2573"]
nick_input: sign-off
completed:
---

# 3032 — Gated sound output on Linux and macOS

## Context

3031 provides Windows/web output over the owned mixer and decoder adapters.
This ticket adds Linux (the Steam Deck included) and macOS through CPAL.
Keep old/new paths side by side; the separate final ticket 3079 owns removal.
See [ADR-0066](../../docs/adr/0066-sound-uses-library-codecs-and-output-with-an-owned-mixer-and-parity-gates.md)
and [2902's evidence](../../docs/research/2902-sound-after-quad-snd.md).

## Nick input

**Sign-off:** listen on the Steam Deck/Linux and a Mac if one is at hand.
If no Mac is available, record that macOS was tested only in CI, as the
original ticket allowed. Automated emitted-output/performance evidence is
still mandatory on every required port; a CI build alone does not supply it.

## Scope

**In:**
- Before new output code or switching a platform, establish its actual
  old-output capture and calibrated speed baseline in 2573's harness.
  Linux reuses the virtual sink. **From 2573 (done):** Linux's baseline is in
  CI; macOS is yours to establish first, and `cargo xtask sound-parity
  port-ready macos` refuses until it has passed evidence. Automated hosted
  macOS capture was not established by research: make a route work on a
  suitable runner or remain blocked there with the old backend. Missing capture, fake callback
  traces and unsupported cells never count as passed audio parity.
- CPAL defaults off (0.18.2 researched Apache-2.0; recheck manifest,
  repository license and full graph), through public APIs. Own glue stays
  original; do not copy implementation source or write raw ALSA/CoreAudio
  backends merely to follow the old outline. Add actual layers edges.
- Linux default ALSA output: negotiated formats, device loss and underrun
  recovery. Audit actual linked/dynamic libraries and the sniper glibc
  floor. Linux packages already require ALSA; do not claim CPAL uses
  `dlopen` without proof. Preserve the planned silent-launch behavior
  without an audio library using a compatible loading adapter/version,
  or retain the old path until resolved. Doctor (2513), setup and package
  requirements/tests follow the actual chosen behavior.
- **Either order with 3071:** if 3071 is done, update its actual CPAL
  library audit/package tests in `crates/cli/export/libraries.ron` and
  `docs/targets/desktop.md`; if not, nothing to do (3071 audits this path).
- macOS default output on Apple silicon and Intel. CPAL's researched
  docs list macOS 14.2; Shuyi has no explicit OS floor. Verify deployment
  rather than declaring a regression or silently raising the minimum.
  Resolve incompatibility with a compatible version/adapter or keep old.
- Extend actual emitted-PCM, equal-work CPU/speed, bounded-memory, hotplug,
  suspend and music-clock checks. Only switch consumers after comparative
  gates and existing stage sign-off pass; record manual hardware limits
  above and leave the reference selectable.
  Hound/lewton remain intentional dependencies.
- Keep the public audio/player API and game's files unchanged. A discovered
  break gets a Breaking line and game handoff, not a silent break, and one only
  the new backend could honour stops this stage until it needn't ([ADR-0068](../../docs/adr/0068-the-sound-replacement-stays-reversible-at-every-stage.md)).
- Stay behind 3031's `in-house-sound`, off by default: making it the default is
  3084's. Backing out is reverting this PR, and CI still records quad-snd.

**Out (do not do):** quad-snd, macroquad audio, feature, source, plugin and
notice removal (3079); new cue rules; mobile backends; optional native
PipeWire/JACK support. No unvalidated target is abandoned to finish this ticket.

## Implementation steps

1. Prove each platform's reference capture/performance gate before its adapter.
2. Integrate CPAL, negotiate/recover, audit package requirements and floors.
3. Extend 2573's scenarios and CI on Linux and both Mac architectures.
4. Build the game against this copy, listen on hardware, record artifacts,
   devices, target floors, dependencies and sign-offs for 3079.

## Acceptance criteria

- [ ] Actual old/new output and calibrated CPU/speed comparisons pass on Linux and macOS; unavailable capture blocks migration.
- [ ] Both Mac architectures and verified deployment floor work; Linux package/doctor/library checks match reality.
- [ ] Planned silent launch without an audio library is preserved and tested, or migration remains blocked.
- [ ] Device recovery, suspend/clock, bounded streams and all required playback scenarios pass.
- [ ] The game builds/plays without code changes; existing stage sign-off and hardware-availability notes are recorded.
- [ ] Quad-snd and the reference path remain available for 3079.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

Unit: format negotiation/conversion and recovery on fakes; doctor/package
requirements. Integration: actual emitted PCM and repeated performance in
2573's harness, null-output golden render, bounded memory/underruns, Mac
universal/export checks and Linux missing-library behavior. Manual: hardware
listening/recovery as above, in addition to automated correctness and speed.

## Completion notes

*(Record platform gate artifacts, deployment audit, devices and sign-offs.)*
