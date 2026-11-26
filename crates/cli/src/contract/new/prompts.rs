//! The prompts `shuyi new` asks its questions with in a terminal (ADR-0013
//! §1, §2 and §6): a line of text for the name and the id, a list to pick
//! one answer from, a list to tick any number of. A question's `ready`
//! answers are its list; its `planned` and `parked` ones are shown under
//! it, greyed, each with its status and tickets, and can't be picked. A
//! question that isn't asked isn't shown: `shuyi new <dir> --questions`
//! lists every question, each skipped one with why (Nick, 2026-11-25).
//!
//! Each prompt goes through a [`Prompter`], one method per kind, so a test
//! answers them from a script; a terminal's is [`Terminal`], `dialoguer`'s.
//! Its keys (the arrows, space and Enter) are the terminal's, not a game's:
//! the help line under each list names them.
//!
//! [`ask`] asks the wizard's questions ([`super::wizard::questions`]) that
//! nothing given answers, in order, and writes each answer into the
//! input, as its flag would: the command `new` prints at the end makes the
//! same game with nothing asked.

use std::fmt::{self, Write as _};
use std::io;
use std::path::Path;

use dialoguer::console::{Term, style};
use dialoguer::theme::{ColorfulTheme, Theme};
use dialoguer::{Input, MultiSelect, Select};

use super::wizard::{self, DIR, ID, NAME, Named, Question, STORAGE, Takes};
use super::{NewInput, answers_of, check_name, crate_name, default_id, folder_name};
use crate::catalog::Status;
use crate::template;

/// The help line under a list to pick one answer from.
pub const ONE_HELP: &str = "up and down arrows to move, Enter to pick";

/// The help line under a list to tick answers in.
pub const MANY_HELP: &str = "up and down arrows to move, space to tick or untick, Enter when done";

/// What a list prompt shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct List {
    /// The question.
    pub question: String,
    /// What can be picked: each answer's id and its text.
    pub choices: Vec<(String, String)>,
    /// What is coming and can't be picked, a line each: shown under the
    /// list, greyed.
    pub coming: Vec<String>,
}

/// Asks the prompts, one method per kind.
pub trait Prompter {
    /// A line of text for `question`, `default` for an empty line (none
    /// when `default` is empty), asked again until `check` takes it.
    fn text(
        &mut self,
        question: &str,
        default: &str,
        check: &dyn Fn(&str) -> Result<(), String>,
    ) -> io::Result<String>;

    /// One of `list`'s choices, by its place; `default` picked at first.
    fn one(&mut self, list: &List, default: usize) -> io::Result<usize>;

    /// Any of `list`'s choices, by their places; those `ticked` ticked at
    /// first.
    fn many(&mut self, list: &List, ticked: &[bool]) -> io::Result<Vec<usize>>;

    /// A line said, not asked: why an answer is asked again.
    fn say(&mut self, line: &str) -> io::Result<()>;
}

/// The list `question` shows: its ready answers to pick, and its others
/// under them, each with its status and its tickets.
pub fn list(question: &Question) -> List {
    let (ready, coming): (Vec<_>, Vec<_>) = question
        .answers
        .iter()
        .partition(|a| a.status == Status::Ready);
    List {
        question: question.text.clone(),
        choices: ready
            .into_iter()
            .map(|a| (a.id.clone(), a.text.clone()))
            .collect(),
        coming: coming
            .into_iter()
            .map(|a| match a.tickets.as_slice() {
                [] => format!("{}: {}", a.text, a.status.name()),
                [ticket] => format!("{}: {}, ticket {ticket}", a.text, a.status.name()),
                tickets => format!(
                    "{}: {}, tickets {}",
                    a.text,
                    a.status.name(),
                    tickets.join(", ")
                ),
            })
            .collect(),
    }
}

/// The ids `question`, a list, is answered with through `prompter`: one
/// for [`Takes::One`], any for [`Takes::Many`]. With nothing ready to pick,
/// it asks nothing and keeps its default.
pub fn pick(prompter: &mut dyn Prompter, question: &Question) -> io::Result<Vec<String>> {
    let list = list(question);
    if list.choices.is_empty() {
        return Ok(question.default.clone());
    }
    let id = |at: usize| list.choices[at].0.clone();
    if question.takes == Takes::Many {
        let ticked: Vec<bool> = list
            .choices
            .iter()
            .map(|(id, _)| question.default.contains(id))
            .collect();
        let picked = prompter.many(&list, &ticked)?;
        return Ok(picked.into_iter().map(id).collect());
    }
    let default = question
        .default
        .first()
        .and_then(|d| list.choices.iter().position(|(id, _)| id == d))
        .unwrap_or(0);
    Ok(vec![id(prompter.one(&list, default)?)])
}

