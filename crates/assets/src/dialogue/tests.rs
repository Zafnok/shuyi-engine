//! Tests of the dialogue format: the parser, the checks against a game's
//! rules, line ids, who is present, and printing, with a made-up game of
//! the tests' own (a `@weather` directive that stays in force, a `@shake`
//! that doesn't, and a `{hero}` token).

use alloc::collections::BTreeSet;

use proptest::prelude::*;

use super::*;
use crate::names::Names;

/// The limits the tests check scripts and packs against.
pub(crate) const LIMITS: DialogueLimits = DialogueLimits {
    box_width: 40,
    box_lines: 2,
    max_text: 80,
    max_option: 30,
    min_options: 2,
    max_options: 3,
    max_reaction_texts: 4,
};

/// The tests' game's directives.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum Cue {
    /// `@weather <id>`: stays in force until the next one.
    Weather(String),
    /// `@shake`: the screen shakes once.
    Shake,
}

impl Directive for Cue {
    fn parse(name: &str, args: &str) -> Option<Parsed<Self>> {
        match name {
            "weather" if args.is_empty() => Some(Parsed::error("@weather needs a kind")),
            "weather" => {
                let mut parsed = Parsed::step(Cue::Weather(args.to_owned()));
                if !is_id(args) {
                    parsed.errors.push(bad_id(args));
                }
                Some(parsed)
            }
            "shake" => Some(Parsed::step(Cue::Shake)),
            _ => None,
        }
    }

    fn print(&self) -> String {
        match self {
            Cue::Weather(kind) => format!("weather {kind}"),
            Cue::Shake => "shake".to_owned(),
        }
    }

    fn kind(&self) -> Option<&'static str> {
        match self {
            Cue::Weather(_) => Some("weather"),
            Cue::Shake => None,
        }
    }
}

/// The tests' game's token: `{hero}`, at most 10 cells.
pub(crate) struct HeroTokens;

impl Tokens for HeroTokens {
    fn longest(&self, token: &str) -> Option<usize> {
        (token == "hero").then_some(10)
    }

    fn reserved_name(&self, id: &str, _token: &str) -> Option<String> {
        (id == "hero").then(|| "write {hero}".to_owned())
    }

    fn written_name(&self, id: &str, form: &str) -> Option<String> {
        (id == "hero").then(|| format!("\"{form}\" is the hero's; write {{hero}}"))
    }
}

/// The tests' game: who may speak, their expressions, its names and cues.
struct Rules {
    limits: DialogueLimits,
    speakers: Option<BTreeSet<&'static str>>,
    names: Option<Names>,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            limits: LIMITS,
            speakers: None,
            names: None,
        }
    }
}

impl SceneRules<Cue> for Rules {
    fn limits(&self) -> &DialogueLimits {
        &self.limits
    }

    fn tokens(&self) -> &dyn Tokens {
        &HeroTokens
    }

    fn names(&self) -> Option<&Names> {
        self.names.as_ref()
    }

    fn can_speak(&self, character: &CharacterId) -> bool {
        self.speakers
            .as_ref()
            .is_none_or(|s| s.contains(character.0.as_str()))
    }

    fn expression_problem(&self, _character: &CharacterId, expression: &str) -> Option<String> {
        (expression == "smug").then(|| "nobody is smug".to_owned())
    }

    fn speech_problem(&self, speaker: &CharacterId, cells: usize) -> Option<String> {
        (speaker.0 == "hero" && cells > 20).then(|| format!("the hero says {cells} cells"))
    }

    fn music_problem(&self, cue: &str) -> Option<String> {
        (cue != "storm").then(|| format!("no music \"{cue}\""))
    }

    fn scene_problems(&self, parsed: &ParsedScene<Cue>) -> Vec<ContentError> {
        let shakes = parsed
            .scene
            .steps
            .iter()
            .filter(|s| matches!(s, Step::Directive(Cue::Shake)))
            .count();
        (shakes > 1)
            .then(|| ContentError::new(&parsed.file, "one shake a scene"))
            .into_iter()
            .collect()
    }
}

type TStep = Step<Cue>;

fn id(s: &str) -> CharacterId {
    CharacterId(s.into())
}

fn say(speaker: &str, expression: Option<&str>, text: &str) -> TStep {
    Step::Say {
        speaker: id(speaker),
        expression: expression.map(Into::into),
        text: text.into(),
        line: LineId::default(),
    }
}

fn place(side: Side, character: &str, expression: &str) -> TStep {
    Step::Place {
        side,
        character: id(character),
        expression: expression.into(),
    }
}

fn narrate(text: &str) -> TStep {
    Step::Narrate {
        text: text.into(),
        line: LineId::default(),
    }
}

/// `steps` without their line ids, to compare with steps built by hand.
fn bare(steps: &[TStep]) -> Vec<TStep> {
    let mut steps = steps.to_vec();
    for step in &mut steps {
        match step {
            Step::Say { line, .. } | Step::Narrate { line, .. } => *line = LineId::default(),
            Step::Choice { options } => {
                for o in options {
                    o.line = LineId::default();
                    o.steps = bare(&o.steps);
                }
            }
            Step::If {
                then, otherwise, ..
            } => {
                *then = bare(then);
                *otherwise = bare(otherwise);
            }
            _ => {}
        }
    }
    steps
}

/// The scenes of `src`, parsed without checks, and without their line ids.
fn scenes(src: &str) -> Vec<Scene<Cue>> {
    let (parsed, errors) = parse_dlg::<Cue>("t.dlg", src);
    assert_eq!(errors, []);
    parsed
        .into_iter()
        .map(|p| Scene {
            steps: bare(&p.scene.steps),
            ..p.scene
        })
        .collect()
}

