//! The dialogue player on scenes of the `.dlg` format, with no lines of a
//! game's own ([`Plain`]) and with a toy game's `@lamp` lines and `{who}`
//! token ([`Lamps`]).

use alloc::collections::BTreeSet;

use proptest::prelude::*;
use shuyi_assets::names::Names;

use shuyi_assets::dialogue::{
    CharacterId, ChoiceOption, DialogueLimits, Directive, LineId, MusicLine, NoDirective, NoTokens,
    Parsed, Present, Scene, SceneRules, Side, Step, check_scene, parse_dlg,
};

use super::*;

/// A toy game's own line: `@lamp on` or `@lamp off`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Lamp(bool);

impl Directive for Lamp {
    fn parse(name: &str, args: &str) -> Option<Parsed<Self>> {
        (name == "lamp").then(|| Parsed::step(Lamp(args == "on")))
    }

    fn print(&self) -> String {
        format!("lamp {}", if self.0 { "on" } else { "off" })
    }
}

/// The toy game's hooks: `{who}` is the hero's name, and the lamp lines
/// light the lamp and count how many times it was switched.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Lamps {
    hero: &'static str,
}

/// What the lamp lines set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Lit {
    on: bool,
    switched: u32,
}

impl DialogueHooks for Lamps {
    type Directive = Lamp;
    type Stage = Lit;

    fn fill<'a>(&self, text: &'a str) -> Cow<'a, str> {
        if text.contains("{who}") {
            Cow::Owned(text.replace("{who}", self.hero))
        } else {
            Cow::Borrowed(text)
        }
    }

    fn direct(&self, stage: &mut Lit, lamp: &Lamp) {
        stage.on = lamp.0;
        stage.switched += 1;
    }
}

fn id(s: &str) -> CharacterId {
    CharacterId(s.to_owned())
}

/// The scenes of `src`, which must parse.
fn scenes<D: Directive>(src: &str) -> Vec<Scene<D>> {
    let (parsed, errors) = parse_dlg::<D>("t.dlg", src);
    assert!(errors.is_empty(), "{errors:?}");
    parsed.into_iter().map(|p| p.scene).collect()
}

fn plain(src: &str) -> DialoguePlayer<Plain> {
    let scene = scenes::<NoDirective>(src).remove(0);
    DialoguePlayer::new(&scene, Plain, &Present::Everyone)
}

fn lamps(src: &str, present: &Present) -> DialoguePlayer<Lamps> {
    let scene = scenes::<Lamp>(src).remove(0);
    DialoguePlayer::new(&scene, Lamps { hero: "Mei" }, present)
}

/// The text on screen now.
fn text<H: DialogueHooks>(p: &DialoguePlayer<H>) -> Option<String> {
    p.current().text.map(Cow::into_owned)
}

const HALL: &str = "\
@scene hall
@caption The hall, at dusk
@left guide neutral
@right scout worried
> The hall is quiet.
guide: Welcome in.
scout[smile]: Thanks.
@right clear
guide[sad]: Alone again.
@end
";

#[test]
fn it_plays_each_text_box_with_the_portraits_and_the_caption_before_it() {
    let mut p = plain(HALL);
    assert_eq!(p.scene_id(), "hall");
    assert_eq!(p.scene().id, "hall");
    let view = p.current();
    assert_eq!(view.text.as_deref(), Some("The hall is quiet."));
    assert!(view.narration);
    assert_eq!(view.speaker, None);
    assert_eq!(view.caption.as_deref(), Some("The hall, at dusk"));
    let left = view.left.unwrap();
    assert_eq!((left.character, left.expression), (&id("guide"), "neutral"));
    assert_eq!(view.right.map(|r| r.expression), Some("worried"));
    assert_eq!(view.choices, None);
    assert_eq!(*view.stage, ());
    p.advance();
    let view = p.current();
    assert_eq!(view.text.as_deref(), Some("Welcome in."));
    assert_eq!(view.speaker, Some(Side::Left));
    assert!(!view.narration);
    // A new expression comes with the line.
    p.advance();
    let view = p.current();
    assert_eq!(view.speaker, Some(Side::Right));
    assert_eq!(view.right.map(|r| r.expression), Some("smile"));
    assert_eq!(view.left.map(|l| l.expression), Some("neutral"));
    // The right side empties; the left changes expression as it speaks.
    p.advance();
    let view = p.current();
    assert_eq!(view.right, None);
    assert_eq!(view.left.map(|l| l.expression), Some("sad"));
    assert!(!p.is_finished());
    p.advance();
    assert!(p.is_finished());
    assert_eq!(text(&p), None);
    assert_eq!(p.line(), None);
    assert_eq!(p.current().speaker, None);
    // Advancing a finished scene does nothing.
    p.advance();
    assert!(p.is_finished());
}

