//! `cargo xtask gates` (ticket 2054): the local gates a diff needs, run
//! fast to slow, and a summary block for the PR's body.
//!
//! Which gates a diff needs comes from CI's own rules: the classifier in
//! `.github/workflows/changes.yml` is read at run time, and its `case`
//! arms decide the `code`, `crates` and `sound` outputs for the diff's
//! paths, as they do on a pull request. Each gate names the CI job it
//! mirrors and the output that job's `if:` reads; a test holds [`GATES`]
//! to `ci.yml`'s jobs, so a job CI gains is a gate here, or a CI-only one
//! with its reason.
//!
//! The commands are the `run-gates` skill's: that skill stays the
//! reference for what each gate checks and how to fix it.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_norway::Value;

/// Which of CI's filters a gate's job runs under: the output of
/// `changes.yml` its `if:` reads, or none. The output is named, not a
/// variant of its own, so giving up the sound replacement (ADR-0068) takes
/// out the gates that read `sound` and leaves this type whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    /// No filter: every pull request runs it.
    Always,
    /// `needs.changes.outputs.<name>`.
    Output(&'static str),
}

/// `code`: anything but docs-only paths.
const CODE: Filter = Filter::Output("code");
/// `crates`: the crates or what builds them.
const CRATES: Filter = Filter::Output("crates");

impl Filter {
    /// The output's name in `changes.yml`, if any.
    fn output(self) -> Option<&'static str> {
        match self {
            Self::Always => None,
            Self::Output(name) => Some(name),
        }
    }
}

/// A narrower rule a local run adds to CI's filter.
#[derive(Debug, Clone, Copy)]
pub enum Local {
    /// As CI.
    AsCi,
    /// Only when a changed path starts with one of these.
    Under(&'static [&'static str]),
    /// Only when a changed path satisfies the test; the text says what the
    /// test looks for, for the summary.
    When(&'static str, fn(&str) -> bool),
}

/// Whether `--fast` runs a gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fast {
    /// Not with `--fast`.
    No,
    /// With `--fast`, when it applies.
    Yes,
    /// With `--fast`, only when a changed path ends with this.
    IfChanged(&'static str),
}

/// One gate: the CI job it mirrors, when it applies, and its commands, or
/// why it is CI's alone.
#[derive(Debug, Clone, Copy)]
pub struct Gate {
    /// Its name in the summary.
    pub name: &'static str,
    /// The `ci.yml` job it mirrors.
    pub job: &'static str,
    /// The job's filter.
    pub filter: Filter,
    /// What a local run adds to the filter.
    pub local: Local,
    /// Whether `--fast` runs it.
    pub fast: Fast,
    /// Folders removed before it runs, from the root.
    pub fresh: &'static [&'static str],
    /// Its commands, in order: an optional `dir: ` before the command
    /// (from the root), `NAME=value` words, then the program and its
    /// arguments. A word may be in double quotes; `{root}` is the root.
    pub steps: &'static [&'static str],
    /// Why CI alone runs it, when it does.
    pub ci_only: Option<&'static str>,
    /// What to say after a failure: how to fix it.
    pub hint: Option<&'static str>,
}

const fn gate(name: &'static str, job: &'static str, filter: Filter) -> Gate {
    Gate {
        name,
        job,
        filter,
        local: Local::AsCi,
        fast: Fast::No,
        fresh: &[],
        steps: &[],
        ci_only: None,
        hint: None,
    }
}

impl Gate {
    const fn steps(mut self, steps: &'static [&'static str]) -> Self {
        self.steps = steps;
        self
    }

    const fn fast(mut self, fast: Fast) -> Self {
        self.fast = fast;
        self
    }

    const fn local(mut self, local: Local) -> Self {
        self.local = local;
        self
    }

    const fn fresh(mut self, fresh: &'static [&'static str]) -> Self {
        self.fresh = fresh;
        self
    }

    const fn hint(mut self, hint: &'static str) -> Self {
        self.hint = Some(hint);
        self
    }

    const fn ci_only(mut self, why: &'static str) -> Self {
        self.ci_only = Some(why);
        self
    }
}

/// The jobs of `ci.yml` that are no gate: the classifier and the checks
/// that read other jobs' results.
#[cfg(test)]
pub const NOT_GATES: &[&str] = &["changes", "api-label", "ci-result"];

/// Whether a changed path can make a tutorial step's lock stale: a
/// manifest or lock of the engine's crates, the root's, or the steps.
fn touches_tutorial_locks(path: &str) -> bool {
    let file = path.rsplit('/').next().unwrap_or(path);
    let manifest = file == "Cargo.toml" || file == "Cargo.lock";
    (manifest && (path.starts_with("crates/") || !path.contains('/')))
        || path.starts_with("site/tutorial/")
}