/// The wizard's questions for what `input` gives so far under `cwd`, or
/// none when the input makes no game (`new` says why).
fn questions_now(
    cwd: &Path,
    input: &NewInput,
    template: &template::Template,
    catalog: &crate::catalog::Catalog,
    targets: &[wizard::Target],
) -> Option<Vec<Question>> {
    let given = answers_of(cwd, template, input).ok()?;
    let name = input
        .name
        .clone()
        .unwrap_or_else(|| folder_name(cwd, input));
    let id = input.id.clone().unwrap_or_else(|| {
        crate_name(&name).map_or_else(|_| String::new(), |krate| default_id(&krate))
    });
    let named = Named::new(input.dir.clone(), name, id, input.storage.clone());
    Some(wizard::questions(
        catalog, template, targets, &named, &given,
    ))
}

/// Whether `name` can be a game's name, as `new` checks it.
fn check_game_name(name: &str) -> Result<(), String> {
    check_name(name)?;
    crate_name(name).map(|_| ())
}

/// Asks, through `prompter`, each of the wizard's questions that nothing
/// in `input` answers (from `cwd`): the name, the id, the saves' folder
/// (the id typed in, so Enter keeps it), the perspectives,
/// then the questions asked again for them (so a concern skipped for the
/// perspectives picked isn't asked), the template's and the targets. Each
/// answer goes into `input` as its flag would put it. A question that
/// isn't asked takes its default unseen. An input that makes no game asks
/// nothing: `new` says what is wrong.
pub fn ask(cwd: &Path, input: &mut NewInput, prompter: &mut dyn Prompter) -> io::Result<()> {
    let (Ok(template), Ok(catalog), Ok(targets)) = (
        template::built_in(),
        wizard::catalog_of(cwd, input),
        wizard::engine_targets(),
    ) else {
        return Ok(());
    };
    let Ok(given) = answers_of(cwd, &template, input) else {
        return Ok(());
    };
    if input
        .name
        .as_deref()
        .is_some_and(|n| check_game_name(n).is_err())
    {
        return Ok(());
    }
    let (name_given, id_given) = (input.name.is_some(), input.id.is_some());
    let storage_given = input.storage.is_some();
    // The questions are listed again after each answer: the name sets the
    // id's default, the id the saves' folder's, and the perspectives what
    // the catalog asks. Their
    // number doesn't change, only what they say.
    let count = questions_now(cwd, input, &template, &catalog, &targets).map_or(0, |q| q.len());
    for at in 0..count {
        let Some(question) = questions_now(cwd, input, &template, &catalog, &targets)
            .and_then(|q| q.into_iter().nth(at))
        else {
            break;
        };
        let answered = match question.id.as_str() {
            DIR => true,
            NAME => name_given,
            ID => id_given,
            STORAGE => storage_given,
            id => given.0.contains_key(id),
        };
        // A skipped question takes its default unseen; `--questions` shows
        // it, with why.
        if answered || question.skipped.is_some() {
            continue;
        }
        let default = question.default.first().cloned().unwrap_or_default();
        match (question.takes, question.id.as_str()) {
            (Takes::Text, NAME) => {
                let name = prompter.text(&question.text, &default, &check_game_name)?;
                if name != default {
                    input.name = Some(name);
                }
            }
            (Takes::Text, ID) => {
                let check = |id: &str| crate::app_info::check_id(id);
                let id = prompter.text(&question.text, &default, &check)?;
                if id != default {
                    input.id = Some(id);
                }
            }
            (Takes::Text, STORAGE) => {
                // Its default is the id: a storage equal to it is none.
                let check = |storage: &str| crate::app_info::check_storage(storage);
                let storage = prompter.text(&question.text, &default, &check)?;
                if storage != default {
                    input.storage = Some(storage);
                }
            }
            (Takes::Text, _) => {}
            _ => {
                let picked = pick(prompter, &question)?;
                input.set.0.insert(question.id.clone(), picked);
            }
        }
    }
    Ok(())
}

