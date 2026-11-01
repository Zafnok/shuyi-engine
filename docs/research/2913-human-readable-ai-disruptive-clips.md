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