/// Every gate, fast to slow: a run stops at the first failure.
pub const GATES: &[Gate] = &[
    gate("fmt", "fmt", CODE)
        .fast(Fast::Yes)
        .steps(&["cargo fmt --all --check"]),
    gate("typos", "tickets", Filter::Always)
        .fast(Fast::Yes)
        .steps(&["typos"]),
    gate("ticket-lint", "tickets", Filter::Always)
        .fast(Fast::Yes)
        .steps(&["cargo xtask ticket-lint {pr-branch}"]),
    gate("engine lint", "test", CODE)
        .fast(Fast::IfChanged(".rs"))
        .steps(&["cargo run -q -p shuyi-cli --locked -- lint"]),
    gate("skill loaders", "tickets", Filter::Always)
        .steps(&["pwsh -File scripts/sync-agent-skills.ps1 -Check"]),
    gate("tickets' checks", "tickets", Filter::Always).steps(&[
        "cargo xtask check-links",
        "cargo xtask changelog >null",
        "cargo xtask check-pins",
        "cargo xtask check-toolchain",
        "cargo xtask check-docs",
        "cargo xtask check-paths",
    ]),
    gate("layers", "layers", CODE).steps(&["cargo xtask check-layers"]),
    // CI runs it on every pull request; it reads the site, the problem
    // codes, Hello, Grid's files and the tickets (the feature cards).
    // As CI: the codes' pages and the playground's files are written
    // before `zola check` reads them.
    gate("site", "site", Filter::Always)
        .local(Local::Under(&[
            "site/",
            "crates/cli/errors/",
            "examples/hello-grid/",
            "tickets/",
        ]))
        .steps(&[
            "bash site/errors.sh",
            "cargo xtask site-data --site site",
            "bash site/copy-playground.sh",
            "site: zola check --skip-external-links",
            "cargo xtask check-book",
            "bash site/check-build.sh",
        ]),
    // CI's `tutorial` job builds and tests the ten steps; whether a lock is
    // stale takes seconds: `cargo metadata --locked` fails on a lock it
    // would have to change (ticket 2073).
    gate("tutorial locks", "tutorial", CODE)
        .local(Local::When(
            "no Cargo.toml or Cargo.lock under crates/, no root Cargo.toml or Cargo.lock, nothing under site/tutorial/",
            touches_tutorial_locks,
        ))
        .hint(
            "refresh the locks: for step in site/tutorial/*/; do (cd \"$step\" && cargo metadata --format-version 1 > /dev/null); done (the run-gates skill, \"The tutorial's steps\")",
        )
        .steps(&[
        "site/tutorial/01-new: cargo metadata --locked --format-version 1 >null",
        "site/tutorial/02-board: cargo metadata --locked --format-version 1 >null",
        "site/tutorial/03-walk: cargo metadata --locked --format-version 1 >null",
        "site/tutorial/04-push: cargo metadata --locked --format-version 1 >null",
        "site/tutorial/05-undo: cargo metadata --locked --format-version 1 >null",
        "site/tutorial/06-levels: cargo metadata --locked --format-version 1 >null",
        "site/tutorial/07-save: cargo metadata --locked --format-version 1 >null",
        "site/tutorial/08-polish: cargo metadata --locked --format-version 1 >null",
        "site/tutorial/09-test: cargo metadata --locked --format-version 1 >null",
        "site/tutorial/10-ship: cargo metadata --locked --format-version 1 >null",
        ]),
    gate("clippy", "clippy", CODE).steps(&[
        "cargo clippy --workspace --all-targets --features shuyi-app/capture,hello-grid/capture --locked -- -D warnings",
    ]),
    gate("docs", "docs", CODE)
        .steps(&["RUSTDOCFLAGS=\"-D warnings\" cargo doc --workspace --no-deps --locked"]),
    gate("deny", "deny", CODE).steps(&["cargo deny check"]),
    // `cargo-machete` itself: started by cargo under `cargo xtask`, it
    // reads its subcommand's name as a folder to check.
    gate("machete", "machete", CODE).steps(&["cargo-machete"]),
    gate("test", "test", CODE)
        .steps(&["cargo test --workspace --locked --no-fail-fast"]),
    gate("no-std", "no-std", CODE).steps(&[
        "cargo build -p shuyi-core -p shuyi-ui -p shuyi-platform -p shuyi-alloc-count --no-default-features --target thumbv7em-none-eabihf --locked",
        "cargo build -p shuyi-assets --no-default-features --target thumbv7em-none-eabihf --locked",
        "cargo build -p shuyi-ui --no-default-features --features raster --target thumbv7em-none-eabihf --locked",
        "cargo build -p shuyi-platform --no-default-features --features capture --target thumbv7em-none-eabihf --locked",
        "RUSTFLAGS=\"--cfg shuyi_playdate_sdk\" cargo build -p shuyi-playdate --target thumbv7em-none-eabihf --locked",
        "cargo build -p hello-grid --lib --no-default-features --features playdate --target thumbv7em-none-eabihf --locked",
        "cargo xtask check-no-std",
        "RUSTFLAGS=\"-D warnings -A unused-imports\" cargo test -p shuyi-core -p shuyi-ui -p shuyi-platform -p shuyi-sound --no-default-features --locked --no-fail-fast",
    ]),
    gate("modules", "modules", CODE).steps(&[
        "cargo xtask check-modules",
        "cargo build -p shuyi --locked",
        "cargo test -p shuyi --all-features --locked",
    ]),
    gate("wasm", "wasm", CODE).steps(&[
        "examples/hello-grid: cargo shuyi check --target all",
        "cargo clippy --target wasm32-unknown-unknown -p shuyi-app --features capture --lib --examples --locked -- -D warnings",
        "cargo clippy --target wasm32-unknown-unknown -p shuyi-script --lib --locked -- -D warnings",
        "examples/hello-grid: cargo shuyi web --release",
    ]),
    gate("templates (grid)", "templates", CODE)
        .fresh(&["target/gates/demo-grid"])
        .steps(&[
            "cargo run -q -p shuyi-cli --locked -- new target/gates/demo-grid --template grid --engine-path {root}",
            "target/gates/demo-grid: cargo fmt --all --check",
            "target/gates/demo-grid: CARGO_TARGET_DIR={root}/target/gates/games cargo clippy --workspace --all-targets -- -D warnings",
            "target/gates/demo-grid: CARGO_TARGET_DIR={root}/target/gates/games cargo test --workspace",
            "target/gates/demo-grid: CARGO_TARGET_DIR={root}/target/gates/games cargo shuyi check",
            "target/gates/demo-grid: CARGO_TARGET_DIR={root}/target/gates/games cargo shuyi check --target all",
            "target/gates/demo-grid: CARGO_TARGET_DIR={root}/target/gates/games cargo shuyi check-keys",
            "target/gates/demo-grid: CARGO_TARGET_DIR={root}/target/gates/games cargo shuyi lint",
        ]),
    gate("templates (blank)", "templates", CODE)
        .fresh(&["target/gates/demo-blank"])
        .steps(&[
            "cargo run -q -p shuyi-cli --locked -- new target/gates/demo-blank --template blank --engine-path {root}",
            "target/gates/demo-blank: cargo fmt --all --check",
            "target/gates/demo-blank: CARGO_TARGET_DIR={root}/target/gates/games cargo clippy --workspace --all-targets -- -D warnings",
            "target/gates/demo-blank: CARGO_TARGET_DIR={root}/target/gates/games cargo test --workspace",
            "target/gates/demo-blank: CARGO_TARGET_DIR={root}/target/gates/games cargo shuyi check",
            "target/gates/demo-blank: CARGO_TARGET_DIR={root}/target/gates/games cargo shuyi check --target all",
            "target/gates/demo-blank: CARGO_TARGET_DIR={root}/target/gates/games cargo shuyi check-keys",
            "target/gates/demo-blank: CARGO_TARGET_DIR={root}/target/gates/games cargo shuyi lint",
        ]),
    // CI's job times the builds too; their budget is a runner's, so a
    // local run holds the crates' count alone.
    gate("compile-times (crates)", "compile-times", CODE)
        .steps(&["cargo xtask compile-times --budget tools/compile-budget.ron --count-only"]),
    gate("skill examples", "skills", CODE).steps(&["cargo xtask skill-examples"]),
    gate("tutorial-tries", "tutorial-tries", CODE)
        .steps(&["cargo xtask tutorial-tries --check"]),
    // CI's alone, with why.
    gate("tutorial", "tutorial", CODE).ci_only(
        "builds and tests the ten steps of the tutorial's game, each from its own checked-in lock, `--locked`",
    ),
    gate("determinism", "determinism", CODE)
        .ci_only("the tests on wasm32 under wasmtime; natively, `test` runs the golden tables"),
    gate("export", "export", CODE)
        .ci_only("exports on each OS's runner and the Steam runtime"),
    gate("render", "render", CODE).ci_only("needs Xvfb and Mesa"),
    gate("web-render", "web-render", CODE)
        .ci_only("needs headless Chromium and Playwright"),
    gate("bench", "bench", CODE).ci_only("counts instructions with Valgrind"),
    gate("coverage", "coverage", CODE)
        .ci_only("cargo llvm-cov and the SonarCloud scan"),
    gate("zizmor", "zizmor", CODE).ci_only("the zizmor action audits the workflows"),
    gate("api", "api", CRATES).ci_only("needs a nightly and two tools"),
    gate("sound parity", "sound-parity", Filter::Output("sound"))
        .ci_only("records the backend's output from PulseAudio and Chromium"),
    gate("sound parity (Windows)", "sound-parity-windows", Filter::Output("sound"))
        .ci_only("records the backend's output on a Windows runner"),
    gate("sound give-up", "sound-give-up", Filter::Output("sound"))
        .ci_only("takes the sound replacement out of a checkout and builds what is left"),
    gate("playtest", "playtest", Filter::Always).ci_only("a bot plays Hello, Grid"),
];

