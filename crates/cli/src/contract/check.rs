//! `check`: the game loads all its content through every validator it
//! has, and the answer lists every error found, each with its file, line
//! and message. It passes when there are none.

use std::fmt::Write as _;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Answer, Op, Problem};
use crate::game::GameTools;

/// What `check` checks: everything the game loads (nothing to choose
/// yet).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CheckInput {}

/// What `check` found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CheckOutput {
    /// Every error, in the order the game's validators found them.
    pub errors: Vec<Problem>,
}

/// `check`: every error of `game`'s content.
pub fn check(game: &dyn GameTools, _input: &CheckInput) -> Answer<CheckOutput> {
    match game.check() {
        Ok(errors) => {
            let errors: Vec<Problem> = errors.iter().map(Problem::from).collect();
            let mut limits = Vec::new();
            if errors.iter().any(|e| e.line.is_none()) {
                limits.push(
                    "some errors are about a whole file: their validator gives no line".to_owned(),
                );
            }
            Answer::new(Op::Check, errors.is_empty(), CheckOutput { errors }, limits)
        }
        Err(e) => Answer::failed(Op::Check, e),
    }
}

/// What `check` found, for people.
pub fn text(output: &CheckOutput) -> String {
    if output.errors.is_empty() {
        return "check: OK\n".to_owned();
    }
    let mut out = format!("check: {} error(s)\n", output.errors.len());
    for error in &output.errors {
        // Writing to a `String` can't fail.
        let _ = writeln!(out, "  {error}");
    }
    out
}

#[cfg(test)]
mod tests {
    use shuyi_assets::ContentError;

    use super::*;
    use crate::contract::fake::Fake;

    #[test]
    fn check_passes_with_no_error_and_lists_each_with_where() {
        let answer = check(&Fake::default(), &CheckInput::default());
        assert!(answer.ok);
        assert_eq!(answer.op, Op::Check);
        assert!(answer.limits.is_empty());
        assert_eq!(text(answer.result.as_ref().unwrap()), "check: OK\n");
        let game = Fake {
            check: Ok(vec![
                ContentError::new("a.ron", "bad").at(2, Some(5)),
                ContentError::new("b.map", "missing"),
            ]),
            ..Fake::default()
        };
        let answer = check(&game, &CheckInput::default());
        assert!(!answer.ok);
        assert_eq!(answer.exit_code(), 1);
        let out = answer.result.as_ref().unwrap();
        assert_eq!(out.errors[0].line, Some(2));
        assert_eq!(
            answer.limits,
            ["some errors are about a whole file: their validator gives no line"]
        );
        assert_eq!(
            text(out),
            "check: 2 error(s)\n  a.ron:2:5: bad\n  b.map: missing\n"
        );
        // Errors that all have a line need no limit.
        let lined = Fake {
            check: Ok(vec![ContentError::new("a.ron", "bad").at(1, None)]),
            ..Fake::default()
        };
        assert!(check(&lined, &CheckInput::default()).limits.is_empty());
    }

    #[test]
    fn a_game_that_cant_check_says_why() {
        let game = Fake {
            check: Err("no data".to_owned()),
            ..Fake::default()
        };
        let answer = check(&game, &CheckInput::default());
        assert_eq!(answer.error.as_deref(), Some("no data"));
    }
}
