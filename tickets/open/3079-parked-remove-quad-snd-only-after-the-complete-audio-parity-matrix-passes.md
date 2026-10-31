---
id: "3079"
title: "Parked: Remove quad-snd only after the complete audio parity matrix passes"
type: infra
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2902", "2573", "3032", "2397", "2234", "2378"]
nick_input: none
completed:
---

# 3079 — Remove quad-snd after complete parity

> **Parked**: Nick required a separate final removal ticket in 2902 on 2026-10-31. The build chain stays parked until he starts it. Whoever unparks it sets `status: todo` and removes `Parked:`.

## Context

2573 establishes working automated reference gates; 2210/3030/3031/3032
and 2397 progressively qualify decoder, mixer, platform and loop behavior.
Those stages retain the old path. This is the sole removal step in
[ADR-0066](../../docs/adr/0066-sound-uses-library-codecs-and-output-with-an-owned-mixer-and-parity-gates.md),
conditional on Nick's correctness/speed requirement, not just successful builds.

## Nick input

None. Reuse 3031/3032's existing listening/device sign-offs and their
hardware-availability rules; no duplicate owner approval is needed.
Missing mandatory automated evidence still blocks removal.

## Scope

**In:**
- Fresh complete 2573 matrix for Windows GNU, Linux, both Mac architectures
  and the actual current browser loader, with all required playback,
  decoder/format, clock, copy/admission, pause/rate, two-stream/fade and
  region-loop scenarios. 2234 supplies optional-format playback; 2378
  supplies the single App admission policy. No stale, unsupported, noisy
  or missing mandatory case counts as passed. Retain existing stage
  sign-offs and recorded manual-hardware limits.
- Run the last old-vs-new comparison on the same harness/corpus/config
  using the pre-removal reference revision in an isolated checkout/tools
  workspace. Preserve raw PCM, stats, hashes and gate/baseline revisions.
  Only after correctness, equal-work CPU/speed, bounded memory, underrun,
  deployment/package and device checks pass, select every migrated consumer.
- Preserve unchanged Playdate SDK/baking regression checks. The future GBA
  backend never used quad-snd and need not be built to remove a desktop/web
  dependency; any currently present target/consumer must be accounted for.
  Do not call SDK traces emitted-audio parity or ignore an actual consumer.
- Remove `quad-snd` from manifests/lock, macroquad `audio`, old device and
  `native_music.rs` paths, obsolete imports/plugins/features, web copied
  scripts/requests and notices. Current test dependencies must not retain
  the old stack. Hound/lewton and CPAL's intentional dependencies remain.
- Audit source/manifests and `cargo tree` for native/wasm targets and all
  relevant features; build exports and verify browser requests. Keep owned
  PCM oracles and future regression/performance gates in current CI.
  Historical reference code remains only in git, never copied into the new
  implementation or vendored as a disguised current test dependency.
- Update architecture/dependency/platform docs and game pin handoff. The
  public player API should remain stable; record any actual break honestly.

**Out (do not do):** weakening parity budgets, silently abandoning a port,
new codecs/mixers/DSP, future GBA implementation, replacing SDK audio.

## Implementation steps

1. Audit every consumer and evidence cell; run the final comparative matrix.
2. With all gates/sign-offs passing, remove the old stack and make new default.
3. Rebuild native/web exports, audit dependencies/plugins/notices, rerun
   persistent regression/performance checks and record the reference revision.

## Acceptance criteria

- [ ] Fresh automated correctness/speed evidence covers every required migrated target/feature; none is skipped or inconclusive; existing stage sign-offs/availability limits are recorded.
- [ ] The final same-config historical-reference comparison passes and artifacts are retained.
- [ ] No current shipping or test consumer selects quad-snd or macroquad audio; native/wasm trees and browser requests prove removal.
- [ ] Owned PCM/performance regression checks, target packaging and unchanged SDK checks still pass after removal.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

Integration: complete actual emitted-PCM/performance matrix before removal,
then current-tree regression suite, native/web exports, dependency/feature
and request/notice audits. Manual: required listening/device checks from
3031/3032 remain additional evidence, never substitutes for automation.

## Completion notes

*(Record matrix/reference revisions, artifacts, removed consumers and sign-offs.)*
