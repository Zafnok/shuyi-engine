# Capture controls, recipient marks and AI-use signals for closed previews

Research for ticket 2915. The subjects are the two author-owned synthetic
fixtures from [ticket 2912](2912-game-cloning-threat-paths.md), captured by the
crate from [ticket 2913](2913-human-readable-ai-disruptive-clips.md). Nothing
here is from *Visions of Shuyi* or a third-party game.

## Decision rules, fixed before any measurement

This section was committed before the first capture or mark was measured. Its
later edits only add results below it.

**Three separate claims.** Every control is judged against only the claim it
can support, and a result for one is never reported as a result for another.

- **Capture friction:** a named software path returns no picture of the window.
- **Secrecy:** the content does not reach a person or model who is shown it.
- **Leak attribution:** an operator can name who received a copy that leaked.

**Capture friction.** A control adds friction on a path only if, in the same
run, that path returns the window's pixels with the control off and does not
with it on. Each path is reported alone. No result can support *secrecy*: a
person who can see the preview can film or describe it, and 2913 showed that a
description can be enough.

**Recipients.** 1,000 recipient codes from a secret key. The scorer is told
neither which recipient a clip belongs to nor the arm's expected result, and
reads the sealed key only after it has computed every score.

**Invisible mark: a pass.** The operator holds the clean capture. The true
recipient's score is at least 6 standard deviations and the highest of the
1,000, and no other recipient reaches 6. Under the null a score is a unit
normal, so 6 is about 1 in 10⁹ for each innocent recipient. For a collusion arm
a pass is at least one colluder at 6 or more and no innocent recipient at 6.

**Visible mark: a pass.** Either the same test against each recipient's drawn
mark, with scores standardized by the decoys' median and spread, or a
tool-free reader model naming the full code from sampled frames. The two are
reported apart: the first is what an operator's tool finds, and the second
stands in for whether a viewer can read it.

**Ordinary control.** The arms are also scored on unmarked clips. Any recipient
at 6 or more on an unmarked clip voids the detector until it is fixed, and the
report says so.

**Viewer cost.** Measured on shown frames, averaged over both games. A candidate
called invisible must reach SSIM of at least 0.98 and luma PSNR of at least
38 dB, or it is reported as visible and not recommended as invisible. A visible
mark must leave the game readable: a reader model's transcription of the game's
text on marked frames scores no more than 5 points of 100 below its score on
unmarked frames. These are machine screens, and one person's look at them is not
a blind study.

**What the recommendation may name.** For the closed-preview scope:

1. A mark is recommended only if it passes `transcode`, `crop`, the real public
   platform and the real phone recording. A pass on simulations alone cannot
   recommend it.
2. A failure against collusion or a removal attack does not drop a mark. It
   limits the words: the recommendation must say what breaks it.
3. A capture flag is recommended only as friction, only for the paths it was
   seen to block here or the vendor documents, and only with its bypass stated
   beside it.
4. A target without a device here is reported from documents, labelled as not
   tested, and any flag on it stays *to check per target* in 3083.

**Signals.** C2PA, text marks and AI-use signals are each placed under training
preference, provenance or inference refusal, by what enforces them. None may be
written as forcing a model to refuse without a test or a vendor's statement that
shows it.

**Budget.** US$10 of model turns.

## Recommendation

Three different things were asked of these controls, and each gets a different
answer.

| Claim | Answer | Why |
| --- | --- | --- |
| Capture friction | **Yes, on named paths** | On Windows, the exclusion flag blocked all five software paths tested |
| Secrecy | **No control gives it** | A phone filmed every excluded frame, and 2913 showed a description can be enough |
| Leak attribution | **Yes, for a copy that keeps the mark, and with limits** | A faint mark named the right 1 of 1,000 recipients on YouTube's encode and a real phone film. A short cleanup erased it, and a description carries no mark |

**Recommended scope for 3083, closed previews only and opt-in:**

