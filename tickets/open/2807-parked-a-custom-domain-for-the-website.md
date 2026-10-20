---
id: "2807"
title: "Parked: A custom domain for the website: Nick's domain, its DNS records, a verified domain on GitHub, and HTTPS"
type: infra
milestone: "Engine: public"
model: sonnet-5
effort: low
status: blocked
blocked_by: ["2801"]
nick_input: setup
completed:
---

# 2807 — A custom domain for the website

> **Parked** (Nick, 2026-10-20: "probably hosted on github pages to start and can use custom domain later"): the site starts on GitHub Pages' own address. Nobody starts this until Nick says he has a domain, or asks for one. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

2801 builds the site with `base_url` passed in by the build, so moving
to a domain changes one value. A domain costs money every year, so
buying one is Nick's (`AGENTS.md`, *Who decides what*).

## Nick input

**Setup** (Nick's, when he wants it):

1. Buy the domain at a registrar of his choice (for example Cloudflare
   Registrar, Porkbun or Namecheap). The session checks prices on the
   day if he asks.
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
- `pages.yml` passes the new `base_url`; the repository's Pages
  settings get the domain, and "Enforce HTTPS" once the certificate is
  issued.
- The README, `Cargo.toml`'s `homepage`, and every crate's
  `documentation` link use the domain.
- The old `zafnok.github.io/shuyi-engine/` addresses redirect (GitHub
  does this once a custom domain is set; check it).

**Out (do not do):**
- buying the domain or touching Nick's registrar account;
- email on the domain.

## Implementation steps

1. Read GitHub's custom domain docs on the day, and write Nick's steps.
2. After his setup, change `pages.yml`, the README and the metadata.
3. Check HTTPS and the redirects.

## Acceptance criteria

- [ ] The site loads over HTTPS at the domain, and `www` redirects to it
      (or the other way, as Nick picks).
- [ ] The old Pages address redirects.
- [ ] `zola check` passes with the new `base_url`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- The 2801 base-URL test, with the domain.

## Completion notes

*(Filled in by the session that completes the ticket.)*
