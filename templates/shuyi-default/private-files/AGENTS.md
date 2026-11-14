## The private files

{{name}}'s writing, art and sounds are kept in a second, private
repository, {{private_url}}, so they can't be copied from this public
one (`shuyi.ron`'s `private_assets`). **Never put one of them in this
repository**, not even a copy for a test: a public repository's history,
and its workflows' artifacts, are anyone's.

- `cargo shuyi private-assets` checks it out into `private/`
  (git-ignored) at the commit `private.rev` names. Its `files/` holds
  files at the path they have in `crates/game/files/`
  (`private/files/sprites/hero.png` ↔ `crates/game/files/sprites/hero.png`);
  its `docs/` the story and design documents, which no build reads;
  `--library` brings `library/` too, the bought packs as they came.
- The build lays `private/files/` over `crates/game/files/` whenever the
  checkout is there: a private file replaces the public one at its path,
  and a file only there is added. Without the checkout, the game builds
  and runs on its public files. After the first checkout, a build made
  before it is caught by `the_private_files_are_the_checkouts` test,
  which says to run `cargo clean -p {{crate_name}}`.
- A change to them is two pull requests: one in the private repository,
  then one here moving `private.rev` with `cargo shuyi private-assets
  --pin` once the first is merged into its `main`.
- CI checks them out before the gates (`.github/actions/private-files`),
  with the secret `PRIVATE_ASSETS_KEY` (`cargo shuyi repos` makes it, with
  the repositories: it acts on the developer's GitHub account, so ask
  them before running it): the gates read the real game. A
  playtest workflow passes the secret to the engine's, by name, so the
  bots play them too. A fork's pull request builds on the public files.
  No workflow may upload an artifact that embeds them, but
  `export.yml` for a release (`release.yml` on a tag), which publishes
  them inside the program anyway. A release's dry run builds without
  uploading, and warns and builds on the public files without the key.
- `cargo shuyi run --watch` watches `private/files/` too.
