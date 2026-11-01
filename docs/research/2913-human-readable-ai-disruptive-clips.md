# Test human-readable, AI-disruptive gameplay clips

Research for ticket 2913, **2026-10-31**. The subjects are the two
author-owned synthetic fixtures from
[ticket 2912](2912-game-cloning-threat-paths.md). Nothing here is from
*Visions of Shuyi* or a third-party game.

## Decision rules, fixed before any attempt

This section was committed before the first attacker ran. Its later edits
only add results below it.

**Attacker.** A model turn with no tools, no settings, no MCP servers and an
empty working folder. It receives the frames and a brief, and answers with one
Python program. Each attempt records the tools it was offered and used. That
is a capability boundary: the attacker has no means of reading the fixture
source, the runner or the hidden cases. It also cannot run or repair its
program, so it is weaker than the 2912 agent with tools. A survival against it
cannot justify a go; a bypass by it is conclusive.

**Grader.** The 2912 `score_footage` grader, with fresh hidden cases from a
seed committed before the first attempt. Programs run in a container with no
network, a read-only filesystem and no capabilities.

**Delivered imitation floor (machine).** In both games, the shown winning
script runs on the shown seed, ends in `Won` on its last record, and matches
the shown frames with mean similarity of at least 0.90. Hidden frames and
outcomes are reported beside it and do not decide the floor, because 2912
found hidden-rule recovery from footage poor even for ordinary clips. This
floor stands in for the blind 0–4 gameplay rubric, which needs people.

**Validity.** A model's failures are credited to a defense only if it passes
the floor at least once in its ordinary-clip control.

**Stop rules, in order. Any one ends the feature proposal.**

1. **Viewer screen.** A candidate that fails the general-flash screen, or has
   mean SSIM below 0.80 on its shown frames, is dropped. Passing this is
   necessary only: it says nothing about human comfort.
2. **Robustness.** A candidate fails if any valid model passes the floor in
   any arm given that candidate. If every candidate fails, the proposal ends.
3. **Description bypass.** If a builder that sees no frames, only a viewer's
   written description, passes the floor, then the description route is a
   bypass that no change to pixels can close while viewers can still read the
   clip. The proposal ends.
4. **Human viewers.** For anything still standing: comprehension no more than
   5 points below the ordinary clip, median discomfort of at most 1 on a 0–4
   scale, at least 90% correct identification of the game. A fail ends the
   proposal.

A failure is monotonic: more models, a real phone recording or a human helper
can only add bypasses. So if rule 2 or 3 ends the proposal, the arms that would
only add bypasses need not be run, and the report says which were not. A *go*
would need all of them.

**Budget.** At most US$4 for each model turn, and US$100 for the study. Tool
failures are retried once with both results kept; they are never a defense
win.

## Recommendation: no-go

**The feature proposal ends.** Rules 2 and 3 each end it independently, and
six of six candidates failed rule 2. No clip filter should be built, promised
or described as protection from the evidence here.

- Five of the six candidates did nothing useful: Sonnet and Opus, the two
  models that passed every ordinary control, passed the floor against all
  five on untouched frames and after transcoding.
- The sixth, motion-defined form, stopped every attacker limited to stills, and
  a script of about 15 lines undid it, even after a transcode.
- A builder that saw no frames passed the floor from a written description
  alone. Nothing done to pixels can close that route while a viewer can read
  the clip.

## What changed after the rules were fixed

Everything below was decided after the rules' commit (`d2f5b28`). None of it
changes a rule.

- **Motion-defined form was added after the first five candidates' results**,
  at the project owner's suggestion. It was not in the first set, and its
  decoding arms were written for it.
- **The brief gained one sentence after the smoke attempt:** an action the game
  refuses or does not know leaves the state unchanged. Programs were exiting on
  the grader's `invalid` actions, which hid the held-out score. The smoke
  attempt is left out of every count here.
- **Elimination was planned, then dropped.** A candidate bypassed by a cheaper
  model need not meet a costlier one, but the costs allowed the full
  direct-and-transcode matrix for all three models, so it was run for the
  transfer data. Crop, stills, recapture and average ran on Sonnet only.
- **Motion-defined form breaks rule 1 by design.** Its stills are noise, so
  SSIM against the clean frame is 0.007. It also fails the flash screen as
  written. It was attacked anyway and is reported as failing both.

## What was tested