/// `gates`' options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// `--base <rev>`: the diff is from where HEAD left it.
    pub base: Option<String>,
    /// `--fast`: only the gates that take seconds.
    pub fast: bool,
    /// `--list`: say what would run, and run nothing.
    pub list: bool,
}

impl Options {
    /// The options `args` give, or `None` on anything else.
    pub fn parse(args: &[String]) -> Option<Self> {
        let mut options = Self::default();
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            match arg.as_str() {
                "--base" => options.base = Some(rest.next()?.clone()),
                "--fast" => options.fast = true,
                "--list" => options.list = true,
                _ => return None,
            }
        }
        Some(options)
    }
}

/// What a diff makes of a gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// It runs.
    Run,
    /// It doesn't apply, and why.
    NotNeeded(String),
    /// CI alone runs it, and why.
    CiOnly(&'static str),
}

/// The value of each of the classifier's outputs for `paths`, from the
/// `case` arms of its script (`changes.yml`'s `classify` step), as on a
/// pull request: an output is `true` when the first arm a path matches in
/// some `case` block sets it.
pub fn classify(script: &str, paths: &[String]) -> Result<BTreeMap<String, bool>, String> {
    let blocks = case_blocks(script);
    if blocks.is_empty() {
        return Err("changes.yml: the classifier has no `case \"$path\" in` block".into());
    }
    let mut outputs: BTreeMap<String, bool> = script
        .lines()
        .filter_map(|line| line.trim().strip_suffix("=false"))
        .filter(|name| {
            !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
        .map(|name| (name.to_owned(), false))
        .collect();
    if outputs.is_empty() {
        return Err("changes.yml: the classifier sets no `<output>=false`".into());
    }
    for path in paths {
        for block in &blocks {
            let Some(body) = block
                .iter()
                .find(|(patterns, _)| patterns.iter().any(|p| case_match(p, path)))
                .map(|(_, body)| body)
            else {
                continue;
            };
            for (name, value) in &mut outputs {
                if body.contains(&format!("{name}=true")) {
                    *value = true;
                }
            }
        }
    }
    Ok(outputs)
}

/// An arm of a `case`: its patterns and its body.
type Arm = (Vec<String>, String);

/// The `case "$path" in … esac` blocks of `script`, each a list of arms.
fn case_blocks(script: &str) -> Vec<Vec<Arm>> {
    let mut blocks = Vec::new();
    let mut current: Option<Vec<Arm>> = None;
    for line in script.lines().map(str::trim) {
        if line == "case \"$path\" in" {
            current = Some(Vec::new());
        } else if line == "esac" {
            if let Some(block) = current.take() {
                blocks.push(block);
            }
        } else if let Some(block) = &mut current
            && let Some((patterns, body)) = line.split_once(')')
        {
            let patterns = patterns.split('|').map(|p| p.trim().to_owned()).collect();
            block.push((patterns, body.trim().to_owned()));
        }
    }
    blocks
}

/// Whether `text` matches the shell `case` pattern `pattern`: `*` is any
/// text, `/` included, `?` any one character.
fn case_match(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    // The classic two-pointer glob match, backtracking to the last `*`.
    let (mut p, mut t) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while t < text.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == text[t]) {
            p += 1;
            t += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some((p, t));
            p += 1;
        } else if let Some((sp, st)) = star {
            p = sp + 1;
            t = st + 1;
            star = Some((sp, st + 1));
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|&c| c == '*')
}

/// The classifier's script in `changes.yml`'s text: the `run` of the
/// `detect` job's step whose `id` is `classify`.
pub fn classifier(changes_yml: &str) -> Result<String, String> {
    let yaml: Value =
        serde_norway::from_str(changes_yml).map_err(|e| format!("changes.yml: {e}"))?;
    yaml["jobs"]["detect"]["steps"]
        .as_sequence()
        .into_iter()
        .flatten()
        .find(|step| step["id"].as_str() == Some("classify"))
        .and_then(|step| step["run"].as_str())
        .map(str::to_owned)
        .ok_or_else(|| "changes.yml: no `classify` step with a `run` in the `detect` job".into())
}

/// Each job of `ci.yml`'s text, with the output of `changes.yml` its `if:`
/// reads, if any.
#[cfg(test)]
pub fn ci_jobs(ci_yml: &str) -> Result<BTreeMap<String, Option<String>>, String> {
    let yaml: Value = serde_norway::from_str(ci_yml).map_err(|e| format!("ci.yml: {e}"))?;
    let jobs = yaml["jobs"]
        .as_mapping()
        .ok_or("ci.yml: no `jobs` mapping")?;
    let mut found = BTreeMap::new();
    for (name, job) in jobs {
        let name = name.as_str().ok_or("ci.yml: a job's name isn't text")?;
        let condition = job["if"].as_str().unwrap_or("");
        let output = condition
            .split_once("needs.changes.outputs.")
            .map(|(_, rest)| {
                rest.chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect::<String>()
            });
        found.insert(name.to_owned(), output);
    }
    Ok(found)
}

/// What `paths` (with the classifier's `outputs`) make of `gate`.
pub fn plan(gate: &Gate, outputs: &BTreeMap<String, bool>, paths: &[String], fast: bool) -> Plan {
    if let Some(why) = gate.ci_only {
        return Plan::CiOnly(why);
    }
    if let Some(output) = gate.filter.output()
        && !outputs.get(output).copied().unwrap_or(true)
    {
        return Plan::NotNeeded(format!("CI's `{output}` filter is false for this diff"));
    }
    if let Local::Under(prefixes) = gate.local
        && !paths
            .iter()
            .any(|p| prefixes.iter().any(|prefix| p.starts_with(prefix)))
    {
        return Plan::NotNeeded(format!("nothing changed under {}", prefixes.join(", ")));
    }
    if let Local::When(what, test) = gate.local
        && !paths.iter().any(|p| test(p))
    {
        return Plan::NotNeeded(format!("nothing changed: {what}"));
    }
    if fast {
        match gate.fast {
            Fast::No => return Plan::NotNeeded("not a fast gate (--fast)".into()),
            Fast::IfChanged(suffix) if !paths.iter().any(|p| p.ends_with(suffix)) => {
                return Plan::NotNeeded(format!("no `{suffix}` file changed (--fast)"));
            }
            Fast::Yes | Fast::IfChanged(_) => {}
        }
    }
    Plan::Run
}

/// One command of a gate, parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepCommand {
    /// The folder it runs in, from the root.
    pub dir: Option<String>,
    /// Its `NAME=value` words.
    pub env: Vec<(String, String)>,
    /// The program and its arguments.
    pub words: Vec<String>,
    /// Whether its standard output is dropped (`>null` at the end).
    pub quiet: bool,
}

