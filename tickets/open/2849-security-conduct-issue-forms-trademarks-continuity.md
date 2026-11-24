---
id: "2849"
title: "The public repository has SECURITY.md, a code of conduct, issue forms, a trademark policy and a continuity plan"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["3528", "3529"]
nick_input: sign-off
completed:
---

# 2849 — SECURITY.md, a code of conduct, issue forms, a trademark policy and a continuity plan

## Context

The repository is public, and outside people can already open issues.
A blind-spot audit (2026-11-23) found that the files an adopter looks
for first are missing. The facts were checked that day with `gh api`:

- GitHub's community profile
  (`gh api repos/Zafnok/shuyi-engine/community/profile`) is at **57%**.
  It has a README, `CONTRIBUTING.md` and the licenses. It has no code of
  conduct, no issue template and no pull request template.
  `.github/ISSUE_TEMPLATE/` doesn't exist.
- There is no `SECURITY.md` at the root. Private vulnerability
  reporting is **already on**
  (`gh api repos/Zafnok/shuyi-engine/private-vulnerability-reporting`
  gives `{"enabled":true}`). 2005's *Nick input* still asks Nick to turn
  it on, so that click is done.
- The ruleset "protect main" (id 25508665) is active. Its rules are
  `deletion`, `non_fast_forward` and `pull_request`. It has no
  `required_status_checks`, and the owner's account is on its bypass
  list (mode `pull_request`). So `ci-result` and `mutants (diff)` bind
  nothing yet.
- GitHub Discussions is on (`has_discussions: true`). The repository
  has no topics (`topics: []`).
- There is no `TRADEMARKS.md`. The README's *The name* section asks a
  fork to take its own name and welcomes "made with Shuyi Engine"
  (`README.md`, line 280 on). `docs/decisions.md`, 2026-10-14, says
  the name is not registered. Nothing says what a third-party
  `shuyi-*` crate or module may imply. Outside modules are planned
  (2526, 2552), so this matters soon.
- **The bus factor is one.** No GitHub account successor is set, and
  no file lists the accounts and secrets the engine runs on. Today they
  are: the `Zafnok` account; the ruleset; `SONAR_TOKEN` (an Actions
  secret, 2007); `DEPENDENTS_DISPATCH_TOKEN`, a secret of the
  `dependents` environment (2031, ADR-0045; parked 2016's list misses
  it); the `github-pages` environment; the SonarCloud organization; and
  the nine placeholder crates on crates.io. The decision of 2026-11-01
  says "No backup is named".

What is already planned, and what this ticket takes:

- **2005** (open, the first release) writes a short `SECURITY.md` and
  holds Nick's two clicks for `main`'s required checks. Nick kept those
  parts there on 2026-11-01 (`docs/decisions.md`, *governance and the
  community files are parked*). This ticket writes `SECURITY.md` before
  the release. 2005 then only changes its *Supported versions* line.
- **1132** (parked: community files) plans `CODE_OF_CONDUCT.md` and the
  issue and PR templates, beside `CONTRIBUTING.md`'s contribution
  policy and a community channel. This ticket takes only the parts that
  set no policy: the code of conduct and the templates. The contribution
  policy, the DCO and the channel stay in 1132.
- **2016** (parked: governance) plans `GOVERNANCE.md`, a backup person,
  `docs/continuity.md` and `cargo xtask repo-settings`. This ticket
  takes only the two parts that cost nothing: the account successor
  (Nick's click, 2016's option B) and the plain list of accounts in
  `docs/continuity.md`. The rest stays in 2016.

Hard rule 3 keeps parked work parked. So this ticket waits for Nick:

- **3529:** whether the security policy, code of conduct and issue
  forms go up now is 3529's question 4, and whether a trademark/name
  policy is written (and registration revisited) is its question 5;
  this ticket writes what Nick chose there.
- **3528** asks whether he names an account successor.

Each part below is done only if his answer allows it. A part he refuses
is left out and stays in the ticket that held it.

## Nick input

**Sign-off:** before the PR merges, Nick reads `SECURITY.md`,
`CODE_OF_CONDUCT.md`, `TRADEMARKS.md` and `docs/continuity.md`. He
comments on anything that speaks for him. These are public promises in
his name. The session lists the four files and a one-line summary of
each in the PR body, and asks him in plain words (`ask-nick`).

**Setup**, Nick's own clicks. An agent never changes the repository's
settings or anyone's account. The session writes each step with its
exact page and checks the result read-only with `gh api`:

1. **Only if Nick's 3528 answer names a successor:** at
   `https://github.com/settings/admin`, under "Successor settings", add
   the person's GitHub login. The person accepts the invitation.
