---
id: "3512"
title: Security releases, advisories and yanks follow a written runbook
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3512 — Security releases, advisories and yanks follow a written runbook

## Context

The engine reads files it can't trust: packs, maps, saves, and mods
through Rhai scripts (ADR-0093). So a security fix is a real prospect.
Today nothing says how one is made and released.

What exists, checked on 2026-11-23:

- 2005 (open, MVP) writes `SECURITY.md`: the latest 0.x release only,
  no backports or LTS; reports through the Security tab's "Report a
  vulnerability", never a public issue; best-effort response time. Its
  version policy ADR gives `shuyi` and every `shuyi-*` crate one
  version. Its release workflow (`.github/workflows/release.yml`, not
  written yet) publishes on a tag, after Nick's go.
- 2028 (open) checks each release against crates.io, and its **Out**
  says: "Yanking or fixing a version already published".
- 2016 (parked) holds the backup owner and the response-time question.
- ADR-0037 and the `dependency-upkeep` skill handle advisories in our
  dependencies. Nothing handles an advisory in our own crates.
- `grep -rni yank tickets docs/adr` finds only 2005's sign-off line,
  2016 and 2028. `grep -rni rustsec tickets/open docs/adr` finds only
  ADR-0037 and three `deny.toml` ignores.
- `cargo xtask changelog` (`crates/xtask/src/changelog.rs`, ticket
  2804) has sections for breaking changes, behaviour changes (ADR-0108,
  `behaviour_lines` in `crates/xtask/src/tickets.rs`), Added, Changed
  and Fixed. It has no Security section, which Keep a Changelog has.

Missing, then: when to yank and when not; whether a yank covers one
crate or the whole version; how to fix a reported hole without showing
it first in a public PR; how to publish a GitHub Security Advisory
(GHSA) and ask GitHub for a CVE; how players and the game hear of it;
and how the advisory reaches RustSec, the database `cargo audit` and
`cargo deny` read (our own weekly check, ADR-0037, included).

RustSec takes advisories as a pull request or an issue on
`rustsec/advisory-db`, or by email. AGENTS.md rule 12 and the
[decision of 2026-11-10](../../docs/decisions.md#2026-11-10-no-prs-or-issues-on-other-projects)
forbid a PR or an issue on another project. Whether that covers filing
our own crate's advisory is Nick's question in decision 3528.

Source: the blind-spot audit of 2026-11-23 (proposed ticket 84).

## Nick input

None for this ticket. The runbook it writes has Nick's steps in it (his
go before a security release is tagged, and his clicks to publish the
advisory and ask for the CVE), as 2005's release has.

**Either order with 3528:** if 3528 is done, the RustSec step of the
skill follows Nick's answer in `docs/decisions.md`; if not, the step
says the advisory is published as a GHSA in this repository only, that
nothing goes to `rustsec/advisory-db` by PR or issue (rule 12), and that
3528 settles whether it may go there or by email.

## Scope

