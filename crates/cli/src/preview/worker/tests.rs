//! The worker over the toy game, as a visitor plays it, and over a game
//! whose tools offer no preview.

use super::*;
use crate::codec::decode_png;
use crate::contract::fake::Fake;
use crate::game::is_unsupported;
use crate::toy::ToyTools;
use shuyi_platform::link::base64::decode;

/// The toy in its lobby (Options, Key bindings, Talk, Quit), a visitor's.
fn lobby() -> Visit {
    let start = [GameArg {
        flag: "--lobby".to_owned(),
        value: None,
    }];
    Visit::start(&ToyTools, &start, 300, 1).unwrap()
}

/// The frame of `reply`, which must be one.
fn frame(reply: Reply) -> Shown {
    assert_eq!((&reply.refused, &reply.failed), (&None, &None));
    reply.frame.unwrap()
}

#[test]
fn the_visible_text_is_the_glyph_rows_trimmed_without_colours_or_overlays() {
    let snapshot = "\n  \nab  \n\n\n c\n   \n--- colours ---\nxx\n--- overlays ---\nover  sprite 0,0 8x8  images/secret.png 0,0 8x8\n";
    assert_eq!(visible_text(snapshot), "ab\n\n c");
    assert_eq!(visible_text(""), "");
}

#[test]
fn a_frame_holds_still_when_its_glyphs_and_overlays_places_do() {
    let frame = |colour: &str, sprite: &str, style: &str| {
        format!(
            "ab\n--- colours ---\n{colour}\n--- legend ---\na = fg:text bg:panel\n\
             --- overlays ---\nover  18,46 11x2  hp_mid\nunder sprite {sprite}  images/a.png 0,0 8x8 opacity=9\n\
             --- glyph styles ---\n0,0  {style},+0\n"
        )
    };
    let still = still_part(&frame("aa", "8,8 80x80", "+0"));
    assert_eq!(still, "ab\nover 18,46 11x2\nunder sprite 8,8 80x80");
    // A colour that pulses, or a glyph that wobbles, isn't a move.
    assert_eq!(still_part(&frame("ab", "8,8 80x80", "+1")), still);
    // A sprite that moves is.
    assert_ne!(still_part(&frame("aa", "9,8 80x80", "+0")), still);
    assert_eq!(still_part("ab\n"), "ab");
}

#[test]
fn a_visitor_starts_on_a_settled_frame_with_the_players_actions_only() {
    let shown = frame(lobby().shown());
    let png = decode(&shown.png).unwrap();
    let (width, height, _) = decode_png(&png).unwrap();
    assert!(width > 0 && height > 0);
    for item in ["Options", "Key bindings", "Talk", "Quit"] {
        assert!(shown.text.contains(item), "{item} in {}", shown.text);
    }
    assert!(!shown.text.contains("colours"), "{}", shown.text);
    // The developer's Debug key is no visitor's.
    assert!(shown.actions.contains(&"CursorDown".to_owned()));
    assert!(shown.actions.contains(&"Confirm".to_owned()));
    assert!(
        !shown.actions.contains(&"Debug".to_owned()),
        "{:?}",
        shown.actions
    );
    assert!(!shown.ended);
}

#[test]
fn an_action_changes_the_picture_and_an_unknown_one_is_refused() {
    let mut visit = lobby();
    let first = frame(visit.shown());
    let moved = frame(visit.act("CursorDown"));
    assert_ne!(moved.png, first.png);
    for refused in ["Debug", "Jump", ""] {
        let reply = visit.act(refused);
        assert_eq!(reply.frame, None, "{refused}");
        let why = reply.refused.unwrap();
        assert!(why.starts_with(&format!("no action `{refused}`")), "{why}");
        assert!(why.contains("CursorDown"), "{why}");
    }
    // Nothing a refusal did: the next frame is the moved one's.
    assert_eq!(frame(visit.shown()).png, moved.png);
}

#[test]
fn quitting_ends_the_visit() {
    let mut visit = lobby();
    for _ in 0..3 {
        frame(visit.act("CursorDown"));
    }
    assert!(frame(visit.act("Confirm")).ended);
}

