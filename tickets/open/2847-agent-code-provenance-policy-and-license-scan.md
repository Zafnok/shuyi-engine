---
id: "2847"
title: Agent-written code has a provenance policy, and a PR that adds license text outside a notice file fails
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: ["3529"]
nick_input: none
completed:
---

# 2847 — Agent-written code has a provenance policy, and PR diffs are scanned for license text

## Context

Almost all of this repository's code is written by coding agents.
`AGENTS.md` says Nick "does not review code", and every recent commit
carries a `Co-Authored-By: Claude` trailer. Nothing in `tickets/`,
`docs/adr/` or `docs/decisions.md` says who the author of that code is,
how much of it is protected, or how copied code is caught. A search for
`copyrightab`, `AI-generated`, `human author` and `Copyright Office` over
the tickets and the docs finds nothing (blind-spot audit, 2026-11-23,
finding 95).

Two things make this worth a short policy and a cheap check:

- **Authorship.** The US Copyright Office's report on copyrightability
  (Part 2, January 2025) and *Thaler v. Perlmutter* (D.C. Cir., March
  2025) hold that material an AI generates without enough human creative
  control is not protected, and that prompts alone are not enough. The
  license grant still works for users. But `LICENSE-MIT`'s one line,
  `Copyright (c) 2026 Nick Wentz`, may be thin for purely agent-written
  parts, and so may the attribution and Apache-2.0's patent conditions
  that rest on it. The human part that exists is real: Nick's decisions,
  and the selection and arrangement written down in tickets, ADRs and
  `docs/decisions.md`. Nobody has written that down as such.
- **Copied code.** The realistic way to infringe is a model reproducing
  code it memorised, GPL or LGPL above all. ADR-0021 §2's clean-room rule
  ("Never open another implementation's source") governs what an agent
  reads, not what a model remembers, so it is not a clean room in the
  legal two-team sense. Some work sits near GPL code: the DOOM tickets
  (2780–2784) drive GPL doomgeneric in a repository of its own, and the
  video research (2909) sits near LGPL libraries. No CI step looks for
  license text in a PR's diff. `cargo deny` (ADR-0002 §8) checks crates,
  not code pasted into ours.

What the repository holds today, checked with `git grep` for licence
phrases (`Permission is hereby granted`, `Licensed under the Apache
License`, `GNU General Public`, `GNU Lesser`, `Mozilla Public License`,
`SPDX-License-Identifier`, `Copyright (c)`): no GPL, LGPL or MPL text
anywhere outside `tickets/` and `docs/`, no SPDX header, and permissive
texts only in notice files: `LICENSE-MIT`,
`crates/assets/src/message/CLDR-LICENSE.txt`,
`crates/cli/web/mq_js_bundle-LICENSE-MIT.txt`,
`crates/cli/web/sapp_jsutils-LICENSE-MIT.txt`, the `Terminus-LICENSE.txt`
copies (under `examples/`, `site/`, `templates/`), and NOTICES.txt's
writer and snapshot (`crates/cli/src/notices.rs`,
`crates/cli/src/notices/`, `crates/cli/src/web.rs`). So a scan of the
diff starts clean.

The policy is Nick's: decision 3529 asks him whether he wants a policy
on agent-written code, how the copyright line should read, and whether
he wants a one-off lawyer's opinion before the first release. This ticket
writes down his answer and builds the scan, which is technical.

Related: 1132 (parked) writes `CONTRIBUTING.md`, which "covers how
AI-assisted work is judged" for outside contributors. 2849 writes the
other community files. 2846 writes the SBOM. Research 2915 sorted
AI-use *signals* for previews (C2PA and so on). That is about media a
game shows, not about this repository's code.

## Nick input

`None.` here: the question is decision 3529, which this ticket waits
for. Its answer in `docs/decisions.md` is what step 1 writes up.

## Scope