/// Every error for one file `t.dlg` under `rules`, as `file:line: message`.
fn errors_with(src: &str, rules: &Rules) -> Vec<String> {
    scripts_from_sources(&[("t.dlg", src)], rules)
        .err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// Every error for one file `t.dlg` under the default rules.
fn errors(src: &str) -> Vec<String> {
    errors_with(src, &Rules::default())
}

/// A valid scene around `body`: the guide on the left, the scout on the
/// right.
fn scene(body: &str) -> String {
    format!("@scene s\n@left guide neutral\n@right scout neutral\n{body}\n@end\n")
}

// --- The fixture ------------------------------------------------------------

/// A script of a game with no gendered hero: plain speakers, its own
/// limits, no game type at all.
const FIXTURE: &str = "\
# A conversation at the gate.
@scene gate
@caption The east gate, dusk
@left mentor neutral
@right scout happy
mentor: Is the road clear?
scout[sad]: Not since noon.
  Riders came over the hill.
> Rain begins to fall.
@music storm
@choice
* earnest: Then we wait.
  mentor: Agreed.
* wry: Then we get wet.
@endchoice
@if scout
scout: I'll keep watch.
@else
> Nobody keeps watch.
@endif
@end
";

/// What the fixture is checked against: its speakers, a narrow box.
struct FixtureRules {
    limits: DialogueLimits,
}

impl SceneRules<NoDirective> for FixtureRules {
    fn limits(&self) -> &DialogueLimits {
        &self.limits
    }

    fn tokens(&self) -> &dyn Tokens {
        &NoTokens
    }

    fn can_speak(&self, character: &CharacterId) -> bool {
        ["mentor", "scout"].contains(&character.0.as_str())
    }
}

#[test]
fn a_fixture_script_parses_and_checks_with_custom_limits() {
    let roomy = FixtureRules {
        limits: DialogueLimits {
            box_width: 30,
            box_lines: 4,
            max_text: 60,
            max_option: 20,
            min_options: 2,
            max_options: 2,
            max_reaction_texts: 1,
        },
    };
    let scripts = scripts_from_sources(&[("story/gate.dlg", FIXTURE)], &roomy).unwrap();
    let scene = scripts.table.get("gate").unwrap();
    assert_eq!(scripts.table.files["gate"], "story/gate.dlg");
    assert_eq!(scene.lines().len(), 8);
    assert_eq!(
        scene.lines()[1].text,
        "Not since noon. Riders came over the hill."
    );
    assert_eq!(scene.steps.len(), 9);
    let gone = scene.resolved(&Present::only([id("mentor")]));
    let last = gone.lines().last().map(|l| l.text.to_owned());
    assert_eq!(last.as_deref(), Some("Nobody keeps watch."));
    // The same script in a smaller box and menu.
    let tight = FixtureRules {
        limits: DialogueLimits {
            max_text: 20,
            max_option: 10,
            max_options: 3,
            min_options: 3,
            ..roomy.limits
        },
    };
    let errors: Vec<String> = scripts_from_sources(&[("gate.dlg", FIXTURE)], &tight)
        .err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        errors,
        [
            "gate.dlg:7: text is 42 characters; the limit is 20",
            "gate.dlg:11: @choice has 2 options; it needs 3",
            "gate.dlg:12: option text is 13 characters; the limit is 10, so it fits the menu",
            "gate.dlg:14: option text is 16 characters; the limit is 10, so it fits the menu",
        ]
    );
}

/// Rules that keep every default: anyone speaks, no names table.
struct Minimal;

impl SceneRules<NoDirective> for Minimal {
    fn limits(&self) -> &DialogueLimits {
        &LIMITS
    }

    fn tokens(&self) -> &dyn Tokens {
        &NoTokens
    }
}

/// Rules with a names table and no tokens of the game's.
struct Named(Names);

impl SceneRules<NoDirective> for Named {
    fn limits(&self) -> &DialogueLimits {
        &LIMITS
    }

    fn tokens(&self) -> &dyn Tokens {
        &NoTokens
    }

    fn names(&self) -> Option<&Names> {
        Some(&self.0)
    }
}

/// A directive that keeps the trait's defaults: it doesn't stay in force.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Beep;

impl Directive for Beep {
    fn parse(name: &str, _args: &str) -> Option<Parsed<Self>> {
        (name == "beep").then(|| Parsed::step(Beep))
    }

    fn print(&self) -> String {
        "beep".to_owned()
    }
}

/// The messages of `src` under `rules`.
fn plain_errors(src: &str, rules: &impl SceneRules<NoDirective>) -> Vec<String> {
    scripts_from_sources(&[("t.dlg", src)], rules)
        .err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn the_rules_defaults_let_everything_through() {
    let src = "@scene a\n@left anyone neutral\nanyone: Hi {n:any_id}.\n@music what\n@end\n";
    assert_eq!(plain_errors(src, &Minimal), [] as [&str; 0]);
    let got = Minimal.scene_problems(&parse_dlg("t.dlg", src).0[0]);
    assert!(got.is_empty(), "{got:?}");
    // Without the game's advice, a name written out gets the engine's.
    let names = Names {
        names: [("smith".to_owned(), "Ada".to_owned())].into(),
    };
    assert_eq!(
        plain_errors("@scene a\n> Ada waits.\n@end\n", &Named(names)),
        [
            "t.dlg:2: \"Ada\" is written out; write {n:smith} (\"Ada\") so a rename reaches this line"
        ]
    );
    assert_eq!(NoTokens.written_name("smith", "Ada"), None);
    assert_eq!(NoTokens.reserved_name("smith", "n:smith"), None);
    // A directive that doesn't say otherwise doesn't stay in force.
    assert_eq!(Beep.kind(), None);
    let (parsed, errors) = parse_dlg::<Beep>("t.dlg", "@scene a\n@beep\n> x\n@end\n");
    assert_eq!(errors, []);
    assert_eq!(
        print_scene(&parsed[0].scene),
        "@scene a\n@beep\n> x\n@end\n"
    );
}

#[test]
fn a_reply_may_be_as_long_as_the_menu() {
    let exact = "x".repeat(LIMITS.max_option);
    assert_eq!(
        errors(&scene(&format!(
            "@choice\n* a: {exact}\n* b: y\n@endchoice\n> z"
        ))),
        [] as [&str; 0]
    );
}

// --- Parsing ----------------------------------------------------------------

#[test]
fn parses_every_line_type() {
    let src = "\
# a comment
@scene ch01

@caption Village of Ost, dusk
  # an indented comment
@left  mentor neutral
@right scout angry
scout: You're late.
mentor[happy]: Better late: than... never.
>   The rain had not stopped.
@right clear
@weather rain
@shake
@music stop
@end
";
    assert_eq!(
        scenes(src),
        [Scene {
            id: "ch01".into(),
            steps: vec![
                Step::Caption {
                    text: "Village of Ost, dusk".into()
                },
                place(Side::Left, "mentor", "neutral"),
                place(Side::Right, "scout", "angry"),
                say("scout", None, "You're late."),
                say("mentor", Some("happy"), "Better late: than... never."),
                narrate("The rain had not stopped."),
                Step::Clear { side: Side::Right },
                Step::Directive(Cue::Weather("rain".into())),
                Step::Directive(Cue::Shake),
                Step::Music(MusicLine::Stop),
            ],
        }]
    );
}

#[test]
fn records_the_line_of_every_step() {
    let (parsed, _) = parse_dlg::<Cue>("t.dlg", "\n@scene a\n\n> one\n  more\nx: two\n@end\n");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].line, 2);
    assert_eq!(parsed[0].lines.steps, [4, 6]);
    assert_eq!(parsed[0].file, "t.dlg");
}

