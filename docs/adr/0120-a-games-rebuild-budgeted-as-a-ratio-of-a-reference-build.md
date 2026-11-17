# ADR-0120: A new game's rebuild is budgeted as a ratio of a reference build on the same runner

- **Status:** Accepted
- **Date:** 2026-11-15
- **Supersedes:** ADR-0049 §2 (the gate in seconds); its §1 and §3–5 stand
- **Related tickets:** 2070; 2009 (the gate), 2064 (calibration runs)

## Context

ADR-0049 §2 failed CI's `compile-times` job when a new `grid` game's
median rebuild after a one-line change took more seconds than
`tools/compile-budget.ron` gave the runner's OS: the slowest of ticket
2009's measures plus 50 %, 1.6 s on Linux and 2.9 s on Windows. It failed
on code that hadn't changed: PR #347 (CSS only) twice on Linux at 1.65
and 1.70 s, where `main` had measured 1.06 to 1.57 s the same day, and
PR #443 on Windows at 2.94 s.

Ticket 2070 named the runner's CPU model and cores in the job's table and
timed, just before each rebuild, the clean build of a fixed reference
crate (`compile_times::reference_source`: 120 structs with derives, no
dependencies, the template's `debug = false`). Twenty runs of the job on
unchanged code (calibration run 43775373036, at 9ac070d6), in seconds:

| Runner's CPU | Linux runs | Linux rebuild | Windows runs | Windows rebuild |
| ------------ | ---------- | ------------- | ------------ | --------------- |
| AMD EPYC 9V45 | 2 | 0.87, 0.98 | 1 | 1.54 |
| AMD EPYC 9V74 | 12 | 1.00 to 1.16 | 9 | 1.89 to 2.07 |
| Intel Xeon Platinum 8573C | 2 | 1.20, 1.22 | 4 | 2.04 to 2.72 |
| Intel Xeon Platinum 8370C | 1 | 1.24 | 1 | 2.50 |
| AMD EPYC 7763 | 3 | 1.30 to 1.33 | 5 | 2.49 to 2.66 |

Every runner had 4 cores. Within one run the three rebuilds agree within
a few hundredths; between runs they spread by half (Linux 0.87 to
1.33 s, Windows 1.54 to 2.72 s), and the CPU model the runner drew
explains it. What else was measured, over the same 20 runs of each OS:

| Measure | Linux: lowest to highest | Windows: lowest to highest |
| ------- | ------------------------ | -------------------------- |
| Median rebuild, seconds | 0.87 to 1.33 (×1.53) | 1.54 to 2.72 (×1.77) |
| Clean build, seconds | 43.6 to 65.0 (×1.49) | 89 to 154 (×1.73) |
| Rebuild over the clean build | ×1.13 | ×1.31 |
| Rebuild over the `check` rebuild | ×1.27 | ×1.28 |
| Median rebuild over median reference | 1.069 to 1.167 (×1.09) | 1.052 to 1.232 (×1.17) |
| Median of each rebuild over the reference just before it | 1.069 to 1.171 (×1.10) | 1.136 to 1.232 (×1.08) |

The last is the steadiest: on Windows one run's reference builds
disagreed (1.82, 2.13, 2.21 s), and pairing each rebuild with the
reference of the same moment keeps that run in line.

## Decision

1. **`cargo xtask compile-times`** also writes a reference crate
   (`<dir>/reference`, its own workspace, no dependencies) and, before
   each of the three rebuilds, times its clean `cargo build` in a fresh
   target folder. The table and the JSON name the CPU model
   (`/proc/cpuinfo`'s `model name` on Linux, the registry's
   `ProcessorNameString` on Windows), the cores
   (`available_parallelism`), the reference builds and the ratio.
   `--out <file>` writes the JSON to a file, which the job uploads.
2. **The gate** holds `Times::ratio`, the median over the three pairs of
   the rebuild's time over its reference's, to
   `tools/compile-budget.ron`'s `rebuild_over_reference` for the
   runner's OS. The seconds of ADR-0049 §2 are gone; the dependency count
   stays as it was.
3. **The budget** is the highest ratio measured over at least 20 runs of
   the job on unchanged code, with `calibrate.yml`, plus 20 %, about
   twice the ratio's spread. The file names the calibration runs beside
   the numbers. A slower rebuild is still found and fixed, not budgeted:
   a number goes up only with a new calibration behind it.
4. **The job** lives in `.github/workflows/compile-times.yml`, which
   `ci.yml`'s `compile-times` job calls and `calibrate.yml` offers
   (`-f job=compile-times`), as ADR-0049's job did inline.

## Consequences

- The gate no longer fails on the CPU the runner drew. A second
  calibration (run 43776381949, 20 runs at 0f8c48fa) passed every run:
  the ratio stayed within 1.066 to 1.174 on Linux and 1.153 to 1.235 on
  Windows, while the seconds spread 0.76 to 1.35 and 1.39 to 2.84, on
  two CPU models the first calibration hadn't drawn (Intel Xeon 6975P-C
  and 6973P-C).
- It still fails a real slowdown: with `debug = true` in the template's
  `dev` profile, the change ADR-0049 took out, every run of a
  calibration on its own branch failed (run 43776383981, 3 runs: ratios
  2.395 to 2.678 on Linux, 2.606 to 2.986 on Windows).
- A change that slows the reference and the rebuild alike is not caught:
  a new Rust release that is slower everywhere, or a slower linker on
  the runner image. That is not the engine's code, which is what the gate
  is for; the table's seconds still show it.
- The job runs three more clean builds of a small crate, about 3 s on
  Linux and 6 s on Windows.
- The budget is two numbers without a unit a developer reads directly;
  the table keeps the seconds beside it.

## Alternatives considered

- **Raise the seconds** — a slowdown up to the new number would pass on
  a fast runner, and the next slow CPU would fail again (the ticket's
  *Out*).
- **Pin the runner's CPU, or a larger runner** — GitHub's hosted runners
  don't choose a CPU model, and a larger runner costs money.
- **The rebuild over the clean build** — the clean build is mostly the
  dependencies, so it moves with the engine's own size and its
  dependencies, not only the runner; on Windows it spread ×1.31.
- **The rebuild over the `check` rebuild** — `check` takes 0.07 s on
  Linux, too short to time steadily (×1.27), and it misses a front-end
  slowdown that both share.
- **The median rebuild over the median reference** — the same on Linux,
  but a run whose references disagree spreads it to ×1.17 on Windows.
- **A budget per CPU model** — new models appear without notice, and each
  needs its own 20 runs.
