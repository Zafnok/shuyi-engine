---
id: "2094"
title: "setup-linux.sh retries a failed package fetch, so a new Codespace's setup doesn't stop at its first boot"
type: bug
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-17
---

# 2094 — `setup-linux.sh` retries a failed package fetch

## Context

Found by 2916 on 2026-11-17 (`docs/research/2916-code-pad.md`, *A
Codespace*). A new Codespace of this repository on `main` (698a964b),
2-core, made with `gh codespace create`, ran the `postCreateCommand` of
`.devcontainer/devcontainer.json` (`bash scripts/setup-linux.sh && …`)
eight seconds after it booted. `apt-get install` fetched its 72 packages,
47.2 MB, then stopped:

```text
E: Failed to fetch http://archive.ubuntu.com/ubuntu/pool/main/libx/libx11/libx11-data_1.8.7-1ubuntu0.1_all.deb  rename failed, No such file or directory (/var/cache/apt/archives/partial/libx11-data_2%3a1.8.7-1ubuntu0.1_all.d…
E: Unable to fetch some archives, maybe run apt-get update or try with --fix-missing?
postCreateCommand from devcontainer.json failed with exit code 100. Skipping any further user-provided commands.
```

So the Codespace opened with no Rust: `~/.cargo/bin` didn't exist, and
the MCP server wasn't registered. Nothing said so in the editor: the
creation log (`/workspaces/.codespaces/.persistedshare/creation.log`) is
the only place. Running `bash scripts/setup-linux.sh` again by hand
four minutes later passed, in 40.5 s. The rename failing in apt's own
`partial/` folder means something else touched the cache while it
downloaded; at a Codespace's first boot that is the image's own start-up
work. It is a race, so it won't fail every time, but the book's quick
start (`site/content/learn/book/quick-start.md`, *Try it with nothing
installed*) and `README.md` link a Codespace, and a reader who hits it
has a Codespace that says Rust is set up and isn't.

## Nick input

None.

## Scope

**In:**
- `scripts/setup-linux.sh`, its `apt` branch: `add` runs `apt-get
  update` and `apt-get install`, and on a failure waits a few seconds and
  tries both again, up to three tries in all, saying on stderr which try
  failed. `dnf` and `pacman` keep one try.
- `.devcontainer/devcontainer.json`: nothing changes but what the script
  does, unless the fix needs the command split (say why in the notes).
- A message the reader can act on when every try fails: the command to
  run again by hand.

**Out (do not do):**
- an SSH server in the dev container (`gh codespace ssh` needs one; the
  editor's terminal doesn't);
- Codespaces prebuilds (Nick's decision, `docs/decisions.md`);
- a dev container per book step (2837).

## Implementation steps

1. In `scripts/setup-linux.sh`, wrap the `apt` branch's `add` in a loop
   of three tries with a short sleep between, the try's number on
   stderr.
2. Test it in a container with `bash`, on `ubuntu:24.04`, by making the
   first try fail (for example a bad mirror in `sources.list` that the
   second try's fixture puts back), and once with nothing to install.
3. Make a new Codespace of the branch (2-core), check its creation log
   says `postCreateCommand` finished and `cargo --version` answers in the
   editor's terminal, then delete it.

## Acceptance criteria

- [x] A first `apt-get` failure is retried, and the script passes when a
      later try does.
- [x] Three failures exit non-zero with the command to run again.
- [x] A new Codespace of the branch has Rust, the web target and the MCP
      server registered when it opens.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- By hand: steps 2 and 3, in the notes.

## Completion notes

- `scripts/setup-linux.sh`, its `apt` branch: `add` runs `apt-get
  update` and `apt-get install` up to three times, 10 s apart, and says
  on stderr which try failed (`setup-linux: apt-get failed on try 1 of
  3; trying again in 10 s`). After the third it says to run `bash
  scripts/setup-linux.sh` again by hand and exits 1. `dnf` and `pacman`
  keep one try. The template's copy
  (`templates/shuyi-default/base/scripts/setup-linux.sh`) changes the
  same: `shuyi new`'s test keeps it byte for byte the engine's.
  `crates/cli/src/template/plans.golden` is blessed for it: only the
  three `scripts/setup-linux.sh` hashes moved.
- `.devcontainer/devcontainer.json` is unchanged: the retry is inside the
  script, so the `postCreateCommand` needs no split.
- Step 2, in Docker on `ubuntu:24.04` (as root), with the mirror in
  `ubuntu.sources` set to `mirror.invalid` and a fixture that puts it
  back once stderr says try 1 failed: try 1 failed ("Unable to locate
  package"), try 2 installed the eight packages, curl and rustup, the
  pinned Rust and the web target; exit 0. Run again in the same
  container: "everything was there already", exit 0. In a fresh
  container with the mirror broken throughout: three tries, the
  command to run again, exit 1 after 43 s.
- Step 3: a new 2-core Codespace of the branch (`gh codespace create -m
  basicLinux32gb`, at `74c03cc2`), made after Nick's go-ahead and deleted
  after. Its creation log: the `postCreateCommand` ran from 07:24:21,
  `setup-linux: done.` with no try failed (`apt-get failed` 0 times: the
  race didn't happen this boot), then `mcp install` found `.mcp.json` and
  `.claude/settings.json` set up and wrote `.vscode/mcp.json`, and `mcp:
  ready (contract 52)` at 07:28:42. In the editor's terminal, `cargo
  --version` answered `cargo 1.99.1`, with `wasm32-unknown-unknown`
  installed. The editor asked to trust the folder before its terminal
  opened, as 2916 saw.
- Before it, the dev container's own image
  (`mcr.microsoft.com/devcontainers/base:ubuntu-24.04`) in Docker, as its
  `vscode` user through `sudo`, with the mirror broken for try 1 and put
  back: try 1 failed, try 2 passed, and `cargo --version` answered with
  the web target installed. That is where the retry ran as the
  Codespace's user: the race itself can't be made on purpose.
- The book's quick start (*Try it with nothing installed*) now says
  what to do when the Codespace's terminal doesn't find `cargo`: run
  `bash scripts/setup-linux.sh` there and open a new terminal.
- 2837's note on the race now says the script retries.
- Behaviour: templates: a new game's `scripts/setup-linux.sh` tries
  `apt-get` three times, 10 s apart, before it fails.
- No break: no Rust changed.