#[test]
fn a_speaker_who_isnt_on_screen_speaks_from_no_side() {
    let mut p = plain("@scene a\n@left guide neutral\nscout: Psst.\n@end\n");
    assert_eq!(text(&p).as_deref(), Some("Psst."));
    assert_eq!(p.current().speaker, None);
    assert!(!p.current().narration);
    p.advance();
    assert!(p.is_finished());
}

#[test]
fn each_line_has_its_id_and_the_hooks_fill_the_text() {
    let src = "@scene a\n@caption {who}'s room\n> {who} wakes.\n> Rain.\n@end\n";
    let mut p = lamps(src, &Present::Everyone);
    assert_eq!(text(&p).as_deref(), Some("Mei wakes."));
    assert_eq!(p.current().caption.as_deref(), Some("Mei's room"));
    // The scene keeps the token.
    assert!(matches!(&p.scene().steps[1], Step::Narrate { text, .. } if text.contains("{who}")));
    let lines: Vec<LineId> = p.scene().lines().iter().map(|l| l.id.clone()).collect();
    assert_eq!(p.line(), Some(&lines[0]));
    p.advance();
    assert_eq!(p.line(), Some(&lines[1]));
    assert!(matches!(p.current().text, Some(Cow::Borrowed("Rain."))));
    assert_eq!(p.hooks().hero, "Mei");
    assert_eq!(Plain.fill("{who}"), "{who}");
}

const ASK: &str = "\
@scene ask
@left hero neutral
@right guide neutral
guide: Ready?
@choice
* yes: Yes.
  guide[smile]: Good.
  > She nods.
* no: Not yet.
* quiet: ...
  guide: Fine.
@endchoice
guide: Let's go.
@end
";

#[test]
fn a_choice_waits_for_a_reply_plays_its_reaction_then_rejoins_the_scene() {
    let mut p = plain(ASK);
    assert_eq!(text(&p).as_deref(), Some("Ready?"));
    assert!(!p.is_choosing());
    assert_eq!(p.reply_line(0), None, "no choice is open");
    p.advance();
    assert!(p.is_choosing());
    let view = p.current();
    let replies: Vec<_> = view
        .choices
        .unwrap()
        .into_iter()
        .map(Cow::into_owned)
        .collect();
    assert_eq!(replies, ["Yes.", "Not yet.", "..."]);
    // The line being answered stays up.
    assert_eq!(view.text.as_deref(), Some("Ready?"));
    let asked = p.line().cloned();
    assert!(asked.is_some());
    // Advancing doesn't answer; nor does a reply that isn't there.
    p.advance();
    p.choose(3);
    assert!(p.is_choosing());
    assert!(p.reply_line(0).is_some() && p.reply_line(3).is_none());
    assert_ne!(p.reply_line(0), p.reply_line(1));
    p.choose(0);
    assert!(!p.is_choosing());
    assert_eq!(text(&p).as_deref(), Some("Good."));
    assert_eq!(p.current().right.map(|r| r.expression), Some("smile"));
    p.advance();
    assert_eq!(text(&p).as_deref(), Some("She nods."));
    p.advance();
    assert_eq!(text(&p).as_deref(), Some("Let's go."));
    // The reaction's expression stays for the rejoin.
    assert_eq!(p.current().right.map(|r| r.expression), Some("smile"));
    // Choosing again, with no choice open, does nothing.
    p.choose(1);
    assert_eq!(text(&p).as_deref(), Some("Let's go."));
    p.advance();
    assert!(p.is_finished());
    // A reply with no reaction goes straight on.
    let mut p = plain(ASK);
    p.advance();
    p.choose(1);
    assert_eq!(text(&p).as_deref(), Some("Let's go."));
}

#[test]
fn skipping_stops_at_each_choice_and_at_the_end() {
    let mut p = plain(ASK);
    p.skip_to_choice();
    assert!(p.is_choosing());
    p.skip_to_choice();
    assert!(p.is_choosing(), "it waits for a reply");
    p.choose(2);
    assert_eq!(text(&p).as_deref(), Some("Fine."));
    p.skip_to_choice();
    assert!(p.is_finished());
}

