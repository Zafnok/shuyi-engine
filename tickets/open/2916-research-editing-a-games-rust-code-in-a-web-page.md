---
id: "2916"
title: "Research: editing a game's Rust code in a web page, what a build server would cost, and whether scripts do the job for free"
type: research
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 2916 — Research: editing a game's Rust code in a web page

## Context

Nick, 2026-11-01: "will we implement a coderpad thing once we have hot
reload? We should do that". A code pad means a reader edits the game's
Rust on the tutorial's page and sees the game change.

Hot reload (2905) alone doesn't give that. It rebuilds on the developer's
own machine, which has a compiler. A browser has none, so the Rust must
compile somewhere else, or the page must run something other than Rust.
The playground for data files (2814, and 2820 for the tutorial) needs
neither, and is free.

Nick, 2026-11-03 ([`docs/decisions.md`](../../docs/decisions.md), *one
book, read at the reader's level*): "we def wanna have a coderpad thing
soon!" He unparked this research; renting a build server still waits for
his answer to what it reports. The tutorial is now the book's project
(2824), and the book has exercises checked by the game (2828), which edit
data files only: the code pad is what would let an exercise edit Rust.

**From 2828 (done):** an exercise is the `exercise` shortcode in a step's
`## Check yourself`, naming a check and the playground file it is done in
([ADR-0116](../../docs/adr/0116-exercises-checks-spliced-into-a-copy-of-each-step-and-run-by-its-web-build.md)).
Its check is Rust in `site/exercises/<step>.rs`, compiled into a copy of the
step (`cargo xtask exercises`), whose web build runs it through
`shuyi_app::run_game_checked` when the page writes the check's name under
`sessionStorage`'s `shuyi-page-check`, and answers
`{ kind: "shuyi-check", name, passed, text }`. An exercise that edits Rust
would need the code pad to build that copy with the reader's code in it.

**Either order with 2905:** if 2905 is done, read its ADR and
`docs/research/code-reload.md`, and measure option 4 with what it chose;
if not, measure option 4 with `shuyi web --serve` (2528) or a rebuild by
hand, and say in the report what hot reload would change, for 2905 to
check.

## Nick input

**Decision:** after this research, whether the code pad is worth its
monthly cost, and who may use it.

## Scope

**In:** compare, each with its running cost, the time from an edit to a
running game, and what it can't do:
1. **A build server:** the page sends the reader's Rust, a server builds
   the wasm and sends it back, as the Rust Playground does. The engine and
   the step are prebuilt so only the game's crate compiles (ADR-0049's
   numbers say how long). What it costs, and how untrusted code is kept
   apart: a build runs `build.rs` and macros, which are arbitrary code.
2. **A compiler in the browser:** whether `rustc` built for wasm can build
   a step at all, how big the download is, and how long it takes.
3. **Scripts:** a tutorial step whose rules are written in the script
   runtime (2101), compiled in the page and reloaded as an asset (2220),
   with no server. What the tutorial would stop teaching if it did.
4. **A reader's own machine:** `shuyi web --serve` (2528) reloading on
   each save, with the tutorial's pages open beside it: free today, and
   what it leaves out.

**Writes:** `docs/research/code-pad.md`, an ADR with the recommendation,
an `ask-nick` question with the cost in dollars a month, and, for the
recommended option, how a book exercise (2828) that edits Rust would
run on it.

**Out (do not do):**
- building any of it beyond what the numbers need;
- renting a server: that is Nick's decision.

## Implementation steps

1. Read 2814's completion notes, ADR-0049, 2828's exercise shape, and
   2905's ADR and research as the clause above says.
2. Measure each option on `site/tutorial/02-board`.
3. Write the report and the ADR, then ask Nick.

## Acceptance criteria

- [ ] Each option has its cost, its time and its limits in
      `docs/research/code-pad.md`.
- [ ] The ADR recommends one, and Nick's answer is in `docs/decisions.md`.
- [ ] The book's chapter is updated (2802), or the completion notes have a
      `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None beyond the measurements; the tickets it writes carry the tests.

## Completion notes

*(Filled in by the session that completes the ticket.)*
