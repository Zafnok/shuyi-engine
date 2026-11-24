//! A task of the eval, `evals/tasks/<id>.ron`: the words an agent gets, the
//! template its game starts from, and the checks its game must pass, each
//! made of the worked-example runner's steps ([`crate::script`]).

use std::path::Path;

use serde::Deserialize;

use crate::script::{Expect, Step};

/// The `cargo` commands of `Gates`: the gates the template's `AGENTS.md`
/// lists (*The gates*), less the two `Check` and `Targets` are.
pub const GATES: [&str; 5] = [
    "fmt --all --check",
    "clippy --workspace --all-targets -- -D warnings",
    "test --workspace",
    "shuyi check-keys",
    "shuyi lint",
];

/// A task.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    /// Its id, its file's name.
    pub id: String,
    /// The words the agent gets, verbatim, and nothing else.
    pub prompt: String,
    /// The template the agent's game starts from, a short name of `shuyi
    /// new --template` (`grid`): the base its lines are counted over.
    pub template: String,
    /// The game's name, `shuyi new --name`'s, for the graded game and its
    /// base alike: the names the template fills in count as no lines.
    pub name: String,
    /// The end goal's hour (2525 holds an agent to it).
    pub time_box_minutes: u32,
    /// The target ids `Targets` checks (`desktop`, `web`).
    pub targets: Vec<String>,
    /// What the game must pass.
    pub checks: Vec<Kind>,
    /// The solution CI grades.
    pub reference: Reference,
    /// Tickets it waits for: a task with any is listed, never graded.
    #[serde(default)]
    pub waits_for: Vec<String>,
    /// The parts of the end goal no check covers yet, each with the
    /// tickets that would.
    #[serde(default)]
    pub unmeasured: Vec<(String, Vec<String>)>,
}

/// A step of a check's input: an action by its name, or a wait. Never a
/// key: a grader can't know which keys an agent's game bound.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum Input {
    /// The action of that name, pressed (`--act`).
    Act(String),
    /// That many seconds with no input (`--wait`).
    Wait(f64),
}

/// What a game must pass. Each reads only the contract's answers, or a
/// command's exit, and what the prompt asks the game to expose.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum Kind {
    /// Each of [`GATES`] exits 0: one verdict a command.
    Gates,
    /// `cargo shuyi check --json` passes.
    Check,
    /// `cargo shuyi check --target <id> --json` passes for each of the
    /// task's targets.
    Targets,
    /// `cargo shuyi export web --json` passes, and the folder it made holds
    /// `index.html` and one `.wasm`.
    Web,
    /// `cargo shuyi export <target> --json` passes, and its package is
    /// there.
    Export(String),
    /// `cargo shuyi run --json` with the steps shows something other than
    /// as many frames with none.
    Responds {
        /// The input.
        steps: Vec<Input>,
    },
    /// `cargo shuyi playtest --tries 1 --json` keeps a record of its try,
    /// and `cargo shuyi replay` plays it twice to equal hashes.
    Replay,
    /// `cargo shuyi scene --json` after the steps has a scene, and each
    /// `(pointer, equals)` holds of its data: a value is there, equal to
    /// `equals` when one is given (a string without its quotes, anything
    /// else as JSON).
    Scene {
        /// The input.
        steps: Vec<Input>,
        /// What its data holds, by JSON pointer (`/turn`).
        has: Vec<(String, Option<String>)>,
    },
}

/// A task's solution.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum Reference {
    /// A folder, from the repository's root, of only the files that differ
    /// from a fresh `shuyi new`, laid over one.
    Overlay(String),
    /// A whole project, from the repository's root, graded where it is.
    Project(String),
    /// None yet.
    None,
}

/// What a check's steps leave for the grader to look at after them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum After {
    /// Nothing.
    Nothing,
    /// The web export's folder: its answer's `result` is in this file.
    WebFolder(String),
    /// An export's package: its answer's `result` is in this file.
    Package(String),
}

/// A check as the grader runs it: its name (`Scene 2`), its steps, and
/// what to look at after them.
#[derive(Debug, Clone, PartialEq)]
pub struct Planned {
    /// Its kind and place in the task, as people read it.
    pub name: String,
    /// The runner's steps, in order; the first that fails fails it.
    pub steps: Vec<Step>,
    /// What to look at once they passed.
    pub after: After,
}

