---
id: "2618"
title: "Host the trails server on Oracle Cloud Always Free: Nick's setup, a reproducible setup script, TLS on an IP certificate, and a systemd service"
type: infra
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2789"]
nick_input: setup
completed:
---

# 2618 — Host the trails server on Oracle Cloud Always Free

## Context

Nick decided on 2026-11-25 ([decisions](../../docs/decisions.md)) that
the next example, trails (2788), goes online. It runs on hosting that is
always free and holds about 10 rooms at once. 2616 builds the headless
server library `shuyi-server`, and 2789 builds the `trails-server`
binary on it. CI builds that binary for Linux x86_64 and aarch64 as the
artifacts `trails-server-<arch>`. ADR-0154 (2613) gives the sync model:
the server sends only inputs, and a full catch-up only on join or
resync. This ticket puts that binary on a free VM that Nick owns,
behind TLS, so the GitHub Pages web build (2790) can reach it over
`wss://`.

**Research of 2026-11-25** (recorded here and in the hosting ADR). Only Oracle
Cloud Infrastructure (OCI) Always Free has both a VM that is always on
and enough egress.

From Oracle's *Always Free Resources* page, fetched 2026-11-25
(<https://docs.oracle.com/en-us/iaas/Content/FreeTier/freetier_topic-Always_Free_Resources.htm>):
- A1.Flex: 1,500 OCPU hours and 9,000 GB hours a month, which the page
  calls equivalent to 2 OCPUs and 12 GB.
- E2.1.Micro: up to 2 instances, each 1/8 OCPU (burstable), 1 GB of
  memory and up to 50 Mbps.
- 10 TB of outbound data a month and 200 GB of block storage.
- Compute and volumes in the home region only.
- Idle reclaim: Oracle "may reclaim" a VM whose p95 CPU, network and
  memory (A1 only) stay under 20% for 7 days.
- After an upgrade, Always Free resources stay free. Only use above the
  limits is billed.

**Rejected:**
- Cloudflare Durable Objects: the free plan has 13,000 GB-s a day, and
  10 rooms always ticking need about 110,600.
- GCP e2-micro: 1 GB of egress a month.
- AWS and Azure: trials (credits, 6 or 12 months), not always free.
- Render spins down an idle service. Railway costs $1 a month. Koyeb has
  no free compute.
- Fly.io: a $5 monthly allowance with a card and no hard spend cap, not
  free. The smallest machine is $2.19 a month and egress is $0.02/GB.
  That is about $2.90 at real load, but about $9.30 in the worst case.

**Bandwidth** (ADR-0154's wire sizes):
- About 112 B on the wire per step per human, which is 1,680 B/s at
  15 Hz.
- 80 humans around the clock is about 1.08 Mbps, or about 355 GB a
  month with catch-ups: 3.6% of 10 TB. Ten per cent humans is about
  35 GB.
- Neither the 10 TB nor the Micro's 50 Mbps is the limit. Reclaim and
  capacity are.

**Fallback:** the same binary behind a Cloudflare named Tunnel. WebSockets
work on every Cloudflare plan, but a named Tunnel needs a domain on
Cloudflare (about $10 a year, Nick's call). On the OCI Micro it shares
Oracle's account risk, so it is a capacity fallback, not a provider
fallback.

**Paid exits, in order:** Fly.io, then Hetzner (about EUR 4 a month,
price not re-fetched). Both are Nick's call.

**Let's Encrypt IP certificates** became generally available on
2026-01-15, for IPv4 and IPv6, on the shortlived profile only (160
hours, about 6 days):
- <https://letsencrypt.org/2026/01/15/6day-and-ip-general-availability>
- An IP identifier allows only http-01 or tls-alpn-01, never dns-01
  (RFC 8738).
- Certbot gained `--ip-address` in 5.3. Use 5.4 or later
  (<https://letsencrypt.org/2026/03/11/shorter-certs-certbot.html>).
  Ubuntu 24.04's apt certbot is far older, so use the snap.
- TLS is required, not optional: the web client is served over HTTPS
  from GitHub Pages, and browsers block plain `ws://` from it.

The Nick-only steps below can be done while 2789 is still in progress.

## Nick input

**Setup.** Nick does every account step. An agent never creates the
account, enters card or password details, accepts terms or changes
billing. The menu names are as of 2026-11 and may move. Steps 3 and 4
are money facts, recorded with the `ask-nick` skill when the ticket is
worked.

1. **Sign up** at <https://signup.cloud.oracle.com>.
   - Choose the **home region** nearest the players. It is permanent,
     and free VMs exist only there.
   - Oracle checks the card with a temporary authorization. Nick ticks
     the agreement himself.
2. **Turn on MFA:** Profile (top right) → *My profile* → *Security*
   (or *Multi-factor authentication*) → enable, with an authenticator
   app.
3. **Decide whether to upgrade to Pay As You Go** (Billing & Cost
   Management → *Upgrade and Manage Payment*).
   - Upgrading is **reported, not documented**, to stop idle reclaim.
     It comes from a forum thread and support emails, not Oracle's docs.
   - One user reported a temporary card hold or charge of about 93 EUR
     when upgrading
     (<https://community.oracle.com/CustomerConnect/discussion/674112/upgrade-from-alwaysfree-to-payasyougo-to-avoid-deletion-with-an-initial-charge-of-93>).
   - Always Free resources stay $0 either way. On PAYG, anything above
     the free limits is billed silently.
   - Without PAYG, the rebuild script and the watch (2619) still bring
     the game back after a reclaim.
4. **Create a budget alert** (whichever account type): Billing & Cost
   Management → *Budgets* → *Create Budget*.
   - Name `trails-guard`, target the root compartment, monthly amount
     $1.
   - Add an alert rule on **actual** spend at 1% of the budget, emailed
     to Nick.
5. **Make the deploy key**, on his own machine in PowerShell. The
   private key stays in `~/.ssh`. Nick pastes only the `.pub` file's
   line in step 6.
   ```powershell
   ssh-keygen -t ed25519 -f "$HOME\.ssh\trails_oci" -C trails-oci
   Get-Content "$HOME\.ssh\trails_oci.pub"
   ```
6. **Create the VM:** Compute → *Instances* → *Create instance*.
   - Image: Canonical Ubuntu 24.04.
   - Shape: first try *VM.Standard.A1.Flex* at 1 or 2 OCPU and at most
     12 GB, never more.
   - If it says "Out of host capacity", try the other availability
     domains, then *VM.Standard.E2.1.Micro*. The server is built to fit
     the Micro.
   - SSH keys: *Paste public keys*, the line from step 5.
   - Boot volume: the default (about 47 GB), within the free 200 GB.
7. **Reserve the public IP.**
   - Check the limit first: Governance & Administration → *Limits,
     Quotas and Usage*, service *IP Management*. Free tenancies are
     reported to get about 2 reserved IPs, but no official page says so.
   - Networking → *IP management* → *Reserved public IPs* → *Reserve
     public IP address*.
   - Attach it: the instance → *Attached VNICs* → the VNIC → *IPv4
     Addresses* → the private IP → *Edit* → *Reserved public IP* → the
     one just made.
8. **Open the ports:** Networking → *Virtual cloud networks* → the VCN →
   *Security Lists* → *Default Security List* → *Add Ingress Rules*.
   - Source CIDR `0.0.0.0/0`, protocol TCP, destination port `80`.
   - Again for `443`. Port 22 is already open.
9. **Tell the agent the reserved IP** and the shape that was granted.
   The work stops until steps 1 to 9 are done.

## Scope

**In:**
- **`deploy/trails/setup.sh`**: idempotent bash, run by the agent over
  SSH as root (`ssh -i ~/.ssh/trails_oci ubuntu@<ip> sudo bash -s`).
  Each step checks before it changes, so a second run changes nothing.
  - `unattended-upgrades` on, for security updates.
  - A `trails` system user with no shell and no home login, and
    `/opt/trails-server/`.
  - iptables: Oracle's Ubuntu image ends INPUT with a REJECT rule.
    - Insert ACCEPT rules for TCP 80 and 443 before that rule. Find its
      line with `iptables -L INPUT --line-numbers`; the number differs
      between images.
    - Persist them with `netfilter-persistent save`.
  - Certbot 5.4 or later from the snap (`snap install --classic
    certbot`); refuse an older one.
  - The first certificate: `certbot certonly --standalone` for the IP,
    on the shortlived profile, with http-01 on port 80.
    - Check the exact `--ip-address` and `--preferred-profile
      shortlived` flags against certbot's docs on the day.
    - Nothing else listens on port 80, so standalone mode works.
  - A deploy hook in `/etc/letsencrypt/renewal-hooks/deploy/`. It
    installs copies of `fullchain.pem` and `privkey.pem` into
    `/etc/trails-server/tls/` (owner root, group `trails`, mode 0640),
    then runs `systemctl reload trails-server`. 2616 reloads the
    certificate on SIGHUP.
    - Copies, not `LoadCredential=`. systemd copies a credential only
      when the service starts, so it may not refresh one on reload.
    - Use `LoadCredential=` instead only if the session proves, on the
      VM's systemd (255 on Ubuntu 24.04), that reload refreshes it. Say
      which in the completion notes.
  - `--check`: a dry run that prints each step it would take and
    changes nothing. It also works where systemd and snap are missing,
    such as a container.
- **`deploy/trails/trails-server.service`**:
  - `Restart=always`, `User=trails`, `MemoryMax=384M` (the server's
    budget is under 256 MB, so it fits the 1 GB Micro).
  - `AmbientCapabilities=CAP_NET_BIND_SERVICE` and the same
    `CapabilityBoundingSet`, so it binds 443 without root.
  - `NoNewPrivileges=yes`, `ProtectSystem=strict`, `ProtectHome=yes`,
    `PrivateTmp=yes`.
  - `StateDirectory=trails-server` for 2616's monthly egress counter.
  - `ExecReload=/bin/kill -HUP $MAINPID`.
- **`deploy/trails/trails-server.ron`**: the production config, in
  2616's config format with its field names. It is named apart from the
  client's `server.ron` below.
  - `rooms_max` 10 and `rooms_warm` 1.
  - The connection cap, the cap per address and the messages per second
    per connection, at 2616's defaults unless 2791's numbers say
    otherwise.
  - An egress budget of 1 TB a month: past it the server refuses new
    players.
  - TLS on 443 from `/etc/trails-server/tls/`.
- **`deploy/trails/cloud-init.yaml`**: a fresh Ubuntu 24.04 VM, given
  this as user data, runs the same `setup.sh` at a pinned commit and
  ends in the same state, but without the binary. `deploy.sh` then puts
  the binary on.
- **`deploy/trails/deploy.sh`**, run by the agent on Nick's machine
  (Git Bash) or in a Linux container:
  - Takes the run id of a green `main` CI run, and finds the arch over
    SSH (`uname -m`).
  - Fetches the artifact `trails-server-<arch>` **by artifact id**
    (`gh api repos/Zafnok/shuyi-engine/actions/runs/<run>/artifacts`,
    then `.../actions/artifacts/<id>/zip`). `gh run download` can return
    an earlier attempt's artifact after a re-run.
  - Copies it over with `scp`, and keeps the running binary as
    `trails-server.prev`.
  - Restarts the service and polls `https://<ip>/healthz` for up to 20
    s. If it fails, it puts `.prev` back, restarts, and exits non-zero.
- **`deploy/trails/README.md`**, the runbook:
  - first setup;
  - deploy, and rollback;
  - renewal, and what to do when it fails;
  - rebuilding after a reclaim: a new VM from `cloud-init.yaml`, the
    same reserved IP re-attached, then `deploy.sh`;
  - the fallback and the paid exits, as Nick's calls;
  - the caveats below.
- **A docs-map row** for the runbook in `docs/README.md`
  (`cargo xtask check-docs` fails a Markdown file with no row).
- **The published address:** `site/static/trails/server.ron`, holding
  `wss://<ip>/play`. Zola serves it at the site's `/trails/server.ron`.
  - It goes there and not under `site/static/examples/trails/`, because
    `pages.yml` deletes `site/public/examples/<name>/` and copies the web
    build over it.
  - 2790's client reads this file. 2619 reads the address from it too.
- **CI:** a `deploy-scripts` job in `ci.yml` on every pull request, as
  it takes seconds.
  - It runs `shellcheck` on `deploy/trails/*.sh`. Pin shellcheck's
    release by its SHA-256, as the `templates` job pins actionlint.
  - It runs `setup.sh --check` in an `ubuntu:24.04` container, pinned by
    digest (zizmor's `unpinned-images`).
  - Add it to `ci-result`'s needs.
- **The hosting ADR** (the `write-adr` skill; its number from `cargo
  xtask next-id adr`, written into the tickets that name "2618's ADR").
  It records the host choice, the rejected hosts with their numbers, the
  fallback, the paid exits and the caveats.

**Caveats the runbook and the hosting ADR state:**
- (a) **Free terms change without notice.** Oracle halved the A1
  allowance in 2026.
  - The date is disputed: InfoQ says 2026-06-15, Linuxiac's article is
    dated 2026-06-14, and a community thread cites 2026-08-18.
  - Whether it applies to PAYG accounts is unresolved. The docs say
    "all tenancies".
  - So stay at 2 OCPU and 12 GB or less, always. Oracle warns it "may
    not be possible to recreate resources above the updated Always Free
    limit".
- (b) **Idle reclaim will match this workload.** About 1% CPU, under
  100 MB of memory and about 1 Mbps meet every idle test.
  - The PAYG exemption is reported, not documented.
  - The real mitigation is the rebuild script plus the watch (2619).
  - Never pad CPU, memory or traffic to dodge reclaim.
- (c) **A1 is often out of capacity**, so the server must fit the
  E2.1.Micro: x86_64, 1 GB of memory and 1/8 OCPU. Its reclaim test has
  no memory criterion.
- (d) **The home region is permanent**, and signup needs a card.
- (e) **Certificates last about 6 days**, so a broken renewal takes the
  game offline within a week. 2619 alerts under 3 days left.
- (f) **Reserved IP limits are not officially documented.**
- (g) **One region**, so far-away players get high latency.
- (h) **The protocol sends inputs only** (ADR-0154), with a full
  catch-up only on join and resync. Sending the whole grid every step
  would be about 6.6 TB a month.
- (i) **The raw IP is exposed.** The server's own limits (2616) on
  connections, messages and egress are the defence.

**Out (do not do):**
- Deploying from CI: no SSH key goes into GitHub secrets. A later ticket
  may add a deploy from a protected environment if Nick wants one.
- A domain or Cloudflare, which cost money. The fallback is only
  documented.
- Monitoring and alerts: 2619 does them, and updates this runbook.
- Paid hosts.
- The game makers' guide to hosting: 2856 writes it from this ticket
  and 2791.
- The relay's hosting (1141, parked).
- The hosted preview (2611) stays undeployed.
- IPv6. The VCN has no IPv6 by default, so it is a follow-up if
  wanted.

**Estimated size:** about 600 lines (scripts 300, unit and configs 60,
the CI job 40, ADR 100, runbook 100).

## Implementation steps

1. Ask Nick for the setup above with the `ask-nick` skill. Record the
   PAYG choice in `docs/decisions.md`. Stop until he gives the IP and
   the shape.
2. Write `setup.sh` (with `--check`), the unit, `trails-server.ron`,
   `cloud-init.yaml`, `deploy.sh` and the deploy hook. Run `shellcheck`
   in WSL, or let the CI job run it.
3. Add the `deploy-scripts` job to `ci.yml`, with shellcheck and
   `setup.sh --check` in the pinned container.
4. Run `setup.sh` on the VM over SSH. Run it a second time and check
   that it changes nothing.
5. Run `deploy.sh` with the run id of a green `main` CI run that has
   2789's artifacts.
6. Verify each acceptance item and write the checklist into the
   completion notes.
7. If the quota allows a second VM (two Micros are free), time a rebuild
   from `cloud-init.yaml` on it, then delete that VM. If not, write the
   steps and say it was not timed.
8. Write `site/static/trails/server.ron`, the runbook, the docs-map row
   and the hosting ADR.

## Acceptance criteria

- [ ] The trails web build on GitHub Pages joins a room at
      `wss://<ip>/play` with no certificate warning, in Chromium and
      Firefox.
- [ ] `certbot renew --dry-run` passes, and the deploy hook runs in it
      (`--run-deploy-hooks`; check the flag on the day). After one
      forced renewal, the server serves the new certificate's serial
      with the same process id: it reloaded and did not restart.
- [ ] A VM reboot brings the service back with no manual step.
- [ ] `deploy.sh` given a broken binary (for example `/bin/false`)
      rolls back, and `/healthz` is 200 afterwards.
- [ ] A second run of `setup.sh` changes nothing: it reports every step
      as already done.
- [ ] Nick confirms the `trails-guard` budget alert exists.
- [ ] The VM's shape is within Always Free: A1.Flex at 2 OCPU and 12 GB
      or less, or E2.1.Micro. The console text goes in the completion
      notes, with no secrets.
- [ ] `site/static/trails/server.ron` holds the live address.
- [ ] The hosting ADR is written and indexed, and the runbook has its
      docs-map row.
- [ ] The completion notes have a `Book:` line: the hosting page is
      2856's.
- [ ] The completion notes have a `Skills:` line: no schema, template,
      format or problem code changes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `shellcheck` over `deploy/trails/*.sh` in CI.
- `setup.sh --check` runs in the pinned `ubuntu:24.04` container job,
  and exits 0.
- Manual, recorded in the completion notes: the setup run and its
  second run, the deploy, the rollback, the reboot, the forced
  renewal, and the browser join.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