1. **A faint per-recipient mark at strength 2**, the `invisible2` mark below. It
   passed `transcode`, `crop`, the real platform and the real phone. It must be
   described as **faint, not invisible**: its machine screens passed, but the one
   person who looked could see it on these flat backgrounds. It must also be
   described as removable: `flatten` erased it completely, and that result says
   nothing about whether anyone would try.
2. **Windows' capture exclusion as friction only**, with its bypasses stated:
   a camera, and code run inside the process. It should stay off when a viewer
   needs a screen magnifier or similar tool, until that is tested.
3. **A visible code is not recommended as drawn.** The tiled code was readable on
   the real platform and phone, but it missed the readability screen by 0.2 of a
   point, and the person who looked found text harder to read where the two
   overlapped. 3083 may draw it away from the game's text, or behind it, but must
   pass this report's screen before shipping one. It has value that attribution
   lacks: the recipient can see it.

**Rejected claims.** None of these may be made from this evidence:

- that a preview or a public trailer cannot be recorded;
- that a mark is invisible, or that it cannot be removed;
- that a mark identifies a person, rather than the copy issued to them;
- that a capture flag keeps content secret, or from an AI model;
- that C2PA, a crawler preference, a training setting or any notice makes a model
  refuse to look at or reason about content.

**Public builds keep no restriction,** as the ticket requires. Nothing in the
engine sets one today, and a public trailer has one copy for everyone, so it
cannot carry a per-recipient mark in any case.

## What changed after the rules were fixed

None of these changes a rule. All were decided after the rules' commit
(`99e7df1`).

- **The Print Screen path was removed.** Pressing the key opened the Snipping
  Tool's overlay on this machine, which dimmed the screen and spoiled that first
  run. Its results are discarded, and the path is not reported.
- **The visible detector's picture was enlarged,** from 320 to 640 wide for the
  corner and tile marks, because a unit test failed. No arm had been recorded.
- **Two arms were added after the first results.** `palette_snap` kept the mark's
  own shades in its palette, so it was a weak attack, and every score so far
  assumed a clean reference. So `flatten`, written knowing how the mark works, and
  `blind_transcode`, with no clean reference, were added.
- **Phone alignment changed after seeing the film, and was checked on the unmarked
  segment only.** Aligning to the game's text left rows up to 30 pixels off,
  because that text is one small cluster. The scorer now takes the screen's
  corners from the white flash, then follows the hand with a small rigid move. It
  also matches flashes to the kit's known schedule, because the phone's exposure
  added 5 false ones. The film and YouTube's encode were scored this way; the
  simulated arms were not.
- **The fit cutoff was lowered** from 0.5 to 0.2 for the same check, because the
  reflection on a glossy screen keeps a good alignment's correlation low. It still
  dropped most `invisible8` frames: 1 of 9 and 4 of 12 states were scored there.

## Capture controls

**Windows, tested** on build 26300. A window of one known colour, drawn with GDI
and again with OpenGL, was asked for by a separate process on each path. Both
kinds gave the same results, except where a cell says otherwise. A cell says what
that path returned of the window.

| Path | No flag | `WDA_MONITOR` | `WDA_EXCLUDEFROMCAPTURE` |
| --- | --- | --- | --- |
| GDI screen copy (BitBlt) | Window | Black | What is behind it |
| ffmpeg `gdigrab` | Window | Black | What is behind it |
| Desktop Duplication (ffmpeg `ddagrab`) | Window | Black | What is behind it |
| `PrintWindow` | Window | Black | Black |
| Windows.Graphics.Capture | Window | Black | No frame in 8 seconds |
| UI Automation | Reads the title | Reads the title | Reads the title |
| **A phone camera** | not run | not run | **The whole picture, 14 of 14 segments** |

While the phone filmed, a software screen copy shared 0% of its pixels with the
picture in all 14 segments; with the flag off, in a short check, it shared 99.99%.
Another process could not clear the flag (error 5, access denied).

