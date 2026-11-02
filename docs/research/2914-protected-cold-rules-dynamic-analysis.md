# Test a cold rules kernel against dynamic analysis

Research for ticket 2914, 2026-11-01. This experiment uses only the
author-owned synthetic `workshop` fixture from [2912](2912-game-cloning-threat-paths.md).
It does not contain a rule or asset from *Visions of Shuyi*. The candidate is
one rare integer predicate, not a whole-game virtual machine or a production
module.

## Kernel and matched builds

The restricted interface is `gather_bonus(gathers: u8, bonus_phase: u8) ->
bool`. It has no I/O, clock, floating point, allocation or device access. The
turn simulation calls it only after an accepted `Gather`; its result awards a
bonus ore. The three arms differ only in this calculation:

| Arm | Calculation |
| --- | --- |
| Ordinary baseline | Direct integer sum and divisibility by three. |
| Layout control | The same direct arithmetic, plus a touched, seed-generated 256-byte layout table that cannot change the answer. |
| Protected candidate | A generated stack bytecode program for the same predicate, with a per-build shuffled opcode assignment, encoded constant and variable no-op padding, run through a small interpreter. |

`SHUYI_KERNEL_BUILD_SEED` reproduces the generated layout and bytecode for a
given source and toolchain. The study builds two seeds of each diversified arm
and checks that native and browser-target Wasm
bytes differ. All arms use the same `Workshop` state, commands, JSON trace,
ASCII scene and full-state hash. `Courier` stays unchanged as a negative
control. The browser target is a `wasm32-unknown-unknown` library with a small
HTML/JavaScript bridge; it is a real browser build of this fixture, not a
Shuyi game export. The earlier 2912 `wasm32-wasip1` command-line module was not
a browser target.

The parity runner generates private seeded traces in the 2912 random-plus-win
shape and adds guaranteed deadline loss, rejected-action and bonus-boundary
families. It checks the complete JSON record at every step, including the
state hash and SHA-256 of the ASCII scene. The latter is a fixture scene
proxy, not `shuyi_ui::Scene`. CI's headless browser gate replays
win, loss and refusal paths for each of the three arms. The kernel unit test
exhausts all `u8` input pairs.

## Dynamic attacker protocol

Each assigned analyst gets one arm's native executable, browser Wasm, browser
page and bridge, the same Node V8 Inspector debugger helper, the same metered
batch-oracle helper and the same instructions. The debugger helper disassembles
the WebAssembly, breaks on the live `shuyi_run` export and records a replay.
It must be used at least once. Analysts submit an independently runnable
program that reproduces full Workshop traces. A wrapper that invokes or embeds
the delivered program is invalid. The grader compares it with the same
precommitted, held-out cases for all arms; it reports exact transitions,
traces, mandatory win and loss paths, and runtime failures. A successful
ordinary baseline is required before any protection delay can be credited.

The assigned caps are four elapsed hours, 500 model/tool steps, 1,200 oracle
action queries and US$100 in direct spend per arm. The query cap is above the
912 queries reported by 2912's successful ordinary pilot; it raises 2912's
proposed 500-query cap before exposure so the ordinary task remains plausibly
solvable, then applies the same limit to every arm. The supplied
SQLite-backed oracle refuses a call that would exceed its cap. Because the
executables themselves are delivered for dynamic analysis, direct runs and
debugger calls also count and must be logged by the analyst; this local meter
cannot prevent bypass or ledger deletion. Model token counts and charges are
unmeasured when the harness does not expose them.

The repository and `target/` remain visible to all agents in this shared
worktree. Bundle allowlists, hashes and marker checks catch accidental source
or grader delivery, but they do not create hard source isolation. Any local
agent result is therefore an **unblinded protocol rehearsal**. Node Inspector
can dynamically debug the browser module on this host. GDB, Ghidra and REA
are not installed, so a native debugger or REA-specific comparison is not
measured. The saved attempt logs do not establish exact analyst model and
effort parity, so that matching condition is unverified. An unsolved candidate
under those gaps would give no evidence of protection.

## Results

