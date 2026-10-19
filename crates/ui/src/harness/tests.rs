//! The Harness's stand-in for the music player. The scripts are tested
//! through the toy game in `tests/it/harness.rs`.

use super::*;

fn command(cue: &str, command: fn(String) -> MusicCommand) -> MusicCommand {
    command(cue.into())
}

fn load(cue: String) -> MusicCommand {
    MusicCommand::Load { cue }
}

fn start(cue: String) -> MusicCommand {
    MusicCommand::Start { cue }
}

fn stop(cue: String) -> MusicCommand {
    MusicCommand::Stop { cue }
}

#[test]
fn the_player_counts_from_a_start_to_its_stop() {
    let mut p = Player::default();
    assert_eq!(p.playing(), None);
    p.advance(1.0);
    p.apply(&command("a", load));
    assert_eq!(p.playing(), None, "loaded, not started");
    p.apply(&command("a", start));
    assert_eq!(p.playing(), Some(("a", 0.0)));
    p.advance(0.5);
    p.advance(0.25);
    assert_eq!(p.playing(), Some(("a", 0.75)));
    // Nonsense frame times don't count.
    for dt in [-1.0, f32::NAN, f32::INFINITY] {
        p.advance(dt);
    }
    assert_eq!(p.playing(), Some(("a", 0.75)));
    // A fade and other tracks' commands leave it playing.
    let fade = MusicCommand::Gain {
        cue: "a".into(),
        gain: 0.5,
    };
    for other in [fade, command("b", load), command("b", stop)] {
        p.apply(&other);
    }
    assert_eq!(p.playing(), Some(("a", 0.75)));
    p.apply(&command("a", stop));
    assert_eq!(p.playing(), None);
    p.advance(1.0);
    assert_eq!(p.playing(), None);
    // The next track counts from its own start; loading a track again
    // replaces it, as the platform does.
    p.apply(&command("b", start));
    p.advance(0.5);
    assert_eq!(p.playing(), Some(("b", 0.5)));
    p.apply(&command("b", load));
    assert_eq!(p.playing(), None);
}

#[test]
fn the_player_sounds_after_its_load_delay_or_never() {
    let mut p = Player {
        load_delay: 0.5,
        ..Player::default()
    };
    p.apply(&command("a", start));
    assert_eq!(p.playing(), None);
    p.advance(0.25);
    assert_eq!(p.playing(), None);
    p.advance(0.25);
    assert_eq!(p.playing(), Some(("a", 0.0)));
    p.advance(0.25);
    assert_eq!(p.playing(), Some(("a", 0.25)));
    p.silent = true;
    assert_eq!(p.playing(), None);
}
