use std::collections::BTreeSet;

use serde_json::Value;

use super::*;
use crate::contract::{cli, op_schemas};

/// A flag no op takes.
const UNKNOWN: &str = "--no-such-flag";

/// The names of the properties of `op`'s input schema, as MCP's tool takes
/// them.
fn input_fields(op: Op) -> BTreeSet<String> {
    let [input, _] = op_schemas(op, false);
    let schema: Value = serde_json::from_str(&input.text).unwrap();
    schema
        .get("properties")
        .and_then(Value::as_object)
        .map_or_else(BTreeSet::new, |properties| {
            properties.keys().cloned().collect()
        })
}

/// An op's parser, which answers an error or nothing.
type Parse = fn(&[String]) -> Result<(), String>;

/// What an op's parser needs before its flags (its positional arguments),
/// and the parser.
fn parser(op: Op) -> (&'static [&'static str], Parse) {
    fn none(args: &[String]) -> Result<(), String> {
        if args.is_empty() {
            Ok(())
        } else {
            Err("no arguments".to_owned())
        }
    }
    match op {
        Op::New => (&["g"], |a| new::questions::parse(a).map(drop)),
        Op::Check => (&[], |a| check::parse_args(a).map(drop)),
        Op::Lint | Op::Modules | Op::Doctor => (&[], none),
        Op::Add => (&["physics"], |a| add::parse_args(a).map(drop)),
        Op::Run | Op::Scene => (&[], |a| run::parse_args(a, &[]).map(drop)),
        Op::Replay => (&["a.ron"], |a| {
            cli::replay_args(a)
                .map(drop)
                .ok_or_else(|| "refused".to_owned())
        }),
        Op::Playtest => (&[], |a| playtest::parse_args(a, &[]).map(drop)),
        Op::Autobalance => (&[], |a| autobalance::parse_args(a, &[]).map(drop)),
        Op::Import => (&["a.png"], |a| import::parse_args(a).map(drop)),
        Op::Bake => (&[], |a| bake::parse_args(a).map(drop)),
        Op::Export => (&["web"], |a| export::parse_args(a).map(drop)),
        Op::Explain => (&[], |a| explain::parse_args(a).map(drop)),
        Op::Attach => (&["127.0.0.1:1", "--token", "t"], |a| {
            attach::parse_args(a).map(drop)
        }),
        Op::Start | Op::Step | Op::FramePng | Op::Events | Op::Perf | Op::Reset | Op::Stop => {
            (&[], none)
        }
    }
}

/// `op`'s parser on its positional arguments and `rest`.
fn parse(op: Op, rest: &[&str]) -> Result<(), String> {
    let (before, parse) = parser(op);
    let args: Vec<String> = before.iter().chain(rest).map(|s| (*s).to_owned()).collect();
    parse(&args)
}

/// Whether `op`'s parser takes `flag`: alone or with a value, it answers
/// something other than what it answers a flag it doesn't know.
fn takes(op: Op, flag: &str) -> bool {
    let unknown = parse(op, &[UNKNOWN]).unwrap_err().replace(UNKNOWN, flag);
    [&[flag][..], &[flag, "1"]]
        .iter()
        .any(|args| parse(op, args).as_ref().err() != Some(&unknown))
}

/// What is wrong with `flag` of `op`, whose input has `fields`: nothing if
/// it is a field or the CLI's alone.
fn misplaced(op: Op, flag: &str, fields: &BTreeSet<String>) -> Option<String> {
    if cli_only(op, flag).is_some() || fields.contains(&field_of(op, flag)) {
        return None;
    }
    Some(format!(
        "`{} {flag}` is not a field of {}'s input, so MCP never gets it: add the \
         field `{}` to the input (or a line to `RENAMED` in contract/flags.rs if \
         its field has another name), or add the flag to `CLI_ONLY` there with why",
        op.name(),
        op.name(),
        field_of(op, flag),
    ))
}

#[test]
fn every_flag_of_an_op_is_a_field_of_its_input_or_the_clis_alone() {
    let mut wrong = Vec::new();
    for &op in Op::ALL {
        let fields = input_fields(op);
        for flag in flags_in(&usage(op)) {
            wrong.extend(misplaced(op, &flag, &fields));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn each_op_s_parser_takes_the_flags_its_usage_names() {
    for &op in Op::ALL {
        assert!(
            parse(op, &[UNKNOWN]).is_err(),
            "{} takes a flag it doesn't know",
            op.name()
        );
        for flag in flags_in(&usage(op)) {
            let first = flag == "--json" || BEFORE_THE_PARSER.contains(&(op, flag.as_str()));
            if !first {
                assert!(takes(op, &flag), "{} doesn't take {flag}", op.name());
            }
        }
    }
}

#[test]
fn a_flag_in_neither_fails_and_says_what_to_do() {
    let fields = input_fields(Op::Bake);
    assert_eq!(misplaced(Op::Bake, "--out", &fields), None);
    let said = misplaced(Op::Bake, "--made-up", &fields).unwrap();
    assert!(said.contains("`bake --made-up`"), "{said}");
    assert!(said.contains("the field `made_up`"), "{said}");
    assert!(said.contains("`CLI_ONLY`"), "{said}");
}

#[test]
fn the_tables_name_only_real_flags_and_fields() {
    for &(op, flag, why) in CLI_ONLY {
        assert!(!why.is_empty(), "{flag} has no reason");
        let ops = op.map_or_else(|| Op::ALL.to_vec(), |op| vec![op]);
        assert!(
            ops.iter()
                .any(|op| flags_in(&usage(*op)).iter().any(|f| f == flag)),
            "CLI_ONLY's {flag} is no op's flag"
        );
        if let Some(op) = op {
            assert!(
                !input_fields(op).contains(&field_of(op, flag)),
                "CLI_ONLY's `{} {flag}` is a field of the input",
                op.name()
            );
        }
    }
    for &(op, flag, field) in RENAMED {
        assert!(
            flags_in(&usage(op)).iter().any(|f| f == flag),
            "RENAMED's `{} {flag}` is not its flag",
            op.name()
        );
        assert!(
            input_fields(op).contains(field),
            "RENAMED's `{field}` is not a field of {}'s input",
            op.name()
        );
    }
}

#[test]
fn a_flag_s_field_is_its_name_with_underscores_or_its_renaming() {
    assert_eq!(field_of(Op::Replay, "--any-stamp"), "any_stamp");
    assert_eq!(field_of(Op::Bake, "--out"), "out");
    assert_eq!(
        field_of(Op::Export, "--detect-recipient"),
        "detect_recipient"
    );
    for &(op, flag, field) in RENAMED {
        assert_eq!(field_of(op, flag), field);
    }
}

#[test]
fn flags_are_read_from_a_text_once_each_and_never_from_inside_a_word() {
    assert_eq!(
        flags_in("usage: x [--a-b N] [--c | --a-b]\n  --d2 x--e --- -- --F (--g)"),
        ["--a-b", "--c", "--d2", "--g"]
    );
    assert_eq!(flags_in(""), Vec::<String>::new());
}
