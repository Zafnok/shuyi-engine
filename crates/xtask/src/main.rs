//! `cargo xtask`: this repository's own tools, never shipped (ticket 2002).
//!
//! - `cargo xtask ticket-lint [--pr-branch <branch>] [--pr-title <title>] [--base <rev>] [--game-repo <path>]`
//! - `cargo xtask check-links`
//! - `cargo xtask check-docs`
//! - `cargo xtask check-paths`
//! - `cargo xtask check-merges [--base <rev>] [--head <rev>]`
//! - `cargo xtask mend-merges`
//! - `cargo xtask check-book`
//! - `cargo xtask next-id ticket <block>` | `cargo xtask next-id adr`
//! - `cargo xtask check-no-std`
//! - `cargo xtask check-layers`
//! - `cargo xtask check-targets`
//! - `cargo xtask check-publish [--order | --index]`
//! - `cargo xtask check-modules`
//! - `cargo xtask check-pins`
//! - `cargo xtask check-toolchain`
//! - `cargo xtask check-devcontainers`
//! - `cargo xtask book-pad [--out <dir>] [--no-build]`
//! - `cargo xtask contract [--base <rev>]`
//! - `cargo xtask api [--base <rev>] [--pr-branch <branch>] [--pr-title <title>] [--summary <file>] [--verdict <file>]`
//! - `cargo xtask skill-examples [<skill>...] [--shard <i>/<n>]`
//! - `cargo xtask eval-grade <task.ron> <project> [--base <dir>] [--out <file>]`
//! - `cargo xtask eval-reference (<id> | --all) [--out <dir>] [--template-only]`
//! - `cargo xtask compile-times [--json] [--budget <file>] [--summary <file>] [--out <file>] [--dir <dir>] [--count-only]`
//! - `cargo xtask template-lines [<folder>...] [--json] [--check] [--summary <file>]`
//! - `cargo xtask gates [--base <rev>] [--fast] [--list]`
//! - `cargo xtask trim-targets [--idle-days N] [--dry-run] [--merged] [--stale-hashes]`
//! - `cargo xtask pr-size [--base <rev>] [--hook] [--ticket <file>] [--pr-branch <branch>] [--pr-title <title>]`
//! - `cargo xtask docs-rules [--base <rev>] [--head <rev>] [--pr-branch <branch>] [--pr-title <title>]`
//! - `cargo xtask flaky rerun|report|calibration …`
//! - `cargo xtask changelog [--site <content dir>] [--format keep-a-changelog] [--base <rev>]`
//! - `cargo xtask llms --public <dir> --base-url <url> --commit <sha>`
//! - `cargo xtask llms --full-only --out <dir> --base-url <url> --commit <sha>`
//! - `cargo xtask site-data --site <dir>`
//! - `cargo xtask tutorial-tries [--check] [<name>...]`
//! - `cargo xtask exercises [--copy] [<step>...]`
//! - `cargo xtask golden --bless`
//! - `cargo xtask seed-target --into <worktree>`
//! - `cargo xtask sound-parity corpus --out <dir> [--oracle]`
//! - `cargo xtask sound-parity gate --dir <dir> --revision <sha>`
//! - `cargo xtask sound-parity port-ready <platform> --matrix <file>`
//! - `cargo xtask sound-parity give-up [--apply]`