**In:**
- An ADR, `docs/adr/NNNN-agent-written-code-provenance-and-the-human-record.md`
  (the `write-adr` skill), that says:
  - what the human authorship is and where it is recorded: Nick's
    decisions, and the selection and arrangement in tickets, ADRs and
    `docs/decisions.md`, linked from each PR by its `[NNNN]` title
    (ADR-0092);
  - that agent output is accepted under the gates (`AGENTS.md` rule 6),
    and that the commit trailers stay as they are;
  - what ADR-0021's clean room does and does not cover, and the added
    rule: **GPL, LGPL, AGPL or MPL source is never pasted into, or kept
    open in, a session that edits this repository**;
  - the copyright line as Nick decided in 3529 (for example "Nick Wentz
    and the Shuyi Engine contributors"), or that it stays;
  - what the scan below catches and what it cannot (a memorised snippet
    with its notice stripped).
- `LICENSE-MIT`'s copyright line (`LICENSE-APACHE` has no appendix or
  copyright line), changed only if 3529's answer says so.
- One line in `AGENTS.md` rule 7 (*Licenses*) pointing to the ADR.
- `cargo xtask check-license-text --base <rev>`: a new module
  `crates/xtask/src/license_text.rs`. It reads the lines the branch adds
  since the merge base (`git diff -U0`, as `pr_size::changed_files` finds
  the base), plus untracked files when run locally, and fails each added
  line that matches a license phrase, outside an allowed path. It prints
  `path:line: <phrase>` and says where a notice belongs
  (`THIRD_PARTY_ASSETS.md` once it exists, a `*-LICENSE*.txt` beside the
  file, and `cargo deny` for a crate).
  - Phrases, case-insensitive: `GNU General Public License`,
    `GNU Lesser General Public`, `GNU Affero`, `Mozilla Public License`,
    `GPL-2.0`, `GPL-3.0`, `LGPL-`, `AGPL-`, `MPL-2.0`,
    `SPDX-License-Identifier`, `Permission is hereby granted`,
    `Licensed under the Apache License`, `All rights reserved`, and a
    line starting (after comment marks) with `Copyright (c)`,
    `Copyright ©` or `Copyright 19`/`20` followed by a year.
  - Allowed paths: `LICENSE-*` at the root, any `*LICENSE*.txt` or
    `*NOTICE*` file, `*.snap`, `tickets/`, `docs/`, and the paths listed
    in `tools/license-text-allow.txt` (one path or glob per line, each
    with a `#` comment saying why). That file starts with the NOTICES.txt
    writer (`crates/cli/src/notices.rs`, `crates/cli/src/web.rs`), this
    check's own source and tests.
  - Removed lines never count. A renamed file counts only its changed
    lines.
- The check in the `tickets' checks` gate (`crates/xtask/src/gates.rs`)
  and as a step of CI's `tickets` job, on pull requests, with
  `--base HEAD^1` like `pr-size` and `check-merges`; and in the
  `run-gates` skill's list.
- A line in the Scope of each still-open DOOM ticket (2780–2784) and of
  2909, if they are open when this is worked: GPL (or LGPL) source is
  never pasted into a session that edits this repository (the ADR).
- No new dependency: the check is plain string matching in `xtask`.
- Not a public API change: `xtask` is not published.

**Out (do not do):**
- `CONTRIBUTING.md` and how outside contributors' AI-assisted PRs are
  judged: 1132 (parked) writes it, and links this ADR.
- `SECURITY.md`, the code of conduct, the issue forms and the trademark
  policy: 2849.