/// Parses a step of [`Gate::steps`], with `{root}` as `root` and
/// `{pr-branch}` as `pr_branch`'s words.
pub fn parse_step(step: &str, root: &str, pr_branch: &[String]) -> StepCommand {
    let (dir, line) = match step.split_once(": ") {
        Some((dir, line)) if !dir.contains(' ') => (Some(dir.to_owned()), line),
        _ => (None, step),
    };
    let mut env = Vec::new();
    let mut words = Vec::new();
    let mut quiet = false;
    for word in split_words(line) {
        let word = word.replace("{root}", root);
        if word == ">null" {
            quiet = true;
        } else if word == "{pr-branch}" {
            words.extend(pr_branch.iter().cloned());
        } else if words.is_empty()
            && let Some((name, value)) = word.split_once('=')
            && !name.is_empty()
            && name.chars().all(|c| c.is_ascii_uppercase() || c == '_')
        {
            env.push((name.to_owned(), value.to_owned()));
        } else {
            words.push(word);
        }
    }
    StepCommand {
        dir,
        env,
        words,
        quiet,
    }
}

/// `line`'s words, split at spaces outside double quotes, the quotes
/// dropped.
fn split_words(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    let mut any = false;
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            ' ' if !quoted => {
                if any {
                    words.push(std::mem::take(&mut word));
                    any = false;
                }
            }
            _ => {
                word.push(c);
                any = true;
            }
        }
    }
    if any {
        words.push(word);
    }
    words
}