mod api;
mod book;
mod book_pad;
mod changelog;
mod compile_times;
mod compress;
mod contract;
mod devcontainers;
mod docs;
mod docs_rules;
mod eval;
mod exercises;
mod flaky;
mod gates;
mod golden;
mod guides;
mod ids;
mod layers;
mod links;
mod llms;
mod merges;
mod modules;
mod no_std;
mod paths;
mod pins;
mod pr_size;
mod publish;
mod script;
mod site_data;
mod skills;
mod sound_parity;
mod targets;
mod template_lines;
mod tickets;
mod toolchain;
mod tries;
mod trim;

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
usage:
  cargo xtask ticket-lint [--pr-branch <branch>] [--pr-title <title>] [--base <rev>] [--game-repo <path>]
      Check tickets/open and tickets/done (tickets/README.md). `game#NNNN` ids
      are checked against the game's checkout at <path>, or on GitHub with `gh`.
      With --pr-branch, check the PR's ticket, read from <title> first
      (`[NNNN] …` is in done/; `Ticket NNNN: …` changes only tickets, against
      <rev>, by default where HEAD left origin/main); tickets/README.md,
      *Pull requests*.
      Warns, without failing, on a clause in an open ticket that waits on a
      done one (\"if NNNN is done\"): settle it (the work-ticket skill, §5).
  cargo xtask check-links
      Check that every relative link in the Markdown files resolves, and
      every docs/ path a Rust comment under crates/ or examples/ names.
  cargo xtask check-docs
      Check that the docs map (docs/README.md) links every Markdown file, and
      that no two ADRs, or two rows of docs/adr/README.md, share a number.
  cargo xtask check-paths
      Check that no tracked path is over 150 characters: Cargo's git checkout
      on Windows adds about 80, and libgit2 stops at 260 (ticket 2071).
  cargo xtask check-merges [--base <rev>] [--head <rev>]
      Check that no merge commit in <base>..<head> (origin/main..HEAD) changes
      more than resolving its two parents needs (`git show --remerge-diff`):
      a file git merged without a conflict, or a line outside a conflict,
      fails, named (ticket 2065).
  cargo xtask mend-merges
      Split each merge check-merges refuses, among the branch's commits no
      remote has, into the merge alone and a commit of its own changes, with
      the same files at the end, so a plain push follows (ticket 3532). The
      post-commit hook runs it after every merge.
  cargo xtask check-book
      Check the marks saying who a passage of Learn is for (ADR-0090 §5): an
      audience of the four, a topic, closed, not nested, on lines of their
      own, and no callout left in the old form (`> **Rust:**`); and that each
      term the book bolds has a row in its glossary (ticket 2852).
  cargo xtask next-id ticket <block>
  cargo xtask next-id adr
      Print the next ticket id in <block> (its two digits, tickets/README.md)
      or the next ADR number that neither origin/main, this checkout nor an
      open PR (`gh pr list`) has taken. Without `gh`, warns that it read
      origin/main and this checkout only.
  cargo xtask check-no-std
      Check that shuyi-core, shuyi-ui and shuyi-platform use no `std::` path
      outside `cfg(feature = \"std\")` (ADR-0008 §5).
  cargo xtask check-layers
      Check the crates' layers against tools/layers.ron (ADR-0032): which
      engine crate depends on which, the outside crates of the pure ones,
      no device crate below them and the float ban in the simulations.
  cargo xtask check-targets
      Check that no two workspace packages build a binary, or an example, of
      the same name: their outputs are one file (ticket 2056).
  cargo xtask check-publish [--order | --index]
      Check that the engine crates are the publish list (ticket 3510,
      ADR-0152): each takes the workspace's version and `publish`, every
      other member says `publish = false` itself, an engine crate's normal
      and build dependencies on engine crates come from
      [workspace.dependencies] pinned to `=` the workspace's version, its
      dev-dependencies on them are path-only, none depends on an unpublished
      member, and they make no cycle. --order prints the engine crates in
      the order they publish, one per line. --index also asks crates.io's
      sparse index for each name (needs the network and `curl`), and fails
      a name that isn't held.
  cargo xtask check-modules
      Check the modules against docs/modules.md and docs/modules.ron (ADR-0009):
      no default crate depends on one, each has a README, its metadata and a
      row, answers the concerns list, alternatives name each other, and each
      feature of the shuyi crate names a row. And the guides, docs/guides/
      (ticket 2004): each answer of perspective, kind and world-drawing (but
      flat) has one, each answer they name is in docs/modules.ron, their
      links and tickets exist, a planned one names an open ticket, and the
      README lists each.
  cargo xtask check-pins
      Check that every workflow's actions are pinned by commit SHA, one SHA per
      action, every template's workflows in .github/dependabot.yml, and the
      templates' requirements equal to the root's (ADR-0037).
  cargo xtask check-toolchain
      Check that the pinned Rust is the same everywhere (the bump-rust skill):
      the root's exact channel, the templates' channels, every workspace's
      rust-version, the members' inherited one and Dependabot's folders.
  cargo xtask check-devcontainers
      Check that the book's dev container (.devcontainer/book/) builds every
      step of site/tutorial/, no more, in order, and keeps the root's image,
      setup and remoteEnv (ticket 2837).
  cargo xtask book-pad [--out <dir>] [--no-build]
      Serve the book from its Codespace (ticket 2840): build the site as CI
      does into <dir> (~/book-site), at the Codespace's forwarded port 4000
      (http://127.0.0.1:4000/ outside one), and serve it on 127.0.0.1:4000,
      each step's play/ from its own dist/web/. A step whose crates/ or
      Cargo.toml changes is built again (`cargo shuyi web`), and its page
      follows it (GET /pad/build/<step>). With --no-build, serve the site
      already in <dir>. A port already taken: says the book is served.
  cargo xtask api[--base <rev>] [--pr-branch <branch>] [--pr-title <title>] [--summary <file>] [--verdict <file>]
      Show the public API change of the engine's crates, against <rev> (by
      default `git merge-base HEAD main`): the listing diff (cargo-public-api)
      and the breaks (cargo-semver-checks), appending the Markdown to <file>.
      With --pr-branch, a break needs a `Breaking:` line in the PR's
      ticket (ADR-0038), read from --pr-title first, and a change that
      changes no file under site/content/learn/ or site/snippets/ a `Book:`
      line saying why the book needs none (ticket 2815). A change to the
      contract's schemas that breaks a client raises its VERSION
      (crates/cli/src/contract.rs) by exactly one over <rev>'s, any other
      leaves it, and each break is a finding of shuyi-cli (ticket 2052,
      ADR-0150). With --verdict, write `none`, `added` or `breaking`
      to <file>, for the PR's `api:` label.
  cargo xtask contract [--base <rev>]
      List what in the contract's schemas breaks a client against <rev>'s
      (origin/main), and check that contract::VERSION rose by one for a
      break and stayed for anything else (ADR-0150): the api job's rule,
      without its tools.
  cargo xtask skill-examples [<skill>...] [--shard <i>/<n>]
      Run the worked example of each skill of the skill pack (skills/, ADR-0043)
      on a fresh `shuyi new` game in target/skill-examples/, or of the skills
      named, or of every <n>th from the <i>th: a verdict per step. The games
      build in target/skill-examples-build with no incremental cache, `shuyi
      new` in this repository's target folder, unless CARGO_TARGET_DIR is set.
  cargo xtask eval-grade <task.ron> <project> [--base <dir>] [--out <file>]
      Grade the game in <project> against an eval task (evals/tasks/, ticket
      2524): each check through the tool contract or cargo, run in its folder,
      and with --base (a fresh `shuyi new` of the task's template and name)
      its lines over it, by `git diff --no-index`. Writes <file> (by default
      eval-result.json). Exits 0 when every check passed, 1 when one failed,
      2 when it couldn't grade.
  cargo xtask eval-reference (<id> | --all) [--out <dir>] [--template-only]
      Grade a task's reference solution, or with --all every task's that has
      one (and list the ones that wait), with eval-grade, over a base: two
      games `shuyi new` makes in target/eval/, one the reference is laid over
      (a whole project is graded where it is), the other the base. Writes
      <dir>/<id>.json (by default eval-results/). With --template-only, grade
      the plain template instead, to show the checks can fail. The games build
      in target/eval-build with no incremental cache unless CARGO_TARGET_DIR
      is set. Exits as eval-grade, its worst over the tasks.
  cargo xtask compile-times [--json] [--budget <file>] [--summary <file>] [--out <file>] [--dir <dir>] [--count-only]
      Time a new `grid` game's clean build, its rebuild after a one-line change
      in main.rs and the same for `cargo check`, a fixed reference build, and
      count a new `blank` game's crates, on games made in <dir>
      (target/compile-times); the table names the CPU and its cores. With
      --budget, fail over its numbers (tools/compile-budget.ron); with
      --summary, append the table's Markdown to <file>; with --out, write the
      JSON to <file>. With --count-only, only count the crates
      (and hold the count to the budget): seconds, not minutes.
  cargo xtask template-lines [<folder>...] [--json] [--check] [--summary <file>]
      Count the lines a game's author owns (ticket 2512): code, tests, data and
      setup, in the game the template makes for each short name (`blank`,
      `grid`, composed, none written) and each folder under examples/, or the
      short names or game folders given. With --json, every file's path, kind
      and lines too; with --summary, append the table's Markdown to <file>.
      With --check, fail when the capped template's code is over
      tools/line-budget.ron's cap, or 10 or more under it.
  cargo xtask gates [--base <rev>] [--fast] [--list]
      Run the local gates the diff from <rev> (by default origin/main's merge
      base) needs, fast to slow, stopping at the first failure (ticket 2054).
      Which apply comes from .github/workflows/changes.yml's rules; the
      commands are the run-gates skill's. Prints a summary block for the PR's
      body on stdout: HEAD, each gate's result, each gate left to CI and why.
      With --fast, only the quick gates (the pre-push hook's); with --list,
      say what would run and run nothing. Locally (not with CI set), a full run
      that passes ends with `trim-targets` with its defaults, in one line;
      SHUYI_NO_TRIM=1 turns it off.
  cargo xtask trim-targets [--idle-days N] [--dry-run] [--merged] [--stale-hashes]
      Free the disk of this repository's other worktrees' target/, keeping
      every worktree (ticket 2076): in each one idle (no build in N days, by
      default 3) and not locked (no cargo running), delete the `incremental`
      folder of each profile folder of target/, target/gates/games,
      target/tutorial and target/skill-examples*. With --merged, all of
      target/ of a worktree whose branch is merged into origin/main or whose
      PR is merged (`gh`). With --stale-hashes (ticket 2092), also the older
      hashes of a crate in those profile folders' deps/ that no build read in
      the N days before the worktree's last build, keeping the newest-read
      one; skipped, said in the summary, where last-access updates are off.
      With --dry-run, list each with its target's size and delete nothing.
  cargo xtask pr-size [--base <rev>] [--hook] [--ticket <file>] [--pr-branch <branch>] [--pr-title <title>]
      Count the PR's changed lines (added + removed) from <rev>'s merge base
      (by default origin/main's) to the working tree, untracked files
      included (ticket 2053): code, tests, docs and hand-written data, with
      snapshots, golden tables, schemas, lock files and generated files left
      out and totalled apart, as the tickets are. Past 1,000 lines it warns;
      past 1,500 the PR's done ticket needs a `Size:` line with the measured
      number and why it wasn't split. With --ticket <file>, or --pr-branch
      (and --pr-title, CI), a missing or wrong `Size:` line fails; otherwise
      it warns. --hook answers a Claude Code PreToolUse hook (its JSON on
      stdin) with a warning before git commit, git push or gh pr create, or
      nothing.
  cargo xtask docs-rules [--base <rev>] [--head <rev>] [--pr-branch <branch>] [--pr-title <title>]
      Check the docs map's rows as rules (tools/docs-rules.ron, ticket 3534):
      each rule whose trigger paths changed from <rev>'s merge base (by
      default origin/main's) to <head> (by default the working tree,
      untracked files included), and none of whose homes did, needs a line
      starting with its mark (`Skills: none, <why>`) in the `## Completion
      notes` of the PR's done ticket. With --pr-branch (and --pr-title, CI)
      a missing line fails; otherwise it warns, and the ticket may still be
      in tickets/open/. A ticket-only, bot, Sonar or game PR is reported,
      not failed. A glob of the rules that matches no tracked file fails.
  cargo xtask flaky rerun --failures <file> --attempt <n> --decision <file>
  cargo xtask flaky report --failures <file> --attempt <n>
  cargo xtask flaky calibration --job <name> --jobs <file> --dir <dir> --revision <sha> --summary <file>
      The flaky gates' ledger, tools/flaky.ron (ticket 2064). `rerun` judges a
      failed CI attempt's jobs (scripts/flaky-failures.sh's JSON) and writes
      `none`, `failed` or a job's id to <file>: a first attempt whose every
      failure is a listed cell is re-run once. `report` warns what attempt <n>
      failed on, for its re-run. `calibration` writes a calibration run's table
      (.github/workflows/calibrate.yml) to <file>.
  cargo xtask changelog [--site <content dir>] [--format keep-a-changelog] [--base <rev>]
      Write the changelog from tickets/done/ and the first-parent history
      (ticket 2804): Markdown on stdout; with --site, Zola pages (changelog.md
      and a post for each release, in news/generated/) in <content dir>; with
      --format keep-a-changelog, the shape of CHANGELOG.md. Needs the tags and
      the whole history (a full clone). Fails a migration block, in a ticket
      added to tickets/done/ since <rev> (default: where HEAD left
      origin/main, else main), whose `since` is not the workspace version
      (ticket 2058).
  cargo xtask llms --public <dir> --base-url <url> --commit <sha>
      After `zola build` into <dir> at <url>, write the docs for agents there
      (ticket 2808): a Markdown twin of each Learn page (its URL + index.md),
      llms.txt, llms-full.txt and the contract's schemas, and check them: each
      twin's page built, no shortcode left, every llms.txt link resolving, and
      llms-full.txt under its budget.
  cargo xtask llms --full-only --out <dir> --base-url <url> --commit <sha>
      Write llms-full.txt alone into <dir>, for a release to attach.
  cargo xtask site-data --site <dir>
      Write <dir>/data/tickets.toml (ticket 2815): the status of each feature
      card of <dir>/data/features.toml that names `tickets`, Ready when all
      are in tickets/done/, In progress when some are, Planned when none is,
      and its link text when it has none. Fails on a card with both `tickets`
      and `status`, or naming a ticket in neither tickets/open/ nor done/.
      Then copy docs/guides/ into <dir>/content/learn/book/guides/ (ticket
      2004), the README as its _index.md, with front matter and the links
      made the site's.
  cargo xtask tutorial-tries [--check] [<name>...]
      Render the tutorial's and the book's pictures from
      site/tutorial/tries.toml (tickets 2818, 2819), or those named: each game
      as it is and, for a try with edits, a copy of it with them made, built
      and run. Writes them to
      site/static/tutorial-tries/; with --check, compares with those kept and
      fails on a stale one. Fails on an edit that isn't exactly once in its
      file and, for the whole list, on an orphan or a try no page shows.
  cargo xtask exercises [--copy] [<step>...]
      Copy each step of site/tutorial/, or those named, to target/exercises/,
      with site/exercises/'s checks of its page's exercises compiled into its
      binary (ticket 2828, ADR-0117), and check them: each page's exercises
      against its step's checks, and in each copy fmt, Clippy and the checks'
      tests. With --copy, only make the copies, for the steps' web builds.
  cargo xtask golden --bless
      Write the golden tables (shuyi_assets::golden, ticket 2116): run every
      test with `golden` in its name with SHUYI_BLESS=1, write each table a
      failed test printed (never outside the repository), show `git diff
      --stat` over them, and run the tests again: they must pass. A changed
      or removed row needs a `Breaking:` line in the ticket.
  cargo xtask seed-target --into <worktree>
      Run in a warm worktree (usually the main one): start <worktree>'s
      target/ from this one's target/debug (ticket 2078). Third-party output
      is hard-linked, what cargo rewrites is copied, what <worktree> already
      built is kept, and the workspace's crates are left to build fresh with
      their own paths. Refuses while a build runs in either, and across
      volumes.
  cargo xtask sound-parity corpus --out <dir> [--oracle]
      Write the sound gates' test signals into <dir>/corpus, each checked
      against tools/sound-parity/corpus.ron; with --oracle, what `oggdec` makes
      of each Ogg file into <dir>/oracle, checked too (ticket 2573).
  cargo xtask sound-parity gate --dir <dir> --revision <sha>
      Hold what tools/sound-parity/capture.py recorded under <dir> to the
      scenarios and budget, write <dir>/matrix.json, and fail unless every
      mandatory cell passed.
  cargo xtask sound-parity port-ready <platform> --matrix <file>
      Fail unless <platform> has passed capture evidence: what a port's new
      output work needs first.
  cargo xtask sound-parity give-up [--apply]
      Check that tools/sound-parity/footprint.ron covers all the sound
      replacement added; with --apply, take it all out. Only on Nick's word:
      the give-up-sound-replacement skill says what else that session does.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = repo_root();
    match args.first().map(String::as_str) {
        Some("ticket-lint") => ticket_lint(&root, &args[1..]),
        Some("check-links") if args.len() == 1 => report("check-links", &links::run(&root), &[]),
        Some("check-docs") if args.len() == 1 => {
            let mut errors = docs::run(&root);
            errors.extend(ids::duplicate_adrs(&root));
            report("check-docs", &errors, &[])
        }
        Some("check-paths") if args.len() == 1 => report("check-paths", &paths::run(&root), &[]),
        Some("check-merges") => check_merges(&root, &args[1..]),
        Some("mend-merges") => mend_merges(&root),
        Some("check-book") if args.len() == 1 => report("check-book", &book::run(&root), &[]),
        Some("next-id") => next_id(&root, &args[1..]),
        Some("check-no-std") if args.len() == 1 => report("check-no-std", &no_std::run(&root), &[]),
        Some("check-layers") if args.len() == 1 => report("check-layers", &layers::run(&root), &[]),
        Some("check-targets") if args.len() == 1 => {
            report("check-targets", &targets::run(&root), &[])
        }
        Some("check-publish") => check_publish(&root, &args[1..]),
        Some("check-modules") if args.len() == 1 => {
            // The modules, then the guides against the same concerns
            // (ticket 2004).
            let mut errors = modules::run(&root);
            errors.extend(guides::run_repo(&root));
            report("check-modules", &errors, &[])
        }
        Some("check-pins") if args.len() == 1 => report("check-pins", &pins::run(&root), &[]),
        Some("check-toolchain") if args.len() == 1 => {
            report("check-toolchain", &toolchain::run(&root), &[])
        }
        Some("check-devcontainers") if args.len() == 1 => {
            let mut errors = devcontainers::run(&root);
            errors.extend(devcontainers::run_features(&root));
            report("check-devcontainers", &errors, &[])
        }
        Some("book-pad") => book_pad(&root, &args[1..]),
        Some("api") => api(&root, &args[1..]),
        Some("contract") => contract_check(&root, &args[1..]),
        Some("skill-examples") => skill_examples(&root, &args[1..]),
        Some("eval-grade") => ExitCode::from(eval::grade::command(
            &root,
            &args[1..],
            &script::System::default(),
        )),
        Some("eval-reference") => {
            let env_target = std::env::var_os("CARGO_TARGET_DIR").is_some();
            let (maker, runner) = skills::runners(&root, eval::reference::BUILD, env_target);
            ExitCode::from(eval::reference::command(&root, &args[1..], &maker, &runner))
        }
        Some("compile-times") => compile_times(&root, &args[1..]),
        Some("template-lines") => template_lines(&root, &args[1..]),
        Some("gates") => gates(&root, &args[1..]),
        Some("pr-size") => pr_size(&root, &args[1..]),
        Some("docs-rules") => docs_rules(&root, &args[1..]),
        Some("trim-targets") => trim_targets(&root, &args[1..]),
        Some("flaky") => {
            let (errors, warnings) = flaky::run(&root, &args[1..]);
            report("flaky", &errors, &warnings)
        }
        Some("changelog") => changelog(&root, &args[1..]),
        Some("llms") => llms(&root, &args[1..]),
        Some("site-data") => site_data(&root, &args[1..]),
        Some("tutorial-tries") => tutorial_tries(&root, &args[1..]),
        Some("exercises") => exercises(&root, &args[1..]),
        Some("golden") if args[1..] == ["--bless"] => bless(&root),
        Some("sound-parity") => report("sound-parity", &sound_parity::run(&root, &args[1..]), &[]),
        Some("seed-target") => seed_target(&root, &args[1..]),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// `check-publish [--order | --index]` (ticket 3510).
fn check_publish(root: &Path, args: &[String]) -> ExitCode {
    let flag = match args {
        [] => None,
        [flag] if flag == "--order" || flag == "--index" => Some(flag.as_str()),
        _ => {
            return report(
                "check-publish",
                &["usage: cargo xtask check-publish [--order | --index]".to_owned()],
                &[],
            );
        }
    };
    let workspace = match publish::read(root) {
        Ok(workspace) => workspace,
        Err(error) => return report("check-publish", &[error], &[]),
    };
    let mut errors = publish::check(&workspace);
    if !errors.is_empty() {
        return report("check-publish", &errors, &[]);
    }
    let order = publish::order(&workspace).unwrap_or_default();
    match flag {
        Some("--order") => {
            let mut stdout = std::io::stdout().lock();
            for name in &order {
                let _ = writeln!(stdout, "{name}");
            }
            ExitCode::SUCCESS
        }
        Some(_) => {
            errors.extend(publish::missing_from_index(&order, publish::on_index));
            report("check-publish", &errors, &[])
        }
        None => report("check-publish", &errors, &[]),
    }
}

/// The repository's root: `crates/xtask/../..`.
fn repo_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .unwrap_or(manifest)
        .to_path_buf()
}

/// `seed-target --into <worktree>`: seeds `<worktree>`'s `target/` from
/// this one's (`shuyi_cli::worktrees::seed`).
fn seed_target(root: &Path, args: &[String]) -> ExitCode {
    use shuyi_cli::worktrees::seed;
    let [flag, into] = args else {
        return report(
            "seed-target",
            &["usage: cargo xtask seed-target --into <worktree>".to_owned()],
            &[],
        );
    };
    if flag != "--into" {
        return report(
            "seed-target",
            &[format!("unknown flag {flag}: use --into <worktree>")],
            &[],
        );
    }
    let packages = match seed::path_packages_of(root) {
        Ok(packages) => packages,
        Err(e) => return report("seed-target", &[e], &[]),
    };
    match seed::seed(root, Path::new(into), &packages) {
        Ok(done) => {
            eprintln!("{done}");
            report("seed-target", &[], &[])
        }
        Err(e) => report("seed-target", &[e.to_string()], &[]),
    }
}

fn check_merges(root: &Path, args: &[String]) -> ExitCode {
    let mut base = "origin/main";
    let mut head = "HEAD";
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        match (flag.as_str(), rest.next()) {
            ("--base", Some(value)) => base = value,
            ("--head", Some(value)) => head = value,
            _ => {
                eprintln!("{USAGE}");
                return ExitCode::FAILURE;
            }
        }
    }
    report("check-merges", &merges::run(root, base, head), &[])
}

fn mend_merges(root: &Path) -> ExitCode {
    match merges::mend(root) {
        Ok(lines) => {
            for line in lines {
                eprintln!("{line}");
            }
            eprintln!("mend-merges: ok");
            ExitCode::SUCCESS
        }
        Err(e) => report("mend-merges", &[e], &[]),
    }
}

fn ticket_lint(root: &Path, args: &[String]) -> ExitCode {
    let mut pr_branch = None;
    let mut pr_title = None;
    let mut base = None;
    let mut game_repo = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match (arg.as_str(), rest.next()) {
            ("--pr-branch", Some(value)) => pr_branch = Some(value.as_str()),
            ("--pr-title", Some(value)) => pr_title = Some(value.as_str()),
            ("--base", Some(value)) => base = Some(value.as_str()),
            ("--game-repo", Some(value)) => game_repo = Some(PathBuf::from(value)),
            _ => {
                eprintln!("{USAGE}");
                return ExitCode::FAILURE;
            }
        }
    }
    let game = match &game_repo {
        Some(path) => tickets::GameSource::Checkout(path),
        None => tickets::GameSource::GitHub,
    };
    let pr = pr_branch.map(|branch| tickets::Pr {
        branch,
        title: pr_title,
    });
    let found = tickets::run(root, pr, base, &game);
    report("ticket-lint", &found.errors, &found.warnings)
}

