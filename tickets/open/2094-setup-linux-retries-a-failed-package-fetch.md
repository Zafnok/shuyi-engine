---
id: "2094"
title: "setup-linux.sh retries a failed package fetch, so a new Codespace's setup doesn't stop at its first boot"
type: bug
milestone: "Engine: public"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] A first `apt-get` failure is retried, and the script passes when a
      later try does.
- [ ] Three failures exit non-zero with the command to run again.
- [ ] A new Codespace of the branch has Rust, the web target and the MCP
      server registered when it opens.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- By hand: steps 2 and 3, in the notes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