2. **Only if Nick's 3529 answer says the checks bind now:** the two
   clicks for the ruleset "protect main", copied from 2005's *Nick
   input* (require `ci-result` and `mutants (diff)` from GitHub
   Actions, require branches to be up to date, and empty the bypass
   list). Check them with
   `gh api repos/Zafnok/shuyi-engine/rulesets/25508665`.
3. **Optional:** on the repository's main page, the gear beside
   "About", add the topics `rust`, `game-engine`, `gamedev` and `2d`.
   They help people find it. Skip this if Nick says no.

## Scope

**In:**
- **`SECURITY.md`** at the root, short, as 2005 specifies it, from
  Nick's rules of 2026-11-01:
  - *Supported versions:* before the first release, `main` only; after
    it, the latest 0.x release only, with no backports or LTS;
  - *How to report:* the Security tab's "Report a vulnerability"
    (private vulnerability reporting, on since before this ticket),
    never a public issue;
  - *Response time:* best effort, with no number;
  - *What counts:* for a file the engine reads, a panic, unbounded
    memory or time, or two different results from the same bytes.
  - **Either order with 2005:** if 2005 is done, `SECURITY.md` exists;
    check it against the list above and add only what is missing; if
    not, write it as above, and 2005 changes its *Supported versions*
    line to "the latest 0.x release" at the release.
  - **Either order with 2020:** if 2020 is done, link
    `docs/formats.md`, *Untrusted files*, from *What counts*; if not,
    2020 adds the link when it lands.
- **`CODE_OF_CONDUCT.md`**: the Contributor Covenant, the newest
  version on its site on the day (2.1 or later). It is licensed
  CC BY 4.0, so keep its attribution section. Its enforcement contact
  is the one Nick names in his 3529 answer. Never invent an address,
  and never write Nick's email unless he gave it for this. If 3529 named
  none, ask him (`ask-nick`) before writing the file.
- **Issue forms** in `.github/ISSUE_TEMPLATE/` (GitHub's YAML issue
  forms):
  - `bug.yml`: the engine's version or git revision; the OS and the
    target (desktop, web, Playdate); the output of
    `shuyi doctor --json`; and a way to reproduce it: a replay record,
    a `shuyi run --script` line, or the steps. The first three are
    required. Label `bug`.
  - `feature.yml`: what the game needs, and an example from a game.
    Label `enhancement`.
  - `extension-point.yml`: for a module author whose module can't hook
    in where it needs to. It asks which hook or trait is missing, what
    the module does, and the workaround used today.
  - `config.yml`: blank issues off. `contact_links` points questions to
    Discussions (on today, `has_discussions: true`; leave the link out
    if it is off on the day) and security reports to the Security tab.
- **`.github/pull_request_template.md`**, a few lines: the ticket id,
  what changed, and that the gates passed. It repeats
  `CONTRIBUTING.md`'s line that outside pull requests are not accepted
  yet, and links it.
- **`TRADEMARKS.md`**, short, after the Rust Foundation's name policy,
  with what Nick's 3529 answer allows. It says:
  - the name isn't registered, and the file is a request, not a legal
    claim (`docs/decisions.md`, 2026-10-14);
  - "made with Shuyi Engine" and "for Shuyi Engine" are welcome;
  - a fork, a product, a course or a tutorial may not present itself as
    official or endorsed;
  - there is no logo yet, so there are no logo terms (no `*logo*` file
    exists under `site/`);
  - the rule for crate and module names.
  - **Either order with 2526:** if 2526 is done, state its naming rule
    from `docs/decisions.md` and link the README's "Naming your crate"
    paragraph; if not, write that the rule for `shuyi-*` names by other
    people is still open, and 2526 adds the rule here when it records
    Nick's answer.
  - The README's *The name* section gets one sentence linking
    `TRADEMARKS.md`.
