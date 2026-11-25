//! The new-game wizard's questions (ADR-0013 §1, §2, §6): what `shuyi new`
//! asks, as data ([`questions`]), listed for an agent by `shuyi new <dir>
//! --questions --json` ([`listed`]), and the one command that makes the
//! same game again with no question ([`command`]).
//!
//! Each concern of the catalog (`docs/modules.ron`) that `shuyi new` asks,
//! all but `time`, has a flag of its id ([`concern_flag`]: `--perspective`,
//! `--bodies`…), and `--targets` takes the engine's target profiles
//! ([`engine_targets`]), the template's always-on parts' targets by
//! default ([`default_targets`]). Asking them in a terminal, as lists to
//! pick from, is ticket 3423's.

use std::fmt::Write as _;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Engine, NewInput, PRIVATE_FILES, answers_of, default_id, engine_root, name_of};
use crate::catalog::{Catalog, Concern, Concerns, Status};
use crate::contract::codes::{self, Code};
use crate::contract::{Answer, Op};
use crate::template::{self, Answers, EXAMPLES, Pick, TARGETS, Template};

/// A target an engine profile adds, as `--targets` takes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// The profile's id (`playdate`).
    pub id: String,
    /// Whether the engine runs on it yet: a planned one is never picked.
    pub ready: bool,
}

/// The targets `template` ships every game on: its parts' that always
/// apply.
pub fn default_targets(template: &Template) -> Vec<String> {
    let mut targets: Vec<String> = Vec::new();
    for part in template
        .parts
        .iter()
        .filter(|p| p.when == template::When::Always)
    {
        for target in &part.targets {
            if !targets.contains(target) {
                targets.push(target.clone());
            }
        }
    }
    targets
}

/// The engine's target profiles, by id.
pub fn engine_targets() -> Result<Vec<Target>, String> {
    let profiles = crate::contract::target::engine_profiles().map_err(|errors| {
        let said: Vec<String> = errors.iter().map(ToString::to_string).collect();
        said.join("; ")
    })?;
    Ok(profiles
        .into_iter()
        .map(|(id, p)| Target {
            id,
            ready: p.profile.status == shuyi_assets::target::Status::Supported,
        })
        .collect())
}

/// The concern `flag` (`--bodies`) answers, if it is one `shuyi new` asks
/// of the engine's catalog as this `shuyi` embeds it.
pub fn concern_flag(flag: &str) -> Option<String> {
    let id = flag.strip_prefix("--")?;
    let concerns = Concerns::embedded().ok()?;
    template::asked(&concerns, id).map(|c| c.id.clone())
}

/// The engine's question of the game's folder.
pub const DIR: &str = "dir";

/// The engine's question of the game's name.
pub const NAME: &str = "name";

/// The engine's question of the game's reverse-DNS id.
pub const ID: &str = "id";

/// The catalog's concern of the perspectives, which the engine always asks.
pub const PERSPECTIVE: &str = "perspective";

/// Who asks a question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
// A set that may grow (ADR-0039, kind 6).
#[non_exhaustive]
pub enum Asker {
    /// The engine: the folder, the name, the id, the perspectives and the
    /// targets.
    Engine,
    /// The catalog, `docs/modules.ron`: one question per concern.
    Catalog,
    /// The template: its own questions.
    Template,
}

/// What a question takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
// A set that may grow (ADR-0039, kind 6).
#[non_exhaustive]
pub enum Takes {
    /// A line of text.
    Text,
    /// One of its answers at most.
    One,
    /// Any number of its answers.
    Many,
}

impl From<Pick> for Takes {
    fn from(pick: Pick) -> Self {
        match pick {
            Pick::One => Takes::One,
            Pick::Many => Takes::Many,
        }
    }
}

/// One answer a question offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An output, read and never built outside this crate: it may grow
// (ADR-0039, kind 3).
#[non_exhaustive]
pub struct Offered {
    /// Its id, as its flag takes it (`top-down-grid`).
    pub id: String,
    /// A few words that say it (`top-down grid`).
    pub text: String,
    /// Whether it is built: only a `ready` answer is picked, and a
    /// `planned` or `parked` one is shown as coming (ADR-0013 §6).
    pub status: Status,
    /// The tickets that build it, if it isn't ready.
    pub tickets: Vec<String>,
}