#[test]
fn continuation_lines_join_with_one_space() {
    let src = "@scene a\nx: One\n  two   \n     three\n> Four\n  five\n\n  six\n@end\n";
    assert_eq!(
        scenes(src)[0].steps,
        [say("x", None, "One two three"), narrate("Four five six")]
    );
}

#[test]
fn several_scenes_per_file_and_crlf() {
    let src = "@scene a\r\n> One\r\n@end\r\n\r\n@scene b\r\n> Two\r\n  more\r\n@end";
    let ids: Vec<String> = scenes(src).into_iter().map(|s| s.id).collect();
    assert_eq!(ids, ["a", "b"]);
    assert_eq!(scenes(src)[1].steps, [narrate("Two more")]);
}

#[test]
fn a_games_directive_is_read_by_the_game() {
    assert_eq!(
        errors(&scene("@weather Rain\n@weather\n> x")),
        [
            "t.dlg:4: \"Rain\" is not a valid id; use lowercase letters, digits and _",
            "t.dlg:5: @weather needs a kind",
        ]
    );
    // Without the game's directives, the same line is unknown.
    let (_, errors) = parse_dlg::<NoDirective>("t.dlg", "@scene s\n@weather rain\n> x\n@end\n");
    let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
    assert_eq!(errors, ["t.dlg:2: unknown directive \"@weather\""]);
    // A directive outside a scene has nowhere to go.
    let (_, errors) = parse_dlg::<Cue>("t.dlg", "@shake\n");
    let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
    assert_eq!(
        errors,
        ["t.dlg:1: line is outside a scene; start one with @scene <id>"]
    );
}

#[test]
fn unknown_directive() {
    assert_eq!(
        errors(&scene("@bogus x\n> x")),
        ["t.dlg:4: unknown directive \"@bogus\""]
    );
}

#[test]
fn directive_arguments() {
    let e = errors(
        "@scene\n> x\n@end\n@scene a b\n> x\n@end x\n@scene c\n@left one\n@right x y z\n\
         @caption\n@music\n@music a b\n@music Storm\n> y\n@end\n",
    );
    assert_eq!(
        e,
        [
            "t.dlg:1: @scene needs one scene id",
            "t.dlg:4: @scene needs one scene id",
            "t.dlg:6: @end takes nothing after it",
            "t.dlg:8: @left needs a character id and an expression, or \"clear\"",
            "t.dlg:9: @right needs a character id and an expression, or \"clear\"",
            "t.dlg:10: @caption has no text",
            "t.dlg:11: @music needs one music cue id, or \"stop\"",
            "t.dlg:12: @music needs one music cue id, or \"stop\"",
            "t.dlg:13: \"Storm\" is not a valid id; use lowercase letters, digits and _",
            "t.dlg:13: no music \"Storm\"",
        ]
    );
}

#[test]
fn invalid_ids() {
    let e = errors("@scene Bad\n@left A-b Happy\nA-b: x\nx[Y]: z\n@end\n");
    let bad = |id: &str| format!("\"{id}\" is not a valid id; use lowercase letters, digits and _");
    assert_eq!(
        e,
        [
            format!("t.dlg:1: {}", bad("Bad")),
            format!("t.dlg:2: {}", bad("A-b")),
            format!("t.dlg:2: {}", bad("Happy")),
            format!("t.dlg:3: {}", bad("A-b")),
            format!("t.dlg:4: {}", bad("Y")),
            "t.dlg:4: \"x\" speaks but is not on screen; place them with @left or @right first"
                .to_owned(),
        ]
    );
    assert!(is_id("a_1") && !is_id("") && !is_id("a b") && !is_id("A"));
}

#[test]
fn unrecognised_lines_and_empty_text() {
    assert_eq!(
        errors(&scene("just words\nscout:\n>\n@caption x")),
        [
            "t.dlg:1: scene \"s\" has no speech or narration",
            "t.dlg:4: unrecognised line; expected \"@directive\", \"> narration\", \"speaker: text\" or \"# comment\"",
            "t.dlg:5: \"scout:\" has no text",
            "t.dlg:6: narration has no text",
        ]
    );
}

#[test]
fn misplaced_indented_lines() {
    assert_eq!(
        errors(&scene("  @left clear\n@left clear\n  more\n> x")),
        [
            "t.dlg:4: directives can't be indented",
            "t.dlg:6: indented line doesn't continue a speech or narration line",
        ]
    );
    // Outside a scene an indented line continues nothing.
    assert_eq!(
        errors("  x\n@scene a\n> x\n@end\n"),
        ["t.dlg:1: indented line doesn't continue a speech or narration line"]
    );
}

#[test]
fn lines_outside_scenes_and_missing_ends() {
    assert_eq!(
        errors("> x\n@end\n@scene a\n> y\n@scene b\n> z\n"),
        [
            "t.dlg:1: line is outside a scene; start one with @scene <id>",
            "t.dlg:2: @end without an open @scene",
            "t.dlg:3: scene \"a\" has no @end",
            "t.dlg:5: scene \"b\" has no @end",
        ]
    );
}