- **`docs/continuity.md`**, public, a first version of 2016's list:
  - one table, with a row per account or secret. Columns: what it
    controls; where it is kept (a settings page, never a value); who
    holds it; the second holder, or "none"; how to rotate or recover
    it; and the ticket that made it;
  - a row for each account that exists on the day. Read them with `gh
    api` (Actions secrets, environments and their secrets, the
    rulesets) and with the file `sonar-project.properties`. The rows
    are at least those in *Context*. If 2806, 2525 or another ticket
    has added an account by then, it gets a row too;
  - a section, *If the owner can't act*: the successor if Nick named
    one (his login only if he said so; otherwise "a successor the
    owner has named"), or "none yet". What a successor can do is
    GitHub's rule: archive or transfer the public repositories once
    GitHub confirms the owner's death, and nothing while he is alive;
  - never a secret's value, an email address, a phone number, or where
    recovery codes are kept.
  - **Either order with 2016:** if 2016 is done, `docs/continuity.md`
    exists; add only the rows it lacks and the successor's line; if
    not, write it as above, and 2016 adds its *The repository's
    settings* section and `cargo xtask repo-settings`'s output when it
    is unparked.
- **Either order with 1132:** if 1132 is done, its code of conduct and
  templates exist; check them against this Scope and add only what is
  missing (the `extension-point.yml` form, the `shuyi doctor --json`
  field); if not, write them as above, and 1132 keeps them when it adds
  the contribution policy.
- **The website's Community page** (`site/content/community.md`) links
  `SECURITY.md`, `CODE_OF_CONDUCT.md` and `TRADEMARKS.md`.
- **The docs map** (`docs/README.md`): a row for each new file (the
  `document` skill).
- No public API changes. Only `xtask` gets code, and it isn't published.

**Out (do not do):**
- The contribution policy, the DCO or CLA, and the community channel:
  1132 (parked) and Nick's 3529 answer on DCO.
- `GOVERNANCE.md`, a backup collaborator, a second crates.io owner,
  Dependabot alerts, and `cargo xtask repo-settings`: 2016 (parked).
- Registering a trademark: Nick's, outside the repository, and it costs
  money.
- How a security fix is released, advised and yanked, and its line in
  `SECURITY.md`: 3512.
- A policy on agent-written code and a license scan of diffs: 2847.
- 2809's `assets-entry.yml` form. It goes in the same folder, and this
  ticket's `config.yml` leaves it in place.
- The README's restructure as a front door: 2843.
- Any change to the repository's settings by an agent.

## Implementation steps

1. Read Nick's answers to 3528 and 3529 in `docs/decisions.md`. List
   which parts he allowed, and drop the others from this PR.
2. Re-read the facts in *Context* with `gh api` (the community profile,
   private vulnerability reporting, the ruleset, the secrets and
   environments). Correct anything that changed.
3. Write `SECURITY.md` (the two either-order clauses with 2005 and
   2020).
4. Write `CODE_OF_CONDUCT.md` from the Contributor Covenant's text, with
   Nick's contact.
5. Write the three issue forms, `config.yml` and the PR template.
6. Write `TRADEMARKS.md` (the clause with 2526), and add the README's
   sentence.
7. Write `docs/continuity.md` from what step 2 read (the clause with
   2016).
8. Add the links to `site/content/community.md` and the rows to
   `docs/README.md`.
9. Add a test in `crates/xtask` that each file in
   `.github/ISSUE_TEMPLATE/` parses as YAML with `serde_norway` (already
   an `xtask` dependency). The test checks that each form has `name`,
   `description` and `body`; that each `body` item has a known `type`
   and a unique `id`; and that `bug.yml` marks the version, target and
   `shuyi doctor --json` fields `required: true`.
10. Hand Nick the setup steps and the sign-off (`ask-nick`). After he
    says "done", check the result with `gh api` and write it in the
    completion notes.

## Acceptance criteria

- [ ] `docs/decisions.md` holds Nick's 3528 and 3529 answers, and this
      PR contains only the parts they allow.
- [ ] `SECURITY.md` names the supported versions, the private channel,
      best effort with no number, and what counts. It links *Untrusted
      files* if 2020 is done.
- [ ] `CODE_OF_CONDUCT.md` is the Contributor Covenant with its
      attribution and the contact Nick named.
- [ ] Opening a new issue on GitHub offers the bug, feature and
      extension-point forms, and no blank issue.
- [ ] `gh api repos/Zafnok/shuyi-engine/community/profile` shows
      `health_percentage` 100 after the merge, or the completion notes
      say which part Nick refused.
- [ ] `TRADEMARKS.md` exists, and the README's *The name* links it.
- [ ] `docs/continuity.md` has a row for every Actions secret,
      environment secret and ruleset that `gh api` lists, and no secret
      value, email or phone number.
- [ ] If Nick named a successor, `docs/continuity.md` says one is set,
      and his click is recorded in the completion notes.
- [ ] If Nick chose to bind the checks now, the ruleset lists
      `ci-result` and `mutants (diff)` as required and has an empty
      bypass list.
- [ ] The website's Community page links the three new policy files.
- [ ] The book's chapter is updated (2802), or the completion notes
      have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/xtask`): the issue-form test of step 9. A form with a
  missing `name`, a repeated `id` or an unknown `type` fails it (one
  case each, from a string in the test).
- The link check (`cargo xtask check-links`) and the docs-map check
  over the new files.
- Typos over the new files.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