**Capture fixture.** The `clip-capture` crate replays each 2912 fixture's shown
winning script on seed 11 and paints each text frame with the engine's software
raster at 1280 × 736. Its tests check that a capture repeats byte for byte.

**Clips.** Each game state holds for 20 subframes at 30 frames a second. An
attacker's sampler takes the middle subframe of each hold: 9 frames of
`courier` and 12 of `workshop`.

| Candidate | What it does |
| --- | --- |
| `noise` | Fresh seeded per-pixel noise, within ±16 of 255 |
| `warp` | Each glyph cell jitters up to 3 pixels, with clutter lines |
| `temporal` | Complementary halves of the glyph pixels on alternate subframes |
| `decoy` | False rules drawn 6 of 255 above the background |
| `surrogate` | PGD at ±8 of 255 against CLIP ViT-B/32, driving cosine similarity to the clean frame to about −0.4 |
| `motion` | Motion-defined form: each frame is binary noise, glyph noise scrolls down and background noise up. Text is enlarged 1.5 times and thickened, and colour is lost |

| Arm | What the attacker received |
| --- | --- |
| `direct` | Lossless frames |
| `transcode` | H.264 at 854 wide, CRF 30, as a social upload might |
| `crop` | The transcode, cropped to the middle 88% and encoded again |
| `frames` | JPEG stills at 640 wide, quality 70, from the transcode |
| `recapture` | The transcode with perspective, blur, moiré and sensor noise: a **simulation**, not a phone |
| `average` | The mean of each hold |
| `decode`, `decode_transcode` | `motion` only: `perturb.motion_decode`'s output, from lossless frames or the transcode |

**Attackers.** Haiku 5.5, Sonnet 5.5 and Opus 5.5. All 99 attempts, and all 18
describer turns, are in the ledger; 98 attempts are counted here, leaving out the smoke attempt. Every
imitation and describer turn was offered no tools and made no tool calls; the
ledger records both. Each answered in one turn, and none needed a retry.

**Spend.** US$30.37 at the CLI's reported costs, about a third of the cap. The
CLI's pricing table did not recognize these model names and marked its cost
basis `unknown`, so treat these as estimates.

## Results

Each cell is attempts over the floor out of attempts assigned. Direct and
transcode sum all three models; the other four arms are Sonnet only, except
`motion`'s average, which is all three models plus a Sonnet repeat.

| Candidate | Direct | Transcode | Crop | Stills | Recapture | Average | Rule 1 | Rule 2 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |
| Ordinary (control) | 4/5 | 2/3 | 1/1 | 1/1 | 1/1 | not run | n/a | n/a |
| `noise` | 3/3 | 3/3 | 1/1 | 1/1 | 1/1 | 1/1 | **fail** (SSIM) | **fail** |
| `warp` | 2/3 | 2/3 | 0/1 | 1/1 | 0/1 | 1/1 | pass | **fail** |
| `temporal` | 3/3 | 3/3 | 1/1 | 0/1 | 0/1 | 1/1 | pass | **fail** |
| `decoy` | 2/3 | 3/3 | 1/1 | 1/1 | 1/1 | 1/1 | pass | **fail** |
| `surrogate` | 2/3 | 3/3 | 1/1 | 1/1 | 1/1 | 1/1 | pass | **fail** |
| `motion` | 0/3 | 0/3 | 0/1 | 0/1 | 0/1 | 0/4 | **fail** (SSIM, flash) | **fail**, see below |

**Validity.** All three models passed the floor in their ordinary control, so
all count. Haiku is noisy: it passed its ordinary control 2 times in 4, so its
misses against a candidate are weak evidence. Sonnet and Opus passed every one
of their ordinary controls.

**Motion-defined form and its decoder.**

| Arm | Over the floor | Exact shown frames, Sonnet / Opus |
| --- | ---: | ---: |
| Lossless stills | 0/3 | 0/21, 0/21 |
| Transcoded stills | 0/3 | 0/21, 0/21 |
| **Decoded from lossless** | **3/3** | 19/21, 19/21 |
| **Decoded from the transcode** | **2/3** | 7/21, 13/21 |

The stills carry nothing: correlation with the glyphs is −0.001 for a still and
−0.001 for an average of a hold, and 0.85 after decoding, or 0.69 after a
transcode and decoding, on one sampled state. The attackers said so unprompted:
each reported static and said the game it delivered was invented.