The private case seed was committed before assigning an attacker:
SHA-256 `5c083397e0c39088e876cc4ae036b82171daaa1f0d8b3c3fb1c589a1734797a8`.
After all submissions, the seed was disclosed as
`9ed80e644ff4b5db3895c131625c63d8`.
The frozen runner was
`e70561bf0ab0e6c22457a806fb8b57820b312f33b3f7a8d02b48566a712ebf28`;
the bundle file hashes were recorded in its evaluator manifest. The key
first-seed artifacts were:

| Arm | Native SHA-256 | Browser Wasm SHA-256 |
| --- | --- | --- |
| Ordinary | `cba3960e70dd67bbc6245d8c5a6e2aa19f6f440a9197d757e5ebad0bbd585cd2` | `935b89bc95e3c785d7390716db2d4badd1ffb191ed7f7e8ae3f7940feb6d43dc` |
| Layout | `e30e046fcc391504882fee294c2cc49011789ce2cd4e84e183ea8de12a1772c1` | `ce894a0c3b511cd43a5e577407fc580fc1a4d6172d0b0a4702f2ffb711d6cd44` |
| VM | `57d35d60bd9c188ab0599dda57c875d52e3c2e290d21d72a8cb826c113e2a94e` | `f2bb9c9e4f9267a4904800aadf2951a810719130badddc7b6117d6e7022b0c40` |

These hashes identify the frozen executables delivered to the analysts.
Subsequent test-coverage edits to this repository can change fresh binary
hashes without changing the recorded artifacts or their measured outcomes.