/// The ticket-lint's `--pr-branch` words for `branch`: only a branch that
/// names a ticket (`tNNNN-…`), as another names none and would fail.
pub fn pr_branch_words(branch: &str) -> Vec<String> {
    let names_a_ticket = branch.len() > 6
        && branch.starts_with('t')
        && branch[1..5].chars().all(|c| c.is_ascii_digit())
        && branch[5..].starts_with('-');
    if names_a_ticket {
        vec!["--pr-branch".to_owned(), branch.to_owned()]
    } else {
        Vec::new()
    }
}

/// What a run of a gate gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// It passed, in this time.
    Passed(Duration),
    /// A step failed, in this time.
    Failed(Duration, String),
    /// An earlier gate failed first.
    NotRun,
}

/// The facts the summary names.
pub struct Header<'a> {
    pub head: &'a str,
    pub base: &'a str,
    pub merge_base: &'a str,
    pub changed: usize,
    pub dirty: bool,
    pub fast: bool,
}

/// The summary block, Markdown for the PR's body.
pub fn summary(header: &Header, rows: &[(&Gate, Plan, Option<Outcome>)]) -> String {
    let mut out = String::new();
    let command = if header.fast {
        "cargo xtask gates --fast"
    } else {
        "cargo xtask gates"
    };
    let _ = writeln!(
        out,
        "**Local gates** (`{command}`) at `{}`, {} changed file(s) against `{}` (`{}`).",
        header.head, header.changed, header.base, header.merge_base
    );
    if header.dirty {
        let _ = writeln!(
            out,
            "The working tree differed from `HEAD`: the gates ran on uncommitted changes."
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "| Gate | CI job | Result |");
    let _ = writeln!(out, "| --- | --- | --- |");
    for (gate, plan, outcome) in rows {
        let result = match (plan, outcome) {
            (Plan::CiOnly(why), _) => format!("CI only: {why}"),
            (Plan::NotNeeded(why), _) => format!("not needed: {why}"),
            (Plan::Run, None) => "would run".to_owned(),
            (Plan::Run, Some(Outcome::Passed(time))) => format!("passed ({})", seconds(*time)),
            (Plan::Run, Some(Outcome::Failed(time, step))) => {
                format!("**failed** ({}): `{step}`", seconds(*time))
            }
            (Plan::Run, Some(Outcome::NotRun)) => "not run: an earlier gate failed".to_owned(),
        };
        let _ = writeln!(out, "| {} | `{}` | {result} |", gate.name, gate.job);
    }
    out
}

fn seconds(time: Duration) -> String {
    format!("{} s", time.as_secs())
}

/// Runs `gates`: what the diff needs, then each in order until one fails.
/// The summary goes to stdout; the gates' own output to stderr.
pub fn run(root: &Path, options: &Options) -> Result<(String, bool), String> {
    let base = match &options.base {
        Some(base) => base.clone(),
        None if git(root, &["rev-parse", "--verify", "-q", "origin/main"]).is_ok() => {
            "origin/main".to_owned()
        }
        None => "main".to_owned(),
    };
    let head = git(root, &["rev-parse", "--short=12", "HEAD"])?;
    let merge_base = git(root, &["merge-base", &base, "HEAD"])?;
    let mut paths = lines(&git(
        root,
        &["diff", "--name-only", "--no-renames", &merge_base, "HEAD"],
    )?);
    let uncommitted = lines(&git(
        root,
        &["diff", "--name-only", "--no-renames", "HEAD"],
    )?);
    let untracked = lines(&git(root, &["ls-files", "--others", "--exclude-standard"])?);
    let dirty = !uncommitted.is_empty() || !untracked.is_empty();
    paths.extend(uncommitted);
    paths.extend(untracked);
    paths.sort();
    paths.dedup();

    let changes_yml = read(root, ".github/workflows/changes.yml")?;
    let outputs = classify(&classifier(&changes_yml)?, &paths)?;
    let branch = git(root, &["branch", "--show-current"]).unwrap_or_default();
    let pr_branch = pr_branch_words(&branch);
    let root_text = root.display().to_string().replace('\\', "/");

    // `--fast` shows only the fast gates: the others are the full run's.
    let mut rows: Vec<(&Gate, Plan, Option<Outcome>)> = GATES
        .iter()
        .filter(|gate| !options.fast || gate.fast != Fast::No)
        .map(|gate| (gate, plan(gate, &outputs, &paths, options.fast), None))
        .collect();
    let mut failed = false;
    if !options.list {
        for (gate, plan, outcome) in &mut rows {
            if *plan != Plan::Run {
                continue;
            }
            if failed {
                *outcome = Some(Outcome::NotRun);
                continue;
            }
            eprintln!("== gate: {}", gate.name);
            let ran = run_gate(root, gate, &root_text, &pr_branch);
            failed = matches!(ran, Outcome::Failed(..));
            *outcome = Some(ran);
        }
    }
    let short_base = merge_base.get(..12).unwrap_or(&merge_base);
    let header = Header {
        head: &head,
        base: &base,
        merge_base: short_base,
        changed: paths.len(),
        dirty,
        fast: options.fast,
    };
    Ok((summary(&header, &rows), !failed))
}

/// Runs each step of `gate` until one fails.
fn run_gate(root: &Path, gate: &Gate, root_text: &str, pr_branch: &[String]) -> Outcome {
    let start = Instant::now();
    for dir in gate.fresh {
        let dir = root.join(dir);
        if dir.exists()
            && let Err(e) = std::fs::remove_dir_all(&dir)
        {
            return Outcome::Failed(start.elapsed(), format!("remove {}: {e}", dir.display()));
        }
    }
    for step in gate.steps {
        let command = parse_step(step, root_text, pr_branch);
        let shown = command.words.join(" ");
        eprintln!("$ {step}");
        let status =
            spawn(root, &command).and_then(|mut child| child.wait().map_err(|e| e.to_string()));
        match status {
            Ok(status) if status.success() => {}
            Ok(status) => {
                let hint = gate.hint.map_or_else(String::new, |h| format!("; {h}"));
                return Outcome::Failed(start.elapsed(), format!("{step} ({status}){hint}"));
            }
            Err(e) => return Outcome::Failed(start.elapsed(), format!("{shown}: {e}")),
        }
    }
    Outcome::Passed(start.elapsed())
}

/// Starts `command`: `cargo xtask` as this program, `bash` as Git's on
/// Windows, its output on stderr so stdout keeps the summary.
fn spawn(root: &Path, command: &StepCommand) -> Result<std::process::Child, String> {
    let (program, args) = command.words.split_first().ok_or("an empty step")?;
    let mut process = if program == "cargo" && args.first().is_some_and(|a| a == "xtask") {
        let mut process = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
        process.args(&args[1..]);
        process
    } else {
        let mut process = Command::new(program_path(program));
        process.args(args);
        process
    };
    let dir = command
        .dir
        .as_ref()
        .map_or_else(|| root.to_path_buf(), |dir| root.join(dir));
    process.current_dir(dir);
    for (name, value) in &command.env {
        process.env(name, value);
    }
    let stdout = if command.quiet {
        Stdio::null()
    } else {
        Stdio::from(std::io::stderr())
    };
    process
        .stdout(stdout)
        .spawn()
        .map_err(|e| format!("did not start ({e})"))
}

/// Where `program` is: on Windows, `bash` is Git's (beside its `git`), as
/// the one in System32 is WSL's.
fn program_path(program: &str) -> PathBuf {
    if cfg!(windows) && program == "bash" {
        // `git --exec-path` is …/Git/mingw64/libexec/git-core.
        if let Ok(exec) = git(Path::new("."), &["--exec-path"]) {
            let bash = Path::new(&exec).join("../../../bin/bash.exe");
            if bash.exists() {
                return bash;
            }
        }
    }
    PathBuf::from(program)
}

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("git did not run ({e})"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn lines(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

fn read(root: &Path, path: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(path)).map_err(|e| format!("{path}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn outputs_for(paths: &[&str]) -> (BTreeMap<String, bool>, Vec<String>) {
        let script = classifier(&read(&repo(), ".github/workflows/changes.yml").unwrap()).unwrap();
        let paths: Vec<String> = paths.iter().map(|p| (*p).to_owned()).collect();
        (classify(&script, &paths).unwrap(), paths)
    }

    /// The gates that would run for `paths`, by name.
    fn running(paths: &[&str], fast: bool) -> Vec<&'static str> {
        let (outputs, paths) = outputs_for(paths);
        GATES
            .iter()
            .filter(|g| plan(g, &outputs, &paths, fast) == Plan::Run)
            .map(|g| g.name)
            .collect()
    }

    #[test]
    fn a_docs_only_diff_runs_the_tickets_checks_and_not_clippy() {
        let gates = running(&["docs/README.md", "tickets/open/2054-x.md"], false);
        for name in [
            "typos",
            "ticket-lint",
            "tickets' checks",
            "skill loaders",
            "site",
        ] {
            assert!(gates.contains(&name), "{name} missing from {gates:?}");
        }
        for name in ["clippy", "fmt", "test", "templates (grid)"] {
            assert!(!gates.contains(&name), "{name} in {gates:?}");
        }
    }

    #[test]
    fn a_template_change_runs_the_templates_and_the_skill_examples() {
        let gates = running(&["templates/shuyi-default/base/Cargo.toml"], false);
        for name in [
            "templates (grid)",
            "templates (blank)",
            "skill examples",
            "clippy",
        ] {
            assert!(gates.contains(&name), "{name} missing from {gates:?}");
        }
        assert!(!gates.contains(&"site"), "{gates:?}");
    }

    #[test]
    fn the_tutorial_locks_run_for_a_manifest_and_not_for_docs() {
        let gates = running(&["crates/cli/Cargo.toml"], false);
        assert!(gates.contains(&"tutorial locks"), "{gates:?}");
        let gates = running(&["Cargo.lock"], false);
        assert!(gates.contains(&"tutorial locks"), "{gates:?}");
        let gates = running(&["site/tutorial/03-walk/Cargo.lock"], false);
        assert!(gates.contains(&"tutorial locks"), "{gates:?}");
        let gates = running(&["crates/cli/src/main.rs"], false);
        assert!(!gates.contains(&"tutorial locks"), "{gates:?}");
        let gates = running(&["docs/x.md"], false);
        assert!(!gates.contains(&"tutorial locks"), "{gates:?}");
    }

    #[test]
    fn the_tutorial_locks_name_each_step_folder() {
        let steps = GATES
            .iter()
            .find(|g| g.name == "tutorial locks")
            .unwrap()
            .steps;
        let mut folders: Vec<String> = std::fs::read_dir(repo().join("site/tutorial"))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        folders.sort();
        let named: Vec<String> = steps
            .iter()
            .map(|s| {
                s.strip_prefix("site/tutorial/")
                    .and_then(|r| r.split_once(": "))
                    .unwrap()
                    .0
                    .to_owned()
            })
            .collect();
        assert_eq!(named, folders);
    }

    #[test]
    fn a_site_change_runs_the_site_gate() {
        let gates = running(&["site/content/learn/_index.md"], false);
        assert!(gates.contains(&"site"), "{gates:?}");
        let step = GATES.iter().find(|g| g.name == "site").unwrap().steps;
        assert!(step.contains(&"bash site/check-build.sh"));
    }

    #[test]
    fn fast_runs_the_quick_gates_and_the_lint_only_on_rust() {
        assert_eq!(
            running(&["crates/core/src/lib.rs"], true),
            ["fmt", "typos", "ticket-lint", "engine lint"]
        );
        assert_eq!(
            running(&["crates/core/Cargo.toml"], true),
            ["fmt", "typos", "ticket-lint"]
        );
        assert_eq!(running(&["docs/x.md"], true), ["typos", "ticket-lint"]);
    }

    #[test]
    fn the_classifier_reads_each_output() {
        let (outputs, _) = outputs_for(&["docs/x.md", "AGENTS.md", ".claude/skills/a/SKILL.md"]);
        assert_eq!(outputs.values().filter(|v| **v).count(), 0, "{outputs:?}");
        let (outputs, _) = outputs_for(&["README.md"]);
        assert!(outputs["code"]);
        assert!(!outputs["crates"]);
        let (outputs, _) = outputs_for(&["crates/core/src/lib.rs"]);
        assert_eq!((outputs["code"], outputs["crates"]), (true, true));
        let (outputs, _) = outputs_for(&["scripts/git-hooks/pre-push"]);
        assert_eq!((outputs["code"], outputs["crates"]), (true, false));
    }

    #[test]
    fn the_first_arm_a_path_matches_decides_each_block() {
        let script = "code=false
narrow=false
while read path; do
  case \"$path\" in
    lib/a*|x.toml) narrow=true ;;
  esac
  case \"$path\" in
    lib/*) code=true ;;
    *.md|docs/*) echo docs ;;
    *) code=true ;;
  esac
done
";
        let classify = |paths: &[&str]| {
            let paths: Vec<String> = paths.iter().map(|p| (*p).to_owned()).collect();
            let outputs = classify(script, &paths).unwrap();
            (outputs["code"], outputs["narrow"])
        };
        assert_eq!(classify(&["docs/a.md", "b.md"]), (false, false));
        assert_eq!(classify(&["lib/a.md"]), (true, true));
        assert_eq!(classify(&["x.toml"]), (true, true));
        assert_eq!(classify(&["lib/b.rs"]), (true, false));
        assert!(super::classify("echo no case", &[]).is_err());
    }

    #[test]
    fn a_case_pattern_matches_as_the_shell_does() {
        assert!(case_match("crates/*", "crates/a/b.rs"));
        assert!(case_match("*.md", "docs/a.md"));
        assert!(case_match(
            "crates/app/src/audio*",
            "crates/app/src/audio/mod.rs"
        ));
        assert!(case_match("Cargo.toml", "Cargo.toml"));
        assert!(!case_match("Cargo.toml", "crates/a/Cargo.toml"));
        assert!(!case_match("*.md", "docs/a.mdx"));
        assert!(case_match("a?c", "abc"));
        assert!(case_match("*", ""));
    }

    /// Every job of `ci.yml` is a gate, or a check of other jobs, and each
    /// gate's filter is its job's.
    #[test]
    fn every_ci_job_is_a_gate_with_its_filter() {
        let jobs = ci_jobs(&read(&repo(), ".github/workflows/ci.yml").unwrap()).unwrap();
        let (outputs, _) = outputs_for(&[]);
        let mut errors = Vec::new();
        for (job, output) in &jobs {
            if let Some(output) = output
                && !outputs.contains_key(output)
            {
                errors.push(format!(
                    "ci.yml's job `{job}` reads `{output}`, no output of changes.yml"
                ));
            }
            if NOT_GATES.contains(&job.as_str()) {
                continue;
            }
            if !GATES.iter().any(|g| g.job == job) {
                errors.push(format!("ci.yml's job `{job}` is in no gate of gates.rs"));
            }
            for gate in GATES.iter().filter(|g| g.job == job) {
                if gate.filter.output() != output.as_deref() {
                    errors.push(format!(
                        "gate `{}`: filter {:?}, but its job `{job}` reads {output:?}",
                        gate.name, gate.filter
                    ));
                }
            }
        }
        for gate in GATES {
            if !jobs.contains_key(gate.job) {
                errors.push(format!(
                    "gate `{}` names job `{}`, not in ci.yml",
                    gate.name, gate.job
                ));
            }
            assert!(
                gate.ci_only.is_some() == gate.steps.is_empty(),
                "{}: steps or ci_only",
                gate.name
            );
        }
        assert!(errors.is_empty(), "{errors:#?}");
    }

    #[test]
    fn a_new_filtered_job_fails_the_coverage() {
        let ci = "jobs:\n  fmt:\n    if: needs.changes.outputs.code == 'true'\n  shiny:\n    needs: changes\n    if: needs.changes.outputs.sound == 'true'\n";
        let jobs = ci_jobs(ci).unwrap();
        assert_eq!(jobs["shiny"].as_deref(), Some("sound"));
        assert_eq!(jobs["fmt"].as_deref(), Some("code"));
        assert!(!GATES.iter().any(|g| g.job == "shiny"));
        let unfiltered = ci_jobs("jobs:\n  x:\n    runs-on: ubuntu-latest\n").unwrap();
        assert_eq!(unfiltered["x"], None);
    }

    #[test]
    fn a_step_parses_its_folder_variables_and_quotes() {
        let step = parse_step(
            "examples/x: RUSTFLAGS=\"-D warnings\" cargo build --dir {root}/t {pr-branch}",
            "D:/repo",
            &["--pr-branch".into(), "t2054-x".into()],
        );
        assert_eq!(step.dir.as_deref(), Some("examples/x"));
        assert_eq!(
            step.env,
            [("RUSTFLAGS".to_owned(), "-D warnings".to_owned())]
        );
        assert_eq!(
            step.words,
            [
                "cargo",
                "build",
                "--dir",
                "D:/repo/t",
                "--pr-branch",
                "t2054-x"
            ]
        );
        let quiet = parse_step("cargo xtask changelog >null", "r", &[]);
        assert!(quiet.quiet && quiet.dir.is_none());
        assert_eq!(quiet.words, ["cargo", "xtask", "changelog"]);
    }

    #[test]
    fn only_a_ticket_branch_is_given_to_the_ticket_lint() {
        assert_eq!(
            pr_branch_words("t2054-gates"),
            ["--pr-branch", "t2054-gates"]
        );
        assert_eq!(pr_branch_words("main"), Vec::<String>::new());
        assert_eq!(pr_branch_words("claude/x-e86c"), Vec::<String>::new());
        assert_eq!(pr_branch_words("t20-x"), Vec::<String>::new());
    }

    #[test]
    fn the_summary_names_head_each_result_and_the_ci_only_gates() {
        let header = Header {
            head: "abc123",
            base: "origin/main",
            merge_base: "def456",
            changed: 2,
            dirty: false,
            fast: false,
        };
        let rows = [
            (
                &GATES[0],
                Plan::Run,
                Some(Outcome::Passed(Duration::from_secs(3))),
            ),
            (
                &GATES[1],
                Plan::Run,
                Some(Outcome::Failed(
                    Duration::from_secs(1),
                    "typos (exit code: 2)".into(),
                )),
            ),
            (&GATES[2], Plan::Run, Some(Outcome::NotRun)),
            (&GATES[3], Plan::NotNeeded("why".into()), None),
            (&GATES[4], Plan::CiOnly("needs Xvfb"), None),
        ];
        let text = summary(&header, &rows);
        assert!(text.contains("at `abc123`"), "{text}");
        assert!(text.contains("| fmt | `fmt` | passed (3 s) |"), "{text}");
        assert!(
            text.contains("| typos | `tickets` | **failed** (1 s): `typos (exit code: 2)` |"),
            "{text}"
        );
        assert!(
            text.contains("| ticket-lint | `tickets` | not run: an earlier gate failed |"),
            "{text}"
        );
        assert!(
            text.contains("| engine lint | `test` | not needed: why |"),
            "{text}"
        );
        assert!(text.contains("CI only: needs Xvfb"), "{text}");
    }
}