The tool-free attackers did not find the decoder themselves. They were handed
its output. So this shows that decoding defeats the filter and that the decoder
is short. It does not measure how quickly an attacker with tools would think of
it. That is a limit on any "stops casual use" reading, and it is not evidence
of protection.

**Description bypass.** A describer saw a clip and wrote at most 400 words; a
builder saw only those words.

| Clip the describer saw | Over the floor | Who passed |
| --- | ---: | --- |
| Ordinary | **1/3** | Opus |
| `noise` | 2/3 | Sonnet, Opus |
| `warp` | 1/3 | Opus |
| `temporal` | 2/3 | Sonnet, Opus |
| `decoy` | 2/3 | Sonnet, Opus |
| `surrogate` | 2/3 | Sonnet, Opus |

The describer stands in for a person, because no person was run. The ordinary
row is the stand-in for a viewer who could read a perturbed clip, including a
`motion` clip, whose own description arm was not run. The describers were not
sparing: Opus's description of the ordinary clip copied the map out as text. A
person might write less, or watch for longer and write more.

**The decoy was noticed, not believed.** All three `direct` attackers and all
three describers read false rules at a contrast of 6 of 255. Their answers
named the text as suspicious, and those that said what they did with it had
ignored it, matching the visible numbers instead. None of the three `transcode`
attackers mentioned it. A hidden message is therefore not a disruption here: it
is either seen through or compressed away. It also tells a careful reader that
someone is trying to mislead them.

**Held-out rules.** Most attempts over the floor matched about 150 of 343 hidden
frames, whichever candidate they faced: for example 149 and 157 for the ordinary
controls, 157 for `noise`. These are not comparable with 2912's 33/343, whose
brief lacked the refusal sentence. Some arms scored lower, such as `warp` at 109
and 117 on untouched frames, but single attempts cannot show that a filter
caused it: one ordinary Opus control scored 3.

## Viewer cost and survival

Measured on the shown frames, averaged over both games. These are machine
screens. They do not show that a person can read or tolerate a clip.

| Candidate | SSIM | Luma PSNR, dB | Flash pixels | Largest field step | Perturbation correlation after transcode | Transcode bytes, both clips | ms per frame |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Ordinary | 1.000 | 99.0 | 0 | 0.000 | 1.000 | 30,859 | 0 |
| `noise` | 0.656 | 32.9 | 2 | 0.001 | −0.016 | 31,800 | 75 |
| `warp` | 0.936 | 22.4 | 0 | 0.013 | 0.973 | 404,327 | 21 |
| `temporal` | 0.986 | 27.1 | 7,864 | 0.008 | 0.935 | 370,346 | 23 |
| `decoy` | 0.989 | 48.0 | 0 | 0.000 | 0.636 | 32,851 | 17 |
| `surrogate` | 0.841 | 35.4 | 0 | 0.000 | 0.635 | 179,734 | 2, plus about 15 s a frame in advance on a CPU |
| `motion` | 0.007 | 6.9 | 912,708 | 0.014 | 0.989 | 2,877,863 | 66 |

- **Transfer.** `noise` is erased by a transcode (correlation −0.016).
  `surrogate` drove its own surrogate's similarity to about −0.4, and that did
  not transfer far enough to stop Sonnet or Opus, which are not that surrogate.
