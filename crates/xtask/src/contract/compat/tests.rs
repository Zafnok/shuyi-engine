use std::path::Path;

use serde_json::json;

use super::*;

/// No break.
const NONE: [String; 0] = [];

/// An object of `fields`, the `required` ones listed.
fn object(fields: &Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": fields, "required": required, "additionalProperties": false })
}

fn answer(old: &Value, new: &Value) -> Vec<String> {
    breaks_in("scene.output.json", old, new)
}

fn input(old: &Value, new: &Value) -> Vec<String> {
    breaks_in("run.input.json", old, new)
}

#[test]
fn the_same_schema_breaks_nothing() {
    let schema = object(&json!({ "frames": { "type": "integer" } }), &["frames"]);
    assert_eq!(answer(&schema, &schema), NONE);
    assert_eq!(input(&schema, &schema), NONE);
}

#[test]
fn an_answer_may_gain_a_field() {
    let old = object(&json!({ "frames": { "type": "integer" } }), &["frames"]);
    let new = object(
        &json!({ "frames": { "type": "integer" }, "dropped": { "type": "integer" } }),
        &["frames", "dropped"],
    );
    assert_eq!(answer(&old, &new), Vec::<String>::new());
}

#[test]
fn a_field_removed_breaks_an_answer_and_an_input() {
    let old = object(
        &json!({ "frames": { "type": "integer" }, "dropped": { "type": "integer" } }),
        &["frames"],
    );
    let new = object(&json!({ "frames": { "type": "integer" } }), &["frames"]);
    assert_eq!(
        answer(&old, &new),
        ["scene.output.json: dropped: the field is gone"]
    );
    assert_eq!(
        input(&old, &new),
        ["run.input.json: dropped: the field is gone"]
    );
}

#[test]
fn a_renamed_field_is_one_gone() {
    let old = object(&json!({ "frames": { "type": "integer" } }), &["frames"]);
    let new = object(&json!({ "steps": { "type": "integer" } }), &["steps"]);
    assert_eq!(
        answer(&old, &new),
        ["scene.output.json: frames: the field is gone"]
    );
}

#[test]
fn an_input_may_gain_an_optional_field_and_not_a_required_one() {
    let old = object(&json!({ "script": { "type": "string" } }), &["script"]);
    let optional = object(
        &json!({ "script": { "type": "string" }, "seed": { "type": ["integer", "null"] } }),
        &["script"],
    );
    assert_eq!(input(&old, &optional), NONE);
    let required = object(
        &json!({ "script": { "type": "string" }, "seed": { "type": "integer" } }),
        &["script", "seed"],
    );
    assert_eq!(
        input(&old, &required),
        ["run.input.json: seed: a new field is required"]
    );
    let made_required = object(
        &json!({ "script": { "type": "string" }, "seed": { "type": ["integer", "null"] } }),
        &["script", "seed"],
    );
    assert_eq!(
        input(&optional, &made_required),
        ["run.input.json: seed: the field is now required"]
    );
}

#[test]
fn an_answers_field_that_may_be_absent_breaks() {
    let old = object(&json!({ "frames": { "type": "integer" } }), &["frames"]);
    let new = object(&json!({ "frames": { "type": "integer" } }), &[]);
    assert_eq!(
        answer(&old, &new),
        ["scene.output.json: frames: the field may now be absent"]
    );
    // An input's field made optional takes what it took.
    assert_eq!(input(&old, &new), NONE);
}

#[test]
fn a_type_changed_breaks() {
    let old = object(&json!({ "frames": { "type": "integer" } }), &["frames"]);
    let new = object(&json!({ "frames": { "type": "string" } }), &["frames"]);
    assert_eq!(
        answer(&old, &new),
        ["scene.output.json: frames: it can now be string"]
    );
    assert_eq!(
        input(&old, &new),
        ["run.input.json: frames: it no longer takes integer"]
    );
}

#[test]
fn null_breaks_an_answer_and_widens_an_input() {
    let old = object(&json!({ "file": { "type": "string" } }), &["file"]);
    let new = object(
        &json!({ "file": { "type": ["string", "null"] } }),
        &["file"],
    );
    assert_eq!(
        answer(&old, &new),
        ["scene.output.json: file: it can now be null"]
    );
    assert_eq!(input(&old, &new), NONE);
}

