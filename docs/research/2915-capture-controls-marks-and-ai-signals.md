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
