# The code pad: editing a game's Rust from the book, and what each way costs

Research for ticket 2916, measured **2026-11-16 and 17**. The choice it
leads to is [ADR-0126](../adr/0126-the-code-pad-a-codespace-of-the-book-not-a-build-server.md);
Nick's answer on paying for a server is in [`decisions.md`](../decisions.md).
This page holds the numbers.

**What a code pad is:** a reader of the book changes a step's Rust (its
rules, its screen) and sees the step's game change, with no Rust installed.
The playground of each step (2814, 2820) already does this for data files,
for free, because the game reads them at load. Rust has to be compiled, and
a browser has no compiler.

**In short:**

| Way | Costs Nick a month | From an edit to the changed game | What it can't do |
| --- | ------------------ | -------------------------------- | ---------------- |
| 1. A build server | about $10 for one small box, plus its upkeep; more if it is abused | about 2 s (1.3 s to build, the rest to fetch and start) | run without someone keeping a server of untrusted code safe, patched and paid |
| 2. A compiler in the browser | nothing | not possible today | link for `wasm32-unknown-unknown`; nobody ships one that does |
| 3. Scripts | nothing | about the playground's **Run**: the game reloads, the script compiles in milliseconds | teach Rust: the book's project teaches the Rust each step uses, and a script is not that Rust |
| 4. The reader's own machine | nothing | 1.3 s for the web build, 1.9 s native, then a reload | start with nothing installed: Rust, and on Windows MSYS2, come first |
| 5. A Codespace | nothing; one prebuild fits Nick's included storage, one a step would bill about $2 | 5 s on its 2 cores, then a reload; about 10 minutes from the link to the first game, less with a prebuild (not measured) | stay in the page, or work without a GitHub account |