All **39** held-out Workshop traces passed on five native and five browser
Wasm builds: 20 random, one mandatory win, six deadline losses, six
rejected-action paths and six bonus-boundary paths. They held **425**
transitions. Every complete JSON trace matched the ordinary build, including
**4,640** matching state-hash records and **4,640** matching ASCII scene hashes
across the ten builds. The unchanged Courier game matched in 50 additional
native control runs. Two layout seeds and two VM seeds produced different
native and Wasm bytes. A separate native/Wasm test matched 104 traces,
including the largest `u64` seed. The actual HTML/browser replay step passed
in [CI's `web-render` job](https://github.com/Zafnok/shuyi-engine/actions/runs/41375368403/job/125054357058).
Local Chrome and Edge headless launches did not complete on this Windows host,
and local Playwright was unavailable.

The ordinary analyst's saved Node Inspector trace disassembled 275 functions
(52,540 WAT lines) in its browser-target module, hit a live breakpoint in
`shuyi_run`, and returned the seed-0 initial replay record. The VM trace
disassembled 276 functions (52,897 lines) and also hit that breakpoint. All
three assigned analysts recorded live Wasm breakpoints. Node reported no Wasm
coverage entries, so the breakpoints and outputs, not its empty coverage
list, establish dynamic execution.

| Arm, first seed | Native bytes | Browser Wasm bytes | Native process + replay median / max | Node + Wasm process + replay median / max | In-process 11-action replay median / p99 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Ordinary | 351,744 | 150,173 | 32.04 / 136.94 ms | 182.24 / 257.38 ms | 600 / 700 ns |
| Layout | 352,256 | 150,438 | 37.68 / 183.95 ms | 243.56 / 2,533.45 ms | 600 / 700 ns |
| VM | 352,768 | 151,440 | 32.91 / 131.92 ms | 216.00 / 319.24 ms | 600 / 700 ns |

Each process cell has 30 runs on this Windows GNU host. The in-process cell
has 20,000 timed replays after 1,000 warmups and includes timer overhead.
The second seeds had the same native byte sizes and distinct hashes; their
browser modules were 150,438 and 151,442 bytes. Process starts on this host
had large outliers, and the 100 ns resolution shown by the in-process results
cannot resolve a small kernel difference. These are **not** interactive frame
p99, a game's browser startup, player completion or blind quality. Thus the
2912 player-cost gate remains unassessable, regardless of the small byte
increase. The VM's first-seed package grew by 1,024 native bytes (0.29%) and
1,267 Wasm bytes (0.84%).

One model-availability HTTP 403 prevented the first ordinary attacker from
receiving an artifact. It is a setup failure, not a protected outcome. A fresh
ordinary attempt and one attempt in each other arm were gradable. All three
submitted independently runnable Python replacements. Manual inspection found
their own transitions, seeded phase and hash logic, with no import, launch or
relay to a delivered artifact. Every original bundle file still matched the
frozen manifest before scoring, and every submission was scored once.

| Arm | Successful / assigned | Final submission from assignment | Exact transitions | Exact traces | Mandatory win / losses | Confirmed action queries |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Ordinary | 1 / 2 (one setup failure) | 21 min 49 s | 425 / 425 | 39 / 39 | yes / 6 of 6 | 237 (236 metered + 1 debugger) |
| Layout | 1 / 1 | 14 min 43 s | 425 / 425 | 39 / 39 | yes / 6 of 6 | 181 (180 metered + 1 debugger) |
| VM | 1 / 1 | 15 min 27 s | 425 / 425 | 39 / 39 | yes / 6 of 6 | 720 (719 metered + 1 debugger) |

The SQLite ledgers independently confirmed 39/236, 28/180 and 62/719
official calls/charged actions in ordinary, layout and VM arms. The direct
debugger run adds one query per arm. The ordinary analyst's later stalled
oracle command did not reserve a ledger entry. Its late shell stalls are
included in the 21:49 elapsed time. Layout had three failed non-query shell
commands and one charged probe whose output was lost, then reproduced. VM had
one failed Windows Python alias launch before querying. All three used the
common Wasm disassembler and a live breakpoint. Known local direct charges
were US$0; model charges, total model/tool steps and human minutes were not
metered. No attack was classified as refusal, context failure, timeout or
invalid submission; one assigned startup failed before exposure.

The **measured VM-minus-ordinary added delay is −6 minutes 22 seconds**:
the VM submission came sooner. Relative to the layout control, the VM took
**44 seconds longer**. These are final-submission times, not independently
clocked first-valid times. The VM analyst used more queries but still
recovered the seeded rule and full state hashes inside the cap. With one
gradable analyst per arm, unequal shell stalls and no hard source isolation,
the small differences do not estimate a population effect. They do establish
that this particular diversified interpreter did not survive a dynamic
analyst with executable access and a live Wasm debugger in this rehearsal.

**Recommendation: stop production work on this candidate.** Its measured
absolute delay is negligible against 2912's minimum seven-day rule; the
observed difference is negative against the ordinary arm. The VM adds 1,024
native bytes and 1,267 browser Wasm bytes, while the in-process 11-action
replay reads 600 ns median / 700 ns p99 in both ordinary and VM builds at
this timer's resolution. Native process-plus-replay medians were 32.04 and
32.91 ms; Node-plus-Wasm medians were 182.24 and 216.00 ms, with large
outliers. Interactive frame p99, actual browser startup and player quality
were not measured, so this is not a pass of the player-cost gate. The negative
delay suffices for a no-go without claiming any offline client opacity.

## Decision rule

The proposed [2912 decision rule](2912-game-cloning-threat-paths.md#proposed-confirmatory-decision-rules)
requires added delay of at least `max(7 days, ceil(D / 4))` for a public-demo
window of `D` days, with actual player costs inside its limits. A four-hour
trial can reject a candidate defeated inside the cap. It cannot demonstrate
seven days of delay. A negligible measured delay ends production work on this
candidate; a timeout only justifies a longer, hard-isolated and fully tooled
trial after successful baselines.

## Reproduce

From the repository root, with the pinned Rust and `wasm32-unknown-unknown`
target installed:

```sh
python tools/research/2914/study.py prepare
python tools/research/2914/study.py compare
python tools/research/2914/study.py cost
python tools/research/2914/study.py freeze
python tools/research/2914/check_bundle.py baseline
python tools/research/2914/study.py score --arm baseline --submission path/to/solution.py
```

`prepare` writes bundles and grader material under `target/cold-rules-study/`.
Keep that directory out of an analyst's access in any later blind trial. It
does not alter engine crates or a game's export.
