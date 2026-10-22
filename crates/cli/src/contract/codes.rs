//! Problem codes (ADR-0028): every error the contract gives carries a
//! stable code, `SH` and four digits, whose first digit is its class, and
//! the class sets the exit code. Each code has a row here, with a help
//! line, and a page in `crates/cli/errors/` that `shuyi explain <code>`
//! prints.
//!
//! | Class | Codes | Exit code | Means |
//! | ----- | ----- | --------- | ----- |
//! | usage | `SH0nnn` | 2 | the arguments or the input are wrong: change the command |
//! | setup | `SH1nnn` | 3 | the project file or the toolchain needs fixing: fix it, then run the same command again |
//! | content | `SH2nnn` | 1 | a file of the game's content has a problem |
//! | operation | `SH3nnn` | 1 | the operation failed on something else: the game's tools, a target, a file it reads or writes |
//!
//! A code is never reused or renumbered: a retired one keeps its row and
//! its page. A new error gets a new code, its row and its page in the same
//! change. The content codes of the game's files are
//! `shuyi_assets::error::codes`; those of its code, `SH21nn` (`lint`), and
//! the tools' own are here.

use std::fmt::Write as _;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub use shuyi_assets::error::Code;
use shuyi_assets::error::codes as content;

use super::Problem;

/// `wrong-arguments`: the arguments or the input are wrong.
pub const WRONG_ARGUMENTS: Code = Code("SH0001");
/// `no-session`: an MCP session tool was called with no game started.
pub const NO_SESSION: Code = Code("SH0002");
/// `no-project`: no `shuyi.ron` here or above.
pub const NO_PROJECT: Code = Code("SH1001");
/// `project-file`: `shuyi.ron` doesn't read, or lacks what the command
/// needs.
pub const PROJECT_FILE: Code = Code("SH1002");
/// `toolchain`: a piece of the toolchain is missing or doesn't start.
pub const TOOLCHAIN: Code = Code("SH1003");
/// `hard-coded-key`: the game's code or text names a key (`lint`'s
/// `check-keys`).
pub const HARD_CODED_KEY: Code = Code("SH2101");
/// `screen-text`: a screen's text is written in its code, or a screen
/// reads a data name itself (`lint`'s `check-text`).
pub const SCREEN_TEXT: Code = Code("SH2102");
/// `rules-fence`: the rules crate uses what its fence bans: a hash map, a
/// float, a clock, file I/O, outside luck (`lint`).
pub const RULES_FENCE: Code = Code("SH2103");
/// `rules-dependency`: the rules crate depends on a screen, device or
/// tool crate, or on outside luck (`lint`).
pub const RULES_DEPENDENCY: Code = Code("SH2104");
/// `not-offered`: the game's tools don't offer the operation.
pub const NOT_OFFERED: Code = Code("SH3001");
/// `game-error`: the game's tools failed.
pub const GAME_ERROR: Code = Code("SH3002");
/// `target`: no such target, or the target profiles don't load.
pub const TARGET: Code = Code("SH3003");
/// `file`: a file the operation reads or writes can't be.
pub const FILE: Code = Code("SH3004");
/// `game-program`: the game's tool program didn't build, or stopped
/// before it answered.
pub const GAME_PROGRAM: Code = Code("SH3005");

/// A code's class: what kind of fix it asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Class {
    /// The arguments or the input are wrong: change the command.
    Usage,
    /// The project file or the toolchain needs fixing: fix it, then run
    /// the same command again.
    Setup,
    /// A file of the game's content has a problem.
    Content,
    /// The operation failed on something else.
    Operation,
}

impl Class {
    /// Every class, by its digit.
    pub const ALL: [Class; 4] = [Class::Usage, Class::Setup, Class::Content, Class::Operation];

    /// The process exit code of an error of this class.
    pub fn exit_code(self) -> u8 {
        match self {
            Class::Usage => 2,
            Class::Setup => 3,
            Class::Content | Class::Operation => 1,
        }
    }

    /// The first digit of its codes.
    pub fn digit(self) -> char {
        match self {
            Class::Usage => '0',
            Class::Setup => '1',
            Class::Content => '2',
            Class::Operation => '3',
        }
    }