#[test]
fn a_choice_with_no_replies_is_passed_over() {
    let scene = Scene::<NoDirective> {
        id: "odd".into(),
        steps: vec![
            Step::Choice {
                options: Vec::<ChoiceOption>::new(),
            },
            Step::Narrate {
                text: "After.".into(),
                direction: None,
                line: LineId::default(),
            },
        ],
    };
    let p = DialoguePlayer::new(&scene, Plain, &Present::Everyone);
    assert!(!p.is_choosing());
    assert_eq!(text(&p).as_deref(), Some("After."));
    // A scene with nothing to say is finished at once.
    let empty = Scene::<NoDirective> {
        id: "none".into(),
        steps: vec![Step::Caption { text: "c".into() }],
    };
    let p = DialoguePlayer::new(&empty, Plain, &Present::Everyone);
    assert!(p.is_finished());
    assert_eq!(p.current().caption.as_deref(), Some("c"));
}

#[test]
fn the_music_asked_for_is_kept_until_taken_and_the_last_wins() {
    let src = "@scene m\n@music calm\n> One.\n@music storm\n@music stop\n> Two.\n> Three.\n@end\n";
    let mut p = plain(src);
    assert_eq!(p.pending_music(), Some(&MusicLine::Cue("calm".into())));
    assert_eq!(p.take_music(), Some(MusicLine::Cue("calm".into())));
    assert_eq!(p.take_music(), None);
    p.advance();
    // Skipped or read, the last line reached is what plays.
    assert_eq!(p.take_music(), Some(MusicLine::Stop));
    p.advance();
    assert_eq!(p.pending_music(), None);
}

#[test]
fn the_scene_plays_for_those_present() {
    let src = "\
@scene who
@if scout
> The scout is here.
@else
> Nobody came.
@endif
> End.
@end
";
    let p = lamps(src, &Present::Everyone);
    assert_eq!(text(&p).as_deref(), Some("The scout is here."));
    let p = lamps(src, &Present::Only(BTreeSet::new()));
    assert_eq!(text(&p).as_deref(), Some("Nobody came."));
    // The scene played has no `@if` left.
    assert!(!p.scene().steps.iter().any(|s| matches!(s, Step::If { .. })));
    let p = lamps(src, &Present::only([id("scout")]));
    assert_eq!(text(&p).as_deref(), Some("The scout is here."));
}

#[test]
fn the_games_lines_play_on_its_stage_read_or_skipped() {
    let src = "@scene l\n@lamp on\n> Light.\n@lamp off\n@lamp on\n> Again.\n@end\n";
    let mut p = lamps(src, &Present::Everyone);
    assert_eq!(
        *p.stage(),
        Lit {
            on: true,
            switched: 1
        }
    );
    assert_eq!(*p.current().stage, *p.stage());
    p.skip_to_choice();
    assert!(p.is_finished());
    assert_eq!(
        *p.stage(),
        Lit {
            on: true,
            switched: 3
        }
    );
    // The game may take what it has played.
    p.stage_mut().switched = 0;
    assert_eq!(p.stage().switched, 0);
    // A copy keeps the scene as it was.
    let mut a = lamps(src, &Present::Everyone);
    let kept = a.clone();
    a.advance();
    assert_ne!(a, kept);
    assert_eq!(text(&kept).as_deref(), Some("Light."));
}

fn smith() -> Arc<Names> {
    let mut names = Names::default();
    names
        .names
        .insert("smith".to_owned(), "ada brook".to_owned());
    Arc::new(names)
}

#[test]
fn named_fills_name_tokens_and_leaves_the_rest_as_written() {
    let hooks = Named::new(smith());
    assert_eq!(hooks.fill("Hello, {n:smith}."), "Hello, ada brook.");
    assert_eq!(hooks.fill("{N:smith} waves."), "Ada brook waves.");
    assert_eq!(hooks.fill("{n:nobody} {hero}"), "{n:nobody} {hero}");
    assert_eq!(Plain.fill("{n:smith}"), "{n:smith}");
}

const NAMED: &str = "\
@scene named
@left guide neutral
@caption The house of {n:smith}
guide: Hello, {n:smith}.
@choice
* yes: Thank you, {n:smith}.
  guide: {N:smith} waves.
* no: Not yet.
@endchoice
guide: {N:smith} leaves.
@end
";

struct NamedRules(Names);

impl SceneRules<NoDirective> for NamedRules {
    fn limits(&self) -> &DialogueLimits {
        &DialogueLimits {
            box_width: 40,
            box_lines: 2,
            max_text: 80,
            max_option: 30,
            min_options: 2,
            max_options: 3,
            max_reaction_texts: 4,
        }
    }

    fn tokens(&self) -> &dyn shuyi_assets::dialogue::Tokens {
        &NoTokens
    }

