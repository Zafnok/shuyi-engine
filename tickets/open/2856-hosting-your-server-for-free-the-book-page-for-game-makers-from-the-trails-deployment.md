---
id: "2856"
title: "Hosting your server for free: the book page for game makers, from the trails deployment and its measured costs"
type: feature
milestone: "Engine: networking"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2618", "2791", "2854"]
nick_input: none
completed:
---

# 2856 — Hosting your server for free: the book page

Unparked by Nick's decision of 2026-11-25 (`docs/decisions.md`): the
next example game, trails, is played online, on always-free hosting.

## Context

- **2618** hosts the trails server on Oracle Cloud Infrastructure (OCI)
  Always Free, and **its ADR** (written there) records why that host,
  the fallback and the paid exits. Its runbook,
  `deploy/trails/README.md`, is written for this repository's
  operators: Nick and the agents who deploy.
- **2791** measures the server at ten full rooms: CPU, memory and bytes
  a player. Those are the numbers a game maker needs to pick a host.
- **2619** watches the server from a scheduled GitHub Actions job.
- **2854** writes the book's chapter on online play. It links 2618's ADR
  where it says where a server runs, and leaves this page and the link
  to it to this ticket.
- A game maker who uses the engine needs the same path, written for
  them and for their own server. That is this page.

The facts this page starts from, checked on 2026-11-25 for the plan
behind 2618 (re-check each on the day this ticket is worked, step 2):