#[test]
fn a_game_whose_session_offers_no_visitor_has_no_preview() {
    let Err(error) = Visit::start(&Fake::default(), &[], 30, 1) else {
        panic!("the fake's session offers no visitor");
    };
    assert!(is_unsupported(&error), "{error}");
    // Wrong start options are said as such.
    let wrong = [GameArg {
        flag: "--moon".to_owned(),
        value: None,
    }];
    let Err(error) = Visit::start(&ToyTools, &wrong, 30, 1) else {
        panic!("--moon is no option of the toy's");
    };
    assert_eq!(error, "unknown option: --moon");
}

/// The replies `serve` wrote, each line's JSON after the mark.
fn replies(output: &[u8]) -> Vec<Reply> {
    let text = std::str::from_utf8(output).unwrap();
    text.lines()
        .map(|line| serde_json::from_str(line.strip_prefix(REPLY).unwrap()).unwrap())
        .collect()
}

/// `line`'s words as arguments.
fn words(line: &str) -> Vec<String> {
    line.split_whitespace().map(str::to_owned).collect()
}

#[test]
fn the_workers_arguments_are_its_bounds_and_the_games_options() {
    let read = |line: &str| parse_args(&words(line), ToyTools.frame_options());
    let plain = read("").unwrap();
    assert_eq!(
        (plain.frames, plain.scale, plain.start),
        (300, 1, Vec::new())
    );
    let args = read("--lobby --frames 40 --burn-frame 3 --scale 2").unwrap();
    assert_eq!((args.frames, args.scale), (40, 2));
    let starts: Vec<(&str, Option<&str>)> = (args.start.iter())
        .map(|arg| (arg.flag.as_str(), arg.value.as_deref()))
        .collect();
    assert_eq!(starts, [("--lobby", None), ("--burn-frame", Some("3"))]);
    let refused = [
        ("--frames 0", "--frames: a whole number 1 to 2000, not 0"),
        ("--scale 9", "--scale: a whole number 1 to 8, not 9"),
        ("--scale", "--scale needs a value"),
        ("--burn-frame", "--burn-frame needs a value"),
        ("--moon", "unknown option: --moon"),
    ];
    for (line, error) in refused {
        assert_eq!(read(line), Err(error.to_owned()), "{line}");
    }
}

#[test]
fn serving_answers_the_start_then_each_line_until_its_input_closes() {
    let args = words("--frames 60 --lobby");
    let input = b"{\"action\": \"CursorDown\"}\nnot json\n{\"action\": \"Debug\"}\n\
        {\"action\": \"Confirm\", \"frames\": 9}\n{\"script\": \"Enter Up\"}\n";
    let mut output = Vec::new();
    assert_eq!(serve(&ToyTools, &args, &input[..], &mut output), 0);
    let answered = replies(&output);
    assert_eq!(answered.len(), 6, "{answered:?}");
    let (start, moved) = (answered[0].frame.as_ref(), answered[1].frame.as_ref());
    assert_ne!(start.unwrap().png, moved.unwrap().png);
    let refused = |i: usize| answered[i].refused.clone().unwrap();
    assert!(refused(2).starts_with("not an action"));
    assert!(refused(3).starts_with("no action `Debug`"));
    // No frame count and no script: an action alone is read.
    assert!(refused(4).starts_with("not an action: unknown field `frames`"));
    assert!(refused(5).starts_with("not an action: unknown field `script`"));
    // A line longer than any action's ends the worker.
    let long = [vec![b'x'; 2_000], b"\n".to_vec()].concat();
    let mut output = Vec::new();
    assert_eq!(serve(&ToyTools, &args, &long[..], &mut output), 1);
    // A game that doesn't start says why, and ends it.
    let mut output = Vec::new();
    let moon = ["--moon".to_owned()];
    assert_eq!(serve(&ToyTools, &moon, &b""[..], &mut output), 1);
    let failed = replies(&output).remove(0).failed.unwrap();
    assert_eq!(failed, "unknown option: --moon");
}