    fn names(&self) -> Option<&Names> {
        Some(&self.0)
    }
}

#[test]
fn a_scene_with_name_tokens_checks_and_plays_with_none_left_on_screen() {
    let names = smith();
    let (parsed, errors) = parse_dlg::<NoDirective>("named.dlg", NAMED);
    assert!(errors.is_empty(), "{errors:?}");
    let rules = NamedRules(Names::clone(&names));
    for p in &parsed {
        assert_eq!(check_scene(p, &rules), vec![]);
    }
    let mut p = DialoguePlayer::new(&parsed[0].scene, Named::new(names), &Present::Everyone);
    let mut seen = 0;
    while !p.is_finished() {
        let v = p.current();
        let shown = [
            v.text.as_deref().map(str::to_owned),
            v.caption.as_deref().map(str::to_owned),
        ]
        .into_iter()
        .flatten()
        .chain(v.choices.iter().flatten().map(ToString::to_string))
        .collect::<Vec<_>>();
        for s in &shown {
            assert!(!s.contains("{n:") && !s.contains("{N:"), "{s}");
        }
        seen += shown.len();
        if p.is_choosing() {
            p.choose(0);
        } else {
            p.advance();
        }
    }
    assert!(seen > 4);
}

#[test]
fn the_direction_is_the_lines_own_or_the_delivery_in_force() {
    let src = "@scene d\n@left guide calm\nguide (softly): One.\n@delivery out of breath\n> Two.\nguide: Three?\n@choice\n* a: Yes.\n@endchoice\n@delivery clear\nguide: Four.\n@end\n";
    let mut p = plain(src);
    let mut seen = Vec::new();
    while !p.is_finished() {
        seen.push(p.direction().map(str::to_owned));
        if p.is_choosing() {
            p.choose(0);
        } else {
            p.advance();
        }
    }
    let breath = Some("out of breath".to_owned());
    assert_eq!(
        seen,
        [
            Some("softly".to_owned()),
            breath.clone(),
            breath.clone(),
            // The choice answers the line before it.
            breath,
            None,
        ]
    );
    assert_eq!(p.direction(), None, "finished");
}

#[test]
fn a_player_started_at_a_text_box_takes_the_replies_it_is_given() {
    let scene = scenes::<NoDirective>(ASK).remove(0);
    let at = |text_box, replies: &[usize]| {
        DialoguePlayer::from_text_box(&scene, Plain, &Present::Everyone, text_box, replies)
    };
    assert_eq!(text(&at(0, &[])).as_deref(), Some("Ready?"));
    assert_eq!(
        at(0, &[]),
        DialoguePlayer::new(&scene, Plain, &Present::Everyone)
    );
    assert_eq!(text(&at(1, &[])).as_deref(), Some("Good."));
    assert_eq!(text(&at(2, &[])).as_deref(), Some("She nods."));
    assert_eq!(text(&at(1, &[2])).as_deref(), Some("Fine."));
    assert_eq!(text(&at(1, &[1])).as_deref(), Some("Let's go."));
    // A reply that isn't there takes the first.
    assert_eq!(at(1, &[9]), at(1, &[0]));
    assert_eq!(
        at(3, &[0]).current().right.map(|r| r.expression),
        Some("smile")
    );
    assert!(at(4, &[]).is_finished());
    assert!(at(2, &[1]).is_finished());
    // A choice before the first text box is answered too.
    let opens = scenes::<NoDirective>(
        "@scene o
@choice
* a: A.
  > Took a.
* b: B.
  > Took b.
@endchoice
@end
",
    )
    .remove(0);
    let first = DialoguePlayer::from_text_box(&opens, Plain, &Present::Everyone, 0, &[1]);
    assert_eq!(text(&first).as_deref(), Some("Took b."));
}

/// The speakers and those tested in the generated scenes.
const WHO: [&str; 3] = ["guide", "scout", "smith"];

fn arb_who() -> impl Strategy<Value = CharacterId> {
    proptest::sample::select(&WHO[..]).prop_map(id)
}

fn arb_text() -> impl Strategy<Value = String> {
    "[A-Z][a-z]{0,8}[.?!]"
}