/// The theme of [`Terminal`]'s lists: `dialoguer`'s colourful one, with
/// the list's coming answers, greyed, and its help line under its last
/// choice.
struct Lists {
    base: ColorfulTheme,
    /// The last choice's text, under which the footer goes.
    last: String,
    /// The lines under the list.
    footer: String,
}

impl Lists {
    fn new(list: &List, help: &str) -> Self {
        let mut footer = String::new();
        for line in &list.coming {
            // Writing to a `String` can't fail.
            let _ = write!(footer, "\n    {}", style(format!("{line} (not yet)")).dim());
        }
        let _ = write!(footer, "\n  {}", style(help).dim().italic());
        Self {
            base: ColorfulTheme::default(),
            last: list
                .choices
                .last()
                .map_or_else(String::new, |(_, t)| t.clone()),
            footer,
        }
    }

    fn footer_after(&self, f: &mut dyn fmt::Write, text: &str) -> fmt::Result {
        if text == self.last {
            f.write_str(&self.footer)?;
        }
        Ok(())
    }
}

impl Theme for Lists {
    fn format_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        self.base.format_prompt(f, prompt)
    }

    fn format_error(&self, f: &mut dyn fmt::Write, err: &str) -> fmt::Result {
        self.base.format_error(f, err)
    }

    fn format_input_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        default: Option<&str>,
    ) -> fmt::Result {
        self.base.format_input_prompt(f, prompt, default)
    }

    fn format_input_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        sel: &str,
    ) -> fmt::Result {
        self.base.format_input_prompt_selection(f, prompt, sel)
    }

    fn format_select_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        self.base.format_select_prompt(f, prompt)
    }

    fn format_select_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        sel: &str,
    ) -> fmt::Result {
        self.base.format_select_prompt_selection(f, prompt, sel)
    }

    fn format_multi_select_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        self.base.format_multi_select_prompt(f, prompt)
    }

    fn format_multi_select_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selections: &[&str],
    ) -> fmt::Result {
        self.base
            .format_multi_select_prompt_selection(f, prompt, selections)
    }

    fn format_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        active: bool,
    ) -> fmt::Result {
        self.base.format_select_prompt_item(f, text, active)?;
        self.footer_after(f, text)
    }

    fn format_multi_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        checked: bool,
        active: bool,
    ) -> fmt::Result {
        self.base
            .format_multi_select_prompt_item(f, text, checked, active)?;
        self.footer_after(f, text)
    }
}

/// The terminal's [`Prompter`]: `dialoguer`'s prompts, drawn on stderr.
pub struct Terminal {
    term: Term,
}

impl Terminal {
    /// The prompts on this process's stderr.
    pub fn new() -> Self {
        Self {
            term: Term::stderr(),
        }
    }
}

impl Default for Terminal {
    fn default() -> Self {
        Self::new()
    }
}

/// `dialoguer`'s error, or Esc, as an I/O error.
fn quit<T>(answer: dialoguer::Result<Option<T>>) -> io::Result<T> {
    match answer {
        Ok(Some(answer)) => Ok(answer),
        Ok(None) => Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "stopped at a question (Esc): nothing was made",
        )),
        Err(dialoguer::Error::IO(e)) => Err(e),
    }
}

impl Prompter for Terminal {
    fn text(
        &mut self,
        question: &str,
        default: &str,
        check: &dyn Fn(&str) -> Result<(), String>,
    ) -> io::Result<String> {
        let theme = ColorfulTheme::default();
        let mut input = Input::<String>::with_theme(&theme)
            .with_prompt(question)
            .validate_with(|line: &String| check(line.trim()));
        if !default.is_empty() && check(default).is_ok() {
            input = input.default(default.to_owned());
        }
        input
            .interact_text_on(&self.term)
            .map(|line| line.trim().to_owned())
            .map_err(|dialoguer::Error::IO(e)| e)
    }

    fn one(&mut self, list: &List, default: usize) -> io::Result<usize> {
        let theme = Lists::new(list, ONE_HELP);
        let texts: Vec<&str> = list.choices.iter().map(|(_, t)| t.as_str()).collect();
        quit(
            Select::with_theme(&theme)
                .with_prompt(&list.question)
                .items(&texts)
                .default(default)
                .interact_on_opt(&self.term),
        )
    }