/// A question `shuyi new` asks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An output, read and never built outside this crate: it may grow
// (ADR-0039, kind 3).
#[non_exhaustive]
pub struct Question {
    /// Its id: the engine's (`dir`, `name`, `id`, `perspective`,
    /// `targets`), a concern's (`bodies`) or the template's
    /// (`private-files`).
    pub id: String,
    /// Who asks it.
    pub asker: Asker,
    /// The words that ask it.
    pub text: String,
    /// What it takes.
    pub takes: Takes,
    /// The answers it offers, every one with its status; none for text.
    pub answers: Vec<Offered>,
    /// What it takes unless another answer is given: the answers given so
    /// far, else its default (for text, the one line).
    pub default: Vec<String>,
    /// The flag that answers it (`--bodies`, `--set examples=`); none for
    /// the folder, `new`'s first word.
    pub flag: Option<String>,
    /// Why it isn't asked, if it isn't: it takes its default.
    pub skipped: Option<String>,
}

/// The questions `shuyi new` asks of a game, for the answers given so far.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An output: it may grow (ADR-0039, kind 3).
#[non_exhaustive]
pub struct Questions {
    /// The template's id (`shuyi-default`).
    pub template: String,
    /// Every question, in the order it is asked.
    pub questions: Vec<Question>,
}

/// What the game is called so far: its folder, its name and its id, each
/// as given or at its default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    /// Its folder, as given.
    pub dir: String,
    /// Its name: `--name`, else its folder's.
    pub name: String,
    /// Its reverse-DNS id: `--id`, else `com.example.<crate name>`.
    pub id: String,
}

/// The template's questions another question answers, each with why it
/// isn't asked itself.
const ANSWERED_ELSEWHERE: [(&str, &str); 1] = [(
    PRIVATE_FILES,
    "the GitHub questions answer it (--github and --private), or --set private-files=yes with \
     --private-url",
)];

/// A text question of the engine's.
fn text_question(id: &str, text: &str, flag: Option<&str>, default: &str) -> Question {
    Question {
        id: id.to_owned(),
        asker: Asker::Engine,
        text: text.to_owned(),
        takes: Takes::Text,
        answers: Vec::new(),
        default: vec![default.to_owned()],
        flag: flag.map(str::to_owned),
        skipped: None,
    }
}

/// Why `concern` isn't asked of a game of the perspectives `picked`, if it
/// isn't: none of its answers is for them, or fewer than two of its
/// answers are ready or planned, so there is nothing to pick between.
fn skipped(concern: &Concern, picked: &[String]) -> Option<String> {
    if !concern.serves_any(picked) {
        let picked = if picked.is_empty() {
            "none is picked".to_owned()
        } else {
            format!("the game's are {}", picked.join(", "))
        };
        return Some(format!(
            "its answers are for the perspectives {}, and {picked}",
            concern.serves.join(", ")
        ));
    }
    let coming: Vec<&str> = concern
        .answers
        .iter()
        .filter(|a| matches!(a.status, Status::Ready | Status::Planned))
        .map(|a| a.id.as_str())
        .collect();
    match coming.as_slice() {
        [] => Some("none of its answers is ready or planned".to_owned()),
        [one] => Some(format!(
            "only `{one}` of its answers is ready or planned: there is nothing to pick between"
        )),
        _ => None,
    }
}

/// The question of catalog concern `concern`, asked by `asker`.
fn concern_question(
    concern: &Concern,
    asker: Asker,
    given: &Answers,
    skipped: Option<String>,
) -> Question {
    let default = given.0.get(&concern.id).cloned().unwrap_or_else(|| {
        concern
            .default_answer()
            .map(|a| a.id.clone())
            .into_iter()
            .collect()
    });
    Question {
        id: concern.id.clone(),
        asker,
        text: concern
            .question
            .clone()
            .unwrap_or_else(|| concern.name.clone()),
        takes: concern.pick.map_or(Takes::One, Takes::from),
        answers: concern
            .answers
            .iter()
            .map(|a| Offered {
                id: a.id.clone(),
                text: a.name.clone(),
                status: a.status,
                tickets: if a.status == Status::Ready {
                    Vec::new()
                } else {
                    a.tickets.clone()
                },
            })
            .collect(),
        default,
        flag: Some(format!("--{}", concern.id)),
        skipped,
    }
}