    /// Its name (`usage`).
    pub fn name(self) -> &'static str {
        match self {
            Class::Usage => "usage",
            Class::Setup => "setup",
            Class::Content => "content",
            Class::Operation => "operation",
        }
    }

    /// The class of `code`, by its first digit.
    pub fn of(code: &str) -> Option<Class> {
        let digit = code.strip_prefix("SH")?.chars().next()?;
        Class::ALL.into_iter().find(|class| class.digit() == digit)
    }
}

/// A code's row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// The code.
    pub code: Code,
    /// Its kebab-case name, as stable as the code (`missing-file`).
    pub name: &'static str,
    /// Its class.
    pub class: Class,
    /// What went wrong, in a few words.
    pub title: &'static str,
    /// How to fix it, in a line.
    pub help: &'static str,
    /// Its page, `crates/cli/errors/<code>.md`.
    pub page: &'static str,
    /// Whether it is retired: no error gives it any more.
    pub retired: bool,
}

/// The row of `code`, named `name`, in `class`.
macro_rules! row {
    ($code:expr, $name:literal, $class:ident, $file:literal, $title:literal, $help:literal) => {
        Entry {
            code: $code,
            name: $name,
            class: Class::$class,
            title: $title,
            help: $help,
            page: include_str!(concat!("../../errors/", $file, ".md")),
            retired: false,
        }
    };
}

/// Every code, in order.
pub static TABLE: &[Entry] = &[
    row!(
        WRONG_ARGUMENTS,
        "wrong-arguments",
        Usage,
        "SH0001",
        "The command's arguments or input are wrong",
        "read the usage, change the command, and run it again"
    ),
    row!(
        NO_SESSION,
        "no-session",
        Usage,
        "SH0002",
        "No game is running in the MCP session",
        "call `start` first (with the game's folder as `project`), then the session's other tools"
    ),
    row!(
        NO_PROJECT,
        "no-project",
        Setup,
        "SH1001",
        "No shuyi.ron here or in a folder above",
        "run the command in a game's folder (one holding shuyi.ron, or below it), or make a game \
         with `shuyi new`"
    ),
    row!(
        PROJECT_FILE,
        "project-file",
        Setup,
        "SH1002",
        "shuyi.ron doesn't read, or lacks what the command needs",
        "fix shuyi.ron as the message says, then run the same command again"
    ),
    row!(
        TOOLCHAIN,
        "toolchain",
        Setup,
        "SH1003",
        "A piece of the toolchain is missing",
        "run `shuyi doctor` and fix each failed check as its help says, then run the same \
         command again"
    ),
    row!(
        content::PARSE,
        "parse",
        Content,
        "SH2001",
        "A text file's syntax is wrong",
        "fix the file at the line and column given: the message says what was expected there"
    ),
    row!(
        content::NOT_TEXT,
        "not-text",
        Content,
        "SH2002",
        "A text file isn't valid UTF-8",
        "save the file as UTF-8 text"
    ),
    row!(
        content::MISSING_FILE,
        "missing-file",
        Content,
        "SH2003",
        "A file isn't there",
        "add the file at that path, or fix the name that asks for it"
    ),
    row!(
        content::AMBIGUOUS_FILE,
        "ambiguous-file",
        Content,
        "SH2004",
        "Several files could be the asset",
        "name the file meant with its extension, or remove the others"
    ),
    row!(
        content::NO_FORMAT,
        "no-format",
        Content,
        "SH2005",
        "No format reads the file",
        "give the file an extension a format of its kind claims, or register a format that \
         reads it"
    ),
    row!(
        content::TOO_BIG,
        "too-big",
        Content,
        "SH2006",
        "A file is bigger than its format reads",
        "make the file smaller, or split it"
    ),
    row!(
        content::PACK_DIFFERS,
        "pack-differs",
        Content,
        "SH2007",
        "The pack differs from its sources",
        "bake the pack again with `shuyi bake`; if it still differs, the kind's packed format \
         loses something"
    ),
    row!(
        content::UNKNOWN_NAME,
        "unknown-name",
        Content,
        "SH2008",
        "A name in the file isn't one it may use",
        "use one of the names the message lists, or fix the spelling"
    ),
    row!(
        content::DUPLICATE,
        "duplicate",
        Content,
        "SH2009",
        "Something given once is given twice",
        "keep one of the two: remove or rename the other"
    ),
    row!(
        content::OUT_OF_RANGE,
        "out-of-range",
        Content,
        "SH2010",
        "A number or a size is out of range",
        "bring it within the range the message gives"
    ),
    row!(
        HARD_CODED_KEY,
        "hard-coded-key",
        Content,
        "SH2101",
        "The game's code or text names a key",
        "react to an action instead, and name a key in text through the player's keymap"
    ),
    row!(
        SCREEN_TEXT,
        "screen-text",
        Content,
        "SH2102",
        "A screen's text is written in its code",
        "put the text in the language file and ask for it by key; show a data name through the game's words helpers"
    ),
    row!(
        RULES_FENCE,
        "rules-fence",
        Content,
        "SH2103",
        "The rules use what keeps them from being pure and deterministic",
        "use what the message's reason names: BTreeMap for a hash map, whole numbers for a float, turns or a Command for the time, crates/game for files, the state's SimRng for luck"
    ),
    row!(
        RULES_DEPENDENCY,
        "rules-dependency",
        Content,
        "SH2104",
        "The rules depend on a screen, device or tool crate",
        "remove the dependency from the rules (the command shows the way in) and move the code that needs it to crates/game"
    ),
    row!(
        NOT_OFFERED,
        "not-offered",
        Operation,
        "SH3001",
        "The game's tools don't offer the operation",
        "write the GameTools method the message names in the game's tool program"
    ),
    row!(
        GAME_ERROR,
        "game-error",
        Operation,
        "SH3002",
        "The game's tools failed",
        "fix what the message says in the game's content or tools, then run the command again"
    ),
    row!(
        TARGET,
        "target",
        Operation,
        "SH3003",
        "No such target, or the target profiles don't load",
        "name a target the message lists, or fix the target profiles' files"
    ),
    row!(
        FILE,
        "file",
        Operation,
        "SH3004",
        "A file the operation reads or writes can't be",
        "check the path and the folder's permissions: the message gives the system's reason"
    ),
    row!(
        GAME_PROGRAM,
        "game-program",
        Operation,
        "SH3005",
        "The game's tool program didn't build, or stopped before it answered",
        "fix what its output in the message says (a compile error, a panic), then call the tool          again; the session, if one ran, has ended: `start` it again"
    ),
];