#[test]
fn non_ascii_text_names_its_column() {
    let (_, errors) = parse_dlg::<Cue>(
        "t.dlg",
        "@scene a\n> It\u{2019}s \u{e9}\n  \u{2026}\n@end\n",
    );
    let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
    assert_eq!(
        errors,
        [
            "t.dlg:2:5: non-ASCII punctuation '\u{2019}'; use ' instead",
            "t.dlg:2:8: unsupported character '\u{e9}'; dialogue text is plain ASCII",
            "t.dlg:3:3: non-ASCII punctuation '\u{2026}'; use ... instead",
        ]
    );
    for (c, ascii) in [
        ('\u{201C}', "\""),
        ('\u{2014}', "--"),
        ('\u{2013}', "-"),
        ('\u{2018}', "'"),
        ('\u{201D}', "\""),
    ] {
        assert_eq!(
            char_problem(c),
            Some(format!("non-ASCII punctuation {c:?}; use {ascii} instead"))
        );
    }
    assert_eq!(char_problem(' '), None);
    assert_eq!(char_problem('~'), None);
    assert!(char_problem('\u{7f}').is_some());
}

// --- Choices and @if blocks -------------------------------------------------

#[test]
fn choices_parse_with_their_reactions() {
    let src =
        "@scene a\n@choice\n* earnest: Yes.\n  > Good.\n* wry: No.\n@endchoice\n> End.\n@end\n";
    let steps = &scenes(src)[0].steps;
    let Step::Choice { options } = &steps[0] else {
        panic!("{steps:?}");
    };
    assert_eq!(options.len(), 2);
    assert_eq!(
        (options[0].tone.as_str(), options[0].text.as_str()),
        ("earnest", "Yes.")
    );
    assert_eq!(options[0].steps, [narrate("Good.")]);
    assert_eq!(options[1].steps, []);
    assert_eq!(steps[1], narrate("End."));
}

#[test]
fn choice_structure_errors() {
    assert_eq!(
        errors(&scene(
            "* x: y\n@choice z\n* earnest\n* Bad: y\n > one space\n@choice\n* a: b\n* c: d\nwords\n@endchoice x\n> e\n@endchoice"
        )),
        [
            "t.dlg:4: an option (\"* tone: text\") must be inside a @choice block",
            "t.dlg:5: @choice takes nothing after it",
            "t.dlg:6: an option needs a tone and text: \"* tone: text\"",
            "t.dlg:7: \"Bad\" is not a valid id; use lowercase letters, digits and _",
            "t.dlg:8: reaction lines are indented by two spaces",
            "t.dlg:9: @choice can't be nested inside another @choice",
            "t.dlg:12: inside @choice, a line is an option (\"* tone: text\"), an indented reaction line or @endchoice",
            "t.dlg:13: @endchoice takes nothing after it",
            "t.dlg:15: @endchoice without an open @choice",
        ]
    );
    assert_eq!(
        errors("@choice\n@scene a\n@choice\n* a: b\n  > x\n* c: d\n@end\n"),
        [
            "t.dlg:1: line is outside a scene; start one with @scene <id>",
            "t.dlg:3: @choice has no @endchoice",
        ]
    );
    assert_eq!(
        errors(&scene(
            "@choice\n  > early\n* a: b\n* c: d\n@endchoice\n> x"
        )),
        ["t.dlg:5: a reaction line needs an option above it: \"* tone: text\""]
    );
}

#[test]
fn if_blocks_parse_with_their_else() {
    let src =
        "@scene a\n@if scout\n> here\n@else\n> gone\n@endif\n@if guide\n> only\n@endif\n@end\n";
    let steps = &scenes(src)[0].steps;
    assert_eq!(
        steps,
        &[
            Step::If {
                character: id("scout"),
                then: vec![narrate("here")],
                otherwise: vec![narrate("gone")],
            },
            Step::If {
                character: id("guide"),
                then: vec![narrate("only")],
                otherwise: vec![],
            },
        ]
    );
    let (parsed, _) = parse_dlg::<Cue>("t.dlg", src);
    let lines = &parsed[0].lines;
    assert_eq!(lines.steps, [2, 7]);
    let parts: Vec<Vec<u32>> = lines
        .blocks
        .iter()
        .map(|b| b.iter().map(|p| p.line).collect())
        .collect();
    // Without an `@else`, its part starts at the `@if`.
    assert_eq!(parts, [vec![2, 4], vec![7, 7]]);
}

#[test]
fn if_structure_errors() {
    assert_eq!(
        errors(&scene(
            "@if\n@endif\n@if a b\n@endif\n@if Bad\n@endif\n@else\n@endif\n@if scout\n> x\n@else x\n@else\n@endif y\n@if scout\n> y"
        )),
        [
            "t.dlg:4: @if needs one character id",
            "t.dlg:6: @if needs one character id",
            "t.dlg:8: \"Bad\" is not a valid id; use lowercase letters, digits and _",
            "t.dlg:10: @else without an open @if",
            "t.dlg:11: @endif without an open @if",
            "t.dlg:14: @else takes nothing after it",
            "t.dlg:15: @if already has an @else, on line 14",
            "t.dlg:16: @endif takes nothing after it",
            "t.dlg:17: @if has no @endif",
        ]
    );
    // An `@if` opened in a reaction ends with it.
    assert_eq!(
        errors(&scene(
            "@choice\n* a: b\n  @if scout\n  > x\n* c: d\n@endchoice\n> y"
        )),
        ["t.dlg:6: @if has no @endif"]
    );
    assert_eq!(
        errors("@if scout\n@scene a\n> x\n@end\n"),
        ["t.dlg:1: line is outside a scene; start one with @scene <id>"]
    );
}

// --- Checks -----------------------------------------------------------------

