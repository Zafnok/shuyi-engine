use std::collections::BTreeMap;

use proptest::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{ErrorKind, Limit, Limits, Runtime, Script, ScriptError, Value};

fn compile(runtime: &Runtime, source: &str) -> Script {
    runtime
        .compile("mods/test.rhai", source)
        .unwrap_or_else(|e| panic!("{e}"))
}

fn run_value(source: &str, view: &Value) -> Result<Value, ScriptError> {
    let runtime = Runtime::default();
    let script = runtime.compile("mods/test.rhai", source)?;
    runtime.run(&script, view)
}

fn error_kind(result: Result<Value, ScriptError>) -> ErrorKind {
    match result {
        Ok(value) => panic!("expected an error, got {value}"),
        Err(e) => e.kind,
    }
}

fn no_view() -> Value {
    Value::Unit
}

// Reading back values.

#[derive(Serialize)]
struct Crop {
    growth: i64,
    rain: bool,
    name: String,
    neighbours: Vec<i64>,
}

#[derive(Debug, PartialEq, Deserialize)]
struct Grown {
    height: i64,
    note: String,
}

fn crop() -> Crop {
    Crop {
        growth: 3,
        rain: true,
        name: "wheat".into(),
        neighbours: vec![1, 2, 4],
    }
}

#[test]
fn a_script_reads_the_view_by_field_and_returns_the_games_type() {
    let runtime = Runtime::default();
    let script = compile(
        &runtime,
        "let height = if view.rain { view.growth * 2 } else { view.growth };\n\
         #{ height: height + view.neighbours.len(), note: view.name + \" grew\" }",
    );
    let grown: Grown = runtime
        .run(&script, &crop())
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        grown,
        Grown {
            height: 9,
            note: "wheat grew".into()
        }
    );
}

#[test]
fn call_runs_one_function_with_the_view_and_not_the_top_level() {
    let runtime = Runtime::default();
    // The top-level loop would stop at the limit if it ran.
    let script = compile(&runtime, "fn grows(crop) { crop.growth + 1 }\nloop { }");
    assert!(script.has_function("grows"));
    assert!(!script.has_function("shrinks"));
    let grows: i64 = runtime
        .call(&script, "grows", &crop())
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(grows, 4);
}