/// The row of `code`.
pub fn entry(code: &str) -> Option<&'static Entry> {
    TABLE.iter().find(|entry| entry.code.0 == code)
}

/// The help line of `code`'s row.
pub fn help_of(code: &str) -> Option<&'static str> {
    entry(code).map(|entry| entry.help)
}

/// The exit code of an error with `code`: its class's, 1 without one.
pub fn exit_code(code: Option<&str>) -> u8 {
    code.and_then(Class::of).map_or(1, Class::exit_code)
}

/// An error of `name` (an op or a command) for people: `name:
/// error[CODE]: message`, then its `help` (multi-line help indented under
/// it) and the `shuyi explain` that prints its page.
pub fn said(name: &str, code: &str, message: &str, help: Option<&str>) -> String {
    let mut out = format!("{name}: error[{code}]: {message}\n");
    if let Some(help) = help {
        let mut lines = help.trim_end().lines();
        out.push_str("  help: ");
        out.push_str(lines.next().unwrap_or_default());
        for line in lines {
            out.push('\n');
            if !line.is_empty() {
                out.push_str("        ");
                out.push_str(line);
            }
        }
        out.push('\n');
    }
    // Writing to a `String` can't fail.
    let _ = writeln!(out, "  see: shuyi explain {code}");
    out
}

/// After a list of `problems`: one `help[CODE]: …` line per code they
/// used, in the order first used.
pub fn help_lines(problems: &[Problem]) -> String {
    let mut seen: Vec<&str> = Vec::new();
    let mut out = String::new();
    for problem in problems {
        if let (Some(code), Some(help)) = (&problem.code, &problem.help)
            && !seen.contains(&code.as_str())
        {
            seen.push(code);
            // Writing to a `String` can't fail.
            let _ = writeln!(out, "help[{code}]: {help}");
        }
    }
    out
}

#[cfg(test)]
mod tests;
