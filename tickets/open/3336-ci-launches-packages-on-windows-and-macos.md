---
id: "3336"
title: CI launches exported packages on Windows and macOS and compares their windows with the raster
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3336 — CI launches exported packages on Windows and macOS and compares their windows with the raster

## Context

Only Linux ever draws a real window in CI. The `render` job
(`.github/workflows/ci.yml`, ticket 2018, ADR-0052) runs the render
card (2392) and Hello, Grid's capture plan under Xvfb with Mesa's
llvmpipe on `ubuntu-latest`. The `web-render` job (2019) draws in
headless Chromium. The `test` job runs on `windows-latest` and
`macos-latest`, but it runs `cargo test` only. No step there opens a
window.

The packages are only checked statically. CI's `export` job runs
`.github/workflows/export.yml` (the byte-equal copy of the file a
game's release runs, ticket 3050) for `windows`, `linux`, `web` and,
on pushes to `main`, `macos`. Its check script,
`tools/export-check/ci.sh`, runs `check.sh` (file lists, symbols,
MinGW DLL imports, the GLIBC floor, `codesign --verify`, the two Mac
slices), the exposure report twice, and `recipient.sh`. Nothing starts
the `.exe` or the `.app`.

So these failures reach players with every job green:

- a GL context miniquad can't create on Windows (a different context
  and DPI path than Linux's GLX);
- a `.app` that won't start once unzipped out of its build folder, or
  a pack it can't find in `Contents/Resources`;
- a colour, alpha, scale or letterbox difference on macOS's GL or
  Windows' WGL that the Linux run never sees.

Nick's machine is Windows with the GNU toolchain only, so macOS is
never tried by hand. ADR-0052 §5's only real-GPU numbers are one manual
table from his Windows desktop.

ADR-0052 §2 rules out GPU runners because they cost money and differ
between runs. That still holds. But two cells cost nothing on a public
repository and still add a second implementation:

- **`macos-latest`:** a GUI session, and Apple's own OpenGL (whatever
  the runner's VM gives; it may have no GPU acceleration, so the log
  must name it).
- **`windows-latest` with Mesa's llvmpipe `opengl32.dll`** beside the
  program (MIT). A Windows runner has no GPU, and its built-in
  `opengl32.dll` is GL 1.1, which miniquad can't use. Mesa beside the
  program wins the DLL search, so the program loads it.

Existing pieces this uses:

- The capture mode (`crates/app/src/capture.rs`, feature
  `shuyi-app/capture`): `SHUYI_CAPTURE=<plan>` runs the plan, compares
  each capture with the raster, writes `capture.txt` and the PNGs under
  `SHUYI_CAPTURE_OUT`, and exits 0, 1 or 2.
- Hello, Grid's feature `capture` (`examples/hello-grid/Cargo.toml`)
  and its plan, `examples/hello-grid/capture/frames.ron`. The packages
  already ship that plan: `tools/export-check/windows.txt` lists
  `{base}/capture/frames.ron`, and `macos.txt` lists
  `{base}/hello-grid.app/Contents/Resources/capture/frames.ron`.
- `shuyi export --with <feature>` turns on a feature of the game's
  package for one build (`ExportInput::with`,
  `crates/cli/src/contract/export.rs`).
- `calibrate.yml` runs a job N times, once that job lives in a reusable
  workflow that `ci.yml` calls.

The capture mode is never on in a shipped build (ADR-0052 §3). So this
ticket launches a second package of the same target, exported with
`--with capture`, from its unzipped archive. It has the same file list
as the shipped package. It is not the shipped program, and the ticket
says so in the ADR.

The audit behind this (2026-11-23, findings 51 and 77) proposed both
the launch and the comparison. They are one job here: a launch that
runs the plan is a smoke run and a picture check at once.

## Nick input

`None.`

## Scope

