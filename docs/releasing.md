# Releasing

What a release publishes and how, and what to do when a publish stops
part way. Why it works this way is
[ADR-0152](adr/0152-publishing-the-engine-crates-the-workspaces-list-exact-pins.md).
The release workflow itself is 2005's.

## What a release publishes

- Every engine crate (`shuyi` and `shuyi-*`) at one version, the
  workspace's. `cargo xtask check-publish --order` lists them in the
  order they go out. Every other member is `publish = false`.
- Each engine crate's dependencies on the others are pinned exactly to
  that version, through `[workspace.dependencies]`, so a game never
  mixes two releases.
- The release workflow runs `cargo publish --workspace`, and cargo
  publishes the crates in their dependency order.
- The version and `CHANGELOG.md` change at release time only: the
  version is chosen from the `Breaking:` lines since the last tag, and
  `cargo xtask changelog` writes `CHANGELOG.md`. A ticket's PR edits
  neither.

A version on crates.io can never be replaced or deleted, only yanked.

## A publish that stopped part way

The publish job failed after some crates went out. Some are live at the
new version and the rest are not.

1. **Read what is live.** `cargo xtask check-publish --order` prints the
   crates in publish order. For each, read its index entry
   (`https://index.crates.io/sh/uy/<crate>`, one JSON line per version)
   or its page (`https://crates.io/crates/<crate>/versions`), and write
   down which have the new version. The index can take a minute or two
   to show a crate that just went out.
2. **A failure outside the code** (the network, a rate limit, a timeout,
   the index slow to show a crate): publish the rest from the same
   commit, with one `--exclude` for each crate already live:

   ```bash
   cargo publish --workspace --locked --exclude shuyi-alloc-count --exclude shuyi-core
   ```

   Don't just run the job again. On the pinned Rust, `cargo publish
   --workspace` refuses the whole run when a version it would publish
   is already on crates.io ("crate … already exists on crates.io
   index"), before it uploads anything. It doesn't skip it. Run it from
   the release workflow with the same tag, never from a machine: the
   workflow's trusted publisher is the only credential.
3. **A failure that needs a code change** (a package that doesn't
   build, a wrong file in a package, a crate crates.io refuses):
   - Never change a version that is live.
   - Fix it on `main` with a ticket, as any change.
   - Release the next patch version of **every** engine crate from the
     fixed commit, all at one version. A crate that went out at the
     stopped version goes out again at the new one.
   - Once the new version is live, yank the stopped version of each
     crate that went out (`cargo yank --version <stopped> <crate>`),
     so no new game resolves a half set.
   - The stopped tag stays. Its GitHub release, if it exists, gets a
     note at the top: the version is incomplete and yanked, use the
     next one.
4. **What comes after the crates** waits until every crate is live: the
   GitHub release, its `llms-full.txt` and the exported packages.
5. **Never yank to undo a release that worked.** A yank doesn't remove a
   version from games that already locked it. It only keeps new
   resolves off it, and a release that went out whole has nothing to
   undo. A bad release is fixed by the next one. Security releases and
   their yanks are 3512's.