#[test]
fn a_valid_scene_loads() {
    let src = scene("guide[sad]: Hi.\n@left clear\n@left hero happy\nhero: Yo.\n@music storm");
    let rules = Rules {
        speakers: Some(["guide", "scout", "hero"].into()),
        ..Rules::default()
    };
    let scripts = scripts_from_sources(&[("t.dlg", src.as_str())], &rules).unwrap();
    assert_eq!(scripts.table.get("s").map(|s| s.steps.len()), Some(7));
    assert_eq!(scripts.parsed.len(), 1);
    assert!(scripts.table.get("nope").is_none());
}

#[test]
fn who_is_on_screen() {
    assert_eq!(
        errors(&scene(
            "mentor: Hi.\n@left scout happy\n@right clear\nscout: x"
        )),
        [
            "t.dlg:4: \"mentor\" speaks but is not on screen; place them with @left or @right first",
            "t.dlg:5: \"scout\" is already on the right; a character can't be on both sides",
        ]
    );
    let rules = Rules {
        speakers: Some(["guide"].into()),
        ..Rules::default()
    };
    assert_eq!(
        errors_with(&scene("scout: x\n@if nobody\n@endif"), &rules),
        [
            "t.dlg:3: unknown character \"scout\"",
            "t.dlg:4: unknown character \"scout\"",
            "t.dlg:5: unknown character \"nobody\"",
        ]
    );
}

#[test]
fn the_games_hooks_are_asked() {
    assert_eq!(
        errors(&scene(
            "@left guide smug\nscout[smug]: x\n@music calm\n@music storm\n@right hero neutral\n\
             hero: This is far too long for a hero.\nhero: Short.\n@shake\n@shake"
        )),
        [
            "t.dlg: one shake a scene",
            "t.dlg:4: nobody is smug",
            "t.dlg:5: nobody is smug",
            "t.dlg:6: no music \"calm\"",
            "t.dlg:9: the hero says 32 cells",
        ]
    );
}

#[test]
fn text_length_limits() {
    let long = "x".repeat(LIMITS.max_text + 1);
    assert_eq!(
        errors(&scene(&format!("scout: {long}\n> {long}"))),
        [
            "t.dlg:4: text is 81 characters; the limit is 80",
            "t.dlg:5: text is 81 characters; the limit is 80",
        ]
    );
    let exact = "x".repeat(LIMITS.max_text);
    assert_eq!(errors(&scene(&format!("scout: {exact}"))), [] as [&str; 0]);
    // A token counts as its longest.
    let near = "x".repeat(LIMITS.max_text - 9);
    assert_eq!(
        errors(&scene(&format!("> {near}{{hero}}"))),
        ["t.dlg:4: text is 81 characters; the limit is 80"]
    );
}

#[test]
fn a_line_must_fit_the_box_once_its_tokens_are_longest() {
    // 40 wide, 2 lines. "{hero}" is 11 cells at its longest.
    let words = ["a".repeat(24), "b".repeat(24)];
    let short = format!("scout: {} {}", words[0], words[1]);
    assert_eq!(errors(&scene(&short)), [] as [&str; 0]);
    // A third long word takes a third line, though the text is short enough.
    let three = format!("scout: {} {} {}", words[0], words[1], words[0]);
    assert_eq!(
        errors(&scene(&three)),
        ["t.dlg:4: text takes 3 lines of 40 characters; the box shows 2"]
    );
    // A short token leaves the line on two pages; its longest value doesn't.
    let pad = "a".repeat(27);
    let fits = format!("scout: {pad} {} {{hero}}", "b".repeat(10));
    assert_eq!(errors(&scene(&fits)), [] as [&str; 0]);
    let spills = format!("scout: {pad} {} {{hero}}", "b".repeat(31));
    assert_eq!(
        errors(&scene(&spills)),
        ["t.dlg:4: text takes 3 lines of 40 characters; the box shows 2"]
    );
}

#[test]
fn option_limits() {
    let long = "x".repeat(LIMITS.max_option + 1);
    let reaction = "  > a\n".repeat(5);
    assert_eq!(
        errors(&scene(&format!(
            "@choice\n* a: {long}\n{reaction}* b: ok\n@endchoice\n@choice\n* c: one\n@endchoice\n\
             @choice\n* d: 1\n* e: 2\n* f: 3\n* g: 4\n@endchoice"
        ))),
        [
            "t.dlg:5: option text is 31 characters; the limit is 30, so it fits the menu".to_owned(),
            "t.dlg:5: reaction has 5 speech or narration lines; the limit is 4, so the scene rejoins quickly".to_owned(),
            "t.dlg:13: @choice has 1 options; it needs 2 or 3".to_owned(),
            "t.dlg:16: @choice has 4 options; it needs 2 or 3".to_owned(),
            "t.dlg:3: scene \"s\" has no speech or narration".to_owned(),
        ]
        .into_iter()
        .filter(|e| !e.contains("no speech"))
        .collect::<Vec<_>>()
    );
    // An `@if` block counts as its longer part.
    let both = "  @if scout\n  > a\n  > b\n  @else\n  > c\n  @endif\n";
    assert_eq!(
        errors(&scene(&format!(
            "@choice\n* a: x\n{both}  > d\n  > e\n* b: y\n@endchoice"
        ))),
        [] as [&str; 0]
    );
    assert_eq!(
        errors(&scene(&format!(
            "@choice\n* a: x\n{both}  > d\n  > e\n  > f\n* b: y\n@endchoice"
        ))),
        [
            "t.dlg:5: reaction has 5 speech or narration lines; the limit is 4, so the scene rejoins quickly"
        ]
    );
    // A range of more than two counts.
    let wide = Rules {
        limits: DialogueLimits {
            min_options: 1,
            max_options: 4,
            ..LIMITS
        },
        ..Rules::default()
    };
    let five = "@choice\n* a: 1\n* b: 2\n* c: 3\n* d: 4\n* e: 5\n@endchoice\n> x";
    assert_eq!(
        errors_with(&scene(five), &wide),
        ["t.dlg:4: @choice has 5 options; it needs 1 to 4"]
    );
}