impl Task {
    /// The task in file `path`, or why it can't be read.
    pub fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        ron::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Its checks as the grader runs them, any file a step saves in the
    /// folder `scratch` (absolute: never the game's).
    pub fn plan(&self, scratch: &Path) -> Vec<Planned> {
        let mut planned = Vec::new();
        let mut scenes = 0;
        let saved = |file: &str| scratch.join(file).to_string_lossy().into_owned();
        for check in &self.checks {
            match check {
                Kind::Gates => {
                    for (i, args) in GATES.iter().enumerate() {
                        let name = format!("Gates {}: cargo {args}", i + 1);
                        let step = Step::Cargo {
                            args: owned(&args.split(' ').collect::<Vec<_>>()),
                            ok: true,
                        };
                        planned.push(plan(name, vec![step], After::Nothing));
                    }
                }
                Kind::Check => {
                    let step = shuyi(owned(&["check"]), vec![]);
                    planned.push(plan("Check", vec![step], After::Nothing));
                }
                Kind::Targets => {
                    let steps = (self.targets.iter())
                        .map(|id| shuyi(owned(&["check", "--target", id]), vec![]))
                        .collect();
                    planned.push(plan("Targets", steps, After::Nothing));
                }
                Kind::Web | Kind::Export(_) => {
                    let target = if let Kind::Export(t) = check {
                        t
                    } else {
                        "web"
                    };
                    let file = saved(&format!("export-{target}.json"));
                    let step = saving(owned(&["export", target]), "/result", &file);
                    let (name, after) = match check {
                        Kind::Web => ("Web".to_owned(), After::WebFolder(file)),
                        _ => (format!("Export {target}"), After::Package(file)),
                    };
                    planned.push(plan(name, vec![step], after));
                }
                Kind::Responds { steps } => {
                    let step = Step::Responds {
                        args: input_args(steps),
                    };
                    planned.push(plan("Responds", vec![step], After::Nothing));
                }
                Kind::Replay => {
                    let record = saved("record.ron");
                    let tries = owned(&["playtest", "--tries", "1"]);
                    let playtest = saving(tries, "/result/0/entrants/0/tries/0/record", &record);
                    let same = Expect::Is("/result/first_difference".to_owned(), "null".to_owned());
                    let replay = shuyi(vec!["replay".to_owned(), record], vec![same]);
                    planned.push(plan("Replay", vec![playtest, replay], After::Nothing));
                }
                Kind::Scene { steps, has } => {
                    scenes += 1;
                    let mut args = vec!["scene".to_owned()];
                    args.extend(input_args(steps));
                    let mut expect = vec![Expect::Has("/result/scene".to_owned())];
                    expect.extend(has.iter().map(|(pointer, equals)| {
                        let at = format!("/result/scene/data{pointer}");
                        match equals {
                            Some(want) => Expect::Is(at, want.clone()),
                            None => Expect::Exists(at),
                        }
                    }));
                    let name = format!("Scene {scenes}");
                    planned.push(plan(name, vec![shuyi(args, expect)], After::Nothing));
                }
            }
        }
        planned
    }
}

/// `args` as owned strings.
fn owned(args: &[&str]) -> Vec<String> {
    args.iter().map(|a| (*a).to_owned()).collect()
}

/// A contract step that must pass with `expect`.
fn shuyi(args: Vec<String>, expect: Vec<Expect>) -> Step {
    Step::Shuyi {
        args,
        ok: true,
        expect,
        save: None,
    }
}

/// A contract step that must pass and saves the value at `pointer` to the
/// absolute path `file`.
fn saving(args: Vec<String>, pointer: &str, file: &str) -> Step {
    Step::Shuyi {
        args,
        ok: true,
        expect: Vec::new(),
        save: Some((pointer.to_owned(), file.to_owned())),
    }
}

/// A check named `name`, its steps and what to look at after them.
fn plan(name: impl Into<String>, steps: Vec<Step>, after: After) -> Planned {
    Planned {
        name: name.into(),
        steps,
        after,
    }
}

/// The contract's arguments for `steps`: `--act <name>` and `--wait
/// <seconds>`, in order.
pub fn input_args(steps: &[Input]) -> Vec<String> {
    steps
        .iter()
        .flat_map(|step| match step {
            Input::Act(name) => ["--act".to_owned(), name.clone()],
            Input::Wait(seconds) => ["--wait".to_owned(), seconds.to_string()],
        })
        .collect()
}
