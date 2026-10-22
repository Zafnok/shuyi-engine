//! `explain [<code>]`: every problem code, or one code's page
//! ([`codes`]). It needs no project, like `doctor` and `schema`, and is
//! how an agent or a person learns what an error's code means and how to
//! fix it (`shuyi explain SH2003`, as `rustc --explain E0277`).

use std::fmt::Write as _;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::codes::{self, Class, Entry};
use super::{Answer, Op};

/// What `explain` explains.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExplainInput {
    /// The code (`SH2003`) or its name (`missing-file`); every code if
    /// none.
    #[serde(default)]
    pub code: Option<String>,
}

/// One code explained.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Explained {
    /// The code (`SH2003`).
    pub code: String,
    /// Its kebab-case name (`missing-file`), as stable as the code.
    pub name: String,
    /// Its class, which sets the exit code.
    pub class: Class,
    /// The exit code of an error with it.
    pub exit_code: u8,
    /// What went wrong, in a few words.
    pub title: String,
    /// How to fix it, in a line.
    pub help: String,
    /// Whether it is retired: no error gives it any more.
    pub retired: bool,
    /// Its page, in Markdown, when one code was asked for.
    pub page: Option<String>,
}

/// What `explain` answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExplainOutput {
    /// The code asked for, or every code in order.
    pub entries: Vec<Explained>,
}

/// `entry`, with its page if `page`.
fn explained(entry: &Entry, page: bool) -> Explained {
    Explained {
        code: entry.code.0.to_owned(),
        name: entry.name.to_owned(),
        class: entry.class,
        exit_code: entry.class.exit_code(),
        title: entry.title.to_owned(),
        help: entry.help.to_owned(),
        retired: entry.retired,
        page: page.then(|| entry.page.to_owned()),
    }
}

/// `explain`: the code `input` asks for, with its page, or every code.
pub fn explain(input: &ExplainInput) -> Answer<ExplainOutput> {
    let entries = match &input.code {
        None => codes::TABLE.iter().map(|e| explained(e, false)).collect(),
        Some(asked) => {
            let found = codes::TABLE
                .iter()
                .find(|e| e.code.0.eq_ignore_ascii_case(asked) || e.name == asked);
            match found {
                Some(entry) => vec![explained(entry, true)],
                None => {
                    return Answer::failed(
                        Op::Explain,
                        codes::WRONG_ARGUMENTS,
                        format!("no problem code {asked}"),
                    )
                    .with_help("`shuyi explain` lists every code");
                }
            }
        }
    };
    Answer::new(Op::Explain, true, ExplainOutput { entries }, Vec::new())
}

/// Reads `explain`'s arguments (not `--json`): none, or a code.
pub fn parse_args(args: &[String]) -> Result<ExplainInput, String> {
    match args {
        [] => Ok(ExplainInput::default()),
        [code] if !code.starts_with("--") => Ok(ExplainInput {
            code: Some(code.clone()),
        }),
        _ => Err("one code, or none".to_owned()),
    }
}

/// `explain`'s usage, with `command` as the tool's name.
pub fn usage(command: &str) -> String {
    format!(
        "usage: {command} explain [<code>] [--json]\n\n\
         Lists every problem code, or prints the page of one (SH2003, or its name)."
    )
}

/// What `explain` found, for people: one code's page, or a line a code.
pub fn text(output: &ExplainOutput) -> String {
    if let [one] = output.entries.as_slice()
        && let Some(page) = &one.page
    {
        return page.clone();
    }
    let mut out = String::new();
    for e in &output.entries {
        let retired = if e.retired { " (retired)" } else { "" };
        // Writing to a `String` can't fail.
        let _ = writeln!(
            out,
            "{}  {:<9}  {:<15}  {}{retired}",
            e.code,
            e.class.name(),
            e.name,
            e.title
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asked(code: &str) -> ExplainInput {
        ExplainInput {
            code: Some(code.to_owned()),
        }
    }

    #[test]
    fn with_no_code_it_lists_every_code() {
        let answer = explain(&ExplainInput::default());
        assert!(answer.ok);
        let output = answer.result.unwrap();
        assert_eq!(output.entries.len(), codes::TABLE.len());
        assert!(output.entries.iter().all(|e| e.page.is_none()));
        let said = text(&output);
        assert_eq!(said.lines().count(), codes::TABLE.len());
        assert!(
            said.contains("SH2003  content    missing-file     A file isn't there\n"),
            "{said}"
        );
        let mut retired = output;
        retired.entries.truncate(1);
        retired.entries[0].retired = true;
        assert!(text(&retired).ends_with(" (retired)\n"));
    }

    #[test]
    fn with_a_code_or_its_name_it_prints_its_page() {
        let answer = explain(&asked("SH2003"));
        let output = answer.result.unwrap();
        let [one] = output.entries.as_slice() else {
            panic!("{output:?}")
        };
        assert_eq!(one.name, "missing-file");
        assert_eq!(one.exit_code, 1);
        assert_eq!(one.class, Class::Content);
        assert!(text(&output).starts_with("# SH2003: A file isn't there\n"));
        let by_name = explain(&asked("missing-file")).result.unwrap();
        assert_eq!(by_name, output);
        assert_eq!(explain(&asked("sh2003")).result.unwrap(), output);
        let json: serde_json::Value =
            serde_json::from_str(&explain(&asked("SH1002")).to_json()).unwrap();
        assert_eq!(json["result"]["entries"][0]["class"], "setup");
        assert_eq!(json["result"]["entries"][0]["exit_code"], 3);
    }

    #[test]
    fn an_unknown_code_is_a_wrong_argument() {
        let answer = explain(&asked("SH9999"));
        assert!(!answer.ok);
        assert_eq!(answer.code.as_deref(), Some("SH0001"));
        assert_eq!(answer.exit_code(), 2);
        assert!(answer.help.unwrap().contains("`shuyi explain`"));
    }

    #[test]
    fn it_takes_one_code_or_none() {
        assert_eq!(parse_args(&[]), Ok(ExplainInput::default()));
        assert_eq!(parse_args(&["SH0001".to_owned()]), Ok(asked("SH0001")));
        assert!(parse_args(&["--x".to_owned()]).is_err());
        assert!(parse_args(&["a".to_owned(), "b".to_owned()]).is_err());
        assert!(usage("shuyi").starts_with("usage: shuyi explain"));
    }
}
