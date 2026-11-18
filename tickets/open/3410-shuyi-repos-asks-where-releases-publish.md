---
id: "3410"
title: "`shuyi repos` asks where a game's releases publish, nowhere by default, and sets the repository's variables"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3406"]
nick_input: none
completed:
---

# 3410 — `shuyi repos` asks where a game's releases publish, nowhere by default, and sets the repository's variables

## Context

Nick decided on 2026-11-18 ([decisions](../../docs/decisions.md), *a
release publishes nowhere until the developer picks where*): "should have
options for all major storefronts. otherwise its not set by default. we
should absolutely not publish to public github pages by default", and a
public GitHub download "can be another option, for where to publish".

3406 ([ADR-0133](../../docs/adr/0133-a-release-publishes-nowhere-by-default-debug-files-in-a-draft-sealed-packages.md))
made each place a repository variable the base's `release.yml` reads:
`PUBLISH_ON_GITHUB` (`true` for a public GitHub Release) and `ITCH_GAME`
(with the secret `BUTLER_API_KEY`); 3321, 3322 and 3323 add Steam, GOG
and the Epic Games Store. Today the developer sets each by hand on
GitHub's settings page. The 2026-11-12 decision (*`shuyi new` sets up a
game's repositories*) says the setup asks and makes what the builds need
([ADR-0112](../../docs/adr/0112-shuyi-repos-makes-a-games-repositories-keys-and-secrets.md)).

## Nick input

None: the default (nowhere) and the options are decided.

## Scope

**In:**
- `shuyi repos` (and `shuyi new --github`, which runs it), in a
  terminal, asks where the game's releases publish, each place a yes or
  no, **no** by default: a public GitHub Release, itch.io (its
  `user/game`), and each store whose job exists when this ticket is
  worked. Flags answer each question (`--publish github,itch` or the
  like: decide the shape, an ADR if it is a new pattern), and without a
  terminal nothing is asked and nothing is set.
- It sets the repository variables with `gh variable set`, and for a
  store's key asks the developer to paste it (`gh secret set` reads it
  from standard input, never an argument), or names the page where it is
  made and the secret to set, when the key can't be pasted.
- Run again, it repairs as ADR-0112 says: it says what is set, and
  changes only what the developer answers.
- Its answer (`--json`) lists each place and whether it is on.
- `docs/contract.md` (the `repos` operation), the base's README
  *Releasing*, the `shuyi-new-game` skill, the book's chapter on making a
  game.

**Out (do not do):**
- The stores' jobs (3321, 3322, 3323).
- Making a store's account or key: the developer's, by hand.

**Estimated size:** about 1,000 lines (code 400, tests 400, docs 200).

**Split:** past 900 lines, split off the stores' keys (asking for a
secret) into a new ticket; the variables stay here.

## Implementation steps

1. Read ADR-0112, ADR-0133 and `crates/cli/src/contract/repos` (or where
   `repos` lives), and how it asks and sets `PRIVATE_ASSETS_KEY`.
2. The questions, the flags, `gh variable set` through the same runner
   its tests fake.
3. The docs and the contract.

## Acceptance criteria

- [ ] With no answer, `shuyi repos` sets no place: a tag's release stays
      a draft.
- [ ] Each yes sets its variable, and re-running changes nothing unasked.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, with `repos`' faked `gh`: no answer sets nothing; each flag sets
  its variable; a re-run with the same answers runs no `gh variable set`;
  a key is passed on standard input, never in the arguments.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