**In:**
- A reusable workflow `.github/workflows/launch.yml` with two cells,
  `launch (windows)` and `launch (macos)`. Each one:
  - exports Hello, Grid with
    `cargo shuyi export <target> --release --with capture --out dist/export-launch/<target>`;
  - unzips the package into a fresh folder under `$RUNNER_TEMP` (`ditto
    -x -k` on macOS, so the mode and the ad-hoc signature stay);
  - runs the packaged program from there, with `SHUYI_CAPTURE` naming
    the plan the package ships and `SHUYI_CAPTURE_OUT` an absolute
    folder;
  - fails on any exit code but 0, or on a timeout;
  - runs the render card too, built as `render` builds it, on the same
    GL (it has no package);
  - writes `capture.txt` into the job summary and uploads every picture
    as `launch-captures-<target>`, as `render` does.
- **Windows' GL:** Mesa's llvmpipe from MSYS2
  (`mingw-w64-x86_64-mesa`, already the toolchain's package manager in
  `export.yml`). Its `opengl32.dll` and the DLLs it needs are copied
  beside the unzipped `hello-grid.exe`, never into the package itself.
  If MSYS2's build can't make a context, use the pinned release of
  `pal1000/mesa-dist-win` (MIT), checked by SHA-256. If llvmpipe still
  fails, try `GALLIUM_DRIVER=d3d12` (Mesa over WARP). Record which one
  worked, and why the others didn't, in the ADR.
- **The renderer in the log:** the capture mode logs the context's
  `ContextInfo::gl_version_string` (miniquad 0.4.11,
  `graphics.rs`) and backend once, at start, after `PREFIX`, and writes
  it as the first line of `capture.txt`. Linux's `render` job gets it
  too. This is what `glxinfo -B` gives `render` today, for systems that
  have no `glxinfo`.
- A script `tools/export-check/launch.sh <target> <game folder>` holds
  the unzip, the run and the checks, so the workflow stays short and
  the script runs by hand on a Mac.
- `check.sh` runs on the capture package too, so its file list is
  proved the same as the shipped one's. If one of `check.sh`'s other
  checks refuses the capture build for a reason only a capture build
  has, add a switch that runs the file list alone. Never loosen a
  check of the shipped package.
- `ci.yml` calls `launch.yml`:
  - `windows` on every PR, under `needs.changes.outputs.code`;
  - `macos` on pushes to `main`, as the `export` job's `macos` leg does
    (the same `exclude` rule), since a macOS build of the universal app
    is the slowest leg.
- `calibrate.yml` offers `launch` as a job, and its runs measure the
  flake rate and the pixel numbers.
- Each new job has `timeout-minutes` (Xvfb-free GUI runs can hang).
- **An ADR that amends ADR-0052 §2:**
  - the two new cells, and what each is;
  - the measured table, as §5 has for llvmpipe: pixels over and the
    worst channel difference per capture, per cell, from at least 10
    calibrate runs;
  - the tolerance. The default (`Tolerance::default()`) is kept if it
    holds. A looser one is named in the plan only if a cell measurably
    needs it, under §5's limits, with the numbers;
  - that the program launched is a capture build of the package, not
    the shipped one, and why;
  - that neither cell is a player's GPU, so §2's reason against GPU
    runners still stands.
  ADR-0052's `Status:` line names the new ADR, and both rows go in
  `docs/adr/README.md` (the `write-adr` skill, step 5).
- `docs/targets/desktop.md`: its Windows and macOS rows say CI launches
  the package and compares its window with the raster, and on what GL.
- **Either order with 3340** (desktop and web frame times measured): if
  3340 is done, the Windows and macOS legs also run
  `cargo shuyi bench --wall` for Hello, Grid in the `wall-times.yml`
  workflow and upload `wall-report.*`. If not, nothing to do (3340 adds
  the Windows and macOS runs to its workflow with this ticket's way of
  opening a window).

**Out (do not do):**
- GPU runners, paid runners, or `windows-11-arm` (Prism). The ADR may
  name `windows-11-arm` as a later cell; no ticket here.
- Native Wayland, or a Linux launch from the package: `render` already
  runs the same program under Xvfb.
