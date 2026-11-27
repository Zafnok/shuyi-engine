use std::path::PathBuf;

use shuyi_platform::crash::CodeFrame;

use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-crash-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A report of a panic at `src/rules.rs:9:5`, with a record of play.
fn report() -> CrashReport {
    let mut report = CrashReport::new(1_700_000_000_123, "nine is too many");
    report.location = Some("src/rules.rs:9:5".to_owned());
    report.thread = Some("main".to_owned());
    report.game = "1.2.0".to_owned();
    report.target = "windows x86_64 release".to_owned();
    report.frame = 42;
    report.top_screen = Some("stage".to_owned());
    report.backtrace = "   0: <unknown>".to_owned();
    report.log = vec!["warn: low on sound".to_owned()];
    report.set_record(Some("turns 1,1,9".to_owned()));
    report
}

#[test]
fn a_report_reads_with_what_it_holds() {
    let dir = scratch("reads");
    fs::write(dir.join("c.ron"), report().to_text()).unwrap();
    let answer = crash(&CrashInput::new("c.ron"), &dir);
    assert!(answer.ok, "{answer:?}");
    assert_eq!(answer.op, Op::Crash);
    assert_eq!(
        answer.limits,
        [
            "no frames: the report is from macOS or the web, which take none, or from an engine \
          older than its frames"
        ]
    );
    let out = answer.result.unwrap();
    assert_eq!(
        (out.message.as_str(), out.location.as_deref(), out.frame),
        ("nine is too many", Some("src/rules.rs:9:5"), 42)
    );
    assert!(out.has_record && out.fatal);
    let engine = shuyi_platform::crash::ENGINE_VERSION;
    assert_eq!(
        text(&out),
        format!(
            "crash: nine is too many at src/rules.rs:9:5\n  thread main, ended the game, at \
             1700000000123 ms\n  engine {engine}, game 1.2.0, windows x86_64 release\n  frame \
             42, top screen stage\n  a record of play: replay --crash replays it\nbacktrace:\n     \
             0: <unknown>\nlog:\n  warn: low on sound\n"
        )
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_report_older_than_its_fields_reads() {
    let dir = scratch("old");
    // Only the two fields every report has.
    fs::write(dir.join("old.ron"), "(version: 1, message: \"boom\")").unwrap();
    let answer = crash(&CrashInput::new("old.ron"), &dir);
    assert!(answer.ok, "{answer:?}");
    let mut out = answer.result.unwrap();
    assert!(!out.has_record && out.fatal);
    out.fatal = false;
    assert_eq!(
        text(&out),
        "crash: boom at an unknown place\n  thread unnamed, the game ran on, at 0 ms\n  engine , \
         game (no version), \n  frame 0, top screen none\n  no record of play\n"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_report_that_cant_be_read_or_isnt_one_fails_with_the_reason() {
    let dir = scratch("unread");
    let missing = crash(&CrashInput::new("nope.ron"), &dir);
    assert_eq!(missing.code.as_deref(), Some("SH3004"));
    assert!(missing.error.unwrap().starts_with("nope.ron: "));
    fs::write(dir.join("notes.txt"), "not a report").unwrap();
    let wrong = crash(&CrashInput::new("notes.txt"), &dir);
    assert_eq!(wrong.code.as_deref(), Some("SH3018"));
    assert_eq!(wrong.exit_code(), 1);
    assert!(wrong.error.unwrap().starts_with("notes.txt: "));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn without_symbols_the_frames_are_offsets_and_say_so() {
    let dir = scratch("offsets");
    let mut report = report();
    report.frames = vec![CodeFrame::new(Some(0x1a2b)), CodeFrame::new(None)];
    fs::write(dir.join("c.ron"), report.to_text()).unwrap();
    let answer = crash(&CrashInput::new("c.ron"), &dir);
    assert!(answer.ok, "{answer:?}");
    assert_eq!(
        answer.limits,
        [
            "no --symbols: the frames are offsets into the program; give the debug file of its \
          build to name them"
        ]
    );
    let out = answer.result.unwrap();
    assert_eq!(
        out.frames,
        [NamedFrame::new(Some(0x1a2b)), NamedFrame::new(None)]
    );
    assert_eq!(out.frames[0].offset.as_deref(), Some("0000000000001a2b"));
    let written = text(&out);
    assert!(
        written
            .contains("replays it\nframes:\n  0000000000001a2b\n  in another module\nbacktrace:\n"),
        "{written}"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_named_frame_is_written_with_what_is_known_of_it() {
    let mut named = NamedFrame::new(Some(0x10));
    named.function = Some("game::rules::step".to_owned());
    named.file = Some("game/src/rules.rs".to_owned());
    named.line = Some(9);
    assert_eq!(
        frame_line(&named),
        "0000000000000010 game::rules::step at game/src/rules.rs:9"
    );
    named.line = None;
    assert_eq!(
        frame_line(&named),
        "0000000000000010 game::rules::step at game/src/rules.rs"
    );
    named.function = None;
    named.line = Some(3);
    assert_eq!(
        frame_line(&named),
        "0000000000000010 at game/src/rules.rs:3"
    );
    named.file = None;
    assert_eq!(frame_line(&named), "0000000000000010");
}

#[test]
fn a_debug_file_that_cant_be_read_or_isnt_a_program_fails_with_the_reason() {
    let dir = scratch("debug");
    let mut report = report();
    report.frames = vec![CodeFrame::new(Some(0x10))];
    fs::write(dir.join("c.ron"), report.to_text()).unwrap();
    fs::write(dir.join("notes.txt"), "not a program").unwrap();
    let with = |symbols: &str| {
        let mut input = CrashInput::new("c.ron");
        input.symbols = Some(symbols.to_owned());
        crash(&input, &dir)
    };
    let missing = with("nope.debug");
    assert_eq!(missing.code.as_deref(), Some("SH3004"));
    assert!(missing.error.unwrap().starts_with("nope.debug: "));
    let wrong = with("notes.txt");
    assert_eq!(wrong.code.as_deref(), Some("SH3020"));
    assert_eq!(wrong.exit_code(), 1);
    assert!(wrong.error.unwrap().starts_with("notes.txt: "));
    // The report is read first: a report that isn't one says so.
    let mut input = CrashInput::new("notes.txt");
    input.symbols = Some("nope.debug".to_owned());
    assert_eq!(crash(&input, &dir).code.as_deref(), Some("SH3018"));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn crash_takes_one_report_and_its_debug_file() {
    let args = |items: &[&str]| -> Vec<String> { items.iter().map(|s| (*s).to_owned()).collect() };
    assert_eq!(parse_args(&args(&["c.ron"])), Ok(CrashInput::new("c.ron")));
    let mut with = CrashInput::new("c.ron");
    with.symbols = Some("g.debug".to_owned());
    for items in [
        &["c.ron", "--symbols", "g.debug"][..],
        &["--symbols", "g.debug", "c.ron"],
    ] {
        assert_eq!(parse_args(&args(items)), Ok(with.clone()), "{items:?}");
    }
    for wrong in [
        &[][..],
        &["a", "b"],
        &["--bogus"],
        &["c.ron", "--symbols"],
        &["c.ron", "--symbols", "--json"],
        &["--symbols", "g.debug"],
        &["c.ron", "--symbols", "a", "--symbols", "b"],
    ] {
        assert!(parse_args(&args(wrong)).is_err(), "{wrong:?}");
    }
    assert_eq!(
        usage("shuyi"),
        "usage: shuyi crash <report> [--symbols <file>] [--json]"
    );
}

/// Whether `value` is what `schema` describes, its `$ref`s read in
/// `root`: the parts of JSON Schema the contract's schemas use (`type`,
/// `properties`, `required`, `additionalProperties: false`, `items`,
/// `const`). Each mismatch is pushed to `wrong` with its path.
fn conforms(
    value: &serde_json::Value,
    schema: &serde_json::Value,
    root: &serde_json::Value,
    at: &str,
    wrong: &mut Vec<String>,
) {
    use serde_json::Value;
    if let Some(Value::String(reference)) = schema.get("$ref") {
        let name = reference.trim_start_matches("#/$defs/");
        return conforms(value, &root["$defs"][name], root, at, wrong);
    }
    if let Some(Value::Array(options)) = schema.get("anyOf").or_else(|| schema.get("oneOf")) {
        let fits = options.iter().any(|option| {
            let mut found = Vec::new();
            conforms(value, option, root, at, &mut found);
            found.is_empty()
        });
        if !fits {
            wrong.push(format!("{at}: {value} fits none of its schemas"));
        }
    }
    if let Some(constant) = schema.get("const")
        && constant != value
    {
        wrong.push(format!("{at}: {value} is not {constant}"));
    }
    let types: Vec<&str> = match schema.get("type") {
        Some(Value::String(t)) => vec![t.as_str()],
        Some(Value::Array(ts)) => ts.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };
    let kind = match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) if n.is_u64() || n.is_i64() => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    };
    if !types.is_empty() && !types.contains(&kind) {
        wrong.push(format!("{at}: a {kind}, not {types:?}"));
    }
    if let (Value::Object(fields), Some(properties)) = (value, schema.get("properties")) {
        for (name, field) in fields {
            match properties.get(name) {
                Some(property) => conforms(field, property, root, &format!("{at}/{name}"), wrong),
                None => wrong.push(format!("{at}/{name}: not in the schema")),
            }
        }
        for name in schema["required"].as_array().into_iter().flatten() {
            if !fields.contains_key(name.as_str().unwrap_or_default()) {
                wrong.push(format!("{at}: no {name}"));
            }
        }
    }
    if let (Value::Array(items), Some(item)) = (value, schema.get("items")) {
        for (n, element) in items.iter().enumerate() {
            conforms(element, item, root, &format!("{at}/{n}"), wrong);
        }
    }
}

#[test]
fn the_answer_is_json_its_schema_describes() {
    let dir = scratch("schema");
    let mut framed = report();
    framed.frames = vec![CodeFrame::new(Some(0x10)), CodeFrame::new(None)];
    fs::write(dir.join("c.ron"), framed.to_text()).unwrap();
    let [_, output] = crate::contract::op_schemas(Op::Crash, false);
    let schema: serde_json::Value = serde_json::from_str(&output.text).unwrap();
    // Frames named by hand: every field of a named frame, set.
    let mut named = crash(&CrashInput::new("c.ron"), &dir);
    if let Some(out) = named.result.as_mut() {
        let first = &mut out.frames[0];
        first.function = Some("game::step".to_owned());
        first.file = Some("game/src/lib.rs".to_owned());
        first.line = Some(3);
    }
    for answer in [
        crash(&CrashInput::new("c.ron"), &dir),
        named,
        crash(&CrashInput::new("nope.ron"), &dir),
    ] {
        let json: serde_json::Value = serde_json::from_str(&answer.to_json()).unwrap();
        let mut wrong = Vec::new();
        conforms(&json, &schema, &schema, "", &mut wrong);
        assert!(wrong.is_empty(), "{wrong:?}\n{json}");
    }
    // The check finds a field the schema doesn't have, in the result too.
    let mut wrong = Vec::new();
    let json = serde_json::json!({ "op": "crash", "extra": 1, "result": { "bogus": 1 } });
    conforms(&json, &schema, &schema, "", &mut wrong);
    assert!(
        wrong.contains(&"/extra: not in the schema".to_owned()),
        "{wrong:?}"
    );
    assert!(
        wrong.iter().any(|w| w.starts_with("/result: ")),
        "{wrong:?}"
    );
    fs::remove_dir_all(&dir).unwrap();
}
