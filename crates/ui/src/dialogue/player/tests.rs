//! The dialogue player on scenes of the `.dlg` format, with no lines of a
//! game's own ([`Plain`]) and with a toy game's `@lamp` lines and `{who}`
//! token ([`Lamps`]).

use alloc::collections::BTreeSet;

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