#[test]
fn values_may_come_and_never_go() {
    let old = json!({ "type": "string", "enum": ["pass", "fail"] });
    let more = json!({ "type": "string", "enum": ["pass", "fail", "skip"] });
    assert_eq!(answer(&old, &more), NONE);
    assert_eq!(input(&old, &more), NONE);
    let fewer = json!({ "type": "string", "enum": ["pass"] });
    assert_eq!(
        answer(&old, &fewer),
        ["scene.output.json: (the root): the value \"fail\" is gone"]
    );
}

/// A tagged variant `{ "kind": <tag>, <fields> }`.
fn variant(tag: &str, fields: &Value) -> Value {
    let mut props = fields.as_object().cloned().unwrap_or_default();
    props.insert("kind".to_owned(), json!({ "type": "string", "const": tag }));
    let mut required: Vec<String> = props.keys().cloned().collect();
    required.sort();
    json!({ "type": "object", "properties": props, "required": required })
}

#[test]
fn tagged_variants_match_by_their_tag() {
    let screen = variant("screen", &json!({ "name": { "type": "string" } }));
    let quit = variant("quit", &json!({}));
    let old = json!({ "oneOf": [screen, quit] });
    // Reordered, and a new variant: nothing breaks.
    let music = variant("music", &json!({}));
    let new = json!({ "oneOf": [quit, music, screen] });
    assert_eq!(answer(&old, &new), NONE);
    // A variant's field gone breaks, at the variant.
    let bare = variant("screen", &json!({}));
    let new = json!({ "oneOf": [bare, quit] });
    assert_eq!(
        answer(&old, &new),
        ["scene.output.json: name: the field is gone"]
    );
    // A variant gone breaks.
    let new = json!({ "oneOf": [screen] });
    assert_eq!(
        answer(&old, &new),
        ["scene.output.json: (the root): the variant kind=\"quit\" is gone"]
    );
}

#[test]
fn refs_are_followed_and_recursion_ends() {
    let tree = |extra: bool| {
        let mut props =
            json!({ "children": { "type": "array", "items": { "$ref": "#/$defs/Node" } } });
        if extra {
            props["name"] = json!({ "type": "string" });
        }
        json!({
            "$ref": "#/$defs/Node",
            "$defs": { "Node": { "type": "object", "properties": props, "required": ["children"] } }
        })
    };
    assert_eq!(answer(&tree(false), &tree(true)), NONE);
    assert_eq!(
        answer(&tree(true), &tree(false)),
        ["scene.output.json: name: the field is gone"]
    );
}

#[test]
fn an_option_made_of_a_field_breaks_an_answer() {
    let old = object(&json!({ "result": { "$ref": "#/$defs/R" } }), &["result"]);
    let mut old = old;
    old["$defs"] = json!({ "R": { "type": "object", "properties": {}, "required": [] } });
    let mut new = object(
        &json!({ "result": { "anyOf": [{ "$ref": "#/$defs/R" }, { "type": "null" }] } }),
        &["result"],
    );
    new["$defs"] = old["$defs"].clone();
    assert_eq!(
        answer(&old, &new),
        ["scene.output.json: result: it can now be null"]
    );
    assert_eq!(input(&old, &new), NONE);
}

#[test]
fn descriptions_and_defaults_are_not_the_shape() {
    let old = json!({ "type": "integer", "description": "Frames.", "default": 1 });
    let new = json!({ "type": "integer", "description": "The frames.", "default": 2 });
    assert_eq!(answer(&old, &new), NONE);
}

#[test]
fn another_keyword_changed_breaks() {
    let old = json!({ "type": "integer", "format": "uint32", "minimum": 0 });
    let new = json!({ "type": "integer", "format": "uint64", "minimum": 0 });
    assert_eq!(
        answer(&old, &new),
        ["scene.output.json: (the root): `format` changed from \"uint32\" to \"uint64\""]
    );
}

#[test]
fn a_file_gone_breaks_and_a_new_one_does_not() {
    let schema = json!({ "type": "object" });
    let old = BTreeMap::from([("scene.output.json".to_owned(), schema.clone())]);
    let new = BTreeMap::from([("perf.output.json".to_owned(), schema)]);
    assert_eq!(
        breaks_between(&old, &new),
        ["scene.output.json: the schema is gone"]
    );
    assert_eq!(breaks_between(&BTreeMap::new(), &new), NONE);
}

#[test]
fn the_checked_in_schemas_break_nothing_against_themselves() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../cli/schemas");
    let schemas = super::super::schemas_in(&dir).unwrap();
    assert!(schemas.len() > 20, "{}", schemas.len());
    assert_eq!(breaks_between(&schemas, &schemas), NONE);
}
