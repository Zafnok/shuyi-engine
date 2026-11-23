---
id: "3529"
title: "Nick decides the project's legal stance: the EU Cyber Resilience Act, agent-written code, the starter code's license, the community files now, a trademark and the DCO"
type: decision
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 3529 — Nick decides the project's legal stance

## Context

A blind-spot audit of the engine (2026-11-23) found the legal basics
strong:
- the dual license, MIT OR Apache-2.0;
- `cargo deny` and a `NOTICES.txt` in every package;
- the game's work kept out of the engine;
- console SDKs kept private.

What it found missing are newer duties and some tidying for a public
repository. Six of them are Nick's to decide, because each is about
money, a legal position or what the project promises. None of them is
urgent today. Each one, once answered, lets a ticket start.

1. **The EU Cyber Resilience Act (CRA, Regulation (EU) 2024/2847).**
   Its duty to report actively exploited holes has applied since
   2026-09-11. Its full requirements apply from 2027-12-11: a list of
   the parts inside a product (an SBOM), handling of security holes,
   and updates for a support period. The regulation's recitals leave
   open-source software made available without a commercial activity
   out of its scope. A paid game sold in the EU, Visions of Shuyi first,
   is very likely a "product with digital elements". Its maker answers
   for the parts inside it, the engine included. The regulation also
   defines an "open-source software steward", with lighter duties, for
   an organisation that steadily supports open-source software meant
   for commercial use.
   - A search for `Cyber Resilience`, `SBOM`, `CycloneDX` and
     `cargo-auditable` over `tickets/`, `docs/` and `.github/` finds
     only 2011 (done), which lists SBOMs under *Out*.
   - 2846 plans an SBOM from `shuyi export`, and a docs page on what the
     engine gives a game for the CRA. That page needs to say what the
     engine itself is. Sponsors (2806) don't change this on their own:
     the recitals say donations made without the aim of a profit are not
     a commercial activity.
2. **Agent-written code.** Almost all of this repository's code is
   written by coding agents, and Nick doesn't review code (`AGENTS.md`).
   - The US Copyright Office's report on copyrightability (Part 2,
     January 2025) and *Thaler v. Perlmutter* (D.C. Cir., March 2025)
     hold that what an AI makes without enough human creative control
     isn't protected by copyright.
   - The license still works for everyone who uses the engine. But
     `LICENSE-MIT`'s one line, `Copyright (c) 2026 Nick Wentz`, may be
     thin for code no person wrote.
   - The human part is real and written down: Nick's decisions, and the
     choice and order of the work in tickets, ADRs and
     `docs/decisions.md`. Nothing says so yet.
   - 2847 writes the policy and a cheap check for licence text pasted
     into a PR. It waits for this answer.
3. **The starter code `shuyi new` writes.**
   - `templates/shuyi-default/template.ron` line 9 says
     `license: "MIT OR Apache-2.0"`, and so does the skill pack
     (`.claude-plugin/marketplace.json` line 13). Nothing reads either
     into the game.
   - A new game's repository is public by default (decision of
     2026-11-12). It gets the engine's starter code with no `LICENSE`
     file, so the game technically owes the MIT notice for its own
     scaffold.
   - Templates elsewhere avoid this with a license that asks for
     nothing: MIT-0 or CC0-1.0.
   - 2848 makes `shuyi new` ask for the game's license, and licenses the
     scaffold as Nick decides here.
4. **The community files.** The repository has been public since
   2026-10-14.
   - It has no `SECURITY.md`, no code of conduct and no issue forms
     (`ls` and GitHub's community profile, 57% healthy, on 2026-11-23).
     Private vulnerability reporting is on.
   - Nick parked the community files with 1132 on 2026-11-01. He kept a
     short `SECURITY.md` and the required checks in the first release,
     2005, which waits on many open tickets.
   - 2849 would put up `SECURITY.md`, a code of conduct and issue forms
     now, without unparking 1132's policy questions (who may send code,
     which channel).
5. **A trademark.** On 2026-10-14 Nick chose the name and no
   registration: "I won't copyright it so she can still use her name"
   (`docs/decisions.md`, *The name*). That worry was never answered.
   - A registered mark on "Shuyi Engine" covers that name for software.
     **It does not stop anyone, Nick's wife included, from using Shuyi
     as their own name**, or anyone from saying "made with Shuyi
     Engine".
   - Without a mark, the README's paragraph *The name* is the only
     rule. It asks forks to pick their own name.
   - Outside modules are planned under `shuyi-*` names (2526, 2552). A
     short written policy on the name keeps "official" clear, and costs
     nothing.
   - Registration costs money: filing fees in each country, and a
     lawyer's time.
   - The game's own check, game ticket 0906 ("title trademark check
     before Steam page"), is open. Its lawyer could look at both names
     together.