fn next_id(root: &Path, args: &[String]) -> ExitCode {
    let Some(kind) = ids::Kind::parse(args) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    match ids::run(root, &kind) {
        Ok((next, warnings)) => {
            for warning in &warnings {
                eprintln!("warning: {warning}");
            }
            if writeln!(std::io::stdout().lock(), "{next}").is_ok() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(errors) => report("next-id", &errors, &[]),
    }
}

fn contract_check(root: &Path, args: &[String]) -> ExitCode {
    let base = match args {
        [] => "origin/main",
        [flag, value] if flag == "--base" => value,
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let checked = contract::check(root, base, &api::System);
    for line in &checked.breaks {
        eprintln!("break: {line}");
    }
    if checked.breaks.is_empty() {
        eprintln!("no break of a client from {base}'s schemas (ADR-0150)");
    }
    report("contract", &Vec::from_iter(checked.error), &[])
}

fn api(root: &Path, args: &[String]) -> ExitCode {
    let Some(options) = api::Options::parse(args) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let outcome = api::run(root, &options, &api::System);
    let mut errors = outcome.errors;
    errors.extend(api::write_summary(
        &outcome.summary,
        options.summary.as_deref(),
        &mut std::io::stdout().lock(),
    ));
    errors.extend(api::write_verdict(
        outcome.label,
        options.verdict.as_deref(),
    ));
    report("api", &errors, &outcome.warnings)
}

fn bless(root: &Path) -> ExitCode {
    let outcome = golden::bless(root, &api::System);
    for line in &outcome.lines {
        eprintln!("{line}");
    }
    report("golden", &outcome.errors, &[])
}

fn skill_examples(root: &Path, args: &[String]) -> ExitCode {
    let options = match skills::Options::parse(args) {
        Ok(options) => options,
        Err(e) => {
            eprintln!(
                "skill-examples: {e}

{USAGE}"
            );
            return ExitCode::FAILURE;
        }
    };
    let env_target = std::env::var_os("CARGO_TARGET_DIR").is_some();
    let (maker, game) = skills::runners(root, skills::BUILD, env_target);
    let outcome = skills::run(root, &skills::games_dir(root), &options, &maker, &game);
    eprint!("{}", outcome.report);
    report("skill-examples", &outcome.errors, &[])
}

fn compile_times(root: &Path, args: &[String]) -> ExitCode {
    let Some(options) = compile_times::Options::parse(args) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    // The budget is read first: a wrong one fails before minutes of builds.
    let budget = match &options.budget {
        Some(file) => match std::fs::read_to_string(file)
            .map_err(|e| e.to_string())
            .and_then(|text| compile_times::parse_budget(&text))
        {
            Ok(budget) => Some((file.display().to_string(), budget)),
            Err(e) => return report("compile-times", &[format!("{}: {e}", file.display())], &[]),
        },
        None => None,
    };
    let games = compile_times::games_dir(root, options.dir.as_deref());
    let maker = script::System::default();
    let game = compile_times::Game {
        target_dir: games.join("timed").join("target"),
    };
    if options.count_only {
        let count = match compile_times::count(root, &games, &maker, &game) {
            Ok(count) => count,
            Err(e) => return report("compile-times", &[e], &[]),
        };
        // A count that can't be printed is still held to the budget.
        let _ = writeln!(
            std::io::stdout().lock(),
            "a new game depends on {count} crates"
        );
        let verdict = budget
            .as_ref()
            .map_or_else(compile_times::Verdict::default, |(file, budget)| {
                compile_times::check_count(count, budget, file)
            });
        return report("compile-times", &verdict.errors, &verdict.warnings);
    }
    let times = match compile_times::run(root, &games, &maker, &game) {
        Ok(times) => times,
        Err(e) => return report("compile-times", &[e], &[]),
    };
    let table = times.table();
    let shown = if options.json {
        times.json().to_string()
    } else {
        table.clone()
    };
    // An answer that can't be printed is still in the summary.
    let _ = writeln!(std::io::stdout().lock(), "{shown}");
    let mut verdict = compile_times::Verdict::default();
    if let Some(file) = &options.summary {
        let written = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file)
            .and_then(|mut f| f.write_all(format!("{table}\n").as_bytes()));
        if let Err(e) = written {
            verdict.errors.push(format!("{}: {e}", file.display()));
        }
    }
    if let Some(file) = &options.out
        && let Err(e) = std::fs::write(file, times.json().to_string())
    {
        verdict.errors.push(format!("{}: {e}", file.display()));
    }
    if let Some((file, budget)) = &budget {
        let found = compile_times::check(&times, budget, file);
        verdict.errors.extend(found.errors);
        verdict.warnings.extend(found.warnings);
    }
    report("compile-times", &verdict.errors, &verdict.warnings)
}

fn template_lines(root: &Path, args: &[String]) -> ExitCode {
    let Some(options) = template_lines::Options::parse(args) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let mut all = match template_lines::run(root, &options) {
        Ok(all) => all,
        Err(e) => return report("template-lines", &[e], &[]),
    };
    let mut errors = Vec::new();
    if options.check {
        let file = root.join(template_lines::BUDGET);
        let budget = match std::fs::read_to_string(&file)
            .map_err(|e| e.to_string())
            .and_then(|text| template_lines::parse_budget(&text))
        {
            Ok(budget) => budget,
            Err(e) => return report("template-lines", &[format!("{}: {e}", file.display())], &[]),
        };
        let capped = template_lines::Options {
            folders: vec![budget.template.clone()],
            ..template_lines::Options::default()
        };
        match template_lines::run(root, &capped) {
            Ok(counts) => {
                for counts in &counts {
                    errors.extend(template_lines::check(counts, &budget));
                }
                // The capped template is in the table, whatever was given.
                if !all.iter().any(|c| c.folder == budget.template) {
                    all.splice(0..0, counts);
                }
            }
            Err(e) => errors.push(e),
        }
    }
    let shown = if options.json {
        template_lines::json(&all)
    } else {
        template_lines::table(&all)
    };
    // The verdict stands even if the table can't be printed.
    let _ = writeln!(std::io::stdout().lock(), "{}", shown.trim_end());
    if let Some(file) = &options.summary {
        let written = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file)
            .and_then(|mut f| {
                f.write_all(
                    format!(
                        "{}
",
                        template_lines::markdown(&all)
                    )
                    .as_bytes(),
                )
            });
        if let Err(e) = written {
            errors.push(format!("{}: {e}", file.display()));
        }
    }
    report("template-lines", &errors, &[])
}

