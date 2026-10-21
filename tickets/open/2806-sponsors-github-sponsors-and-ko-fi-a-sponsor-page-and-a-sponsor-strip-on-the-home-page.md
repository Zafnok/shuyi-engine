---
id: "2806"
title: "Sponsors: GitHub Sponsors and Ko-fi, the repository's Sponsor button, a sponsor page, and a sponsor strip on the home page"
type: infra
milestone: "Engine: public"
model: sonnet-5
effort: low
status: todo
blocked_by: ["2801"]
nick_input: setup
completed:
---

# 2806 — Sponsors

## Context

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md)): "some sponsor
segment", through **GitHub Sponsors and Ko-fi**. Bevy has a Donate page
and sponsor logos on its home page, and its foundation takes the money.
Here the money goes to Nick: his accounts, his bank and tax details,
his words about what it pays for.

## Nick input

**Setup**, before the code (these are Nick's alone; nobody does them for
him):

1. **GitHub Sponsors:** sign in as `Zafnok`, open
   `https://github.com/sponsors/accounts`, pick "Get started" (or
   "Join the waitlist") for your personal account, and:
   - fill in the profile: a short bio and what sponsorship pays for;
   - add tiers (for example $5, $15 and $50 a month, and one-time
     amounts);
   - connect the payout account (Stripe Connect) and fill in the tax
     form GitHub asks for;
   - submit the profile, and wait for GitHub to approve it.
2. **Ko-fi:** sign up at `https://ko-fi.com`, choose the page name (for
   example `shuyiengine`), and connect PayPal or Stripe for payouts.
3. Tell the session your Ko-fi page name, the text for "what the money
   pays for", and which sponsors may be listed by name (GitHub lets each
   sponsor choose to be public).

**Sign-off:** Nick reads the Sponsor page before it goes live.

The session checks both platforms' fees and terms on the day it starts
and tells Nick in one line each. Neither was checked on 2026-10-20.

## Scope

**In:**
- **`.github/FUNDING.yml`:**
  ```yaml
  github: [Zafnok]
  ko_fi: <Nick's page name>
  ```
  which puts the "Sponsor" button on the repository.
- **The Sponsor page** (`site/content/sponsor.md`, replacing 2801's
  placeholder):
  - Nick's text on what the money pays for;
  - a button to each platform;
  - the sponsors, from `site/data/sponsors.toml` (name, optional link,
    optional logo in `site/static/sponsors/`, tier), hand-kept, listing
    only people who said yes;
  - a line that sponsoring buys no say over the roadmap (decisions stay
    Nick's, `AGENTS.md`, *Who decides what*) unless Nick says otherwise.
- **The home page's sponsor strip:** the sponsors' names or logos, and a
  "Sponsor" button. It hides the list when `sponsors.toml` is empty.
- **The README:** a line and a link to the Sponsor page.
- **`docs/continuity.md`** (2016): rows for GitHub Sponsors and Ko-fi:
  what each controls, that Nick holds it, where its settings are (a
  settings page, never a value), and this ticket. No payout, bank or tax
  details. **Either order with 2016:** if 2016 is done, add the rows; if
  not, 2016 adds them.
- A sponsor's logo is their own mark, shown with their permission. It
  isn't the engine's asset, so it goes in `THIRD_PARTY_ASSETS.md` as
  "shown with permission", not under a license.

**Out (do not do):**
- creating, signing in to or setting up either account;
- a sponsor list fetched from GitHub's API (needs a token Nick makes;
  a follow-up if the list grows);
- Open Collective, a foundation, or anything with a fiscal host (Nick
  chose against it on 2026-10-20);
- perks that promise work.

## Implementation steps

1. Wait for Nick's setup and his text.
2. Check both platforms' fees and terms; tell Nick.
3. Write `FUNDING.yml`, the page, the data file, the strip and the
   README line, and the two rows in `docs/continuity.md` if 2016 is
   done.
4. Run the gates, and ask for the sign-off.

## Acceptance criteria

- [ ] The repository shows the Sponsor button, linking both platforms.
- [ ] The Sponsor page is live with Nick's text and both buttons.
- [ ] With an empty `sponsors.toml` the home page shows the button and no
      empty list; with one entry it shows the entry.
- [ ] If 2016 is done, `docs/continuity.md` has a row each for GitHub
      Sponsors and Ko-fi, with no payout, bank or tax detail.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `zola check`; a CI build with an empty and a one-entry `sponsors.toml`
  (a fixture), grepping the output.

## Completion notes

*(Filled in by the session that completes the ticket.)*