/// A step of a generated scene that is no block.
fn arb_simple() -> impl Strategy<Value = Step<Lamp>> {
    let say = (
        arb_who(),
        proptest::option::of("[a-z]{1,6}"),
        arb_text(),
        proptest::option::of("[a-z]{1,6}"),
    )
        .prop_map(|(speaker, expression, text, direction)| Step::Say {
            speaker,
            expression,
            text,
            direction,
            line: LineId::default(),
        });
    let narrate = arb_text().prop_map(|text| Step::Narrate {
        text,
        direction: None,
        line: LineId::default(),
    });
    let side = prop_oneof![Just(Side::Left), Just(Side::Right)];
    prop_oneof![
        3 => say,
        2 => narrate,
        1 => arb_text().prop_map(|text| Step::Caption { text }),
        1 => (side.clone(), arb_who(), "[a-z]{1,6}").prop_map(|(side, character, expression)| {
            Step::Place {
                side,
                character,
                expression,
            }
        }),
        1 => side.prop_map(|side| Step::Clear { side }),
        1 => "[a-z]{1,6}".prop_map(|cue| Step::Music(MusicLine::Cue(cue))),
        1 => Just(Step::Music(MusicLine::Stop)),
        1 => proptest::bool::ANY.prop_map(|on| Step::Directive(Lamp(on))),
    ]
}

/// A step with `@if` blocks in it.
fn arb_with_ifs() -> impl Strategy<Value = Step<Lamp>> {
    arb_simple().prop_recursive(2, 8, 3, |inner| {
        let steps = || proptest::collection::vec(inner.clone(), 0..3);
        (arb_who(), steps(), steps()).prop_map(|(character, then, otherwise)| Step::If {
            character,
            then,
            otherwise,
        })
    })
}

/// Any step: a reply choice, each reply with a reaction, among them.
fn arb_step() -> impl Strategy<Value = Step<Lamp>> {
    let option = (
        "[a-z]{1,6}",
        arb_text(),
        proptest::collection::vec(arb_with_ifs(), 0..3),
    )
        .prop_map(|(tone, text, steps)| ChoiceOption {
            tone,
            text,
            line: LineId::default(),
            steps,
        });
    let choice =
        proptest::collection::vec(option, 1..4).prop_map(|options| Step::Choice { options });
    prop_oneof![4 => arb_with_ifs(), 1 => choice]
}

fn arb_present() -> impl Strategy<Value = Present> {
    prop_oneof![
        Just(Present::Everyone),
        proptest::sample::subsequence(WHO.to_vec(), 0..=WHO.len())
            .prop_map(|who| Present::only(who.into_iter().map(id))),
    ]
}

proptest::proptest! {
    /// A player started at a text box is the one read up to it, and shows
    /// the text box `ParsedScene::text_boxes` says is there.
    #[test]
    fn starting_at_a_text_box_equals_reading_up_to_it(
        steps in proptest::collection::vec(arb_step(), 0..10),
        present in arb_present(),
        replies in proptest::collection::vec(0..4_usize, 0..4),
        text_box in 0..10_usize,
    ) {
        use shuyi_assets::dialogue::print_scene;
        let written = Scene { id: "gen".into(), steps };
        let (parsed, errors) = parse_dlg::<Lamp>("gen.dlg", &print_scene(&written));
        proptest::prop_assert_eq!(errors, vec![]);
        let parsed = &parsed[0];
        let hooks = Lamps { hero: "Mei" };
        let started =
            DialoguePlayer::from_text_box(&parsed.scene, hooks.clone(), &present, text_box, &replies);
        let mut read = DialoguePlayer::new(&parsed.scene, hooks, &present);
        let mut given = replies.iter().copied();
        let on_text = |p: &DialoguePlayer<Lamps>| !p.is_choosing() && !p.is_finished();
        let mut shown = usize::from(on_text(&read));
        while !(read.is_finished() || on_text(&read) && shown > text_box) {
            if read.is_choosing() {
                let count = read.current().choices.map_or(0, |c| c.len());
                read.choose(given.next().filter(|&r| r < count).unwrap_or(0));
            } else {
                read.advance();
            }
            if on_text(&read) {
                shown += 1;
            }
        }
        proptest::prop_assert_eq!(started.current(), read.current());
        proptest::prop_assert_eq!(started.pending_music(), read.pending_music());
        proptest::prop_assert_eq!(started.line(), read.line());
        proptest::prop_assert_eq!(started.direction(), read.direction());
        let boxes = parsed.text_boxes(&present, &replies);
        match boxes.get(text_box) {
            Some(b) => {
                let shown = text(&started);
                proptest::prop_assert_eq!(shown.as_deref(), b.step.text());
                let line = match b.step {
                    Step::Say { line, .. } | Step::Narrate { line, .. } => Some(line),
                    _ => None,
                };
                proptest::prop_assert_eq!(started.line(), line);
            }
            None => proptest::prop_assert!(started.is_finished()),
        }
    }
}