/// The questions `shuyi new` asks of the game `named`, with `template` over
/// `catalog`'s concerns and the engine's `targets`, for the answers `given`
/// so far: the folder, the name and the id; the perspectives, always asked;
/// one question per concern the catalog asks (all but `time`), in its
/// order; the template's own; then the targets. A question that isn't
/// asked says why in [`Question::skipped`].
pub fn questions(
    catalog: &Catalog,
    template: &Template,
    targets: &[Target],
    named: &Named,
    given: &Answers,
) -> Vec<Question> {
    let mut out = vec![
        text_question(DIR, "Which folder is the game made in?", None, &named.dir),
        text_question(
            NAME,
            "What is the game called?",
            Some("--name"),
            &named.name,
        ),
        text_question(
            ID,
            "What is its reverse-DNS id, which its saves are kept under?",
            Some("--id"),
            &named.id,
        ),
    ];
    let concerns = &catalog.concerns.concerns;
    let asked = |id: &str| concerns.iter().find(|c| c.id == id && c.pick.is_some());
    let mut picked = Vec::new();
    if let Some(perspective) = asked(PERSPECTIVE) {
        let question = concern_question(perspective, Asker::Engine, given, None);
        picked.clone_from(&question.default);
        out.push(question);
    }
    for concern in concerns
        .iter()
        .filter(|c| c.pick.is_some() && c.id != PERSPECTIVE)
    {
        let why = skipped(concern, &picked);
        out.push(concern_question(concern, Asker::Catalog, given, why));
    }
    for question in &template.questions {
        let skipped = ANSWERED_ELSEWHERE
            .iter()
            .find(|(id, _)| *id == question.id)
            .map(|(_, why)| (*why).to_owned());
        out.push(Question {
            id: question.id.clone(),
            asker: Asker::Template,
            text: question.text.clone(),
            takes: question.pick.into(),
            answers: question
                .answers
                .iter()
                .map(|a| Offered {
                    id: a.id.clone(),
                    text: a.text.clone(),
                    status: Status::Ready,
                    tickets: Vec::new(),
                })
                .collect(),
            default: given
                .0
                .get(&question.id)
                .cloned()
                .unwrap_or_else(|| question.default.clone()),
            flag: Some(format!("--set {}=", question.id)),
            skipped,
        });
    }
    out.push(Question {
        id: TARGETS.to_owned(),
        asker: Asker::Engine,
        text: "Which targets does it ship on?".to_owned(),
        takes: Takes::Many,
        answers: targets
            .iter()
            .map(|t| Offered {
                id: t.id.clone(),
                text: if t.ready {
                    t.id.clone()
                } else {
                    format!("{}: the engine doesn't run on it yet", t.id)
                },
                status: if t.ready {
                    Status::Ready
                } else {
                    Status::Planned
                },
                tickets: Vec::new(),
            })
            .collect(),
        default: given
            .0
            .get(TARGETS)
            .cloned()
            .unwrap_or_else(|| default_targets(template)),
        flag: Some("--targets".to_owned()),
        skipped: None,
    });
    out
}

/// The catalog of the engine `input` builds with: its checkout's at
/// `--engine-path` (from `cwd`), else the one this `shuyi` embeds.
pub(super) fn catalog_of(cwd: &Path, input: &NewInput) -> Result<Catalog, (Code, String)> {
    match &input.engine {
        Engine::Path { path } => {
            let root = engine_root(cwd, path).map_err(|e| (codes::WRONG_ARGUMENTS, e))?;
            Catalog::read(Path::new(&root))
        }
        _ => Catalog::embedded(),
    }
    .map_err(|e| (codes::ENGINE_DEPENDENCY, e))
}

/// `new <dir> --questions`: the questions `shuyi new` asks of the game
/// `input` names under `cwd`, for the answers it gives so far. It makes
/// nothing.
pub fn listed(cwd: &Path, input: &NewInput) -> Answer<Questions> {
    match list(cwd, input) {
        Ok(questions) => Answer::new(Op::New, true, questions, Vec::new()),
        Err((code, e)) => Answer::failed(Op::New, code, e),
    }
}

/// [`listed`]'s questions, or the problem code and why there are none.
fn list(cwd: &Path, input: &NewInput) -> Result<Questions, (Code, String)> {
    let refused = |e| (codes::WRONG_ARGUMENTS, e);
    let (name, krate) = name_of(cwd, input).map_err(refused)?;
    let template = template::built_in().map_err(|e| (codes::FILE, e.to_string()))?;
    let catalog = catalog_of(cwd, input)?;
    let given = answers_of(cwd, &template, input).map_err(refused)?;
    let targets = engine_targets().map_err(|e| (codes::FILE, e))?;
    let named = Named {
        dir: input.dir.clone(),
        name,
        id: input.id.clone().unwrap_or_else(|| default_id(&krate)),
    };
    Ok(Questions {
        template: template.id.clone(),
        questions: questions(&catalog, &template, &targets, &named, &given),
    })
}

/// The answers' ids, each with its status if it isn't ready, joined by
/// commas: `none` for none.
fn shown_answers(question: &Question) -> String {
    let shown: Vec<String> = question
        .answers
        .iter()
        .map(|a| match a.status {
            Status::Ready => a.id.clone(),
            status if a.tickets.is_empty() => format!("{} ({})", a.id, status.name()),
            status => format!("{} ({}: {})", a.id, status.name(), a.tickets.join(", ")),
        })
        .collect();
    shown.join(", ")
}