#[test]
fn a_scene_needs_text() {
    assert_eq!(
        errors("@scene a\n@left guide neutral\n@choice\n* a: b\n* c: d\n@endchoice\n@end\n"),
        ["t.dlg:1: scene \"a\" has no speech or narration"]
    );
    // Text in a reaction or a block counts.
    assert_eq!(
        errors("@scene a\n@choice\n* a: b\n  > x\n* c: d\n@endchoice\n@end\n"),
        [] as [&str; 0]
    );
    assert_eq!(
        errors("@scene a\n@if x\n@else\n> y\n@endif\n@end\n"),
        [] as [&str; 0]
    );
}

#[test]
fn every_part_of_a_block_leaves_the_same_screen_caption_and_weather() {
    assert_eq!(
        errors(&scene(
            "@if scout\n@left clear\n@endif\n> x\n@if scout\n@caption A\n@else\n@caption B\n@endif\n\
             @if scout\n@weather rain\n@endif\n@if scout\n@weather fog\n@else\n@weather fog\n@shake\n@endif"
        )),
        [
            "t.dlg:4: with \"scout\" here nobody is on the left and scout on the right; without, \
             guide is on the left and scout on the right; an @if block must leave the same \
             characters on screen either way",
            "t.dlg:10: this @if block leaves a different caption with \"scout\" here and without; \
             it must leave the same caption either way",
            "t.dlg:13: this @if block leaves a different @weather with \"scout\" here and without; \
             it must leave the same weather either way",
        ]
    );
    assert_eq!(
        errors(&scene(
            "@choice\n* a: x\n  @right clear\n* b: y\n@endchoice\n@choice\n* a: x\n  @caption A\n* b: y\n@endchoice\n\
             @choice\n* a: x\n  @weather rain\n* b: y\n  @shake\n@endchoice\n> z"
        )),
        [
            "t.dlg:7: after this reaction guide is on the left and scout on the right; after the \
             first option's guide is on the left and nobody on the right; every option must leave \
             the same characters on screen",
            "t.dlg:12: this reaction leaves a different caption from the first option's; every \
             option must leave the same caption",
            "t.dlg:17: this reaction leaves a different @weather from the first option's; every \
             option must leave the same weather",
        ]
    );
    // After a block, its first part's state holds.
    assert_eq!(
        errors(&scene(
            "@if scout\n@right clear\n@else\n@right clear\n@endif\nscout: x"
        )),
        ["t.dlg:9: \"scout\" speaks but is not on screen; place them with @left or @right first"]
    );
}

#[test]
fn tokens_and_names() {
    let names = Names {
        names: [("smith", "Ada Brook"), ("hero", "Ed")]
            .map(|(i, n)| (i.to_owned(), n.to_owned()))
            .into(),
    };
    let rules = Rules {
        names: Some(names),
        ..Rules::default()
    };
    assert_eq!(
        errors_with(
            &scene(
                "> {hero} {x} {n:nobody} {n:Bad} {n:hero} {\n> Ada Brook and Ed.\n@caption {n:nobody}"
            ),
            &rules
        ),
        [
            "t.dlg:4: unknown token \"{x}\"; use a name, {n:<id>}",
            "t.dlg:4: unknown name id \"nobody\" in {n:nobody}",
            "t.dlg:4: unknown name id \"Bad\" in {n:Bad}",
            "t.dlg:4: write {hero}",
            "t.dlg:4: \"{\" has no closing \"}\"",
            "t.dlg:5: \"Ada Brook\" is written out; write {n:smith} (\"Ada Brook\") so a rename reaches this line",
            "t.dlg:6: unknown name id \"nobody\" in {n:nobody}",
        ]
    );
    assert_eq!(
        errors_with(&scene("> Ed waits."), &rules),
        ["t.dlg:4: \"Ed\" is the hero's; write {hero}"]
    );
    // Without a table any valid id goes, and names aren't looked for.
    assert_eq!(errors(&scene("> {n:nobody} Ada Brook")), [] as [&str; 0]);
    // A name token counts as the longest name.
    let long = Names {
        names: [("a".to_owned(), "x".repeat(70))].into(),
    };
    assert_eq!(
        longest_width("{n:a} {n:b}", &NoTokens, Some(long.longest())),
        141
    );
    assert_eq!(longest_width("{n:a} {zz}", &NoTokens, None), 10);
    assert_eq!(longest_width("{hero}!", &HeroTokens, None), 11);
    assert_eq!(longest_width("{a", &HeroTokens, None), 2);
    assert_eq!(
        token_problems("{n:a} {b}", &NoTokens, Some(&long)),
        ["unknown token \"{b}\"; use a name, {n:<id>}"]
    );
}

#[test]
fn reply_names_and_tokens_are_checked_too() {
    let names = Names {
        names: [("smith".to_owned(), "Ada".to_owned())].into(),
    };
    let rules = Rules {
        names: Some(names),
        ..Rules::default()
    };
    assert_eq!(
        errors_with(
            &scene("@choice\n* a: Ada {x}\n* b: y\n@endchoice\n> z"),
            &rules
        ),
        [
            "t.dlg:5: unknown token \"{x}\"; use a name, {n:<id>}",
            "t.dlg:5: \"Ada\" is written out; write {n:smith} (\"Ada\") so a rename reaches this line",
        ]
    );
}

#[test]
fn line_ids_that_clash_are_reported() {
    // Two different lines with the same hash (found by search).
    let a = "Count 598878.";
    let b = "Count 655542.";
    assert_eq!(line_hash("guide", a), line_hash("guide", b));
    assert_eq!(
        errors(&scene(&format!("guide: {a}\nguide: {b}\nguide: {a}"))),
        [format!(
            "t.dlg:5: this line and line 4 (\"{a}\") get the same line id hash {}; reword one of them",
            line_hash("guide", a)
        )]
    );
}

#[test]
fn duplicate_scene_ids_across_files_and_line_order() {
    let a = "@scene s\n> x\n@end\n";
    let b = "\n@scene s\n> y\n@end\n@scene t\n> {\n@end\n";
    let errors: Vec<String> =
        scripts_from_sources(&[("a.dlg", a), ("b.dlg", b)], &Rules::default())
            .err()
            .unwrap_or_default()
            .iter()
            .map(ToString::to_string)
            .collect();
    assert_eq!(
        errors,
        [
            "b.dlg:2: duplicate scene id \"s\"; first used at a.dlg:1",
            "b.dlg:6: \"{\" has no closing \"}\"",
        ]
    );
}