- Snippet matching against a database of open-source code (SCANOSS,
  ScanCode's matching): it sends code to an outside service or needs a
  large index. The ADR says why it was left out. A ticket only if Nick's
  3529 answer asks for one.
- A lawyer's opinion: Nick's call in 3529, outside the repository.
- The game's own policy: the game repository's business.
- Rewriting history or commit trailers.
- Whether the first release (2005) waits for this: decision 3528 asks
  Nick which audit tickets join the MVP. Don't add it to 2005's
  `blocked_by` here.

**Estimated size:** about 600 lines (code 220, tests 220, docs 160).

## Implementation steps

1. Read Nick's answer to 3529 in `docs/decisions.md`. Write the ADR as
   *Scope* says, quoting the decision and linking ADR-0002 §8, ADR-0021
   §2 and ADR-0092. Add it to `docs/adr/README.md`. If the answer
   changes the copyright line, change `LICENSE-MIT` and every place
   that copies it
   (`git grep -n "Copyright (c) 2026 Nick Wentz"`: today
   `crates/cli/src/web.rs` and NOTICES.txt's snapshot).
2. Add the line to `AGENTS.md` rule 7.
3. Write `crates/xtask/src/license_text.rs`:
   - `pub fn added_lines(repo_root, base) -> Result<Vec<Added>, String>`
     with `Added { path, line, text }`, from `git diff -U0 --no-color
     --no-ext-diff -M <merge-base>` and the untracked files;
   - `pub fn matches(text: &str) -> Option<&'static str>` (the phrase it
     found), and `fn allowed(path, allow: &[Pattern]) -> bool`;
   - `pub fn run(repo_root, base) -> Vec<String>`, the errors.
   Keep the phrases in one `const` table, built so the module's own
   source does not trip a naive scan (it is allowed by
   `tools/license-text-allow.txt` anyway).
4. Wire `check-license-text` into `crates/xtask/src/main.rs` beside
   `check-merges`, with `--base` (default: the merge base with
   `origin/main`, as `pr_size::base_of` does). Add it to the
   `tickets' checks` gate and to CI's `tickets` job.
5. Write `tools/license-text-allow.txt` with the starting entries and a
   comment on each.
6. Run it over the whole tree once (base = the root commit) and confirm
   it reports nothing outside the allow list; any hit is a finding to
   fix or to allow with a reason.
7. Add the GPL line to 2780–2784 and 2909 if they are still open.
8. Update the `run-gates` skill (and run
   `pwsh -File scripts/sync-agent-skills.ps1`).

## Acceptance criteria

- [ ] The ADR exists, is listed in `docs/adr/README.md`, quotes Nick's
      3529 answer, and states the GPL-in-a-session rule.
- [ ] `LICENSE-MIT`'s copyright line matches Nick's answer (changed, or
      the ADR says it stays).
- [ ] `AGENTS.md` rule 7 links the ADR.
- [ ] A branch adding `// GNU General Public License` to a file under
      `crates/` fails `cargo xtask check-license-text` with its path and
      line.
- [ ] A branch adding `SPDX-License-Identifier: MIT` to a `.rs` file
      fails too (a notice goes in a notice file, not a header).
- [ ] The same lines added under `tickets/` or `docs/`, or in a new
      `foo-LICENSE.txt`, pass.
- [ ] A path listed in `tools/license-text-allow.txt` passes; one not
      listed fails.
- [ ] A branch that only removes such a line passes.
- [ ] `Copyright (c) 2031 Someone` at the start of a comment fails;
      the word `copyright` in running prose in a `.rs` doc comment
      ("the copyright line") passes.
- [ ] Run over the whole tree from the root commit, the check reports
      nothing.
- [ ] CI's `tickets` job runs it on pull requests, and the `tickets'
      checks` gate runs it locally.
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not (repository tooling, not something a
      game uses).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/xtask/src/license_text/tests.rs`): `matches` on each
  phrase, on mixed case, after `//`, `#`, `/*` and `*` comment marks,
  and on prose that only mentions copyright or a license name without
  a notice (`MIT OR Apache-2.0` passes; `GPL-3.0` fails); `allowed` on
  each allowed path kind and on a glob from the allow file.
- Integration: a temporary git repository (as `merges.rs`'s tests make
  one) with a base commit and a branch that adds, removes and renames
  files; `run` returns exactly the expected `path:line` errors.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
