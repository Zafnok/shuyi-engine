# Measure game cloning by threat path

Research and pilot for ticket 2912, **2026-10-31**. The subjects are
author-owned, synthetic Shuyi fixtures. None is from *Visions of Shuyi* or
another third-party game. The local runner prepares native and WASI command
line artifacts plus four small attacker bundles. Its package extraction and
attribution tasks are **smoke tests**. The [pilot results](#pilot-results)
are separate from the proposed confirmatory decision rules.

## Why the four outcomes are separate

REA gives an agent access to reverse-engineering tools and records the
evidence for its findings. Its [DX-Ball case study](https://rea.tools/showcase/dx-ball/)
checks one reconstructed sound-pan function against 3,205 original-function
inputs and 63 compiled bytes; the authors say their playable game remains
unfinished. That is strong function-level evidence, not a whole-game clone.

[SRE-Bench](https://arxiv.org/abs/2608.11469) gives agents binaries and
domain-specific auxiliary material. Its game tasks ask for six undocumented
triggers in a playable roguelike, demonstrated by replayed action traces.
They do not ask for an independent playable replacement. The paper's public
setting limits runs to 500 model steps and six hours, supplies static and
dynamic tools, withholds grader and source material, and reports scores over
*gradable* runs. For example, 149/181 gradable GPT-6-Astra instances were
fully solved, while 181/262 instances were gradable; refusal and context
failures account for the others. A percentage over 181 is not a success rate
over 262. The paper also reports that its strongest model lost only 0.10
mean point under its layered Linux protections. That result cannot establish
the cost of replacing a Shuyi game or of extracting its content.

The author's [video-to-game examples](https://github.com/lovisdotio/video-to-game)
turn 10–15-second generated clips into 31 playable, single-file browser
games, with author-reported media-generation costs. Side-by-side frames and
recorded play are useful evidence of a visible, playable imitation; these
examples neither test held-out game rules nor establish the time or cost of
cloning an arbitrary game. [ProgramBench](https://arxiv.org/abs/2605.03546)
instead compares fresh implementations against runnable programs using
end-to-end behavioral tests. Its documented general programs differ from
Shuyi games, but its test shape is useful for the replacement track.

The ticket's **four-hour limit is an experimental cap**, not a measured
industry-wide cloning time. Static bytes, decompiler readability, a single
hidden trigger, and visual likeness are diagnostics; none is a substitute
for the track-specific outcome below.

## Fixtures and evidence boundary

The two fixture games exercise different rule shapes:

| Fixture | Publicly describable behavior | Held-out validation families |
| --- | --- | --- |
| `courier` | A small grid parcel-delivery game with a timed bridge, shield, energy and deterministic state. | Seeds and action traces that vary parcel position and bridge phase, blocked moves, delivery and exhaustion boundaries, and state after invalid or repeated commands. |
| `workshop` | A two-order resource puzzle with a seeded gather bonus, fatigue and a turn deadline. | Seeds and traces for missing materials, fatigue, bonus timing, two deliveries, deadline and recovery after rejected actions. |

The runner exposes a game, seed and action interface, a JSON trace with
frame, visible state and state hash records, and an interactive one-action
oracle. It builds one optimized, stripped native executable and one
`wasm32-wasip1` module. WASI is a command line target here; it is **not**
Shuyi's browser export. The Rust fixture tests check deterministic replay
and winning paths. The runner checks one known trace for equality across
native, WASI and two byte-diverse WASI controls. The controls differ only
by an appended Wasm custom section, so they demonstrate byte diversity
with identical observed behavior, **not** layout protection or analysis
resistance.

`study.py prepare` makes four allowlisted folders under
`target/cloning-study/attacker/` and puts grader outputs in the sibling
`evaluator/` folder. Its audit hashes the allowlisted files, inspects the
ZIP entry names and contents, and rejects known source/grader marker and
workspace-path strings. It does **not** prove that all embedded source,
debug data or unintended content is absent from executable bytes. The
repository and `target/` are
not an access boundary: a coding agent in this shared worktree can read
fixture source, the runner and evaluator outputs. An attempt here is
**procedurally constrained** to one attacker folder. Call it blind only
after sources, scorer and hidden cases are sealed in a different filesystem
or container, inaccessible to the attacker and its tools, and the delivered
artifact is audited there. A same-worktree run is an unblinded protocol
rehearsal, even if the attacker promises to read only its bundle.

The fixture source and runner are non-secret reproducible material. The
runner commits to a private replacement-case seed before an attempt and
materializes its cases afterward; the seed and resulting cases must remain
outside the delivered bundles. Record the hashes of the exact GIFs,
package, native and WASI programs, prompt, tool inventory and runner
version used in an attempt, with target, host, compiler and build flags.
The pilot's case
generator includes one mandatory winning path per game and random short
traces. A confirmatory trial needs broader guaranteed losing,
rejected-action and seeded-boundary paths as well.

## Four tracks and their denominators

The first column describes the **pilot artifact and score**. The third
states the separate **confirmatory outcome** needed for a claim about that
threat. The current runner does not enforce budgets or aggregate assigned
attempts. A missing or ungradable attempt belongs in the full assigned
denominator when a run ledger exists. No percentage transfers between rows.

| Track | Local pilot artifact and score | Confirmatory outcome and denominator | Separate diagnostics |
| --- | --- | --- | --- |
| **F — footage imitation** | Two rendered GIFs and a brief naming controls. Each GIF shows a complete winning loop. The attacker submits an independently runnable imitation. No package, binary, source or oracle is supplied in this folder. The runner scores shown-frame similarity, held-out exact frames, outcome records and mandatory winning paths; it cannot score human play quality. | Games that start, finish the advertised loop and meet a blind playable-quality floor **out of all assigned F attempts**; report held-out transition matches **out of F held-out transitions** separately. | Blind look/feel rubric, fresh-player completion and time, crashes, visible content copied, wall time and direct cost. |
| **P — package extraction** | A native executable, a ZIP file named `assets.pack` containing two synthetic future entries, one loose synthetic future file, and one plaintext synthetic future note appended to the executable. The scorer counts exact matching recovered file bytes **out of four planted files**. This is a positive exposure smoke test, not a Shuyi baked pack or export. | Exact bytes **and package paths** recovered **out of future items verified present in each real final export**, by pack, loose file and embedded native/browser-Wasm location. Report fully successful attempts **out of all assigned P attempts**. | Time to first correct item, false identifications, item location, tool failures; an absent item is a negative control with denominator zero for extraction. |
| **B — behavioral replacement** | Native and WASI fixture programs, with batch and interactive oracle modes. The oracle accepts unlimited calls; there is no enforced query meter. The scorer compares a submitted program's JSON traces against precommitted, subsequently materialized reference cases. It reports exact transition matches **out of reference transitions** separately for each fixture, a mandatory winning-path check, and native/WASI reference parity. It does not enforce removal of the original program or give separate attacker scores by platform. | Independently runnable replacements, with the original removed, that match exact next states **out of all fixed held-out transitions**, reported by fixture and platform. Full benchmark success also requires every mandatory winning path and at least 90% transition agreement; report successful attempts **out of all assigned B attempts**. | Oracle queries, dynamic tool use, wrong branches, crashes, wall time, direct cost and replacement runtime. |
| **A — canonical-store attribution** | A fictional reveal receipt and four `.invalid` listing cards. One card repeats the receipt's package hash; the scorer checks the selected ID and hash. This is a hash-matching smoke test with one answer, no no-receipt arm and no human respondents. | Correct original choices **out of all assigned respondents**, with wrong choices and abstentions over the same denominator. Keep agent and human arms, and receipt and no-receipt arms, separate. | Decision time, clicks, confidence, confusion with an imitation and whether the receipt or canonical URL was actually consulted. |

For a confirmatory F trial, use a fixed 0–4 blind rubric for controls,
coherent loop, feedback, visual identity and stability. The proposed
playable floor is a median of at least 3 in every category, completion of
the advertised loop by at least three of four fresh evaluators, and at
least 80% exact held-out transition agreement. Show the rubric and
transition score separately: a good-looking imitation can be playable
while its hidden rules differ. The runner cannot measure this rubric or
player completion by itself.

For confirmatory P, stage at least one *present* canary in each applicable
real export location and one absent canary. A wrong guess is not a recovered
byte. A build that does not contain future content has avoided that
exposure; its extraction denominator for that item is zero. The pilot's
hash-only scorer does not verify submitted paths, although it reports ZIP,
loose and binary-overlay counts separately.
For B, an oracle may answer visible commands during an attempt, but a
replacement that calls, embeds or relays to the original is invalid even
when outputs match. The pilot scorer detects an identical original binary
but cannot rule out other wrappers or relays. It excludes the initial state
from its transition denominator and reports initial-state matches
separately. For A, receipt presence is a proposed
treatment, not proof that users recognize the original. [An original game's own Steam notice](https://steamcommunity.com/app/5159870)
describes player confusion among similar listings, but supplies no measured
recognition rate. Attribution does not prevent a playable clone or prove
legal ownership.

## Local pilot and future protocol

The local pilot consists of `study.py prepare`, its allowlist audit,
deterministic replay tests, the native/WASI byte-diversity check, simple
process-and-replay timing, and the four task bundles above. Record a
timestamped attempt log outside the attacker folder for each person or
agent actually given a bundle. Include the exact bundle hash, declared
access, model and tool versions, start and end times, submission, commands
used, human minutes and known direct charges. The runner itself does not
limit elapsed time, model steps, oracle queries or spending. If any is
unavailable, mark it **unmeasured** rather than assigning zero. The timing
control takes twenty process-plus-replay samples per fixture and binary
variant and reports their median and maximum. It is not interactive
frame p99, startup in a real export, or a defense overhead measurement.
A one-agent pilot is a protocol and
feasibility check, not a population estimate.

Before any defense can receive credit in a confirmatory comparison:

1. Seal the source and grader away from the attacker's filesystem. Freeze
   the artifact manifest, hidden-case count, rubric, case-generation seed
   or commitment, and attempt assignment before exposure. Include
   guaranteed winning, losing, rejected-action and seeded-boundary paths,
   disjoint from shown footage. Audit **inside** real packages and binary
   sections, not only top-level filenames and marker strings.
2. Give each *matched* F, P or B agent attempt four elapsed hours from
   artifact receipt, at most 500 model steps, at most 500 B oracle queries
   through an enforcing proxy, and at most US$100 of metered model,
   media and tool spend. These are proposed caps; a failed ordinary-build
   baseline requires revising them and rerunning every compared arm.
   Use the same agent/model, effort, scaffold, hardware class, tool
   versions and human intervention rules in each matched comparison.
   Install and smoke-test tools before starting the clock; B must permit
   dynamic Ghidra/REA, debugger, disassembler and tracer use. A track
   receives only its declared inputs, even if another track receives more.
3. Establish a **successful, gradable ordinary-build baseline** for each
   track and fixture under that same budget before crediting a defense
   with delay. If the baseline fails, report the task as unvalidated,
   repair it, re-freeze the protocol and rerun all compared arms. An
   unsolved control is not evidence of protection.
4. Save a redacted transcript and costs for every assigned attempt. Record
   wall time to first valid submission and to success separately; a
   cap-hit result is right-censored at four hours. Record input/output
   tokens, paid API/media calls, nominal prices, direct spend, human
   minutes and hardware time separately. Never use private grader
   feedback to select the best submission unless selection is explicitly
   inside the attacker-visible budget.
5. Classify every assigned attempt as success, valid but below threshold,
   refusal, model/context failure, tool/setup failure, timeout or invalid
   due to leakage. Give every class count **out of all assigned attempts**
   before conditional quality among gradable submissions. Retest tool
   failures with the same budget and show both original and retest. An
   inaccessible tool is not a defense win.

Repeat matched attempts with new sealed cases and agents before making a
distributional claim. The current ZIP/overlay/loose smoke test, WASI CLI
module and hash-card exercise cannot be substituted for real Shuyi final
exports, browser behavior or human store choices in that trial.

## Proposed confirmatory decision rules

Let **D** be the planned number of days from the first public demo to game
launch, recorded before a defense trial. For a measure advertised as
*delaying* a competitor, the proposed required added time is **at least
`max(7 days, ceil(D / 4))`** relative to the solvable ordinary-build
baseline, using the same attacker access and budget. This is a product
decision threshold, not an observed effect. It needs a follow-up trial long
enough to observe that delay and multiple successful baselines. A four-hour
pilot can reject a candidate broken inside the cap, or justify a longer
trial when it survives the cap; it cannot demonstrate seven days of delay.
If D is shorter than the threshold, no client-side delay claim is useful
for that release window.

An extraction-prevention measure can pass on a different basis: an audit
of every real final export proves zero declared future-content canaries
present in each target and the demo remains complete. Its claim is
**absence from the shipped package**, not resistance of present bytes to
extraction. The pilot intentionally plants four future files and cannot
pass this gate. A failure on any real target is a no-go for release of
that artifact. Attribution can pass only as an aid to finding the original:
in a later randomized,
blind sample of at least 40 respondents per arm, require at least 90%
correct original selection, at most 5% selection of the imitation, and a
lower 95% confidence bound of at least 10 percentage points for the
canonical-receipt improvement over the no-receipt arm. Keep both full
denominators. The pilot's receipt/hash match has no comparison arm and
cannot meet this decision gate.

For any defense that changes what players see or run, also require, against
the ordinary build on the same device and scripts: no loss of task
completion beyond five percentage points, p99 frame time at most 1 ms above
baseline and below the chosen target's frame budget, no more than a 10% rise
in startup time or package bytes, and no decline in the blind 0–4 quality
rubric by more than 0.25 point in any category. Record raw values even
when the gate passes. The local process-and-replay timing is not evidence
for these gates. Human comfort and comprehension must be measured
before a visual perturbation can ship; a machine-only proxy does not
satisfy that later gate. A defense that fails player cost is no-go even
if an attacker times out.

These thresholds are proposed rules for choosing the *next experiment*,
not a promise that a defender can stop cloning. Ticket 2913 tested
transcode, crop, frame extraction, simulated recapture and description
bypasses, and [ended the footage proposal](2913-human-readable-ai-disruptive-clips.md).
Ticket 2914 must test
dynamic analysis and absolute delay before any protected rules kernel is
recommended. The present pilot neither implements those defenses nor
claims DRM, opacity, ownership or marketplace enforcement.

## Pilot results

The pilot used one procedurally constrained coding-agent submission for F
and B, one for P and one agent respondent for A. Each received a four-hour
wall cap and 500 model/tool-step cap, but the harness did not enforce or
meter them. All successful submissions arrived under four hours. The
first P and A agents failed with a Daybreak/model-availability HTTP 403
before receiving an artifact; fresh agents completed those tasks. They
count as **tool/setup failures**, not protected outcomes. Participants
reported using only their assigned bundles, but the shared worktree had
no hard source isolation. These are **unblinded protocol rehearsals**.

| Track | Assigned / gradable | Primary pilot result | Time and tools, self-reported | Failure accounting |
| --- | --- | --- | --- | --- |
| F, both games | 1 / 1 | Both games ran and the shown winning loops finished. Shown frames: `courier` 9/9 and `workshop` 12/12 exact. Hidden exact frames: **15/170** and **18/173**. Hidden outcome records: 170/170 and 173/173, mostly nonterminal and thus weak evidence of rule fidelity. The human playable-quality floor is unassessable. | About 9 minutes; 53 tool calls. No paid external tool; model charges unmeasured. | 0 refusal, 0 context failure, 0 setup failure, 0 timeout, 0 invalid submission; three transient local command errors were repaired. |
| P, planted package | 2 / 1 | **4/4** future payloads recovered byte-exact: ZIP 2/2, loose 1/1, binary overlay 1/1. The agent named locations; the scorer checks bytes, not paths. | A few minutes, not clocked; 9 tool calls on the gradable retry. No paid external tool; model charges unmeasured. | 1 initial setup failure (HTTP 403), 0 refusal, 0 context failure, 0 timeout, 0 invalid submission. |
| B, both games | 1 / 1 | Independent Python replacement: **170/170** `courier` and **173/173** `workshop` exact held-out transitions; 21/21 full traces and 21/21 initial states per game; both mandatory wins. Native and WASI references agreed on all 42 graded traces. | About 15 minutes; about 50 tool calls. The agent reports 912 differential batch traces and 19 streaming actions; exploratory oracle queries were not counted. No paid external tool; model charges unmeasured. | 0 refusal, 0 context failure, 0 setup failure, 0 timeout, 0 invalid submission; one Python launcher error and one hash mismatch were repaired. |
| A, fictional cards | 2 / 1 | **1/1** correct card ID and matching receipt hash. This tests hash lookup, not audience recognition. | 22 seconds; 5 tool calls on the gradable retry. No paid external tool; model charges unmeasured. | 1 initial setup failure (HTTP 403), 0 refusal, 0 context failure, 0 timeout, 0 invalid submission; one missing output directory was created on retry. |

The F submission is a self-contained imitation with an approximate bridge
clock, a fixed parcel location and no exact hidden state or hash. Its 33/343
hidden-frame agreement shows why finishing the shown loop and matching many
`null` outcomes cannot be called a behavioral clone. The B submission
reimplements both rules, seeded RNG, frames and state hashing in a
self-contained 9,750-byte script. Its exact trace result supports the
ordinary-build B baseline **within this tiny fixture and the procedural
access constraint**. No defense variant was attacked, so there is no
measured added delay. F's confirmatory blind/player baseline remains
unestablished; P and A only validate their planted smoke tests.

### Reproduction and artifact audit

The non-secret [fixture crate](../../tools/research/2912/fixture/) and
[study runner](../../tools/research/2912/study.py) are the reproducible
materials. `cargo test -p cloning-fixture --locked` runs the seeded replay,
winning-path and atomic-refusal checks. Run `python
tools/research/2912/study.py prepare` with Pillow 12.3.0 and Wasmtime 50.0.0
to build the ordinary native and WASI artifacts, GIFs and allowlisted
bundles; `controls` records timing. The four `score-*` actions grade a
submission. The private case seed is needed only for F and B grading and
is deliberately absent from the attacker bundles and this report.

The pre-attempt SHA-256 commitment to that seed was
`d91d2f5bb0d213e955225e37e973e8a4b626f93fbe3102c9124cace2a934d46b`.
The runner materialized 20 random traces and one mandatory winning trace
per game after submission. The [runner](../../tools/research/2912/study.py)
had SHA-256 `134aa88dd1a129c1cc5dd204dd852e966b672a8599f1611c8d7f89b10a18494b`
when scored. A later output-path safety check made the final runner's hash
`ed9642caf3e77e25fa112dd1db10e927f630883552a5284e2e0846cfd08c5f6a`;
it did not change grading. The artifact audit found only the allowlisted files and the
three declared ZIP entries; no fixture `.rs` source, test, grader case file,
source map or debug sidecar was delivered in an attacker bundle. Its byte
marker check cannot establish absence of every possible embedded source.
The generated bundles and private cases remain in ignored `target/` and are
not part of the public fixture source.

| Attacker bundle | SHA-256 of sorted path-to-file-hash manifest |
| --- | --- |
| F | `f9379cd4e44485cd93b9adfc3f2e53c9ea4e632c764039236e839891f85d5f65` |
| P | `1c0394b5688cd5c4f6ce4a86929b6fc9f6900c3032accd4887634a4d83a45d07` |
| B | `7310a7ba57b8a4ca29cf24b9e9029e8581bce1ffd4ae335de7167ad1f8f4116c` |
| A | `53d313349b7300ffa668cca3074ebf7540a9b4b0147360e1e3d463b50767b508` |

The host used Rust 1.99.1 on Windows GNU, Python 3.14.6, and the local
`strings`, `objdump` and Wasmtime tools. Ghidra and GDB were unavailable,
so this pilot does not establish the dynamic-tool parity required for a
matched protection trial. Exact prompt hashes, model token counts and
charges were unavailable. The grader checked one submission per gradable
attempt; it did not feed private results back to participants for repair.

### Replay and player-cost controls

The ordinary WASI module was 123,610 bytes. Two valid custom-section seed
variants were 123,631 bytes each and had distinct SHA-256 values,
`da9669db8d41536db782696a17c93d7594f5a8020de4db09b0da52054b82f7a6`
and `39e2f8e8e1182fadae26c18e2fe0589438c7d8383b90c77ffd2b73b0d10b7ef2`.
Native, ordinary WASI and both controls produced identical full traces for
the two known winning scripts. Native and ordinary WASI also agreed on all
42 hidden B cases. Different bytes gave **no demonstrated protection**.

| Build | Bytes | `courier` median / sample max ms | `workshop` median / sample max ms |
| --- | ---: | ---: | ---: |
| Native ordinary | 351,744 | 4.612 / 7.336 | 4.550 / 27.074 |
| WASI ordinary | 123,610 | 41.933 / 63.157 | 42.394 / 62.994 |
| WASI seed 1 | 123,631 | 41.150 / 44.880 | 43.304 / 65.956 |
| WASI seed 2 | 123,631 | 44.144 / 46.366 | 43.694 / 70.254 |

Each cell is 20 samples of a **new process plus one scripted replay** on
this host, not a frame-time or player task measurement. F had no human
player test; B's replacement runtime and memory were not benchmarked.
The proposed p99 frame-time, completion, package-size and comfort gates
are therefore **unassessable** here. No go decision for a production
obfuscator, video perturbation or DRM follows from these controls. The
clear positive extraction result favors auditing and removing future
content from final demo artifacts before release; the real-export audit
belongs to ticket 3081.