#[test]
fn files_that_are_not_utf8() {
    let ok = "@scene a\n> x\n@end\n";
    let bad = "@scene b\n@end\n";
    let load = |files: &[(&str, Option<&str>)]| {
        let files: Vec<(String, Option<&str>)> =
            files.iter().map(|&(f, s)| (f.to_owned(), s)).collect();
        scripts_from_files(&files, &Rules::default())
            .map_err(|e| e.iter().map(ToString::to_string).collect::<Vec<_>>())
    };
    assert_eq!(
        load(&[("a.dlg", Some(ok))]).map(|t| (t.table.scenes.len(), t.parsed.len())),
        Ok((1, 1))
    );
    assert_eq!(
        load(&[("a.dlg", Some(ok)), ("x.dlg", None)]).map(|t| t.table.scenes.len()),
        Err(vec!["x.dlg: file is not valid UTF-8".to_owned()])
    );
    assert_eq!(
        load(&[("x.dlg", None), ("b.dlg", Some(bad))]).map(|t| t.table.scenes.len()),
        Err(vec![
            "x.dlg: file is not valid UTF-8".to_owned(),
            "b.dlg:1: scene \"b\" has no speech or narration".to_owned(),
        ])
    );
}

// --- Steps, sides and music ---------------------------------------------------

#[test]
fn sides_steps_and_music() {
    assert_eq!(Side::Left.name(), "left");
    assert_eq!(Side::Right.name(), "right");
    assert_eq!(Side::Left.other(), Side::Right);
    assert_eq!(Side::Right.other(), Side::Left);
    assert_eq!(say("a", None, "x").text(), Some("x"));
    assert_eq!(narrate("y").text(), Some("y"));
    for step in [
        Step::Caption { text: "c".into() },
        place(Side::Left, "a", "b"),
        Step::Clear { side: Side::Left },
        Step::Music(MusicLine::Stop),
        Step::Directive(Cue::Shake),
        Step::Choice { options: vec![] },
        Step::If {
            character: id("a"),
            then: vec![narrate("x")],
            otherwise: vec![],
        },
    ] {
        assert_eq!(step.text(), None, "{step:?}");
    }
    let option = |steps| ChoiceOption {
        tone: "a".into(),
        text: "b".into(),
        line: LineId::default(),
        steps,
    };
    assert!(
        Step::<Cue>::Choice {
            options: vec![option(vec![narrate("x")])]
        }
        .has_text()
    );
    assert!(
        !Step::<Cue>::Choice {
            options: vec![option(vec![])]
        }
        .has_text()
    );
    let when = |then, otherwise| Step::<Cue>::If {
        character: id("a"),
        then,
        otherwise,
    };
    assert!(when(vec![], vec![narrate("x")]).has_text());
    assert!(when(vec![narrate("x")], vec![]).has_text());
    assert!(!when(vec![], vec![]).has_text());
    assert_eq!(MusicLine::Cue("storm".into()).arg(), "storm");
    assert_eq!(MusicLine::Stop.arg(), "stop");
    assert_eq!(Parsed::<Cue>::error("e").step, None);
    assert_eq!(Cue::parse("nope", ""), None);
    assert_eq!(Cue::Shake.kind(), None);
}

#[test]
fn a_table_and_scripts_start_empty() {
    let table = DialogueTable::<Cue>::default();
    assert!(table.scenes.is_empty(), "{:?}", table.scenes);
    assert!(table.files.is_empty(), "{:?}", table.files);
    let scripts = Scripts::<Cue>::default();
    assert!(scripts.parsed.is_empty(), "{:?}", scripts.parsed);
    assert!(
        scripts.table.scenes.is_empty(),
        "{:?}",
        scripts.table.scenes
    );
    assert_eq!(Scene::<Cue>::default().id, "");
    assert!(NoDirective::parse("weather", "x").is_none());
    assert_eq!(NoTokens.longest("hero"), None);
}

// --- Who is present -----------------------------------------------------------

#[test]
fn a_scene_resolves_for_those_present() {
    let src = "@scene a\n@if scout\n> here\n@else\n> gone\n@endif\n@choice\n* a: b\n  @if guide\n  > g\n  @endif\n* c: d\n@endchoice\n@end\n";
    let (parsed, _) = parse_dlg::<Cue>("t.dlg", src);
    let scene = &parsed[0].scene;
    let texts = |present: &Present| -> Vec<String> {
        let resolved = scene.resolved(present);
        assert_eq!(resolved.id, "a");
        resolved.lines().iter().map(|l| l.text.to_owned()).collect()
    };
    assert_eq!(texts(&Present::Everyone), ["here", "b", "g", "d"]);
    assert_eq!(
        texts(&Present::only([id("guide")])),
        ["gone", "b", "g", "d"]
    );
    assert_eq!(texts(&Present::only([])), ["gone", "b", "d"]);
    // The lines keep their ids.
    let all = scene.lines();
    let resolved = scene.resolved(&Present::Everyone);
    assert_eq!(resolved.lines()[0].id, all[0].id);
    assert!(Present::Everyone.has(&id("anyone")));
    assert!(!Present::only([id("a")]).has(&id("b")));
    assert!(resolved.has_text());
    let quiet = Scene::<Cue> {
        id: "q".into(),
        steps: vec![Step::If {
            character: id("x"),
            then: vec![narrate("x")],
            otherwise: vec![],
        }],
    };
    assert!(quiet.has_text());
    assert!(!quiet.resolved(&Present::only([])).has_text());
}

// --- Line ids -----------------------------------------------------------------

/// The published FNV-1a 64 test vectors.
#[test]
fn fnv1a64_matches_its_test_vectors() {
    use super::line_id::fnv1a64;
    assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
    assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
}