/// The questions, for people: a line each, its flag and its default, then
/// its answers, or why it isn't asked.
pub fn text(listed: &Questions) -> String {
    let mut out = format!(
        "new: {} question(s), from {}\n",
        listed.questions.len(),
        listed.template
    );
    for question in &listed.questions {
        let flag = question.flag.as_deref().unwrap_or("<dir>");
        let default = if question.default.is_empty() {
            "none".to_owned()
        } else {
            question.default.join(",")
        };
        // Writing to a `String` can't fail.
        let _ = writeln!(out, "{} ({flag}, now {default})", question.text);
        if let Some(why) = &question.skipped {
            let _ = writeln!(out, "  not asked: {why}");
        } else if !question.answers.is_empty() {
            let takes = if question.takes == Takes::Many {
                "any of"
            } else {
                "one of"
            };
            let _ = writeln!(out, "  {takes}: {}", shown_answers(question));
        }
    }
    out
}

/// `word` quoted so PowerShell and a POSIX shell read it the same: as it
/// is when it holds only letters, digits and `_ . / : = , + -`, else in
/// single quotes, or in double quotes when it holds a `'`. `None` for a
/// word with a `'` and a character double quotes don't keep in both
/// (`$`, `` ` ``, `!`, `"`, `\`).
pub fn quote(word: &str) -> Option<String> {
    let bare = !word.is_empty()
        && word.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | ':' | '=' | ',' | '+' | '-')
        });
    if bare {
        Some(word.to_owned())
    } else if !word.contains('\'') {
        Some(format!("'{word}'"))
    } else if word.contains(['$', '`', '!', '"', '\\']) {
        None
    } else {
        Some(format!("\"{word}\""))
    }
}

/// `picked`, the answers of question `id`, as words of a command: by
/// `flag`, or as `--set <id>=` when none is picked or it has no flag.
fn set(words: &mut Vec<String>, flag: Option<&str>, id: &str, picked: &[String]) {
    match (flag, picked) {
        (Some(flag), [_, ..]) => words.extend([flag.to_owned(), picked.join(",")]),
        _ => words.extend(["--set".into(), format!("{id}={}", picked.join(","))]),
    }
}

/// The command that makes the game `input` made again, named `name`, with
/// no question: `shuyi new <dir> --no-input --name <name>`, `--id` if given,
/// each concern of `concerns` that `answers` answers by its flag (none as
/// `--set <id>=`), `--targets`, `--examples` or `--no-examples`, `--set`
/// for `template`'s other questions, then the engine and `--private-url`;
/// each word quoted by [`quote`]. With it, whether every word could be
/// quoted for both shells: when not, those are quoted for a POSIX shell.
pub fn command(
    input: &NewInput,
    name: &str,
    answers: &Answers,
    concerns: &Concerns,
    template: &Template,
) -> (String, bool) {
    let mut words: Vec<String> = vec!["shuyi".into(), "new".into(), input.dir.clone()];
    words.extend(["--no-input".into(), "--name".into(), name.to_owned()]);
    if let Some(id) = &input.id {
        words.extend(["--id".into(), id.clone()]);
    }
    for concern in concerns.concerns.iter().filter(|c| c.pick.is_some()) {
        if let Some(picked) = answers.0.get(&concern.id) {
            set(
                &mut words,
                Some(&format!("--{}", concern.id)),
                &concern.id,
                picked,
            );
        }
    }
    if let Some(picked) = answers.0.get(TARGETS) {
        set(&mut words, Some("--targets"), TARGETS, picked);
    }
    for question in &template.questions {
        let Some(picked) = answers.0.get(&question.id) else {
            continue;
        };
        match (question.id.as_str(), picked.as_slice()) {
            (EXAMPLES, [yes]) if yes == "yes" => words.push("--examples".into()),
            (EXAMPLES, [no]) if no == "no" => words.push("--no-examples".into()),
            _ => set(&mut words, None, &question.id, picked),
        }
    }
    match &input.engine {
        Engine::Rev { rev } => words.extend(["--engine-rev".into(), rev.clone()]),
        Engine::Path { path } => words.extend(["--engine-path".into(), path.clone()]),
        _ => {}
    }
    if let Some(url) = &input.private_url {
        words.extend(["--private-url".into(), url.clone()]);
    }
    let mut alike = true;
    let quoted: Vec<String> = words
        .iter()
        .map(|word| {
            quote(word).unwrap_or_else(|| {
                alike = false;
                format!("'{}'", word.replace('\'', "'\\''"))
            })
        })
        .collect();
    (quoted.join(" "), alike)
}

#[cfg(test)]
mod tests;
