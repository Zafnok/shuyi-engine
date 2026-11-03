---
id: "2043"
title: "Mutation-test shuyi-sound's adapters, which a run on one package doesn't build"
type: infra
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-03
---

# 2043 — Mutation-test `shuyi-sound`'s adapters

## Context

Found by 3031. CI's mutation job (`.github/workflows/mutants.yml`) tests a mutant
with `cargo nextest` on its own package, with that package's default features.
`shuyi-sound`'s `wav.rs` and `vorbis.rs` are built only with its `wav` and `vorbis`
features, off by default since 3030 took the dev-dependency that turned them on so
that a run without `std` is one. So every mutant there is built into nothing and
reported as missed, whatever `tests/adapters.rs` says. 3031 added both files to
`exclude_globs` in `.cargo/mutants.toml`, with the reason, so that a PR touching them
isn't failed for it. Their tests still run, with `--all-features`, in CI's
`sound-parity` job: only the mutation testing of them is gone.

## Nick input

None.

## Scope

**In:**

- Mutation testing that reaches `wav.rs` and `vorbis.rs` with their tests built. One
  way: a second invocation in `mutants.yml` limited to those two files with
  `--features wav,vorbis`, in its own shard, without changing the features of every
  other package's run.
- Take both out of `exclude_globs`. Keep the `VorbisSource::next_packet` exclusion if
  it still costs a shard its timeout.
- Kill or explain what it finds.

**Out (do not do):**

- `--all-features` for the whole run: other crates' features aren't meant to combine.
- Changing the `no-std` run's features, which is why the dev-dependency left.

## Implementation steps

1. Add the invocation, reading `cargo mutants --help` for how features are passed.
2. Run it on a PR that touches `wav.rs`, and read the results.
3. Add tests or reasoned exclusions for each mutant it misses.

## Acceptance criteria

- [x] A mutant in `crates/sound/src/wav.rs` is caught by `crates/sound/tests/adapters.rs`
      in CI, seen in the job's log.
- [x] `exclude_globs` no longer names the two files.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- None of its own: the mutation job's log is the evidence.

## Completion notes

What was done:

- `mutants.yml` has a new `adapters` job for a PR. It runs `cargo mutants` on
  `wav.rs` and `vorbis.rs` alone, with `--features wav,vorbis`, in one shard
  under nextest's `mutants` profile. It runs its own unmutated baseline, because
  the shared one has no features. `plan` counts these mutants apart (an
  `adapters` output), the shards leave the two files out with `-e`, and
  `mutants (diff)` needs the new job. The weekly run does the same: the full
  shards leave the files out, and a `weekly-adapters` job runs all their
  mutants.
- `cargo mutants` adds its `-e` and `-f` globs to the config's `exclude_globs`
  and doesn't replace them (checked with `--list`). So the two files left
  `.cargo/mutants.toml`, which now says where they're tested.
- The `VorbisSource::next_packet` exclusion matched nothing anymore: its regex
  read `<.a>`, from when the source borrowed `'a`, and it is now `<B>`. It is
  `<\w+>` now, and it still applies, since that mutant hangs every read.

The evidence comes from the workflow, dispatched on this branch, because this
PR's diff touches no `.rs` file. Only its adapters job was read, and each run
was cancelled after it.

- Run 41595994623 (the first version, under `cargo test`): 98 mutants, 89
  caught, 6 unviable, 3 timeouts, none missed. The timeouts were `read -> Ok(1)`
  in both sources and `sent *= count` in Vorbis's `read`. Each makes a read
  never end. A test that checks one read catches each of them at once (for
  example `a_wav_read_with_too_little_room_says_so` expects `Ok(0)`), but
  `cargo test` waits for the tests that keep reading. **Deviation:** the
  weekly adapters job now runs under nextest's fail-fast profile too, as the
  PR's does.
- Run 41598493652: 98 mutants, **92 caught, 6 unviable**, in 4 minutes.
  `crates__sound__src__wav.rs_line_88_col_30.log` in its `mutants.out`
  shows `wav.rs:88:30` (`/` to `%` in `WavSource::open`) failing
  `shuyi-sound::adapters a_16_bit_wav_gives_its_own_numbers`.
- The 6 unviable mutants don't build. Four are `Default::default()` for
  `Error`, `PcmFormat` or `WavSource`, which have no `Default`. One is
  `Ok(Default::default())` for `WavSource::open`. The last is `GRANULE_AT - 8`
  in `last_page`, a constant that underflows. Nothing needed killing.

Gates: no Rust changed. Ran the skill-loader check, `check-pins`,
`check-links`, `check-docs`, `ticket-lint`, `typos` and zizmor. zizmor's one
finding is the same as on `main`, the `changes` job's `uses:` line.

No break. No follow-up tickets. Nick input: none.