/// Ids are stored in translation and voice files, so these strings must
/// never change.
#[test]
fn the_hash_of_known_lines_is_fixed() {
    assert_eq!(line_hash("a", "b"), "0415e670");
    assert_eq!(line_hash(NARRATION_SPEAKER, "Hello."), "10c29f83");
    assert_eq!(line_hash(REPLY_SPEAKER, "Hello."), "bc2c818f");
    assert_eq!(
        caption_key("s", "Dusk"),
        format!("caption.s_{}", line_hash("@caption", "Dusk"))
    );
}

#[test]
fn every_line_gets_an_id_in_script_order() {
    let src = "@scene s\n@left guide neutral\nguide: Go.\n> Go.\n@choice\n* a: Go.\n  guide: Go.\n* b: No.\n@endchoice\n@if guide\nguide: Go.\n@else\n> Go.\n@endif\n@end\n";
    let (parsed, _) = parse_dlg::<Cue>("t.dlg", src);
    let ids: Vec<String> = parsed[0]
        .scene
        .lines()
        .iter()
        .map(|l| l.id.to_string())
        .collect();
    let guide = line_hash("guide", "Go.");
    let narration = line_hash(NARRATION_SPEAKER, "Go.");
    assert_eq!(
        ids,
        [
            format!("s_{guide}"),
            format!("s_{narration}"),
            format!("s_{}", line_hash(REPLY_SPEAKER, "Go.")),
            format!("s_{guide}_2"),
            format!("s_{}", line_hash(REPLY_SPEAKER, "No.")),
            format!("s_{guide}_3"),
            format!("s_{narration}_2"),
        ]
    );
    let speakers: Vec<&str> = parsed[0].scene.lines().iter().map(|l| l.speaker).collect();
    assert_eq!(speakers, ["guide", ">", "*", "guide", "*", "guide", ">"]);
    // Building by hand, then assigning.
    let mut scene = Scene::<Cue> {
        id: "h".into(),
        steps: vec![narrate("x")],
    };
    scene.assign_line_ids();
    assert_eq!(
        scene.lines()[0].id.as_str(),
        format!("h_{}", line_hash(">", "x"))
    );
}

#[test]
fn a_line_id_names_its_scene() {
    let scene_id = |id: &str| LineId::new(id).scene_id().to_owned();
    assert_eq!(scene_id("s_9e73f1df"), "s");
    assert_eq!(scene_id("gate_talk_9e73f1df_12"), "gate_talk");
    assert_eq!(scene_id("turn_3_d962db86"), "turn_3");
    for odd in [
        "s",
        "s_9e73f1d",
        "s_9e73f1dg",
        "s_9e73f1df_",
        "s_9e73f1df_x",
        "s_x_2",
    ] {
        assert_eq!(scene_id(odd), odd);
        assert_eq!(scene_of(odd), None, "{odd}");
    }
    assert_eq!(LineId::new("a_0123abcd").to_string(), "a_0123abcd");
}

// --- Printing ---------------------------------------------------------------

#[test]
fn print_writes_every_step_back() {
    let src = "@scene s\n@caption Dusk\n@left guide neutral\n@right clear\nguide[sad]: Hi.\nguide: Yo.\n> x\n@music storm\n@music stop\n@weather rain\n@shake\n@choice\n* a: b\n  > c\n  @if guide\n  > d\n  @else\n  > e\n  @endif\n* f: g\n@endchoice\n@if guide\n> h\n@endif\n@end\n";
    let (parsed, errors) = parse_dlg::<Cue>("t.dlg", src);
    assert_eq!(errors, []);
    assert_eq!(print_scene(&parsed[0].scene), src);
}

fn arb_id() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,6}"
}

fn arb_text() -> impl Strategy<Value = String> {
    "[A-Za-z][A-Za-z ,.!?]{0,20}[A-Za-z.!?]"
}

fn arb_side() -> impl Strategy<Value = Side> {
    prop_oneof![Just(Side::Left), Just(Side::Right)]
}

fn arb_simple_step() -> impl Strategy<Value = TStep> {
    prop_oneof![
        arb_text().prop_map(|text| Step::Caption { text }),
        (arb_side(), arb_id(), arb_id()).prop_map(|(side, c, e)| place(side, &c, &e)),
        arb_side().prop_map(|side| Step::Clear { side }),
        (arb_id(), proptest::option::of(arb_id()), arb_text()).prop_map(|(s, e, text)| say(
            &s,
            e.as_deref(),
            &text
        )),
        arb_text().prop_map(|text| narrate(&text)),
        arb_id().prop_map(|cue| Step::Music(MusicLine::Cue(cue))),
        Just(Step::Music(MusicLine::Stop)),
        arb_id().prop_map(|kind| Step::Directive(Cue::Weather(kind))),
        Just(Step::Directive(Cue::Shake)),
    ]
}

fn arb_with_ifs(leaf: impl Strategy<Value = TStep> + 'static) -> impl Strategy<Value = TStep> {
    leaf.prop_recursive(2, 8, 3, |inner| {
        let steps = || proptest::collection::vec(inner.clone(), 0..3);
        (arb_id(), steps(), steps()).prop_map(|(c, then, otherwise)| Step::If {
            character: id(&c),
            then,
            otherwise,
        })
    })
}

fn arb_step() -> impl Strategy<Value = TStep> {
    let option = (
        arb_id(),
        arb_text(),
        proptest::collection::vec(arb_with_ifs(arb_simple_step()), 0..4),
    )
        .prop_map(|(tone, text, steps)| ChoiceOption {
            tone,
            text,
            line: LineId::default(),
            steps,
        });
    prop_oneof![
        4 => arb_with_ifs(arb_simple_step()),
        1 => proptest::collection::vec(option, 0..4).prop_map(|options| Step::Choice { options }),
    ]
}

proptest! {
    /// Printing a scene and parsing it gives the scene back.
    #[test]
    fn print_parse_round_trip(id in arb_id(), steps in proptest::collection::vec(arb_step(), 0..12)) {
        let scene = Scene { id, steps };
        let (parsed, errors) = parse_dlg::<Cue>("t.dlg", &print_scene(&scene));
        prop_assert_eq!(errors, vec![]);
        prop_assert_eq!(parsed.len(), 1);
        prop_assert_eq!(bare(&parsed[0].scene.steps), scene.steps);
    }
}
