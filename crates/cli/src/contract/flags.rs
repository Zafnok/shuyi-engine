//! Where each operation's command-line flags go (ticket 3429). The MCP
//! server's tools are made from each op's input schema ([`crate::mcp::tools`]),
//! so a flag that is not a field of the input never reaches MCP. Each flag
//! an op's usage names is a field of its input (its name with `-` read as
//! `_`, or as [`RENAMED`] says), or it is the CLI's alone, listed in
//! [`CLI_ONLY`] with why. The test in this module checks that for every op
//! in [`Op::ALL`]; `docs/contract.md` points here.
//!
//! An op's flags are the ones its usage text names ([`usage`]): the parsers
//! are loops of match arms beside flags that come from the game (its start
//! options) or the engine's catalog (`new`'s concerns), so the usage, which
//! people read, is the one list, and the test drives each op's parser with
//! each flag it names, to check that the parser takes it.

use super::{
    Op, add, attach, autobalance, bake, check, doctor, explain, export, import, lint, modules, new,
    playtest, replay, run, sfx,
};

/// Every op's flag that is the CLI's alone: the op (`None` for every op),
/// the flag, and why it is not a field of the input.
pub(crate) const CLI_ONLY: &[(Option<Op>, &str, &str)] = &[
    (
        None,
        "--json",
        "output only: an MCP tool always answers JSON",
    ),
    (
        Some(Op::New),
        "--no-input",
        "a terminal's prompts: MCP never asks",
    ),
    (Some(Op::New), "--questions", "3426 moves it into the input"),
    (
        Some(Op::New),
        "--github",
        "a terminal's repository step, which MCP refuses",
    ),
    (
        Some(Op::New),
        "--no-github",
        "a terminal's repository step, which MCP refuses",
    ),
    (
        Some(Op::New),
        "--owner",
        "a terminal's repository step, which MCP refuses",
    ),
    (
        Some(Op::New),
        "--visibility",
        "a terminal's repository step, which MCP refuses",
    ),
    (
        Some(Op::New),
        "--assets-visibility",
        "a terminal's repository step, which MCP refuses",
    ),
    (
        Some(Op::New),
        "--private",
        "a terminal's repository step, which MCP refuses",
    ),
    (
        Some(Op::New),
        "--private-folder",
        "a terminal's repository step, which MCP refuses",
    ),
    (
        Some(Op::Run),
        "--watch",
        "the game in its window, a debug build reloading its files: not headless",
    ),
    (
        Some(Op::Playtest),
        "--report",
        "output only: the answer also written to a file, which an MCP client has",
    ),
    (
        Some(Op::Playtest),
        "--brief",
        "output only: the text without its line per try",
    ),
];

/// A flag whose field of the input has another name: the op, the flag and
/// the field.
pub(crate) const RENAMED: &[(Op, &str, &str)] = &[
    // `new`'s concerns and examples answer questions, as `--set` does.
    (Op::New, "--perspective", "set"),
    (Op::New, "--bodies", "set"),
    (Op::New, "--online", "set"),
    (Op::New, "--world-drawing", "set"),
    (Op::New, "--kind", "set"),
    (Op::New, "--scripting", "set"),
    (Op::New, "--targets", "set"),
    (Op::New, "--examples", "set"),
    (Op::New, "--no-examples", "set"),
    (Op::New, "--engine-rev", "engine"),
    (Op::New, "--engine-path", "engine"),
    // A script's steps.
    (Op::Run, "--keys", "script"),
    (Op::Run, "--pad", "script"),
    (Op::Run, "--act", "script"),
    (Op::Run, "--wait", "script"),
    (Op::Run, "--drop", "script"),
    (Op::Run, "--any-stamp", "session"),
    (Op::Scene, "--keys", "script"),
    (Op::Scene, "--pad", "script"),
    (Op::Scene, "--act", "script"),
    (Op::Scene, "--wait", "script"),
    (Op::Scene, "--drop", "script"),
    (Op::Scene, "--any-stamp", "session"),
    (Op::Playtest, "--entrant", "entrants"),
    (Op::Autobalance, "--entrant", "entrants"),
    (Op::Attach, "--for", "follow_for"),
    (Op::Attach, "--keys", "steps"),
    (Op::Attach, "--script", "steps"),
    (Op::Attach, "--pad", "steps"),
    (Op::Attach, "--act", "steps"),
    (Op::Attach, "--wait", "steps"),
    (Op::Attach, "--crank", "steps"),
];

/// A flag the CLI reads before the op's parser, which never sees it.
pub(crate) const BEFORE_THE_PARSER: &[(Op, &str)] = &[(Op::Run, "--watch")];

/// The field of `op`'s input that `flag` sets: [`RENAMED`]'s, or the flag's
/// name with `-` read as `_` (`--any-stamp`, `any_stamp`).
pub(crate) fn field_of(op: Op, flag: &str) -> String {
    RENAMED
        .iter()
        .find(|(o, f, _)| *o == op && *f == flag)
        .map_or_else(
            || flag.trim_start_matches("--").replace('-', "_"),
            |(_, _, field)| (*field).to_owned(),
        )
}

/// Why `flag` of `op` is the CLI's alone, if [`CLI_ONLY`] lists it.
pub(crate) fn cli_only(op: Op, flag: &str) -> Option<&'static str> {
    CLI_ONLY
        .iter()
        .find(|(o, f, _)| o.is_none_or(|o| o == op) && *f == flag)
        .map(|(_, _, why)| *why)
}

/// `op`'s usage text, with `shuyi` as the tool's name and no game options.
pub(crate) fn usage(op: Op) -> String {
    let command = "shuyi";
    match op {
        Op::New => new::usage(command),
        Op::Check => check::usage(command),
        Op::Lint => lint::usage(command),
        Op::Modules => modules::usage(command),
        Op::Add => add::usage(command),
        Op::Run | Op::Scene => run::usage(command, op, &[]),
        Op::Replay => replay::usage(command),
        Op::Playtest => playtest::usage(command, &[]),
        Op::Autobalance => autobalance::usage(command, &[]),
        Op::Import => import::usage(command),
        Op::Bake => bake::usage(command),
        Op::Export => export::usage(command),
        Op::Doctor => doctor::usage(command),
        Op::Sfx => sfx::usage(command),
        Op::Explain => explain::usage(command),
        Op::Attach => attach::usage(command),
        Op::Start | Op::Step | Op::FramePng | Op::Events | Op::Perf | Op::Reset | Op::Stop => {
            String::new()
        }
    }
}

/// The flags `text` names (`--frames`), each once, in order.
pub(crate) fn flags_in(text: &str) -> Vec<String> {
    let mut flags: Vec<String> = Vec::new();
    let bytes = text.as_bytes();
    let mut at = 0;
    while let Some(found) = text[at..].find("--") {
        let start = at + found;
        let before = start.checked_sub(1).map(|i| bytes[i]);
        let end = text[start + 2..]
            .find(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
            .map_or(text.len(), |n| start + 2 + n);
        let flag = &text[start..end];
        let starts_a_word = before.is_none_or(|b| !b.is_ascii_alphanumeric() && b != b'-');
        if starts_a_word
            && flag.len() > 2
            && flag.as_bytes()[2].is_ascii_lowercase()
            && !flags.iter().any(|f| f == flag)
        {
            flags.push(flag.to_owned());
        }
        at = end.max(start + 2);
    }
    flags
}

#[cfg(test)]
mod tests;