**In:**
- An ADR, *Security releases: a private fix, one patch version, a GHSA
  with a CVE, and a yank only of what is unsafe*. It decides:
  - **What counts:** a hole a hostile file, save, mod or network
    message can reach (memory unsafety, a crash or hang on input the
    engine says it handles, a sandbox escape from a script), or a
    published file that should never have shipped. An ordinary bug is a
    normal ticket, not a security release.
  - **The fix in private:** the advisory's temporary private fork, with
    its ticket file written there, so the ticket and the fix go public
    together. Rules 1 and 2 still hold: one ticket, one PR.
  - **The release:** a patch release of every crate at the one version
    (2005's policy), never a backport, since only the latest is
    supported.
  - **The yank:** every crate of each affected version, once the fixed
    version is on crates.io, and only for a security hole or a version
    that can't be used safely. Not for an ordinary bug, and not for a
    break the API gate missed (that gets a patch release and a
    `Breaking:` line). A yank never removes a file, so a leaked secret
    is revoked, not yanked.
  - **The advisory:** a GHSA in this repository, with the affected range
    (`>= X, < fixed`), the fixed version and a workaround if one exists,
    and a CVE asked for through GitHub's advisory form.
  - **Who hears:** the News post (2804's release post, from the
    `Security:` lines below), and a line in the GHSA saying which games
    must rebuild and how.
  - **RustSec:** as 3528 decides (see the clause above).
- A skill, `.claude/skills/security-release/SKILL.md`, the runbook: the
  steps in order, from a private report to a closed advisory, with the
  stop where Nick says go. Its Codex loader from
  `scripts/sync-agent-skills.ps1`.
- A `Security:` line in a ticket's completion notes, read like
  `Behaviour:` (`notes_lines`), which `cargo xtask changelog` lists in a
  *Security* section before *Breaking changes*, in both the Markdown and
  the Keep a Changelog output. `tickets/README.md` and the `work-ticket`
  skill say when to write one.
- **Either order with 2005:** if 2005 is done, add to `SECURITY.md` one
  line on how a fix is released (a patch release of every crate, a GHSA,
  affected versions yanked) linking the ADR, and have the skill name
  `release.yml` as the publish step; if not, 2005 adds that line when it
  writes `SECURITY.md`, and the skill names the release workflow 2005
  writes.
- **Either order with 2846:** if 2846 is done, add a link to this
  ticket's ADR in `docs/cyber-resilience.md`'s *What the engine gives
  you* section (how a security fix is made and released); if not,
  nothing to do (2846 links the ADR). The SBOM and the CRA page are
  2846's.
- No public Rust type changes; ADR-0039 doesn't apply.

**Out (do not do):**
- A publish that stops part way (resume, or the next patch and a yank
  of the half set): 3510's runbook, `docs/releasing.md`. This ticket's yank and
  security-release runbook links it.
- Checking a release against the last version on crates.io: 2028.
- A backup crates.io owner, a successor, and a promised response time:
  2016, parked.
- The community files, the issue forms and the rest of `SECURITY.md`:
  2005, 2849 and 1132 (parked).
- The threat model for untrusted files, and fuzzing: 2020.
- Advisories in our dependencies: ADR-0037 and the `dependency-upkeep`
  skill, unchanged. The runbook says when one becomes ours: only when
  the engine's own published code is exploitable through it.
- Opening a PR or an issue on `rustsec/advisory-db` or any other
  project (rule 12), unless 3528 records that Nick allows it.
- Backports or an LTS line: 2005's policy is latest only.

**Estimated size:** about 900 lines (code 120, tests 220, docs 560).

## Implementation steps

1. Read 2005's version-policy ADR if it exists, ADR-0037, ADR-0108,
   2005's `SECURITY.md` section and the decision of 2026-11-10. Check on
   the day, in GitHub's docs: how a temporary private fork merges,
   whether Actions run there, and how a CVE is asked for. Check
   crates.io's policy on yanks. Cite what you read in the ADR.
2. Write the ADR (`write-adr` skill), with the decisions listed in
   Scope. If 2005's ADR exists, this one amends it on fixes; if not,
   it stands alone, and 2005's ADR cites it.
3. Write `.claude/skills/security-release/SKILL.md`:
   1. A report arrives (the Security tab). Answer it privately. Decide
      with the ADR whether it counts; if not, say so to the reporter and
      write a normal ticket.
   2. Nick opens a draft GHSA and its temporary private fork (exact
      clicks, `https://github.com/Zafnok/shuyi-engine/security/advisories`).
   3. In the fork: `cargo xtask next-id`, the ticket, the fix, a
      regression test, a `Security:` line in its notes. If CI doesn't
      run there, run `cargo xtask gates` locally before the merge.
   4. Nick says go. Merge, raise the patch version, tag; the release
      workflow publishes every crate.
   5. Nick publishes the GHSA and asks for the CVE. Then yank every
      crate of each affected version (`cargo yank --version X.Y.Z
      <crate>`, by the owner), as the ADR allows.
   6. RustSec, as 3528 decides.
   7. Check the News post lists the fix, and tell the reporter.
   8. A dependency's advisory: hand it to `dependency-upkeep` unless the
      engine's own code is exploitable through it.
4. Run `pwsh -File scripts/sync-agent-skills.ps1`, then `-Check`.
5. `crates/xtask/src/tickets.rs`: `security_lines`, over `notes_lines`
   with `"Security:"`, beside `behaviour_lines`.
6. `crates/xtask/src/changelog.rs`: read the lines into each release and
   write a *Security* section first in `notes` (Markdown and Keep a
   Changelog).
7. `tickets/README.md`, after the `Behaviour:` text, and the
   `work-ticket` skill's notes step: when a `Security:` line is written
   and its shape (`Security: <crate>[, <crate>…]: <what was fixed, and
   the GHSA id>`).
8. If 2005 is done, add the `SECURITY.md` line (Scope). If 2846 is
   done, link the ADR from `docs/cyber-resilience.md` (Scope). Link
   3510's runbook for a half-finished publish (`docs/releasing.md`) from the skill's yank
   step.

## Acceptance criteria

- [ ] An ADR in `docs/adr/` decides what counts, the private fix, one
      patch version of every crate, when to yank and when not, the GHSA
      and CVE, who hears, and the RustSec route; the ADR index lists it.
- [ ] `.claude/skills/security-release/SKILL.md` walks from a report to
      a closed advisory, names each of Nick's clicks with its URL, and
      `scripts/sync-agent-skills.ps1 -Check` passes.
- [ ] No step of the skill opens a PR or an issue on another project,
      unless `docs/decisions.md` records that Nick allows it (3528).
- [ ] A done ticket with `Security: shuyi-assets: …` gives a
      *Security* section with that line in `cargo xtask changelog`,
      before *Breaking changes*, in both formats; a release with none
      has no such section.
- [ ] A `Security:` line outside the completion notes is not read.
- [ ] `tickets/README.md` and the `work-ticket` skill describe the line.
- [ ] If 2005 is done, `SECURITY.md` links the ADR. If 2846 is done,
      `docs/cyber-resilience.md` links it.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `security_lines` reads the completion notes only, with and
  without a `- ` marker and a CR before the LF, and not a `Breaking:`
  or `Behaviour:` line.
- Unit: `changelog` over a fake history: a release with a `Security:`
  line has the section first; one without has none; the Keep a
  Changelog output names it `### Security`.
- Integration: `scripts/sync-agent-skills.ps1 -Check` and the docs
  map's check (`cargo xtask check-docs`) pass with the new skill.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
