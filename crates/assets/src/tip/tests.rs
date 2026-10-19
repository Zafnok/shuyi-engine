//! Tests of the tip schema, with a made-up game's triggers.

use serde::Deserialize;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
enum Trigger {
    Jumped,
    Fell,
    Swam,
}

const LIMITS: TipLimits = TipLimits {
    text_lines: 2,
    line_chars: 10,
    title_chars: 6,
};

fn rules_with<'a>(
    placeholder: &'a dyn Fn(&str) -> bool,
    face_problem: &'a dyn Fn(&str) -> Option<String>,
) -> TipRules<'a> {
    TipRules {
        limits: LIMITS,
        placeholder,
        face_problem,
        default_face: "calm",
    }
}

/// Every error for tips `source`: placeholders `{Jump}` only, and no `sad`
/// face.
fn errors(source: &str) -> Vec<String> {
    let placeholder = |name: &str| name == "Jump";
    let face = |face: &str| (face == "sad").then(|| format!("no face \"{face}\""));
    from_source::<Trigger>("t.ron", source, &rules_with(&placeholder, &face))
        .err()
        .unwrap_or_default()
        .into_iter()
        .map(|e| e.message)
        .collect()
}

#[test]
fn a_good_file_loads_in_order() {
    let source = r#"(tips: [
        (id: "jump", trigger: Jumped, title: "Jump", text: "Press\n{Jump}"),
        (id: "fall", trigger: Fell, title: "Fall", text: "Ouch", face: "wince"),
    ])"#;
    let yes = |_: &str| true;
    let none = |_: &str| None;
    let table = from_source::<Trigger>("t.ron", source, &rules_with(&yes, &none)).unwrap();
    assert_eq!(table.tips.len(), 2);
    assert_eq!(
        table.for_trigger(Trigger::Fell).map(|t| t.id.as_str()),
        Some("fall")
    );
    assert_eq!(table.for_trigger(Trigger::Swam), None);
    assert_eq!(table.tips[0].face_or("calm"), "calm");
    assert_eq!(table.tips[1].face_or("calm"), "wince");
    assert!(TipTable::<Trigger>::default().tips.is_empty());
}

#[test]
fn every_problem_is_reported() {
    let source = r#"(tips: [
        (id: "Bad", trigger: Jumped, title: "", text: ""),
        (id: "Bad", trigger: Jumped, title: "Too long", text: "a\nb\nc"),
        (id: "c", trigger: Fell, title: "T", text: "much too long\n{Run} {Jump}", face: "sad"),
    ])"#;
    assert_eq!(
        errors(source),
        [
            "tip \"Bad\": the id must be lowercase letters, digits and `_`",
            "tip \"Bad\": the title must be 1 to 6 characters",
            "tip \"Bad\": the text must have 1 to 2 lines",
            "tip \"Bad\": the id must be lowercase letters, digits and `_`",
            "tip \"Bad\": the id is used twice",
            "tip \"Bad\": Jumped already has a tip",
            "tip \"Bad\": the title must be 1 to 6 characters",
            "tip \"Bad\": the text must have 1 to 2 lines",
            "tip \"c\": a text line is over 10 characters: \"much too long\"",
            "tip \"c\": a text line is over 10 characters: \"{Run} {Jump}\"",
            "tip \"c\": unknown placeholder {Run}",
            "tip \"c\": no face \"sad\"",
        ]
    );
    // The default face is asked about too, and an empty id is bad.
    let source = r#"(tips: [(id: "", trigger: Jumped, title: "T", text: "x")])"#;
    let placeholder = |_: &str| true;
    let face = |face: &str| (face == "calm").then(|| "no calm".to_owned());
    let errors: Vec<String> =
        from_source::<Trigger>("t.ron", source, &rules_with(&placeholder, &face))
            .err()
            .unwrap_or_default()
            .into_iter()
            .map(|e| e.message)
            .collect();
    assert_eq!(
        errors,
        [
            "tip \"\": the id must be lowercase letters, digits and `_`",
            "tip \"\": no calm",
        ]
    );
}

#[test]
fn limits_count_cells_and_are_inclusive() {
    let source = r#"(tips: [(id: "a_1", trigger: Jumped, title: "Sixsix", text: "Ten chars!\nTen chars!")])"#;
    assert_eq!(errors(source), [] as [&str; 0]);
    let wide = r#"(tips: [(id: "a", trigger: Jumped, title: "Été", text: "ééééééééé")])"#;
    assert_eq!(errors(wide), [] as [&str; 0]);
    assert_eq!(errors("(tips: [").len(), 1);
}

#[test]
fn placeholders_are_the_names_between_braces() {
    assert_eq!(
        placeholders("{Confirm} then {Cancel}"),
        ["Confirm", "Cancel"]
    );
    assert_eq!(placeholders("{a}{b} {"), ["a", "b"]);
    assert_eq!(placeholders("none"), [] as [&str; 0]);
    assert_eq!(placeholders("{open"), [] as [&str; 0]);
}