#[test]
fn every_plain_value_reads_back() {
    let out = run_value(
        "[(), true, 7, \"seven\", 'c', [1, [2]], #{ b: 1, a: #{ c: false } }]",
        &no_view(),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let expected = Value::List(vec![
        Value::Unit,
        Value::Bool(true),
        Value::Int(7),
        Value::Text("seven".into()),
        Value::Text("c".into()),
        Value::List(vec![Value::Int(1), Value::List(vec![Value::Int(2)])]),
        Value::map([
            ("a", Value::map([("c", Value::Bool(false))])),
            ("b", Value::Int(1)),
        ]),
    ]);
    assert_eq!(out, expected);
    assert_eq!(
        out.to_string(),
        "[(), true, 7, \"seven\", \"c\", [1, [2]], #{\"a\": #{\"c\": false}, \"b\": 1}]"
    );
}

#[derive(Serialize)]
enum Outcome {
    Won { turn: i64 },
    Lost,
}

#[test]
fn an_enum_in_the_view_is_externally_tagged() {
    let runtime = Runtime::default();
    let script = compile(
        &runtime,
        "if type_of(view) == \"map\" { view.Won.turn } else { view }",
    );
    let won: Value = runtime
        .run(&script, &Outcome::Won { turn: 3 })
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(won, Value::Int(3));
    let lost: Value = runtime
        .run(&script, &Outcome::Lost)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(lost, Value::Text("Lost".into()));
}

#[test]
fn a_map_iterates_in_key_order() {
    let view = Value::map([
        ("pear", Value::Int(2)),
        ("apple", Value::Int(1)),
        ("fig", Value::Int(3)),
    ]);
    let out = run_value(
        "let s = \"\"; for k in view.keys() { s += k; s += \",\"; } s",
        &view,
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(out, Value::Text("apple,fig,pear,".into()));
}

#[test]
fn what_doesnt_read_as_the_type_asked_is_a_value_error() {
    let runtime = Runtime::default();
    let script = compile(&runtime, "\"not a number\"");
    let error = runtime
        .run::<_, i64>(&script, &no_view())
        .err()
        .unwrap_or_else(|| panic!("a string read as a number"));
    assert!(matches!(error.kind, ErrorKind::Value(_)), "{error}");
    assert_eq!(error.script, "mods/test.rhai");
}

#[test]
fn a_view_the_script_values_cant_hold_is_a_view_error() {
    let runtime = Runtime::default();
    let script = compile(&runtime, "view");
    let error = runtime
        .run::<_, Value>(&script, &1.5_f64)
        .err()
        .unwrap_or_else(|| panic!("a float went in"));
    assert!(matches!(error.kind, ErrorKind::View(_)), "{error}");
}

// Compile errors and run errors name the file and line.

#[test]
fn a_compile_error_names_the_file_line_and_column() {
    let runtime = Runtime::default();
    let error = runtime
        .compile(
            "mods/crops/rain.rhai",
            "let a = 1;\nlet b = 2;\nlet c = (a +;\n",
        )
        .err()
        .unwrap_or_else(|| panic!("it compiled"));
    assert_eq!(error.script, "mods/crops/rain.rhai");
    assert_eq!(error.line, Some(3));
    assert!(error.column.is_some());
    assert!(matches!(error.kind, ErrorKind::Compile(_)), "{error}");
    assert!(
        error.to_string().starts_with("mods/crops/rain.rhai:3:"),
        "{error}"
    );
}

#[test]
fn a_misspelt_variable_doesnt_compile() {
    let runtime = Runtime::default();
    let error = runtime
        .compile("mods/test.rhai", "let growth = 1;\ngrowht + 1")
        .err()
        .unwrap_or_else(|| panic!("it compiled"));
    assert_eq!(error.line, Some(2));
    assert!(matches!(error.kind, ErrorKind::Compile(_)), "{error}");
}

#[test]
fn a_run_error_inside_a_function_names_its_line() {
    let runtime = Runtime::default();
    let script = compile(
        &runtime,
        "fn grows(crop) {\n    let x = crop.growth;\n    x / 0\n}",
    );
    let error = runtime
        .call::<_, i64>(&script, "grows", &crop())
        .err()
        .unwrap_or_else(|| panic!("it divided by zero"));
    assert_eq!(error.line, Some(3), "{error}");
    assert!(matches!(error.kind, ErrorKind::Run(_)), "{error}");
}

#[test]
fn a_missing_field_is_an_error_not_unit() {
    let kind = error_kind(run_value(
        "view.growht",
        &Value::map([("growth", Value::Int(1))]),
    ));
    assert!(matches!(kind, ErrorKind::Run(_)), "{kind}");
}

#[test]
fn a_missing_function_is_an_error() {
    let runtime = Runtime::default();
    let script = compile(&runtime, "fn grows(crop) { 1 }");
    let error = runtime
        .call::<_, i64>(&script, "shrinks", &crop())
        .err()
        .unwrap_or_else(|| panic!("it called a function that isn't there"));
    assert!(matches!(error.kind, ErrorKind::Run(_)), "{error}");
}

// Integers only, checked.

#[test]
fn floats_dont_compile() {
    let kind = error_kind(run_value("1.5 * 2", &no_view()));
    assert!(matches!(kind, ErrorKind::Compile(_)), "{kind}");
}

#[test]
fn an_overflow_is_an_error() {
    let kind = error_kind(run_value("9223372036854775807 + 1", &no_view()));
    assert!(matches!(kind, ErrorKind::Run(_)), "{kind}");
}

#[test]
fn division_rounds_toward_zero() {
    let out =
        run_value("[7 / 2, -7 / 2, 7 % 3, -7 % 3]", &no_view()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        out,
        Value::List(vec![
            Value::Int(3),
            Value::Int(-3),
            Value::Int(1),
            Value::Int(-1)
        ])
    );
}

// No clock, no luck, no files, no network, no output.

#[test]
fn a_script_cant_read_a_clock() {
    for source in ["timestamp()", "let t = timestamp(); t.elapsed"] {
        let kind = error_kind(run_value(source, &no_view()));
        assert!(matches!(kind, ErrorKind::Run(_)), "{source}: {kind}");
    }
}

#[test]
fn a_script_cant_draw_a_random_number() {
    for source in [
        "rand()",
        "random(6)",
        "rand_int(1, 6)",
        "shuffle([1, 2, 3])",
    ] {
        let kind = error_kind(run_value(source, &no_view()));
        assert!(matches!(kind, ErrorKind::Run(_)), "{source}: {kind}");
    }
}

#[test]
fn a_script_cant_read_a_file() {
    let kind = error_kind(run_value(
        "import \"saves/slot1\" as save; save::gold",
        &no_view(),
    ));
    assert!(matches!(kind, ErrorKind::Compile(_)), "{kind}");
    for source in ["open(\"saves/slot1\")", "read_file(\"saves/slot1\")"] {
        let kind = error_kind(run_value(source, &no_view()));
        assert!(matches!(kind, ErrorKind::Run(_)), "{source}: {kind}");
    }
}

#[test]
fn a_script_cant_reach_the_network() {
    for source in [
        "http_get(\"https://example.com\")",
        "connect(\"example.com\", 80)",
    ] {
        let kind = error_kind(run_value(source, &no_view()));
        assert!(matches!(kind, ErrorKind::Run(_)), "{source}: {kind}");
    }
}

#[test]
fn a_script_cant_print_or_run_text_as_code() {
    for source in ["print(\"hi\")", "debug(\"hi\")", "eval(\"1 + 1\")"] {
        let kind = error_kind(run_value(source, &no_view()));
        assert!(matches!(kind, ErrorKind::Compile(_)), "{source}: {kind}");
    }
}

#[test]
fn the_view_cant_be_changed() {
    let kind = error_kind(run_value("view = 2; view", &Value::Int(1)));
    assert!(matches!(kind, ErrorKind::Run(_)), "{kind}");
}

// Limits.

#[test]
fn a_script_that_loops_forever_stops_at_the_operation_limit() {
    for source in ["loop { }", "while true { }", "let i = 0; loop { i += 1; }"] {
        let kind = error_kind(run_value(source, &no_view()));
        assert_eq!(kind, ErrorKind::Limit(Limit::Operations), "{source}");
    }
}

#[test]
fn a_function_called_forever_stops_at_the_call_depth() {
    let runtime = Runtime::default();
    let script = compile(&runtime, "fn deeper(n) { deeper(n + 1) }");
    let error = runtime
        .call::<_, i64>(&script, "deeper", &0)
        .err()
        .unwrap_or_else(|| panic!("it returned"));
    assert_eq!(error.kind, ErrorKind::Limit(Limit::CallDepth), "{error}");
}

#[test]
fn growing_values_stop_at_their_size_limits() {
    let cases = [
        ("let s = \"ab\"; loop { s += s; }", Limit::Text),
        ("let l = [1]; loop { l += l; }", Limit::List),
        (
            "let m = #{}; let i = 0; loop { let e = #{}; e[`k${i}`] = i; m += e; i += 1; }",
            Limit::Map,
        ),
    ];
    for (source, limit) in cases {
        let kind = error_kind(run_value(source, &no_view()));
        assert_eq!(kind, ErrorKind::Limit(limit), "{source}");
    }
}

#[test]
fn a_map_grown_key_by_key_stops_at_the_operation_limit() {
    // Rhai doesn't check a map's size on `m[k] = v` (rhaiscript/rhai issue
    // 1213): each insert's operations still bound it (ADR-0093 §3).
    let source = "let m = #{}; let i = 0; loop { m[`k${i}`] = i; i += 1; }";
    let kind = error_kind(run_value(source, &no_view()));
    assert_eq!(kind, ErrorKind::Limit(Limit::Operations));
}

#[test]
fn a_deep_expression_stops_at_the_expression_depth() {
    let source = format!("{}1{}", "(".repeat(200), ")".repeat(200));
    let runtime = Runtime::default();
    let error = runtime
        .compile("mods/test.rhai", &source)
        .err()
        .unwrap_or_else(|| panic!("it compiled"));
    assert_eq!(
        error.kind,
        ErrorKind::Limit(Limit::ExpressionDepth),
        "{error}"
    );
}

#[test]
fn a_limit_of_zero_is_one() {
    let runtime = Runtime::new(Limits {
        operations: 0,
        ..Limits::new()
    });
    let script = compile(&runtime, "let a = 1; let b = 2; a + b");
    let error = runtime
        .run::<_, i64>(&script, &no_view())
        .err()
        .unwrap_or_else(|| panic!("no limit was on"));
    assert_eq!(error.kind, ErrorKind::Limit(Limit::Operations));
}

#[test]
fn smartstring_is_never_linked() {
    // Rhai's `compact_str` feature keeps smartstring's MPL-2.0 code out of
    // every program (deny.toml's exception, docs/decisions.md 2026-11-08).
    let name = core::any::type_name::<rhai::Map>();
    assert!(name.contains("compact_str"), "{name}");
}

// The same script on the same view gives the same values.

#[derive(Debug, Clone, Serialize)]
struct Field {
    turn: i64,
    tiles: Vec<i64>,
    owners: BTreeMap<String, i64>,
    raining: bool,
}

fn field() -> impl Strategy<Value = Field> {
    (
        -1_000_000i64..1_000_000,
        prop::collection::vec(-1_000i64..1_000, 0..40),
        prop::collection::btree_map("[a-z]{1,6}", -100i64..100, 0..12),
        any::<bool>(),
    )
        .prop_map(|(turn, tiles, owners, raining)| Field {
            turn,
            tiles,
            owners,
            raining,
        })
}

/// A rule that touches every part of the language a view meets: fields,
/// lists, maps in order, loops, functions, strings and integer maths.
const RULE: &str = "
fn score(tiles, bonus) {
    let s = 0;
    for (t, i) in tiles { s += t * (i + 1) % 97; }
    s + bonus
}
let names = \"\";
let total = 0;
for k in view.owners.keys() { names += k; total += view.owners[k]; }
let grown = view.tiles.filter(|t| t > 0).map(|t| t / 3);
#{
    score: score(view.tiles, if view.raining { 2 } else { 0 }),
    names: names,
    total: total,
    grown: grown,
    half: view.turn / 2,
    sign: view.turn.sign(),
    turn: `turn ${view.turn}`,
}
";

proptest! {
    #[test]
    fn the_same_script_on_the_same_view_gives_the_same_values(field in field()) {
        let first = Runtime::default();
        let second = Runtime::default();
        let a: Value = first.run(&compile(&first, RULE), &field).unwrap_or_else(|e| panic!("{e}"));
        let b: Value = second.run(&compile(&second, RULE), &field).unwrap_or_else(|e| panic!("{e}"));
        prop_assert_eq!(&a, &b);
        // And the values a Rust oracle gives.
        let Value::Map(out) = a else { panic!("not a map: {a}") };
        let score: i64 = field.tiles.iter().zip(0..).map(|(t, i)| t * (i + 1) % 97).sum::<i64>()
            + if field.raining { 2 } else { 0 };
        prop_assert_eq!(&out["score"], &Value::Int(score));
        prop_assert_eq!(&out["names"], &Value::Text(field.owners.keys().cloned().collect()));
        prop_assert_eq!(&out["total"], &Value::Int(field.owners.values().sum()));
        let grown: Vec<Value> = field.tiles.iter().filter(|t| **t > 0).map(|t| Value::Int(t / 3)).collect();
        prop_assert_eq!(&out["grown"], &Value::List(grown));
        prop_assert_eq!(&out["half"], &Value::Int(field.turn / 2));
        prop_assert_eq!(&out["sign"], &Value::Int(field.turn.signum()));
        prop_assert_eq!(&out["turn"], &Value::Text(format!("turn {}", field.turn)));
    }
}