6. **Who may send code, and a promise about the license.**
   `docs/decisions.md`, *Still open*, still lists who may contribute.
   The recommendation there is pull requests under a Developer
   Certificate of Origin (DCO) once outside pull requests open.
   - Outside pull requests stay closed at the first release (decision of
     2026-11-01), so this isn't urgent.
   - The README already says contributions come in under the same dual
     license. Under it, Nick may use them in the paid game and in
     private console backends, so no Contributor License Agreement (CLA)
     is needed for that.
   - A CLA matters only for relicensing the engine later. Godot, Bevy
     and Rust take a DCO or nothing, with no CLA.
   - 1132 (parked) asks who may send code. A recorded answer means 1132
     later only turns things on.

Two related questions are asked elsewhere: whether `main` requires CI's
checks (2060), and a GitHub account successor (3528).

## Nick input

**Decision:** six questions, asked with the `ask-nick` skill, one at a
time, in plain words. None of this is legal advice. Each option says
what it costs and what it lets start.

1. **The EU's new security law (the CRA).** How should the engine
   describe itself?
   - **A. A non-commercial open-source project.** The engine has no
     duties of its own under the law. Its docs say so, and still give a
     game's maker what they need (the parts list from 2846). Lets 2846's
     docs page say it. Recommended.
   - **B. A "steward" once sponsors or a foundation exist.** Light
     duties: a written security policy, and help to the authorities if
     asked. The same docs, plus a line that this may change.
   - **C. Ask a lawyer first.** Costs money. 2846 writes the parts list
     and leaves the engine's position out until then.

   The answer fills the section *The engine's own position* of 2846's
   page, `docs/cyber-resilience.md`.

   And: should Visions of Shuyi get its own ticket, in the game, for
   its duties as a paid game sold in the EU? (Yes: a game ticket. No:
   nothing.)
2. **Code written by AI agents.** Do you want a written policy on it?
   - **A. Yes, a short policy.** It says that your decisions and plans
     are the human part, that agent code is accepted when the checks
     pass, and that a check catches pasted licence text. The copyright
     line stays `Nick Wentz`. Lets 2847 start.
   - **B. Yes, and the copyright line becomes "Nick Wentz and the Shuyi
     Engine contributors".** The same, with a line that also fits
     outside contributors later.
   - **C. A or B, plus a one-off lawyer's opinion before the first
     release.** Costs money. 2847 starts; the opinion is yours to
     arrange.
   - **D. No policy.** 2847 builds only the licence-text check.

   Unblocks 2847, which writes the answer into an ADR on agent-written
   code's provenance, sets `LICENSE-MIT`'s copyright line as decided,
   and adds a CI check for licence text in PR diffs.
3. **The starter code a new game gets.** Should it be licensed so a
   game owes nothing for it?
   - **A. Yes: "MIT-0 OR MIT OR Apache-2.0".** MIT-0 asks for no
     notice; the other two stay for whoever prefers them. Applies to the
     templates and to the skills copied into a game. The engine's own
     crates keep their notices, which `NOTICES.txt` already ships. Lets
     2848 start. Recommended.
   - **B. Yes, as CC0-1.0** (public domain, where the law allows).
   - **C. No.** It stays MIT OR Apache-2.0, and `shuyi new` writes the
     notice into each new game.

   Any answer unblocks 2848: `shuyi new` writes the chosen license into
   `template.ron`, the shipped skills and `marketplace.json`, and names
   it in each new game's README.
4. **The community files.** May a short security policy, a code of
   conduct (Contributor Covenant 2.1) and bug-report forms go up now,
   before the first release?
   - **A. All three now.** Takes those parts out of parked 1132,
     without unparking its questions on who may send code. Lets 2849
     start.
   - **B. Only the security policy now.** The rest stays with 1132.
   - **C. No.** As decided on 2026-11-01: `SECURITY.md` comes with the
     first release (2005), the rest with 1132.

   With this question, also ask:
   - If the code of conduct goes up now: who receives reports under
     it? The contact may be printed in `CODE_OF_CONDUCT.md`, so it must
     be one Nick gives for this.
   - May the repository get the optional topics `rust`, `game-engine`,
     `gamedev` and `2d` (one click of his, on the main page)? Today it
     has none.

   2849 reads both answers.