The flag hides pixels, not the accessibility tree. A game that draws its own text
exposes little there anyway. Game Bar, Snipping Tool, Remote Desktop, Magnifier, a
capture card, a virtual machine and code injection were not tested.

**The target matrix.** *Tested* means here, on this machine. Everything else is
from documents, and is labelled as Documented (the vendor says so) or Reported (a
third party does). Pages were read through a summarizing fetch, so wording here is
paraphrase and should be checked at the source before quoting.

| Target | Software capture control | Accessibility | Recapture and other bypass |
| --- | --- | --- | --- |
| Windows | **Tested,** above. Documented: Microsoft says the flag is [not a security feature or DRM](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowdisplayaffinity), though [its Recall page](https://learn.microsoft.com/en-us/windows/client-management/manage-recall) calls it a DRM flag. It covers top-level windows of the calling process | Reported: [Signal](https://signal.org/blog/signal-doesnt-recall/) says its screen security can stop screen readers and magnifiers. No Microsoft guidance found | **Tested:** camera. Reported: [code injected into the owning process](https://www.ioactive.com/the-drm-flag-that-isnt-drm/) clears it. Reports on Remote Desktop conflict |
| Web | **Tested** in Chrome: a WebGL page captured itself with `getDisplayMedia`, and read its own canvas back. Documented: the [Screen Capture draft](https://www.w3.org/TR/screen-capture/) has no page opt-out. Protected playback needs a proprietary CDM, which is out of scope, and [Clear Key](https://www.w3.org/TR/encrypted-media-2/) requires no client-side protection | Unaffected, as nothing restricts it | Everything. There is nothing to bypass without a CDM, and a game's canvas is not an encrypted media element |
| macOS | **Not tested,** no device. Documented: Apple calls [`sharingType = .none`](https://developer.apple.com/documentation/appkit/nswindow/sharingtype-swift.enum/none.md) a legacy constant not to be used for hiding content, and [says](https://developer.apple.com/forums/thread/792152) no public API prevents capture. Reports conflict on whether ScreenCaptureKit honours it | Unknown: no source covers Zoom or VoiceOver | Camera. Reported: QuickTime and ScreenCaptureKit [capture such a window](https://developer.apple.com/forums/thread/808016) |
| Android | **Not tested,** no device. Documented: [`FLAG_SECURE`](https://developer.android.com/security/fraud-prevention/activities) blanks screenshots and non-secure displays. Android 14 and 15 add detection, which does not block | Documented: [Accessibility Scanner](https://support.google.com/accessibility/android/faq/6376582?hl=en-GB) shows black. Unknown for TalkBack and magnification | Camera. Reported: [root modules](https://github.com/LSPosed/DisableFlagSecure) remove it. [scrcpy](https://github.com/Genymobile/scrcpy/issues/3515) shows black |
| Public store and social playback | **None is possible.** The game isn't running: the platform's player shows its own encode. Documented: [Steam](https://partner.steamgames.com/doc/store/trailer) re-encodes uploads, and its page offers no trailer protection | Unaffected | **Tested:** YouTube's encode. Anyone can record or download a public trailer |

Only Windows and Android have a flag worth checking, and only in a controlled preview. The web has none, and Apple documents macOS's as not for this.

**Public builds, verified.** Three checks, in `verify_public.py`:

- **Source:** no restricting call in `crates`, which holds the web shell, nor in
  `templates`, `examples` or `skills`, nor in macroquad 0.4.16 and miniquad 0.4.11,
  which holds the Android Java.
- **Binary:** no such string in a built `hello-grid.exe`.
- **Black box:** each software path above captured Hello, Grid's real window, and
  Chrome captured a WebGL page, using the same kind of canvas as the engine's web
  shell.

Android, macOS and an exported web build were covered by the source scan only. The
built binary was a debug build.

## Marks

**Clips.** The 2912 fixtures' shown winning scripts on seed 11: 9 frames of
`courier` and 12 of `workshop`, at 1280 × 736, holding each state for 20 frames at
30 a second.

| Mark | What it is |
| --- | --- |
| `invisible2`, `4`, `8` | A smooth pattern of light and dark 32-pixel cells, changing each state, from a recipient's secret code. The number is its height in gray levels of 255 |
| `visible_corner` | One small code in the bottom right |
| `visible_tile` | The code repeated across the frame at 20% opacity |
| `visible_drift` | The same, moved to a new place each state |

**Detector.** It aligns what leaked, subtracts the clean frame and removes slow
changes across the frame, such as glare and vignetting. It then scores each of the
1,000 recipients' codes. On unmarked clips the highest score anywhere was 3.8, and
the highest innocent score on any marked clip was 5.1, so the detector stands.

**Results.** Each cell is games passed out of 2, then the lower true score, where 6
is the pass mark. The tool-free reader's readings are in the next table: a visible
mark can pass by either route.

| Arm | `invisible2` | `invisible4` | `invisible8` | `visible_corner` | `visible_tile` | `visible_drift` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Lossless | 2/2 (69) | 2/2 (73) | 2/2 (73) | 2/2 (8) | 2/2 (7) | 2/2 (6) |
| `transcode`: H.264, 854 wide, CRF 30 | 2/2 (67) | 2/2 (69) | 2/2 (70) | 0/2 (1) | 1/2 (6) | 0/2 (3) |
| `ladder720`: H.264, 720p, capped at 1.5 Mbps | 2/2 (67) | 2/2 (70) | 2/2 (71) | 1/2 (−1) | 1/2 (6) | 0/2 (4) |
| `vp9_480`: VP9, 480p | 2/2 (67) | 2/2 (70) | 2/2 (67) | 1/2 (1) | 1/2 (5) | 0/2 (4) |
| `crop`: the transcode, middle 88%, encoded again | 2/2 (58) | 2/2 (59) | 2/2 (55) | 0/2 (0) | 1/2 (5) | 0/2 (4) |
| One still from the transcode | 2/2 (22) | 2/2 (23) | 2/2 (22) | 0/2 (0) | 2/2 (6) | 0/2 (0) |
| `recapture`: a **simulated** film | 2/2 (70) | 2/2 (70) | 2/2 (51) | 0/2 (2) | 0/2 (3) | 0/2 (3) |
| **Real YouTube encode** | **2/2 (68)** | **2/2 (70)** | **2/2 (73)** | 0/2 (6) | 2/2 (8) | 1/2 (5) |
| **Real phone film** | **2/2 (67)** | **2/2 (72)** | **2/2 (24)** | 0/2 (−3) | 0/2 (1) | 0/2 (1) |
| Denoised | 2/2 (42) | 2/2 (61) | 2/2 (68) | 0/2 (2) | 2/2 (6) | 0/2 (3) |
| `palette_snap` | 2/2 (20) | 2/2 (13) | 2/2 (13) | 0/2 (−1) | 0/2 (0) | 0/2 (0) |
| Median of all states | 2/2 (21) | 2/2 (20) | 2/2 (21) | 0/2 (1) | 1/2 (6) | 0/2 (1) |
| **`flatten`** | **0/2 (0)** | **0/2 (0)** | **0/2 (0)** | 0/2 (1) | 1/2 (6) | 0/2 (4) |
| No clean reference | 2/2 (67) | 2/2 (69) | 2/2 (73) | 0/2 (3) | 0/2 (1) | 0/2 (2) |
| 2 colluders, averaged | 2/2 (47) | 2/2 (56) | 2/2 (54) | 0/2 (2) | 0/2 (4) | 0/2 (3) |
| 3 colluders, averaged | 2/2 (36) | 2/2 (41) | 2/2 (42) | 0/2 (1) | 0/2 (2) | 0/2 (2) |
| 5 colluders, averaged | 2/2 (19) | 2/2 (33) | 2/2 (33) | 0/2 (2) | 0/2 (2) | 0/2 (2) |
| 2 colluders, alternating states | 2/2 (38) | 2/2 (40) | 2/2 (38) | 0/2 (0) | 0/2 (4) | 0/2 (1) |
| 2 colluders, darkest pixel of each | 2/2 (42) | 2/2 (47) | 2/2 (46) | 0/2 (1) | 0/2 (4) | 0/2 (3) |

Collusion arms were transcoded afterward. A pass names at least one colluder.

**The reader on visible codes,** one attempt on one frame per game. The model is a
stand-in for whether a viewer can read a code, and it was offered no tools.

| Arm | `visible_corner` | `visible_tile` | `visible_drift` |
| --- | --- | --- | --- |
| Lossless | 2/2 | 2/2 | 2/2 |
| `transcode` | 0/2 | 2/2 | 2/2 |
| `crop` | 0/2 | 2/2 | 2/2 |
| `recapture`, simulated | 0/2 | 0/2 | 2/2 |
| **Real YouTube encode** | 1/2 | **2/2** | **2/2** |
| **Real phone film** | 2/2 | **2/2** | **2/2** |
| 2 colluders, averaged | 0/2 | 0/2 | 0/2 |
| 2 colluders, darkest pixel | 0/2 | 0/2 | 0/2 |

No code was read from any invisible mark, as expected, including the ones a person
could see.

**A real phone beat its simulation for visible codes.** The simulated film defeated
the reader on the tile, and the real one did not: 4K holds far more detail than the
simulation's output. The simulation is not a good guide to a phone in either
direction, so a mark must be tested on a real one.

**Against the rules.**

| Mark | `transcode` | `crop` | YouTube | Phone | Viewer screen | Result |
| --- | --- | --- | --- | --- | --- | --- |
| `invisible2` | Pass | Pass | Pass | Pass | Pass: SSIM 0.998, 45.1 dB | **Recommended, as faint** |
| `invisible4` | Pass | Pass | Pass | Pass | Pass: SSIM 0.993, 39.4 dB | Survives no better than strength 2, and costs more; not recommended |
| `invisible8` | Pass | Pass | Pass | Pass, on few states | **Fail:** SSIM 0.976, 33.4 dB | Visible, and not recommended as invisible |
| `visible_corner` | Fail | **Fail** | Reader on 1 of 2 | Reader | Pass: 4.5 points down | Lost to any crop; not recommended |
| `visible_tile` | Reader | Reader | Both | Reader | **Fail: 5.2 points down** | Not recommended as drawn |
| `visible_drift` | Reader | Reader | Reader | Reader | **Fail: 7.8 points down** | Not recommended as drawn |

The readability figure is the reader's game-text score on `direct`, `transcode`,
`crop`, `recapture`, YouTube and phone frames, against 99.5 for unmarked frames. It
is noisy: single attempts, with a recurring score near 69 on `courier` that also
appears on `invisible4` over the phone, where nothing covers the text. It may
therefore be overstating the cost. The tile's miss is 0.2 of a point. The rule is
applied as written all the same, and the person who looked supports it.

**One person's look.** The project owner watched the kit while filming it, on one
laptop, and reported two things:

- A blocky background from the third clip, which is `invisible2`, the weakest mark.
- Game text "a bit hard to read" where it overlapped the codes.

This is not blind: the instructions said a pattern would appear. A blind study on
real art is what the word *invisible* would need.

## What attribution can and cannot do

- **Removal is easy here.** `flatten` sends every pixel near the commonest colour to
  that colour, in a few lines. It took each invisible mark's score to exactly 0.
  The fixtures favour it: they are flat, and a game with detailed art would not
  flatten so cleanly. Regenerating a frame with a generative model was not tested;
  [Zhao and others](https://arxiv.org/html/2306.01953v3) report it removing several
  invisible marks.
- **Collusion was not the weak point, in the forms tried.** Averaging 5 copies still
  named a colluder at strength 2. This is only simple averaging and splicing, by
  copies of a mark whose design was not used against it. Other work on
  [fingerprinting codes](https://arxiv.org/abs/1010.2621) shows that resistance holds
  only up to a limit, which grows with code length.
- **A clean reference did not matter here, and may elsewhere.** The detector worked
  without one on these flat fixtures, because the background estimates itself. On
  detailed art it may need the clean frame, and so a live session's recorded inputs.
- **A single leaked still can be enough here,** with scores above 21 at each strength.
- **A mark names a copy.** It says which issued copy leaked, not who leaked it. A
  shared or stolen copy points at its recipient. No mark survives a person
  describing the game, which 2913 showed can be enough to rebuild a shown loop.
- **The list of recipients is personal data.** Whoever holds the table from code to
  person holds data about an identifiable person, on a plain reading of
  [GDPR Recital 26](https://gdpr-info.eu/recitals/no-26/). This is not legal advice,
  and 3083 must write down its retention and expiry.
- **Precedent is thin for games.** The [Caridi screener case](https://www.justice.gov/archive/criminal/cybercrime/press-releases/2004/spragueArrest.htm)
  is a film precedent for tracing a recipient. No confirmed case of a mark tracing a
  game's leaker was found. Published screen-camera tests include
  [StegaStamp](https://ar5iv.arxiv.org/html/1904.05343), with real cameras under
  controlled conditions; [Video Seal](https://arxiv.org/html/2412.09492v1) and
  [TrustMark](https://ar5iv.labs.arxiv.org/html/2311.18297) appear to be simulated
  only, in what was read. None tested gameplay footage.

## Signals: what each one can claim

Sorted by what enforces them. A signal at a site does not travel with a file, so a
transcode or a phone film leaves it behind. **No signal found supports inference
refusal.**

| Signal | Claim it can support | What enforces it |
| --- | --- | --- |
| [C2PA manifest](https://spec.c2pa.org/specifications/specifications/2.5/specs/C2PA_Specification.html) | **Provenance** | Validators only. Reported as stripped by [platforms in one test](https://incidentdatabase.ai/reports/6301), and a film carries none |
| [Soft bindings](https://c2pa.org/faqs/) | **Provenance:** finding a stripped credential again | A lookup. No measured video or phone survival found |
| [`cawg.training-mining`](https://cawg.io/training-and-data-mining/1.1/) | **Training preference.** It also lists inference, with nothing behind it | The reader's choice |
| [robots.txt](https://www.rfc-editor.org/rfc/rfc9309.html) | **Training preference,** by convention | Voluntary, and by its own RFC not access authorization |
| [IETF AIPREF](https://datatracker.ietf.org/wg/aipref/documents/) | **Training preference** | Drafts, unfinished. Enforcement is out of its scope |
| [Cloudflare Content Signals](https://blog.cloudflare.com/content-signals-policy/), [RSL](https://rslstandard.org/rsl), [TDMRep](https://www.w3.org/community/reports/tdmrep/CG-FINAL-tdmrep-20240510/), [`noai`](https://metawatch.iptc.org/ai-policy/noai-meta/) | **Training preference** | Voluntary, unless a server separately blocks or licenses. No party honouring `noai` was found |
| [llms.txt](https://llmstxt.org/) | None: a guide for agents | Nothing |
| [YouTube's third-party training setting](https://support.google.com/youtube/answer/15509945) | **Training preference,** off by default | YouTube says it cannot control those companies |
| [SynthID](https://deepmind.google/models/synthid/) | **Provenance:** that a Google or partner model generated it | A detector |
| Visible or hidden notice in a picture | None shown | 2913: hidden text was read, then disbelieved or compressed away. OpenAI's [Model Spec](https://model-spec.openai.com/2026-10-29.html) gives such content no authority. Notices meant to make an agent refuse are out of scope and were not tried |
| [DSM Art. 4(3)](https://eulawradar.com/case-c-25025-like-company-does-a-chatbot-copy-when-it-predicts/) reservation, with [AI Act Art. 53](https://ai-act-service-desk.ec.europa.eu/en/ai-act/article-53) | **Training preference with legal effect** in the EU | Law, not technology, and only for a machine-readable reservation, as [OLG Hamburg](https://www.technologyslegaledge.com/2025/12/robert-kneschke-v-laion-judgment-of-10-december-2025-ref-5-u-104-24/) read it. An appeal is reported pending. Not legal advice |

A person pasting a screenshot into a chatbot is outside most of this. OpenAI [says](https://developers.openai.com/api/docs/bots)
robots rules may not apply to a user-initiated request, and the AIPREF draft leaves
out assets a user provides directly. No vendor statement or test showing a model
refuses a screenshot on such a signal was found. None was tried here either.

## What was not tested

- **macOS and Android devices,** and iOS: documents only. 3083's per-target matrix
  must run them before any flag is used there.
- **Steam, TikTok, X and Discord:** only YouTube's encode was run, at 720p and from
  YouTube Studio's download. A viewer's stream may differ. Valve's ladder is unknown.
- **More than one phone, screen or person.** One handheld 4K film of a glossy laptop
  screen in a lit room. A low-resolution or further-away film could lose the mark.
- **Real game art.** The fixtures are flat text, which helps the detector, helps
  `flatten` and makes the pattern easiest to see. All three could change.
- **A live session.** Each clip is one known replay.
- **Generative removal, an agent with tools asked to remove a mark, and adaptive
  collusion.** Any could add bypasses.
- **A blind human study, and repeated reader attempts.**
- **Windows paths named above** as untested.

## Costs

- **Spend:** US$2.21 of reader turns at the CLI's reported costs, under the US$10
  cap.
- **Bytes:** at the same CRF, `invisible2` costs about 2.5 times the transcode's
  bytes, 32,386 against 12,607 on `courier`. These are small clips, and a real game
  is not flat.

## Reproduction

The runner (`study.py`), marks (`marks.py`), Windows probe (`probe.py`), build check
(`verify_public.py`) and tests (`test_study.py`) are in `tools/research/2915/`. The
fixtures come from 2913's `clip-capture` crate, so its folder must exist to re-run
this study. In a virtual environment with Pillow 12.3.0, NumPy 2.5.3, imageio-ffmpeg
0.7.0 (ffmpeg 7.1), opencv-python-headless 5.0.0.93 and windows-capture 2.0.1:

```text
cargo build --release --locked --manifest-path tools/research/2913/capture/Cargo.toml --target-dir target/clip-capture
python tools/research/2915/test_study.py
python tools/research/2915/study.py capture
python tools/research/2915/study.py marks
python tools/research/2915/study.py reader
python tools/research/2915/study.py master
python tools/research/2915/probe.py matrix <ffmpeg>
python tools/research/2915/verify_public.py scan
python tools/research/2915/verify_public.py window <game.exe> <its folder> <ffmpeg>
python tools/research/2915/verify_public.py web
```

The `cloning-fixture` binary is built the same way with `-p cloning-fixture`. None
of these packages ships.

**The manual results.** `study.py kit` writes the frames. `probe.py play
target/capture-study/kit/exclude_on.json <record.json>` shows them full screen with
exclusion on, while a person films from flash to end. `study.py master` writes the
same sequence for an unlisted upload. `study.py recording --file <video> --label
<name>` scores either, and `read-recording` runs the reader on its frames. The
scores are `results/recording-phone.json` and `results/recording-platform.json`,
which hold each file's SHA-256.

`tools/research/2915/results/` also holds the fixtures' hashes (`fixtures.json`), all
capture checks (`capture.json`), every simulated arm (`marks.json`), the readers
(`reader*.json`) and the viewer metrics (`viewer_metrics.json`). The recordings, the
frames, the kit and the secret key stay in the ignored `target/`. The phone film
shows a room and a reflection, so no frame of it is in this repository.

**Removing the study.** `git rm -r tools/research/2915` removes it, and nothing else
depends on it. This report names those files as text, so it stays valid without
them, though it then cannot be re-run.