- **OCI Always Free**
  ([Always Free Resources](https://docs.oracle.com/en-us/iaas/Content/FreeTier/freetier_topic-Always_Free_Resources.htm)):
  - A1.Flex: 1,500 OCPU hours and 9,000 GB hours a month, which the page
    calls the equal of 2 OCPUs and 12 GB.
  - E2.1.Micro: up to 2 instances, 1/8 OCPU burstable, 1 GB, up to
    50 Mbps.
  - 10 TB of outbound data a month, 200 GB of block storage, and
    everything in the home region.
  - Idle reclaim: Oracle "may reclaim" an instance whose 95th percentile
    CPU, network and (A1 only) memory stay under 20% for 7 days.
- **The 2026 halving of A1** is disputed in date and in scope:
  [InfoQ](https://www.infoq.com/news/2026/07/oracle-cloud-free-tier-limits/),
  [Linuxiac](https://linuxiac.com/oracle-quietly-cuts-free-tier-ampere-a1-resources-in-half/),
  [a community thread](https://community.oracle.com/customerconnect/discussion/964620/do-the-new-reduced-ampere-a1-free-tier-limits-apply-to-existing-users).
  Stay at 2 OCPU / 12 GB or less.
- **Upgrading to Pay As You Go to avoid reclaim** is reported in forums,
  not documented, and one user reported a charge of about 93 EUR when
  upgrading
  ([thread](https://community.oracle.com/CustomerConnect/discussion/674112/upgrade-from-alwaysfree-to-payasyougo-to-avoid-deletion-with-an-initial-charge-of-93)).
  A1 creation often fails for lack of capacity, so the server must fit
  the Micro.
- **Let's Encrypt IP-address certificates**: generally available since
  2026-01-15, on the shortlived profile only (about 6 days)
  ([announcement](https://letsencrypt.org/2026/01/15/6day-and-ip-general-availability)).
  Only http-01 or tls-alpn-01 work for an IP address. dns-01 is
  forbidden ([RFC 8738](https://rfc-editor.org/rfc/rfc8738.html),
  [challenge types](https://letsencrypt.org/docs/challenge-types/)).
  Certbot 5.4 or later
  ([post](https://letsencrypt.org/2026/03/11/shorter-certs-certbot.html)).
- **Reserved public IPs** in a free tenancy: about 2 by forum reports,
  with no official page
  ([thread](https://community.oracle.com/CustomerConnect/discussion/592763/how-many-reserved-public-ip-can-i-make-in-the-free-tier)).
- **Cloudflare**: WebSockets on every plan
  ([docs](https://developers.cloudflare.com/network/websockets/)). A
  named Tunnel needs a domain on Cloudflare, about $10 a year. Durable
  Objects' free plan allows 13,000 GB-s a day, far under the roughly
  110,600 that ten always-ticking rooms need
  ([pricing](https://developers.cloudflare.com/durable-objects/platform/pricing/)).
- **Fly.io**: the smallest machine is $2.19 a month and egress $0.02/GB,
  against a $5 monthly allowance that needs a card. It has no hard spend
  cap ([pricing](https://docs.fly.io/about/pricing)).
- **Not fit**: Google Cloud's free tier (1 GB of egress a month), AWS
  (credits for 6 months), Azure (12 months), Render (spins down when
  idle), Railway ($1 a month). These were not re-checked on 2026-11-25.
- **Bandwidth, estimated before any code**: about 88 to 112 B on the
  wire per player per step (ADR-0152's frame, and the infra check's
  36 B payload with WebSocket, TLS and TCP headers), at 15 steps a
  second, so about 1.3 to 1.7 KB/s a player. 80 players around the
  clock is about 275 to 350 GB a month, against 10 TB. Sending the
  whole grid each step instead would be about 6.6 TB a month. 2791's
  measured numbers replace these estimates on the page.

## Nick input

None. The page states facts and dates. It recommends no paid host.
Spending money stays the reader's call, as it is Nick's for trails.

## Scope

**In:**

- **The page** `site/content/learn/book/hosting-a-server.md`, titled
  *Hosting your server*, with `extra.part = "chapters"`,
  `extra.chapter = 12` and `weight = 120`, right after *Playing online*
  (2854, chapter 11). Its sections:
  1. **What a game server needs.** Always on, a WebSocket port, TLS
     because the web build's page is https and browsers block plain
     `ws://` from it, and outbound data.
  2. **Hosts, dated.** A table of the hosts in the Context, each with
     what is free, its catch, and a source link, under a line giving
     the date checked. It says plainly that free terms change without
     notice. Durable Objects, Google Cloud, AWS, Azure, Render and
     Railway are rows with why each doesn't fit. Fly.io and Hetzner are
     the paid exits, as 2618's ADR orders them. Hetzner's price is
     checked on the day before it is written, or left out.
  3. **The steps, for the reader's own server**, generalized from 2618:
     - the account steps (signup, card, MFA, a budget alert, creating
       the VM) are the reader's own, done by hand;
     - a reserved public IP, and why (the certificate is for the IP);
     - opening the ports in both the cloud's firewall and the OS's;
     - a Let's Encrypt IP certificate: the shortlived profile, http-01
       on port 80, certbot 5.4 or later, and a deploy hook that reloads
       the server;
     - a systemd service;
     - `deploy/trails/` as the worked example to copy, linked file by
       file.
  4. **What it costs**: the bandwidth math, with 2791's measured table
     (CPU, memory, bytes a player) and the server's own guards (the
     egress budget, room and connection caps).
  5. **Watching it**: 2619's workflow as a template to copy, and why a
     6-day certificate needs an alert under 3 days.
  6. **Idle reclaim**: what it is, why this workload will likely match
     it, and why a rebuild script and an alert beat padding CPU or
     memory (which this page never suggests).
  7. **Privacy**: the server keeps no personal data and logs no
     addresses (as 2616 builds it). A game that adds names or chat
     takes on moderation, and that is the reader's decision.
- **The link from *Playing online***: 2854's section on where a server
  runs now links this page as well as 2618's ADR.
- **Glossary rows** for each word the page bolds (at least: TLS,
  certificate, egress, idle reclaim, reserved IP), so
  `cargo xtask check-book` passes.
- **The docs map**: a page of the book needs no new row in
  `docs/README.md`. 2855 adds a row for `deploy/trails/README.md`.
  **Either order with 2855:** if 2855 is done, link this page from
  that row as the game maker's version; if not, nothing to do (2855
  links it).

**Out (do not do):**

- Promoting any vendor beyond the facts: no affiliate links, no "best
  host".
- Steps that enter a card, a password or a code for the reader, or that
  an agent does on their account.
- Docker and Kubernetes guides.
- Changing `deploy/trails/` or the trails server. 2618, 2619 and 2791
  own them.

**Estimated size:** about 450 lines (the page 350, glossary and links
100).

## Implementation steps

1. Draft the page from 2618's ADR, `deploy/trails/README.md` (2618), the
   monitor workflow (2619) and 2791's table.
2. Re-check each host's free limits in the Context on the day, from the
   official pages linked there. Write the date checked on the page. A
   fact that changed is written as it is now, and the completion notes
   list what changed since 2026-11-25.
3. Write the page, with `newcomer` notes for a reader new to servers
   (what TLS, a port and a certificate are).
4. Add the glossary rows, the link from *Playing online*, and the docs
   map link.
5. Run the site's checks as the `run-gates` skill lists them (its site
   job), and `cargo xtask check-links`.

## Acceptance criteria

- [ ] The page renders as chapter 12, after *Playing online*, and
      *Playing online* links it.
- [ ] Every number on the page has a source link beside it or comes
      from 2791's measured table.
- [ ] The hosts' table gives the date the facts were checked, and says
      free terms change without notice.
- [ ] The page never asks an agent to create an account, enter a card
      or upgrade billing, and never suggests padding CPU or memory.
- [ ] `cargo xtask check-book` and the link checks pass.
- [ ] The book's chapter is updated (2802): this is that update.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `cargo xtask check-book`, the site build (`bash site/check-build.sh`)
  and the link check, in CI.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