    fn many(&mut self, list: &List, ticked: &[bool]) -> io::Result<Vec<usize>> {
        let theme = Lists::new(list, MANY_HELP);
        let texts: Vec<&str> = list.choices.iter().map(|(_, t)| t.as_str()).collect();
        quit(
            MultiSelect::with_theme(&theme)
                .with_prompt(&list.question)
                .items(&texts)
                .defaults(ticked)
                .interact_on_opt(&self.term),
        )
    }

    fn say(&mut self, line: &str) -> io::Result<()> {
        self.term.write_line(&style(line).dim().to_string())
    }
}

/// A [`Prompter`] answering from a script, for tests, keeping what each
/// prompt showed.
#[cfg(test)]
pub mod scripted {
    use std::collections::VecDeque;
    use std::io;

    use super::{List, Prompter};

    /// One answer of a script.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Step {
        /// Enter: the default, or what is ticked.
        Enter,
        /// A line typed.
        Type(String),
        /// The choices picked or ticked, by id: a planned or parked one
        /// is no choice, and fails.
        Pick(Vec<String>),
    }

    /// The script and what was shown.
    #[derive(Debug, Default)]
    pub struct Scripted {
        /// The answers left.
        pub steps: VecDeque<Step>,
        /// What was shown: each prompt's question, its choices, the
        /// coming answers and the lines said.
        pub shown: Vec<String>,
        /// The lists asked, in order.
        pub lists: Vec<List>,
    }

    impl Scripted {
        /// A script of `steps`.
        pub fn new(steps: impl IntoIterator<Item = Step>) -> Self {
            Self {
                steps: steps.into_iter().collect(),
                ..Self::default()
            }
        }

        /// `n` presses of Enter.
        pub fn enters(n: usize) -> Self {
            Self::new(std::iter::repeat_n(Step::Enter, n))
        }

        fn next(&mut self, question: &str) -> io::Result<Step> {
            self.steps.pop_front().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    format!("no answer to: {question}"),
                )
            })
        }

        fn places(list: &List, ids: &[String]) -> io::Result<Vec<usize>> {
            ids.iter()
                .map(|id| {
                    list.choices
                        .iter()
                        .position(|(choice, _)| choice == id)
                        .ok_or_else(|| {
                            io::Error::new(
                                io::ErrorKind::InvalidInput,
                                format!("`{id}` is no choice of: {}", list.question),
                            )
                        })
                })
                .collect()
        }

        fn show(&mut self, list: &List) {
            self.shown.push(list.question.clone());
            self.lists.push(list.clone());
        }
    }

    /// A script's line typed for a text prompt.
    pub fn typed(line: &str) -> Step {
        Step::Type(line.to_owned())
    }

    /// A script's pick of `ids`.
    pub fn picked(ids: &[&str]) -> Step {
        Step::Pick(ids.iter().map(|id| (*id).to_owned()).collect())
    }

    impl Prompter for Scripted {
        fn text(
            &mut self,
            question: &str,
            default: &str,
            check: &dyn Fn(&str) -> Result<(), String>,
        ) -> io::Result<String> {
            self.shown.push(question.to_owned());
            loop {
                let line = match self.next(question)? {
                    Step::Enter => default.to_owned(),
                    Step::Type(line) => line.trim().to_owned(),
                    Step::Pick(_) => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("a pick for the text prompt: {question}"),
                        ));
                    }
                };
                match check(&line) {
                    Ok(()) => return Ok(line),
                    Err(e) => self.shown.push(format!("  {e}")),
                }
            }
        }

        fn one(&mut self, list: &List, default: usize) -> io::Result<usize> {
            self.show(list);
            match self.next(&list.question)? {
                Step::Enter => Ok(default),
                Step::Pick(ids) => match Self::places(list, &ids)?.as_slice() {
                    [one] => Ok(*one),
                    _ => Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("one pick for: {}", list.question),
                    )),
                },
                Step::Type(_) => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("a line typed for the list: {}", list.question),
                )),
            }
        }

        fn many(&mut self, list: &List, ticked: &[bool]) -> io::Result<Vec<usize>> {
            self.show(list);
            match self.next(&list.question)? {
                Step::Enter => Ok(ticked
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| **t)
                    .map(|(at, _)| at)
                    .collect()),
                Step::Pick(ids) => Self::places(list, &ids),
                Step::Type(_) => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("a line typed for the list: {}", list.question),
                )),
            }
        }

        fn say(&mut self, line: &str) -> io::Result<()> {
            self.shown.push(line.to_owned());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
