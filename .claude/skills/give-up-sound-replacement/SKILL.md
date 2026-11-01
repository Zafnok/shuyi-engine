---
name: give-up-sound-replacement
description: Scrap the whole quad-snd replacement in one session, when Nick says to ("let's give up on sound replacement"), at any stage until quad-snd is removed. Only ever on Nick's own word; never because a ticket got hard or long.
---

# Give up on the sound replacement

Nick may decide at any stage that the replacement isn't worth it. His word is
all this needs: don't ask him to confirm, and don't ask him anything technical.
It ends in one PR, in this session
([ADR-0068](../../../docs/adr/0068-the-sound-replacement-stays-reversible-at-every-stage.md)).

**Nobody else decides this.** A coding agent that hits a roadbump finishes what
it can, writes the tickets the roadbump needs, and tells Nick plainly what it
found and what it now costs. It never gives up, stops the chain or drops a
ticket on its own judgement, however large the cost looks.

If 3079 is in `tickets/done/`, quad-snd is already gone and this skill with it:
going back is then a revert of that one commit, and harder.

## Steps

1. Branch from an up-to-date `main`: `give-up-sound-replacement`.
2. Take the code out, then let Cargo drop what only it used:

   ```bash
   cargo xtask sound-parity give-up --apply
   ```

   ```bash
   cargo check --workspace --all-targets
   ```

   If the first says the footprint has drifted it changes nothing: fix what it
   names by hand (it is the same removal), and carry on.
3. Settle the tickets, as 2211–2214 were dropped. Each of 2210, 3030, 3031, 3032,
   2397, 3084 and 3079 still in `tickets/open/` gets `status: done`, today's
   `completed`, a `> **Closed as dropped (Nick, <date>):**` note under its heading,
   its criteria struck through, a completion note saying nothing was left in the
   tree, and moves to `tickets/done/`. Take them out of the `blocked_by` of any
   other ticket, and rewrite what `cargo xtask ticket-lint` then names. Work
   that doesn't depend on the replacement, like 2234 and 2378, goes on.
4. Write the record:
   - `docs/decisions.md`: a dated part with his words, verbatim;
   - `docs/ROADMAP.md`: the sound line says the chain was scrapped, and quad-snd
     stays;
   - ADR-0066, ADR-0067 and ADR-0068: only their `Status:` line, and their rows in
     the index, say `Abandoned <date>`, with a new short ADR saying why, in his
     terms. 2902's research stays as it is.
5. If the game moved its pin past 3084, or has a loop region in its files, write
   the game's handoff (its `engine-change` skill): it moves its pin to this PR
   and drops those fields. Say so in the PR. Until then it stays on its old pin
   and keeps working.
6. Run the `run-gates` skill, then open the PR with the `work-ticket` skill's
   format, `Nick input: he decided this`, and a `Breaking:` line only if 2397 had
   merged. Tell Nick in plain words what is gone and that the game sounds as it did.