**The choice** (ADR-0126, Nick's decision of 2026-11-17): way 5, one
Codespace of the book's project with every step built, prebuilt once within
Nick's included storage, opened from a link on each step's page, with a
command that runs an exercise's check there. It is free to Nick, teaches the
Rust the book teaches, and runs every check the site runs. A build server
(way 1) is the only way to edit Rust in the page itself; Nick chose not to
have one.

## Method

- **The machine:** Nick's (Windows 11, x86-64, 24 cores), Rust 1.99.1
  (`x86_64-pc-windows-gnu`), the engine at `698a964b`.
- **The step:** `site/tutorial/02-board`, the book's project at step 2
  (663 lines of Rust in its game crate, a `build.rs`, the engine by path).
- **A rebuild** is the time cargo reports (`Finished … in`) after a line is
  appended to `crates/game/src/rules.rs`, three times, as ADR-0049 times a
  rebuild. Builds went to a target folder of their own outside the
  repository. The script is below, *Running it again*.
- **The Codespace:** a new one of this repository on `main`, made with
  `gh codespace create -m basicLinux32gb` (2 cores, 8 GB, 32 GB), on Nick's
  account, deleted after.
- **The web:** prices and limits read on the pages linked, on 2026-11-16.

## What the site's step builds

The site's step 2 plays `play/box-pusher.wasm`: 2,933,507 bytes, 972,439
sent gzipped. The book's copy builds it with `cargo shuyi web --release`
(ADR-0117 §2), whose profile is the template's: thin LTO and one codegen
unit.

Each step is its own workspace with its own lock, and they all name the
engine by path, so a build of one step builds the engine once into that
target folder, and every later edit rebuilds only the step's crate.

## 1. A build server

The page sends the reader's files to a server; the server builds the step
with them and sends the `.wasm` back, as the Rust Playground does
(`github.com/rust-lang/rust-playground`). The engine and the step's other
crates are built once, ahead, so only the step's own crate compiles.

### How long a build takes

On `02-board`, built for `wasm32-unknown-unknown`, in seconds:

| Profile | Clean (the engine too) | Rebuild after an edit | Rebuild, 2 jobs (`-j 2`) | Its `.wasm` (gzipped) |
| ------- | ---------------------- | --------------------- | ------------------------ | --------------------- |
| The site's (`release`: thin LTO, 1 codegen unit) | 46.3 | 16.8, 17.1, 17.1 | 21.1, 21.2, 21.1 | 2,931,788 (960,495) |
| `release` without LTO, 16 codegen units | 33.3 | 4.84, 4.86, 4.95 | 9.20, 9.54, 13.22 | 3,125,481 (920,649) |
| `dev` (the template's: no debug info, dependencies at `opt-level = 2`) | 26.8 | 1.19, 1.34, 1.29 | 1.26, 1.33, 1.35 | 8,148,284 |
| `dev` with `strip = true` | — | 1.20, 1.22 | — | 5,111,665 (1,180,977) |

What it says:

- **The site's own profile can't serve a pad.** Its LTO links the whole
  program again on every edit: 17 s on 24 cores, 21 s on two.
- **`dev` is the pad's profile.** Its rebuild is 1.3 s whatever the cores,
  because one crate compiles and the link is small: a server with 2 vCPU
  does it as fast as Nick's machine. The engine is still built optimized
  (`opt-level = 2` for dependencies), so the game plays at speed; only the
  reader's crate isn't. Stripped, it sends 1.2 MB gzipped, 0.2 MB more than
  the site's build.
- From the reader's click to the changed game: about 0.1 s to send the
  files, 1.3 s to build, 0.2–0.5 s to fetch 1.2 MB, then the game starts as
  the playground's **Run** starts it. About 2 s.
- **What the server keeps:** the step's target folder, built ahead. Both
  web profiles of one step take 603 MB, `release` alone 140 MB. The ten
  steps lock the same engine, so one image of about 1–2 GB holds them all.

### Keeping the reader's code apart

A build runs code of the reader's: `build.rs` (every step has one), a
procedural macro, and `include_bytes!` or `include_str!` of any path on the
server, which reads a file at compile time without running anything. So the
build runs where there is nothing to read and nothing to reach:

- **What the Rust Playground does**
  (`compiler/base/orchestrator/src/coordinator.rs`): a Docker container a
  job, `--net none`, `--memory 512m`, `--pids-limit 512`, `--cap-drop=ALL`,
  and a time limit. Its wasm output removes every dependency, so it can't
  build a game.
- **Stronger:** gVisor (`runsc`) puts a kernel of its own between the
  build and the host, and is slower on calls to the system, which a
  compiler makes many of. Firecracker gives each build a small virtual
  machine, the strongest, and needs a host with KVM (bare metal).
- **And still:** a time and memory limit per build, a size limit on what
  is sent, the server accepting only the step's `src/` (never
  `Cargo.toml`, `build.rs` or a new dependency), and a limit on builds per
  reader. Without a sign-in, anyone can make the server compile, so a
  public pad is also a way to spend Nick's money or CPU.

### What it costs

Prices on 2026-11-16, before tax:

| Host | What | A month |
| ---- | ---- | ------- |
| Hetzner Cloud (`hetzner.com/cloud`) | CX33, 4 vCPU, 8 GB, x86, always on (Germany, Finland) | €8.99 (about $10.50) |
| Hetzner Cloud | CPX31, 4 vCPU, 8 GB, in the US | €62.99 |
| Fly.io (`fly.io/pricing`) | `performance-2x`, 4 GB, run only while building (per second), stopped otherwise | under $1 at 100 builds a day, plus $0.15 a GB a month of stopped disk, about $0.30 for 2 GB |
| Fly.io | `performance-4x`, 8 GB, always on | $132 |
| Google Cloud Run (`cloud.google.com/run/pricing`, not confirmed: the page didn't render) | per vCPU-second and GiB-second; the free tier is 180,000 vCPU-seconds and 360,000 GiB-seconds | $0 at 100 builds of 5 s a day on 4 vCPU |
| AWS Fargate (`aws.amazon.com/fargate/pricing`) | $0.0405 a vCPU-hour, $0.00445 a GB-hour, 1 minute at least a task | about $3.50 at 100 builds a day |

The compute is cheap: **about $10 a month for one always-on box** in
Europe, or a dollar or two scaled to zero (where a build waits a second
or more for the machine to start, and the image of 1–2 GB to load). The
real costs are elsewhere:

- **Upkeep:** a server is a thing to patch, watch and keep safe, for as
  long as the pad is up. Nick reviews no code (`AGENTS.md`), so an agent's
  ticket keeps it, and someone pays when it breaks.
- **Abuse:** a public build service is spent by whoever finds it. A
  sign-in (GitHub's) and a limit per reader keep the bill bounded, and are
  more to build.
- **A new kind of thing:** nothing in the engine runs a server today, and
  the site is static on GitHub Pages.

## 2. A compiler in the browser

Whether `rustc` itself, built for WebAssembly, could build a step in the
page:

- **rubrc** (`github.com/oligamiq/rubrc`, MIT OR Apache-2.0) runs rustc
  with LLVM, Cargo and clang in a browser's workers. It links programs
  for `wasm32-wasip1` and `x86_64-unknown-linux-musl` only: "Other targets
  are known to fail during the linking process". A step's game is
  `wasm32-unknown-unknown`. It is a pre-release, "not yet ready for general
  production use", and doesn't say its download size.
- **bjorn3's `wasm-rustc`** is a patched rustc for `wasm32-wasip1-threads`,
  used by `browser_wasi_shim`'s demo; what it can link isn't stated.
- **Cranelift**, the fast back end, can't emit WebAssembly: its back ends
  are x86-64, aarch64, s390x and riscv64.
- **Miri in the browser** (`rubri`) interprets, with no crates and slowly.

Even working, the page would download rustc and LLVM (hundreds of MB),
plus the engine and its dependencies already compiled for
`wasm32-unknown-unknown` (the 140 MB release folder above holds them), and
compile in a wasm32 memory of 4 GB at most. Nothing here was measured on a
step, because nothing links one: **not possible today**. Worth looking
again when a browser rustc links `wasm32-unknown-unknown`.

## 3. Scripts

A step whose rules are a script of the script runtime (2101, Rhai,
ADR-0093), which the game compiles at load, could take the reader's script
as the playground takes a data file: written to `sessionStorage`, laid over
the game's own by `page_layer` (3069), read at load. No server, no
compiler.

- **Time to the changed game:** the playground's **Run**, which reloads the
  game's frame with its `.wasm` cached; compiling the script adds little,
  as Rhai compiles 300 functions in 2.0 ms on Nick's machine (ADR-0093).
- **Cost to the game:** the runtime adds 1,373,820 bytes to a web build
  (2101's measure on Hello, Grid), against the 2.9 MB of step 2.
- **What it needs that isn't there:** a script asset kind and format, so
  a script is a file the registry decodes and 2220 reloads (2905's option 4
  names the same gap), and rules a game writes as a script: a
  `TurnRules` whose `check` and `apply` call the script's functions, with
  the state passed in and out as values (ADR-0093 §5's views and values).
  Neither exists.

**What the book would stop teaching.** The book's project is how a reader
learns Rust as they make the game (decision of 2026-11-01; each step's *What
you learned* lists its Rust). Step 3 teaches `TurnRules` and iterator
chains; step 4 enum variants with fields and "letting the compiler's errors
walk you through a change"; step 5 ownership and `clone`. In a script none
of that is there: Rhai has no types to declare, no traits, no ownership and
no compiler errors before the game runs. A script pad would edit a
different language from the one the page teaches, beside it. It can be the
right tool for a chapter on scripting, or for modders; it is not a Rust
code pad.

## 4. The reader's own machine

The reader installs Rust (the quick start), edits the step in their own
copy, and runs it. Free, and the fastest loop:

- **Native:** a one-line edit rebuilds in 1.85–1.91 s, then `cargo run`
  opens the window.
- **The web build:** the `.wasm` rebuilds in 1.2–1.3 s (`dev`); `cargo
  shuyi web` also rebuilds the step's tool program, which links the step's
  crate (in the Codespace below, 4.9 s in all on 2 cores), then a reload
  of the page. 2528 (`shuyi web --serve`, open) will serve the folder
  and reload the page on each change; until then the reader serves `dist/`
  and reloads by hand.
- **Hot reload of code** (2905, open) would keep the game's moment across
  the rebuild instead of starting it over; it doesn't change the rebuild's
  time, which is what the reader waits for. 2905 is to check this when it
  chooses.
- **The first build** builds the engine: 31 s native, 27 s for the web,
  on 24 cores; CI's 4-core runners take 44–65 s on Linux and 89–154 s on
  Windows for a new game (ADR-0120).

**What it leaves out:**

- **Starting with nothing:** rustup and Rust first, and on Windows MSYS2
  and its compiler (`AGENTS.md`, *Environment*), before the first build. A
  reader new to programming is the book's reader.
- **The exercises' checks:** they are compiled into the site's copy of a
  step and run by its page (ADR-0117). A reader's own build has none, and
  their project is their own folder, not the engine's repository, so
  `cargo xtask exercises` isn't there either.

## 5. A Codespace

Way 4, on a machine GitHub runs and the reader's own account pays for,
opened from a link, with nothing installed. The repository has a dev
container: `.devcontainer/devcontainer.json` runs `scripts/setup-linux.sh`
and registers the MCP server. The book's quick start links one
(`codespaces.new/Zafnok/shuyi-engine?quickstart=1`).

### What it costs, and who pays

From `docs.github.com/en/billing/concepts/product-billing/github-codespaces`
on 2026-11-16:

- **The reader pays,** from their own account: "the person who creates
  the codespace", unless an organization pays for its members. A personal
  account can't pay for others. The creation page says so ("Codespace
  usage for this repository is paid for by …").
- **Free each month:** 120 core-hours and 15 GB-month on GitHub Free, 180
  and 20 on Pro. The smallest machine (2 cores, 8 GB, 32 GB) counts twice,
  so **60 hours of a 2-core Codespace a month** on a free account. Past
  that, $0.18 an hour, or nothing more runs if the account has no payment
  method.
- **It stops** after 30 minutes idle (the default), and a stopped one keeps
  only its disk.

### The time to a running game

On a new 2-core Codespace (`basicLinux32gb`) of `main` at `698a964b`, in
seconds:

| Step | Seconds | What happens |
| ---- | ------- | ------------ |
| `gh codespace create` until it is *Available* | 40 | GitHub makes the machine and clones the repository |
| `bash scripts/setup-linux.sh` | 40.5 | the system packages, rustup, the pinned Rust, the web target (run by hand, after its first try failed: below) |
| `cargo build --bin shuyi` | 135.8 | the engine's tools, for the MCP server the `postCreateCommand` registers |
| `cargo shuyi web --release` in `02-board`, the first | 412.9 | the step's tool program (native, about 306 s), then the web build (107 s) |
| `cargo shuyi web`, `dev`, the first | 94.5 | the web build in `dev` (93 s); the tool program was built |
| an edit, then `cargo shuyi web` | 4.9 | the web build, 0.91 s; the tool program, which links the step's crate, built again |
| an edit, then `cargo shuyi web --release` | 37.2 | the web build with LTO, 35.8 s |
| `cargo test`, the first | 11.7 | |
| the game from a forwarded port | 0.7 | GitHub's sign-in hop, then the 2.9 MB `.wasm` |

So, without a prebuild, **about 10 minutes from the link to the game**: 40 s
for the machine, 3 minutes of setup the `postCreateCommand` runs, then the
reader's first `cargo shuyi web`, about 6.5 minutes (the tool program and
the engine for the web, in `dev`). After that, **5 s from an edit to a new
build** on 2 cores, then a reload. The disk held 4.8 GB at the end (the
step's target 1.6 GB, the root's 0.9 GB, Rust and its tools 1.2 GB).

**Its first boot failed.** The `postCreateCommand` stopped 8 s in:
`apt-get` fetched its 72 packages and then failed to rename one in its own
`partial/` folder ("rename failed, No such file or directory"), which
means something else touched apt's cache at the same moment. The
Codespace opened with no Rust and said nothing in the editor; the creation
log is the only place it shows. The same script, run again by hand, passed
in 40.5 s. It is a race at the first boot, so it won't fail every time, but
a reader of the quick start can meet it today: 2094 retries the fetch.

Also seen:

- **The editor asks whether to trust the folder** before it opens a
  terminal. A reader has to say yes.
- **No SSH server** is in the image, so `gh codespace ssh` fails until one
  is installed; the editor's terminal works without it.
- **No window:** the game shows as its web build, served on a forwarded
  port. A port is private by default: only the reader, signed in to
  GitHub, can open it, which is what a reader wants.

### A prebuild

A prebuild is a Codespace already set up and built, which a new one copies
instead of running the setup. The repository's owner pays for it
(`docs.github.com/en/codespaces/prebuilding-your-codespaces/about-github-codespaces-prebuilds`):

- the GitHub Actions minutes that make it, each time it is made;
- its storage, as a Codespace's ($0.07 a GB-month), once per region and per
  version kept: "4 regions with 2 retained versions could mean charges for
  up to 8 prebuilds".

With the Codespace above (4.8 GB used after the step's builds), one
region and one version kept, made only when the dev container changes
(not on every push):

- **storage:** about 5 GB a prebuild, at $0.07 a GB-month: $0.35 a
  month for one, $3.50 for one a step (ten), first from the storage
  Nick's plan includes (20 GB-month on Pro), which his own Codespaces
  share. One fits it; ten would bill about $2 a month past it;
- **making it:** about 10 minutes of a 2-core runner each time a dev
  container changes, a few times a month;
- **what it saves a reader:** the setup and the first build, most of the
  10 minutes above. How long a Codespace takes to start from one was not
  measured: no prebuild was made.

It is Nick's money and his decision (the ticket's *Out*): this ticket
turns nothing on. He chose one prebuild, within his included storage
(`decisions.md`, 2026-11-17), which a single dev container for the whole
project allows: a prebuild is made per dev container.

### A link to one step

A repository can have several dev containers, each
`.devcontainer/<name>/devcontainer.json`, one folder deep; the creation
page offers them in a list. A link names one with `devcontainer_path`,
which `gh codespace create --devcontainer-path` and the REST API take too.
Tried on 2026-11-16, the creation page took `machine=standardLinux32gb`
from a link (it showed 4-core) and, for a `devcontainer_path` that doesn't
exist, fell back to the one there is without a word. So a link can name
a container of the book's project as soon as it exists; it can open the
project's folder (`workspaceFolder`), build the steps in its
`onCreateCommand` (which a prebuild runs), and forward the web build's
port. A link can't open a folder of its own choosing, so one container for
every step opens at the project's folder, and the step's page names the
step's.

### An exercise's check there

The checks live in `site/exercises/` and run in a copy of the step
(ADR-0117), so in a Codespace of this repository they are there, beside the
step the reader edits. `cargo xtask exercises <step>` already copies the
step as it is and runs the checks' tests, but those tests are the author's
(each check passes on the step's own files *with its exercise's edit*, laid
over them by `edits.rs`), not the reader's. What it lacks is one command
that runs one check on the step as the reader left it and prints its
answer, something like:

```text
$ cargo xtask exercise 03-walk wall-right
Not yet. The player walked into the wall at (4, 2): `check` should refuse a
move onto a wall.
```

That is a copy as today, then a small headless program in the copy that
reads the step's data as its web build does and calls the check by name;
the checks' shape (`fn(&BasicData) -> Result<String, String>`) doesn't
change. An exercise that edits Rust is then written as today, with a check
that plays commands through the reader's rules (`TurnRules::apply`, as step
3's `wall-right` does), and its page says to run that command, in place of
**Check my work**.

## Running it again

The rebuilds, from the root, in Git Bash (`<out>` a folder outside the
repository):

```bash
step=site/tutorial/02-board; rules=$step/crates/game/src/rules.rs
cd $step
CARGO_TARGET_DIR=<out> cargo build -p box-pusher --target wasm32-unknown-unknown
for i in 1 2 3; do echo "// edit $i" >> crates/game/src/rules.rs
  CARGO_TARGET_DIR=<out> cargo build -p box-pusher --target wasm32-unknown-unknown; done
git checkout -- crates/game/src/rules.rs
```

`--release`, with `--config profile.release.lto=false --config
profile.release.codegen-units=16` for the second row, and `-j 2` for two
jobs. The Codespace's run is the same steps after `bash
scripts/setup-linux.sh`, with `cargo shuyi web` in place of `cargo build`.