5. **The name as a trademark.** You chose no registration on
   2026-10-14 so your wife could still use her name. A mark on "Shuyi
   Engine" covers that name for software only. It doesn't stop her, or
   anyone, from using Shuyi as their own name. Should registration be
   looked at again?
   - **A. No registration, but a short written policy on the name.**
     What "made with Shuyi Engine" and `shuyi-*` module names may imply,
     and that outside modules say "unofficial". Free. 2849 writes it.
     Recommended.
   - **B. Register "Shuyi Engine".** Costs fees in each country and a
     lawyer's time. Yours to arrange; the written policy as in A as
     well.
   - **C. Ask the game's lawyer about both names together** (game
     ticket 0906), then decide.
   - **D. As today:** the README's paragraph only.
6. **Outside code, once pull requests open.** Contributors sign off
   each commit (the DCO), with no agreement to sign, and the project
   promises never to relicense away from MIT OR Apache-2.0. Yes or no?
   - **A. Yes, both.** The same as Bevy and Rust. Recorded now; 1132
     turns it on when it is unparked.
   - **B. The DCO, without the promise.** You keep the option to change
     the license of your own code later.
   - **C. A contributor agreement (CLA).** Lets you relicense
     contributions too. Puts off some contributors, and needs a service
     to collect signatures.
   - **D. Not now.** Decide when 1132 is unparked.

## Scope

**In:**
- Ask the six questions and record each answer, in Nick's words, in
  `docs/decisions.md`, with the rules that follow from it.
- Question 6's answer takes "who may contribute code" out of
  `docs/decisions.md`'s *Still open* list, unless Nick answers D.
- For each answer that needs work, the ticket it lets start: 2846
  (question 1), 2847 (2), 2848 (3), 2849 (4 and 5), and a game ticket if
  Nick says yes to question 1's second part (written in the game with
  the game's `write-ticket`, linked from the decision).
- Each parked or open ticket an answer makes wrong is updated in the
  same PR. For example, if question 4 is A, 1132's Scope loses the code
  of conduct and the issue forms, and 2005's `SECURITY.md` step becomes
  a change to the supported-versions line.
- **Either order with 1132:** if 1132 is done, its answer on who may
  send code stands, and question 6 asks only about the relicensing
  promise; if not, ask question 6 whole, and 1132 reads the answer from
  `docs/decisions.md` instead of asking it again.
- **Either order with 2846:** if 2846 is done, the answer becomes one
  paragraph in `docs/cyber-resilience.md`'s section *The engine's own
  position*, which until then says no position has been taken; if not,
  nothing to do (2846 writes that section from the answer in
  `docs/decisions.md`).

**Out (do not do):**
- Any setting, filing or payment: registering a mark, a lawyer, a CLA
  service. Nick does these himself.
- Writing the files or the code: 2846, 2847, 2848 and 2849 do.
- Whether `main` requires CI's checks and refuses a push that isn't a
  PR: 2060 asks it. That includes `launch (windows)` and
  `launch (macos)` (3336), which land as checks that are not required.
- A GitHub account successor and a second crates.io owner: 3528 asks
  them, and 2016 (parked) holds the rest of governance.
- Which community channel opens: 1132 asks it.
- Legal advice. The questions say what the law and the cases say, and
  what each choice costs; a lawyer is one of the options.

## Implementation steps

1. Read this ticket's Context, `docs/decisions.md` (*The name* and
   *Still open* of 2026-10-14, and 2026-11-01), and the tickets each
   question lets start, so the options still match them.
2. Ask with `ask-nick`, one question at a time, in the words above.
   Correct a fact that has changed since 2026-11-23 before asking.
3. Record each answer verbatim in `docs/decisions.md`, with its rules.
4. Update the tickets the answers change: set the Scope of 2846–2849,
   1132 and 2005 to what Nick chose. A "no" turns the ticket
   it would have started into its smaller form (as each option says),
   or moves it to `tickets/done/` as not wanted, with a line saying so.
5. If question 1's second part is yes, write the game's ticket, and
   link it here and from the decision.

## Acceptance criteria

- [ ] `docs/decisions.md` has the six answers in Nick's words, and the
      rules each leads to.
- [ ] Each answer that needs work names its ticket, and that ticket's
      Scope matches the answer.
- [ ] *Still open* no longer lists who may contribute, unless question 6
      was answered D.
- [ ] 1132 and 2005 say the same as the answers to questions 4 and 6.
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None (a decision).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
