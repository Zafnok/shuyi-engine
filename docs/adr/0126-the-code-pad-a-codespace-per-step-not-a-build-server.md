# ADR-0126: The code pad: a Codespace per step of the book's project, with a command for an exercise's check, and no build server until Nick asks for one

- **Status:** Accepted
- **Date:** 2026-11-17
- **Related tickets:** 2916; 2837 and 2838 (they build it), 2094 (the dev
  container's first boot); 2905 (hot reload of code), 2528 (`shuyi web
  --serve`), 2828 (exercises)
- **Extends:** ADR-0117 (an exercise's check, run outside the page)

## Context

Nick wants a code pad (2026-11-01: "will we implement a coderpad thing once
we have hot reload? We should do that"; 2026-11-03: "we def wanna have a
coderpad thing soon!", `docs/decisions.md`): a reader of the book edits a
step's Rust and sees its game change. The page's playground (2814, 2820)
edits data files only, and the book's exercises (2828, ADR-0117) are checked
on data files only, because the game reads those at load and Rust must be
compiled.

2916 measured five ways on step 2 of the book's project
(`docs/research/2916-code-pad.md`):

1. **A build server** compiles the reader's crate in the template's `dev`
   profile in 1.3 s, on 24 cores or 2 alike (the site's LTO profile takes
   17 s), and sends back 1.2 MB gzipped: about 2 s from a click to the
   game. It costs about $10 a month on one small box in Europe, or a dollar
   or two scaled to zero, but it runs untrusted code (`build.rs`, macros,
   `include_bytes!` of any path), must be kept apart (a container with no
   network, gVisor or Firecracker), patched and watched, and can be spent
   by anyone without a sign-in. Nothing in the engine runs a server today.
2. **A compiler in the browser** doesn't exist for this: the one rustc that
   runs in a browser and links (rubrc) links `wasm32-wasip1` and musl only,
   and the page would download rustc, LLVM and the engine compiled.
3. **Scripts** (2101) cost nothing and reload as fast as the playground,
   but need a script asset kind and scripted `TurnRules`, neither of which
   exists, and would edit Rhai beside a page that teaches Rust.
4. **The reader's own machine** rebuilds in 1.3–1.9 s, free, but starts
   with installing Rust (and MSYS2 on Windows), and has none of the
   exercises' checks.
5. **A Codespace** is way 4 on a machine the reader's own GitHub account
   pays for (60 free hours a month on 2 cores), opened from a link, in the
   engine's repository, where the checks of `site/exercises/` already are.
   Its first boot failed once on a race in `apt` (2094).

## Decision

### 1. The code pad is a Codespace, one per step of the book's project

- Each step of the book's project with Rust to edit gets a dev container,
  `.devcontainer/<step>/devcontainer.json` (`02-board`, …), one folder
  deep as Codespaces reads them. It is the root's container (the same
  image, `scripts/setup-linux.sh`) with the step's folder as
  `workspaceFolder`, its `postCreateCommand` building the step's web build
  once in `dev` (`cargo shuyi web`) after the setup, and the port the web
  build is served on forwarded, private (only the reader, signed in, opens
  it), opened in a preview tab.
- The step's page links it, `https://codespaces.new/Zafnok/shuyi-engine?devcontainer_path=.devcontainer/<step>/devcontainer.json&machine=basicLinux32gb`,
  beside its playground, saying what it is in the book's words: a computer
  in the browser, paid from the reader's own GitHub account's free hours,
  where they change the step's Rust. The smallest machine is named in the
  link, because the step's rebuild is no faster on more cores.
- The step's game shows as its web build, served from the Codespace and
  rebuilt by hand (`cargo shuyi web`, then reload) until 2528's `shuyi web
  --serve` rebuilds and reloads it on save; the dev container then runs
  that instead. 2905's hot reload, when it lands, changes what the reader
  sees after a rebuild (the same moment, not the start), not the way.
- The page's playground stays as it is, for data files, with no account.
- 2837 builds this.

### 2. An exercise's check runs as a command

- `cargo xtask exercise <step> <check>` makes the step's copy as `cargo
  xtask exercises` does (ADR-0117 §1), from the step's folder as the
  reader left it, builds a headless program in the copy that reads the
  step's data as its web build does and runs the named check from its
  `CHECKS`, and prints `Done.` or `Not yet.` and the check's text, with
  exit code 0 only when it passed. A step whose files have problems prints
  them, as the page does, and no result.
- The checks' shape doesn't change: `fn(&BasicData) -> Result<String,
  String>`. A check of the reader's Rust plays commands through the step's
  rules (`TurnRules::apply`), as step 3's `wall-right` already does.
- An exercise that edits Rust is the `exercise` shortcode with a new
  `code` (in place of `file`): the step's source file it is done in. Its
  page says to do it in the step's Codespace and run the command, in place
  of **Check my work**; the twin says the same. `cargo xtask exercises`
  checks its tests as today: each passes on the step with its exercise's
  edit and fails without.
- 2838 builds this, with the first such exercise.

### 3. No build server, until Nick decides to pay for one

A pad in the page itself is a build server (way 1), as §1 of the research
says; it waits for Nick's answer to its monthly cost, recorded in
`docs/decisions.md`. If he says yes, its ticket starts from the research's
numbers: the template's `dev` profile stripped, the steps' target folders
built into one image, a container a build with no network and a time,
memory and size limit, only the step's `src/` accepted, and a sign-in with
a limit per reader.

## Consequences

- A reader edits the step's real Rust, with the compiler's own errors, in
  the book's own project, and the exercises can ask for Rust: nothing the
  book teaches is left out.
- It costs Nick nothing: each reader's Codespace is on their own account.
  A prebuild, which would skip the first build, is his to turn on.
- A reader needs a GitHub account, and the pad is a new tab with an
  editor, not the page. The first Codespace takes minutes before the game
  shows (the research's table); a later one resumes in seconds.
- A reader in a Codespace of the engine's repository sees the whole
  repository; the step's own container opens its folder, and the rest is
  there if they look.
- Ten more dev containers to keep in step with the root's: they are the
  same file but for the folder; 2837's test checks that.

## Alternatives considered

- **A build server now** — the only way to edit Rust in the page, and
  cheap to run (about $10 a month), but it costs more in upkeep and risk
  than in money, and it is Nick's money (§3).
- **A compiler in the browser** — no browser rustc links
  `wasm32-unknown-unknown` today; to look at again when one does.
- **Scripts in place of Rust** — free and fast, but the book teaches Rust
  through its project, and a script pad edits another language; it needs
  two pieces that don't exist. Kept for a chapter on scripting, if the
  book gets one.
- **The reader's own machine only** — free and the fastest loop, but a
  newcomer installs Rust (and MSYS2 on Windows) before the first edit, and
  the checks aren't there. The command of §2 works there too for a reader
  with a clone of the repository.
- **One Codespace for the whole repository** (the quick start's link) — it
  opens at the root, with no step built and no port forwarded; a reader
  must find the step's folder and serve its build themselves.
