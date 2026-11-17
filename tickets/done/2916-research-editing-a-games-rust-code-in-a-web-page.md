---
id: "2916"
title: "Research: editing a game's Rust code in a web page, what a build server would cost, and whether scripts do the job for free"
type: research
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: decision
completed: 2026-11-17
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
([ADR-0117](../../docs/adr/0117-exercises-checks-spliced-into-a-copy-of-each-step-and-run-by-its-web-build.md)).
Its check is Rust in `site/exercises/<step>.rs`, compiled into a copy of the
step (`cargo xtask exercises`), whose web build runs it through
`shuyi_app::run_game_checked` when the page writes the check's name under
`sessionStorage`'s `shuyi-page-check`, and answers
`{ kind: "shuyi-check", name, passed, text }`. An exercise that edits Rust
would need the code pad to build that copy with the reader's code in it.

Nick, 2026-11-16: the playground edits data files only, "but what if they
open in their codespace, it should work right? we already mention
codespaces". The book's quick start already links one
(`site/content/learn/book/quick-start.md`), and a Codespace bills the
reader's own GitHub account, so the compiler costs Nick nothing and
GitHub keeps the reader's code apart. Checked that day:

- **It has:** `.devcontainer/devcontainer.json` runs
  `scripts/setup-linux.sh`, which installs the pinned Rust, the C linker,
  the system libraries and `wasm32-unknown-unknown`, and registers the
  MCP server. Each step in `site/tutorial/` is its own crate with its own
  `Cargo.lock`, so a reader can edit its Rust, build it and test it, and
  an agent can play it headless (`frame_png`, `scene`).
- **It lacks:**
  - a window: a Codespace has no display, so `shuyi run` can't open one;
    the game shows only as its web build served on a forwarded port, by
    hand until 2528 (`shuyi web --serve`) serves and reloads it;
  - the exercises' checks: they are spliced into the site's copy of a
    step (ADR-0117) and run by the page, not by the reader's own build;
  - a quick first build: a new Codespace builds the engine first, and
    prebuilds that would skip it bill the repository's owner;
  - a link to one step: the quick start's link opens the whole engine
    repository; `codespaces.new` takes a `devcontainer_path`, so a step
    could have its own;
  - no sign-in: it needs a GitHub account, and its free hours end, so it
    adds to the page's playground and doesn't replace it.

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
5. **A Codespace:** option 4 on the reader's GitHub account, from a link
   on the step's page, with nothing installed. The time from opening the
   link to a running game, on the smallest machine, with and without a
   prebuild; what a prebuild would cost Nick a month; how many hours of a
   free account a reader gets; how the step's page links to one step
   (its own `devcontainer_path`); and how an exercise's check runs there
   without the page (a command on the step, for one).

**Writes:** `docs/research/code-pad.md`, an ADR with the recommendation,
an `ask-nick` question with the cost in dollars a month, and, for the
recommended option, how a book exercise (2828) that edits Rust would
run on it.

**Out (do not do):**
- building any of it beyond what the numbers need;
- renting a server, or turning on the Codespaces prebuilds: each costs
  Nick money, so each is his decision.

## Implementation steps

1. Read 2814's completion notes, ADR-0049, 2828's exercise shape, and
   2905's ADR and research as the clause above says.
2. Measure each option on `site/tutorial/02-board`, option 5 in a new
   Codespace of this repository.
3. Write the report and the ADR, then ask Nick.

## Acceptance criteria

- [x] Each option has its cost, its time and its limits in
      `docs/research/code-pad.md`.
- [x] The ADR recommends one, and Nick's answer is in `docs/decisions.md`.
- [x] The book's chapter is updated (2802), or the completion notes have a
      `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- None beyond the measurements; the tickets it writes carry the tests.

## Completion notes

**What was done**

- [`docs/research/2916-code-pad.md`](../../docs/research/2916-code-pad.md):
  the five ways measured on `site/tutorial/02-board`, each with its cost,
  its time from an edit to the game, and its limits.
  - **A build server:** a `dev` rebuild of the step's wasm takes 1.3 s on
    24 cores or two (the site's LTO profile 17 s, 21 s on two); about
    $10 a month for one small box, plus upkeep and the risk of untrusted
    code (`build.rs`, macros, `include_bytes!`).
  - **A compiler in the browser:** none links `wasm32-unknown-unknown`
    (rubrc links `wasm32-wasip1` and musl only).
  - **Scripts:** free and as fast as the playground, but two missing
    pieces (a script asset kind, scripted `TurnRules`) and Rhai beside a
    page that teaches Rust.
  - **The reader's own machine:** 1.3 s (web) and 1.9 s (native) a
    rebuild, after installing Rust.
  - **A Codespace:** a new 2-core one of `main` at 698a964b, made and
    deleted on Nick's account: 40 s to *Available*, 40.5 s of setup,
    136 s for the MCP server's tools, about 6.5 minutes for step 2's
    first web build, 4.9 s an edit, 4.8 GB of disk; the game played from a
    private forwarded port.
- [ADR-0126](../../docs/adr/0126-the-code-pad-a-codespace-of-the-book-not-a-build-server.md):
  one Codespace of the book's project, prebuilt once; `cargo xtask
  exercise <step> <check>` for exercises that edit Rust; no build server.
  It extends ADR-0117 (its status line and the index say so).
- Nick's answer in `docs/decisions.md` (2026-11-17): Codespaces only, and
  one prebuild kept within his plan's included storage.
- Tickets: 2094 (the dev container's setup retries a failed fetch), 2837
  (the book's Codespace, with Nick's prebuild setup), 2838 (the check
  command and the first exercise that edits Rust), in `docs/ROADMAP.md`.

**Deviations**

- The report is `docs/research/2916-code-pad.md`, not
  `docs/research/code-pad.md`: every report there starts with its
  ticket's number.
- The Codespace's first boot failed: its `postCreateCommand` stopped on a
  race in `apt` ("rename failed" in apt's `partial/` folder), and the
  Codespace opened with no Rust. The setup was run again by hand and
  timed (it passed); 2094 fixes the script.
- The image has no SSH server, so `gh codespace ssh` failed; one was
  installed in that Codespace (`apt-get install openssh-server`, from its
  editor's terminal) to run the timings from here. Nothing in the
  repository changed for it.
- The ticket asked for a link to one step (`devcontainer_path`). Nick's
  choice of one free prebuild made it one container for the whole
  project instead, since a prebuild is per container (ADR-0126's
  alternatives).
- Option 4 was measured with a rebuild by hand: 2528 (`shuyi web
  --serve`) and 2905 are open. The report says what each would change;
  2905's clause on this ticket now says what holds, and 2528 names 2837.
- The playground's **Run** wasn't timed in the page: the browser pane was
  hidden, so its frames didn't run. The scripts' time is given as the
  playground's reload plus Rhai's compile time from ADR-0093.

Book: none, no public API changed; the book changes with 2837 and 2838,
which link the Codespace from each step's page and add exercises that
edit Rust.

No break.

Nick input: answered (`docs/decisions.md`, 2026-11-17). 2837 asks him to
set up the one prebuild once its container is on `main`.