fn gates(root: &Path, args: &[String]) -> ExitCode {
    let Some(options) = gates::Options::parse(args) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    match gates::run(root, &options) {
        Ok((summary, passed)) => {
            // The summary is the answer: one that can't be printed fails.
            if writeln!(std::io::stdout().lock(), "{summary}").is_err() || !passed {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => report("gates", &[e], &[]),
    }
}

fn trim_targets(root: &Path, args: &[String]) -> ExitCode {
    let Some(options) = trim::parse(args) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    match shuyi_cli::worktrees::trim::run(root, &options, &mut std::io::stdout().lock()) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => report("trim-targets", &[e], &[]),
    }
}

fn pr_size(root: &Path, args: &[String]) -> ExitCode {
    let Some(options) = pr_size::Options::parse(args) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    if options.hook {
        let mut stdin = String::new();
        // No input (a person running it by hand) measures this checkout.
        let _ = std::io::Read::read_to_string(&mut std::io::stdin(), &mut stdin);
        let answer = pr_size::hook_answer(&stdin, root, options.base.as_deref());
        // The hook never gets in the way: it always succeeds.
        let _ = write!(std::io::stdout().lock(), "{answer}");
        return ExitCode::SUCCESS;
    }
    // The checkout it is run in, which may be another worktree or an older
    // revision than the one xtask was built from.
    let here = pr_size::repo_of(Path::new(".")).unwrap_or_else(|_| root.to_path_buf());
    let outcome = pr_size::run(&here, &options);
    // A report that can't be printed leaves the verdict as it is.
    let _ = write!(std::io::stdout().lock(), "{}", outcome.report);
    report("pr-size", &outcome.errors, &outcome.warnings)
}

fn docs_rules(root: &Path, args: &[String]) -> ExitCode {
    let Some(options) = docs_rules::Options::parse(args) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    // The checkout it is run in, as `pr-size` reads it.
    let here = pr_size::repo_of(Path::new(".")).unwrap_or_else(|_| root.to_path_buf());
    let outcome = docs_rules::run(&here, &options);
    // A report that can't be printed leaves the verdict as it is.
    let _ = write!(std::io::stdout().lock(), "{}", outcome.report);
    report("docs-rules", &outcome.errors, &outcome.warnings)
}

fn changelog(root: &Path, args: &[String]) -> ExitCode {
    let Some(options) = changelog::Options::parse(args) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let git = changelog::system_git(root);
    match changelog::run(root, &options, &git) {
        Ok(changelog::Output::Stdout(text)) => {
            let _ = write!(std::io::stdout().lock(), "{text}");
            ExitCode::SUCCESS
        }
        Ok(changelog::Output::Pages(pages)) => {
            let written = options
                .site
                .as_deref()
                .map_or(Ok(()), |dir| changelog::write_pages(dir, &pages));
            report(
                "changelog",
                &written.err().into_iter().collect::<Vec<_>>(),
                &[],
            )
        }
        Err(errors) => report("changelog", &errors, &[]),
    }
}

fn llms(root: &Path, args: &[String]) -> ExitCode {
    let Some(options) = llms::Options::parse(args) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let date = llms::git_date(root);
    let env = llms::Env {
        date: &date,
        budget: llms::FULL_BUDGET_BYTES,
    };
    let outcome = llms::run(root, &options, &env);
    let mut stdout = std::io::stdout().lock();
    for note in &outcome.notes {
        // A note that can't be printed leaves the check's verdict as it is.
        let _ = writeln!(stdout, "{note}");
    }
    report("llms", &outcome.errors, &[])
}

fn site_data(root: &Path, args: &[String]) -> ExitCode {
    let Some(options) = site_data::Options::parse(args) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    // The feature cards' statuses, then the book's guides (ticket 2004).
    let mut errors = site_data::run(root, &options).err().unwrap_or_default();
    errors.extend(guides::write_site(root, &options.site).err());
    report("site-data", &errors, &[])
}

fn book_pad(root: &Path, args: &[String]) -> ExitCode {
    let Some(options) = book_pad::Options::parse(args) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let errors: Vec<String> = book_pad::run_pad(root, &options)
        .err()
        .into_iter()
        .collect();
    report("book-pad", &errors, &[])
}

fn exercises(root: &Path, args: &[String]) -> ExitCode {
    let Some(options) = exercises::Options::parse(args) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    report("exercises", &exercises::run(root, &options), &[])
}

fn tutorial_tries(root: &Path, args: &[String]) -> ExitCode {
    let Some(options) = tries::Options::parse(args) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    report("tutorial-tries", &tries::run(root, &options), &[])
}

/// Prints what a check found on stderr, and on GitHub Actions each warning on
/// stdout as a workflow command too, so it shows on the PR.
fn report(check: &str, errors: &[String], warnings: &[String]) -> ExitCode {
    let on_github = std::env::var("GITHUB_ACTIONS").is_ok_and(|v| v == "true");
    let mut stdout = std::io::stdout().lock();
    for warning in warnings {
        eprintln!("warning: {warning}");
        if on_github {
            // A warning that can't be annotated is still on stderr.
            let _ = writeln!(stdout, "{}", annotation(warning));
        }
    }
    for error in errors {
        eprintln!("error: {error}");
    }
    if errors.is_empty() {
        eprintln!("{check}: ok");
        ExitCode::SUCCESS
    } else {
        eprintln!("{check}: {} error(s)", errors.len());
        ExitCode::FAILURE
    }
}

/// `warning` as a GitHub workflow command: `::warning::` and the text, with
/// `%`, CR and LF escaped as workflow commands require.
fn annotation(warning: &str) -> String {
    let text = warning
        .replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A");
    format!("::warning::{text}")
}

#[cfg(test)]
mod tests {
    use super::annotation;

    #[test]
    fn an_annotation_is_a_warning_command() {
        assert_eq!(annotation("plain text"), "::warning::plain text");
    }

    #[test]
    fn an_annotation_escapes_percent_cr_and_lf() {
        assert_eq!(
            annotation("100% done\r\nnext%0A"),
            "::warning::100%25 done%0D%0Anext%250A"
        );
    }
}
