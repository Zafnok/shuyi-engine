---
id: "2619"
title: "Watch the trails server: an external uptime, certificate and reclaim check that opens an issue for Nick"
type: infra
milestone: "Engine: networking"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2618"]
nick_input: none
completed:
---

# 2619 — Watch the trails server

## Context

2618 puts the trails server on an Oracle Cloud Always Free VM
(ADR-0151). Two things can take it down without anyone noticing:
- **Reclaim.** Oracle may reclaim an idle Always Free VM, and this
  workload meets every idle test (CPU, network and memory under 20% for
  7 days). 2618's caveat (b) covers this.
- **A broken renewal.** Let's Encrypt IP certificates last about 6 days
  (the shortlived profile), so a renewal that fails takes the game
  offline within a week.

Nick needs to hear about either without watching a dashboard. A
scheduled GitHub Actions job on this public repository costs nothing,
and an issue assigned to Nick reaches him by GitHub's email.

The server answers `/healthz` and `/stats` on its one port (2616).
`/stats` gives the rooms, the humans, and the bytes sent this month
against the egress budget. Use the field names 2616 documents.

## Nick input

None.

## Scope

**In:**
- **`.github/workflows/server-watch.yml`:**
  - `schedule` every 15 minutes (`*/15 * * * *`), plus
    `workflow_dispatch` with a `dry_run` input.
  - `permissions: contents: read` (to check out and read the address)
    and `issues: write`, nothing else.
  - Every action is pinned by SHA, so zizmor passes with no new ignore.
- **`scripts/server-watch.sh`**, which the workflow runs:
  - It reads the address from `site/static/trails/server.ron` (2618),
    with `wss://<ip>/play` giving `https://<ip>`.
  - **Health:** `GET https://<ip>/healthz` answers 200 within 10 s.
  - **Stats:** `/stats` parses with `jq`. Rooms are at most 10, and the
    bytes sent this month are under 80% of the egress budget.
  - **Certificate:** read its expiry with `openssl s_client -connect
    <ip>:443` and `openssl x509 -noout -checkend 259200`. More than 3
    days must remain.
  - Flags for the test: `--address`, `--cacert <file>` (trust a test
    certificate) and `--dry-run`. A dry run prints the action it would
    take (`open`, `update`, `close` or `none`) and the failing check, and
    never calls GitHub.
- **On a failure**, it opens one issue labelled `server-down`, assigned
  to `Zafnok` so GitHub emails Nick, or updates the one already open.
  - The issue names the failing check and the time.
  - It keeps a line `Failures in a row: N` in its body, updated each
    run. That is the only state the job needs.
  - From the second unreachable run in a row, the body says the VM may
    be stopped or reclaimed, and to rebuild it with
    `deploy/trails/README.md` (2618).
  - A certificate under 3 days says to check renewal (`certbot renew
    --dry-run` on the VM, and the runbook's renewal section).
  - Egress over 80% says that new players will be refused at 100%.
- **When healthy again**, it closes the issue with a comment saying how
  long it was down.
- **Weekly:** the first run after Monday 00:00 UTC writes rooms, humans
  and egress this month to the job summary, and never opens an issue
  for it.
- **The `server-down` label**, created with `gh label create`.
- **The runbook:** a section in `deploy/trails/README.md` (2618) on the
  watch.
  - GitHub disables a scheduled workflow after 60 days with no activity
    in the repository, and the job's own runs and summaries do not
    count.
  - To re-enable it: the Actions tab → *Server watch* → *Enable
    workflow*, or `gh workflow enable server-watch.yml`.
- **CI:** shellcheck over `scripts/server-watch.sh` in 2618's
  `deploy-scripts` job, plus the dry-run tests below.

**Out (do not do):**
- Paging services, metrics databases and dashboards.
- A live player count on the website. It needs CORS on the server and a
  page script. File a follow-up if Nick wants one.
- Restarting or rebuilding the VM automatically. No SSH key goes into
  GitHub secrets (2618).
- Watching hosts other than the one in `site/static/trails/server.ron`.

**Estimated size:** about 250 lines (script 120, workflow 50, test
fixture and CI steps 60, runbook 20).

## Implementation steps

1. Write `scripts/server-watch.sh` with its checks and its `--address`,
   `--cacert` and `--dry-run` flags, and shellcheck it.
2. Write a small fixture server for the test,
   `scripts/server-watch-fixture.py`. It uses Python's standard
   library, plain or TLS, and answers `/healthz` and a `/stats` body in
   2616's shape.
   - The test certificates come from `openssl req -x509 -newkey ed25519
     -subj "/CN=127.0.0.1" -addext "subjectAltName=IP:127.0.0.1"`, with
     `-days 30` for a healthy one and `-days 1` for an expiring one.
3. Add the dry-run cases to the `deploy-scripts` job (2618): healthy,
   down (nothing listening), expiring (the 1-day certificate) and egress
   over 80%.
4. Write `.github/workflows/server-watch.yml`. It calls the script, then
   opens, updates or closes the issue with `gh issue` and
   `GH_TOKEN: ${{ github.token }}`.
5. Create the `server-down` label.
6. Add the watch section to `deploy/trails/README.md`.
7. Verify against the live server, as the acceptance criteria say.

## Acceptance criteria

- [ ] The live server is stopped (`systemctl stop trails-server`) and a
      manual dispatch opens one `server-down` issue assigned to Nick.
      A second dispatch updates it, with `Failures in a row: 2` and the
      reclaim hint, and does not open another.
- [ ] Starting the server again and dispatching closes the issue with
      a comment.
- [ ] In dry-run mode, a certificate under 3 days from expiry is
      flagged and a 30-day one is not.
- [ ] In dry-run mode, egress over 80% of the budget is flagged.
- [ ] A healthy run leaves no issue open and makes no API call to
      issues.
- [ ] zizmor passes with no new ignore, and the workflow's permissions
      are `contents: read` and `issues: write` only.
- [ ] The runbook has the watch section, with how to re-enable the
      workflow.
- [ ] The completion notes have a `Book:` line: the hosting page is
      2856's.
- [ ] The completion notes have a `Skills:` line: no schema, template,
      format or problem code changes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- In CI, the script's dry-run mode runs against the fixture, plain and
  over TLS, started in the job: healthy, down, an expiring certificate
  and egress over 80%. Each asserts the action printed and the exit
  code.
- shellcheck over `scripts/server-watch.sh`.
- Manual, recorded in the completion notes: the open, update and close
  against the live server.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