- Launching the shipped (non-capture) program. It has no way to stop
  itself, and ADR-0052 §3 keeps the capture mode out of it.
- Changing `export.yml`. It is byte-equal to the template's
  (`crates/cli/src/contract/new/tests.rs` checks it), and a game's
  release has no plan to run.
- Making the new cells required checks on `main`: branch protection is
  Nick's (decision ticket 3529).
- A lint that every CI job has a timeout, and CI's cost ledger: 3508.
  This ticket sets its own jobs' timeouts only.
- What a target's support tier promises: 3338.
- The web build in Firefox and WebKit: 3507.
- Measured frame times on devices: 3340.
- Signing and notarization: 3051.

**Estimated size:** about 900 lines (code 80, CI workflows and scripts
400, tests 120, docs 300).

**Split:** past 800 lines, split off the Windows cell (the Mesa set-up
and its half of the ADR's table) into a new ticket, leaving the macOS
cell.

## Implementation steps

1. Read ADR-0052, `crates/app/src/capture.rs`,
   `.github/workflows/ci.yml`'s `render` and `export` jobs,
   `.github/workflows/export.yml`, `.github/workflows/calibrate.yml`
   and `tools/export-check/`.
2. In `crates/app/src/capture.rs`, log the renderer line at start and
   write it first in `capture.txt` (from `get_internal_gl()`'s
   context's `info()`). Add the line to the `render` job's expectations
   if any step reads `capture.txt`.
3. Write `tools/export-check/launch.sh`: export with
   `--with capture`, run `check.sh` on that package, unzip it to a
   fresh folder, run the program with the shipped plan, print
   `capture.txt`, and fail on a nonzero exit. On Windows, copy the Mesa
   DLLs beside the unzipped program first.
4. Write `.github/workflows/launch.yml` (inputs: `target`, `host`,
   `dir`). Set up the target as `export.yml` does (MSYS2 GCC and the
   GNU target on Windows; both Mac targets), then Mesa on Windows, then
   `launch.sh`, then the render card, then the summary and the upload.
   Set `timeout-minutes` on the job.
5. Call it from `ci.yml` with the matrix rule above. Add a `launch`
   block to `calibrate.yml` and add it to `runs`' list.
6. Push and read the first runs. Get a window on each cell (the
   Windows fallbacks in order). Then run `calibrate.yml` with
   `job=launch`, `count=10` on each cell, and take the numbers.
7. Write the ADR with the table, and set the tolerance from it.
8. Update `docs/targets/desktop.md`.

## Acceptance criteria

- [ ] `launch (windows)` runs on a PR, and `launch (macos)` on a push to
      `main`. Each exports Hello, Grid with `--with capture`, unzips it,
      runs the packaged program, and passes every capture of its plan.
- [ ] Each cell also runs the render card and passes it.
- [ ] The package each cell launches has exactly the files of
      `tools/export-check/<target>.txt`.
- [ ] The log and the first line of `capture.txt` name the GL version
      and backend, on both new cells and in `render`.
- [ ] A plan that drew `Item::Rect` one window pixel to the right fails
      on each cell, with its pixel numbers recorded in the ADR (as
      ADR-0052 §5 records them for llvmpipe).
- [ ] A program that can't start (a missing pack, tried once by hand by
      deleting it from the unzipped folder) fails the cell with a
      nonzero exit, not a pass.
- [ ] `calibrate.yml` accepts `job=launch`. Ten runs per cell are in
      the ADR's table, with the flake rate.
- [ ] The new ADR amends ADR-0052 §2, ADR-0052's `Status:` line names
      it, and both rows are in `docs/adr/README.md`.
- [ ] No step changes `export.yml`, and the template test of it still
      passes.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the renderer line's text in `capture.txt` (the report's writer,
  in `crates/app/src/capture/tests.rs`).
- Integration: the two `launch` cells themselves, and their 10
  calibrate runs each.
- Manual, once: the missing-pack failure, and the shifted-rect failure,
  each recorded in the completion notes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