- **Flash.** The pre-fixed screen counts single pixels against
  [WCAG 2.3.1](https://www.w3.org/TR/WCAG22/#three-flashes-or-below-threshold)'s
  rate and area. `motion` fails it over nearly the whole frame. Averaged over
  64-pixel cells, its largest step is 0.014, under the threshold of 0.1, so the
  screen probably overstates fine noise that the eye averages. That does not
  clear it: neither figure measures comfort with a screen of moving static, and
  rule 1 stands as written.
- **Capture quality.** `motion` costs 93 times the bytes of the ordinary
  transcode at the same CRF. A platform that caps bitrate would damage it
  further, and that is untested. Its text must also be enlarged and thickened,
  and it loses colour. Any colour that marks a region would show in a still.
- **Maintenance.** All of this would be a video pipeline outside the engine,
  run per trailer, with a filter whose bypass is one published script away.
  `surrogate` also needs a model and a re-run for each encode. No candidate's
  cost is justified by its result.

## What was not tested

The stop rules ended the proposal first, and each item below could only add a
bypass. A *go* would have needed all of them.

- **Human viewers:** no comprehension, comfort or identification data exists.
  One informal look at a `motion` clip was requested from the project owner; it
  would not be blind, and it is not recorded here as a result.
- **A real phone recording:** `recapture` is a simulation.
- **Other model families:** only three Anthropic tiers were available here. No
  OpenAI, Google or open-weight model was run.
- **A human-assisted attacker:** the describer is a model standing in for one.
- **An attacker with tools:** every attacker had one turn and no tools, as the
  rules say. It could not run or repair its program, or write a decoder.

## Limits of the result

- **The fixtures favor the attacker.** They are small, text-heavy games on a
  flat background, so a model can copy them almost exactly. A game with sprites
  could behave differently, though rule 3 does not depend on the picture.
- **The floor is a machine floor, and lenient.** One Opus control passed with
  only 2 of 21 frames exact, on 0.99 similarity. The blind 0–4 rubric has not
  been applied, so "over the floor" means a program finishes the shown loop and
  looks like the clip, not that a person enjoyed it.
- **One attempt per cell.** Most cells are a single attempt. Rule 2 needs only
  one bypass, so this suffices for a failure. The counts are not rates, and
  they cannot rank one candidate over another.
- **Surrogate.** `open_clip` warned of an activation mismatch with these
  weights. The attack still reached the similarity above, but other surrogates
  and a stronger ensemble were not tried.

## Blind human protocol

This protocol is ready and was not run, because rule 4 is reached only by a
candidate that survives rules 1 to 3. `study.py viewer-kit` writes numbered
clips in `viewer-kit/participant/` with `answers.csv`, each participant in a
separate shuffled order, and seals the key in `viewer-kit/sealed/`. The
evaluator who hands out clips does not see the key.

**Viewers**, at least 12, each watching every clip once, on one device and at a
normal volume and distance. Anyone with a history of photosensitive epilepsy is
excluded, and anyone may stop at any time; stopping counts as a discomfort
score of 4. For each clip they answer:

- `understood_goal`: could they say what the player is trying to do (yes or no)?
- `final_answer`: the outcome the clip ended in.
- `comfort_0_4`: 0 is none and 4 is had to look away.
- `identified_game`: which game this was, from a list with decoys.

`score_viewers` gives each arm's scores against rule 4. A fail ends the proposal.

**Gameplay**, at least four fresh evaluators, who do not know which clip a
program was built from. Each plays it through its `--oracle` mode and scores
controls, coherent loop, feedback, visual identity and stability from 0 to 4, as
2912 proposes. `score_rubric` applies 2912's floor: a median of at least 3 in
every category, with at least three of four finishing the loop.

## Reproduction

The runner (`study.py`), perturbations (`perturb.py`), tests (`test_study.py`)
and capture crate (`capture/`) are in `tools/research/2913/`. In a virtual
environment with Pillow 12.3.0, NumPy 2.5.3, imageio-ffmpeg 0.7.0 (ffmpeg 7.1),
torch 2.15.0 and open_clip_torch 3.3.0, and with Docker's `python:3.14-slim`:

```text
python tools/research/2913/test_study.py
python tools/research/2913/study.py capture
python tools/research/2913/study.py surrogate
python tools/research/2913/study.py encode
python tools/research/2913/study.py commit-seed
python tools/research/2913/study.py attack --stage control
python tools/research/2913/study.py score
```

Run `attack` with `--model` and `--everything` for the stages `candidates` and
`description`, and with `--model` for `motion`, `transforms` and `bypass`, then
`score` and `publish`. None of these packages ships: they are research tools in
a virtual environment under `target/`.

The capture crate is not in the repository's workspace, so the workspace's tests
skip it. Check it with
`cargo test --manifest-path tools/research/2913/capture/Cargo.toml`.

`tools/research/2913/results/` holds the attempt ledger (`attempts.json`), the
hash of each evaluated encode and frame set (`encodes.json`), the per-arm summary
(`summary.json`) and the viewer metrics (`viewer_metrics.json`). Programs,
responses, clips and the case seed stay in the ignored `target/`.

**Removing the study.** Everything is in that one folder and nothing depends on
it, so `git rm -r tools/research/2913` removes it. This report deliberately names
those files as text, so it stays valid without them, though the study then cannot
be re-run or its hashes checked.

The SHA-256 commitment to the grader's seed, made before the first attempt, was
`0d6db84e77d18b74c3d4ba51214176de8123865b68e1b1d53ca088d0b9277965`.
