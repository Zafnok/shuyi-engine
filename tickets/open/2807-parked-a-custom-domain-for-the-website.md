---
id: "2807"
title: "Parked: A custom domain for the website: Nick's domain, its DNS records, a verified domain on GitHub, and HTTPS"
type: infra
milestone: "Engine: public"
model: sonnet-5
effort: low
status: blocked
blocked_by: ["2801", "2016", "2808"]
nick_input: setup
completed:
---

# 2807 — A custom domain for the website

> **Parked** (Nick, 2026-10-20: "probably hosted on github pages to start and can use custom domain later"): the site starts on GitHub Pages' own address. Nobody starts this until Nick says he has a domain, or asks for one. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

2801 builds the site with `base_url` passed in by the build, so the
site itself moves by changing one value. A few places outside the site
name its address too: the README, the crates' metadata, and 2808's
`ENGINE_SITE`, which new games' `AGENTS.md` links. 2016's
`docs/continuity.md` lists the accounts the engine runs on, with a
"none yet, 2807" row for the domain. A domain costs money every year,
so buying one is Nick's (`AGENTS.md`, *Who decides what*).

## Nick input

**Setup** (Nick's, when he wants it):

1. Buy the domain at a registrar of his choice (for example Cloudflare
   Registrar, Porkbun or Namecheap), with auto-renew on, and tell the
   session the registrar and the month it renews. The session checks
   prices on the day if he asks.
2. Verify it on GitHub, so nobody else can point a Pages site at it:
   GitHub → Settings → Pages → "Add a domain", add the TXT record it
   shows at the registrar, then "Verify".
3. Add the DNS records the session gives him (step 1 below) at the
   registrar.

## Scope

**In:**
- The DNS records, from GitHub's Pages docs on the day: for the bare
  domain, `A` and `AAAA` records to GitHub Pages' addresses; for `www`,
  a `CNAME` to `zafnok.github.io`. Written out for Nick in the order he
  types them.
- `pages.yml` passes the new `base_url`: its one `BASE_URL` feeds both
  `zola build` and `cargo xtask llms` (2808), and `ci.yml`'s site check
  passes the same URL. The repository's Pages settings get the domain,
  and "Enforce HTTPS" once the certificate is issued.
- The README, `Cargo.toml`'s `homepage`, every crate's `documentation`
  link, and `ENGINE_SITE` in `crates/cli/src/contract/new.rs` (2808,
  linked from new games' `AGENTS.md`) use the domain.
- A row in `docs/continuity.md` (2016) for the domain, in place of its
  "none yet, 2807": the registrar's name, that auto-renew is on, the
  renewal month, and that Nick holds it; no account details.
- The old `zafnok.github.io/shuyi-engine/` addresses redirect (GitHub
  does this once a custom domain is set; check it).

**Out (do not do):**
- buying the domain or touching Nick's registrar account;
- email on the domain.

## Implementation steps

1. Read GitHub's custom domain docs on the day, and write Nick's steps.
2. After his setup, change `pages.yml`, `ci.yml`, the README, the
   metadata and `ENGINE_SITE`, and write the `docs/continuity.md` row.
3. Check HTTPS and the redirects.

## Acceptance criteria

- [ ] The site loads over HTTPS at the domain, and `www` redirects to it
      (or the other way, as Nick picks).
- [ ] The old Pages address redirects.
- [ ] `zola check` passes with the new `base_url`.
- [ ] `llms.txt` at the domain links pages at the domain, and a game
      made by `shuyi new` on the engine's `main` links it there in its
      `AGENTS.md`.
- [ ] `docs/continuity.md`'s domain row names the registrar, auto-renew
      and the renewal month, and no account details.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- The 2801 base-URL test, with the domain.
- 2808's `new.rs` unit tests of `{{engine.docs}}`, with the domain.

## Completion notes

*(Filled in by the session that completes the ticket.)*
