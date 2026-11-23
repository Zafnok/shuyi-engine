//! The game's side of the hosted preview: `preview-worker [--frames N]
//! [--scale N] [the game's options]`, run in the game's tool program, one
//! process a session. It starts the game's session with the game's
//! options, makes it a visitor's ([`Session::for_visitor`]), and answers on
//! stdout, one line each after the mark [`REPLY`]: first with the frame it
//! starts on, then each line of stdin, an [`Act`], with the frame after
//! that action ([`Reply`]), each console pixel `--scale` × `--scale`
//! pixels (1 by default). It reads nothing else: no script, no frame
//! count, no scene, snapshot or log, and it ends when its stdin closes.
//!
//! After an action it runs the frames in sixes until the frame has held
//! still for [`QUIET`] of them, or `--frames` have run (300 by default): a
//! walk ends, a foe's turn is shown, before the picture is taken. Holding
//! still is its glyphs and where its overlays are ([`still_part`]): a
//! cursor that pulses in colour doesn't keep it running.

use std::io::{BufRead, Read, Write};

use serde::{Deserialize, Serialize};
use shuyi_platform::harness::MAX_FRAMES;

use crate::codec::encode_png;
use crate::contract::session::Session;
use crate::frame_png::{MAX_SCALE, Script, Step};
use crate::game::{GameArg, GameOption, GameTools, check_start, game_arg, unsupported};
use crate::mcp::base64;

/// What starts each reply line, so the server tells a reply from what the
/// game prints itself.
pub(crate) const REPLY: &str = "shuyi-preview-reply ";

/// The most frames an action runs by default: five seconds of the game.
const FRAMES: u32 = 300;

/// The longest request line a worker reads, in bytes.
const MAX_LINE: u64 = 1_024;

/// The frames run at once while a frame settles.
const CHUNK: u32 = 6;

/// The frames a frame holds still for before its picture is taken.
const QUIET: u32 = 30;

/// An action a visitor pressed: the request, a JSON line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Act {
    /// The action's name.
    pub(crate) action: String,
}

/// What a visitor is shown of a frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Shown {
    /// The picture, a PNG in base64.
    pub(crate) png: String,
    /// The text it shows ([`visible_text`]).
    pub(crate) text: String,
    /// The actions a visitor may press.
    pub(crate) actions: Vec<String>,
    /// Whether the game has quit: the session is over.
    pub(crate) ended: bool,
}

/// A worker's answer: a frame, an action refused, or the game failed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Reply {
    /// The frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) frame: Option<Shown>,
    /// Why the action wasn't pressed: the session goes on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) refused: Option<String>,
    /// Why the game can't go on: the session is over.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) failed: Option<String>,
}

/// A game a visitor plays.
pub(crate) struct Visit {
    session: Box<dyn Session>,
    actions: Vec<String>,
    frames: u32,
    scale: u32,
}

impl Visit {
    /// `game` started as `start` says, a visitor's, settled on its first
    /// frame: each action then runs at most `frames` frames, and each
    /// picture is painted at `scale`.
    pub(crate) fn start(
        game: &dyn GameTools,
        start: &[GameArg],
        frames: u32,
        scale: u32,
    ) -> Result<Self, String> {
        check_start(game.frame_options(), start)?;
        let mut session = game.session(start)?;
        let actions = session
            .for_visitor()
            .ok_or_else(|| unsupported("a preview", "session, with a Headless over its Harness"))?;
        if actions.is_empty() {
            return Err("the game has no action a player can press".to_owned());
        }
        let mut visit = Self {
            session,
            actions,
            frames,
            scale,
        };
        visit.settle();
        Ok(visit)
    }

    /// The action `action` pressed, and the frame after it; refused if it
    /// isn't one of a visitor's.
    pub(crate) fn act(&mut self, action: &str) -> Reply {
        if !self.actions.iter().any(|a| a == action) {
            let refused = format!(
                "no action `{action}`: a visitor's are {}",
                self.actions.join(", ")
            );
            return Reply {
                refused: Some(refused),
                ..Reply::default()
            };
        }
        let script = Script::new(vec![Step::Act(action.to_owned())]);
        self.session.play(&script);
        self.settle();
        self.shown()
    }

    /// Runs frames until the frame holds still for [`QUIET`] frames, or
    /// the most an action may run have run.
    fn settle(&mut self) {
        let mut last = still_part(&self.session.snapshot());
        let (mut ran, mut still) = (0, 0);
        while ran < self.frames && still < QUIET {
            let chunk = CHUNK.min(self.frames - ran);
            self.session.idle(chunk);
            ran += chunk;
            let now = still_part(&self.session.snapshot());
            if now == last {
                still += chunk;
            } else {
                (last, still) = (now, 0);
            }
        }
    }

    /// The frame now, as a visitor is shown it.
    pub(crate) fn shown(&self) -> Reply {
        let picture = self
            .session
            .picture(Some(self.scale))
            .and_then(|image| encode_png(image.width, image.height, &image.rgba));
        match picture {
            Ok(png) => Reply {
                frame: Some(Shown {
                    png: base64(&png),
                    text: visible_text(&self.session.snapshot()),
                    actions: self.actions.clone(),
                    ended: self.session.quit_frame().is_some(),
                }),
                ..Reply::default()
            },
            Err(e) => Reply {
                failed: Some(e),
                ..Reply::default()
            },
        }
    }
}

/// What of a frame's `snapshot` holds still when nothing moves: its rows
/// of glyphs, and each overlay's layer, kind and rectangle, up to its
/// size (`over sprite 8,8 80x80`). Colours, what an overlay shows and how
/// bright, and the sections after the overlays are left out.
pub(crate) fn still_part(snapshot: &str) -> String {
    let is_size = |token: &&str| {
        let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
        token
            .split_once('x')
            .is_some_and(|(w, h)| digits(w) && digits(h))
    };
    let mut lines = snapshot.lines();
    let mut still: Vec<String> = lines
        .by_ref()
        .take_while(|row| *row != "--- colours ---")
        .map(str::to_owned)
        .collect();
    let overlays = lines.skip_while(|line| *line != "--- overlays ---").skip(1);
    for line in overlays.take_while(|line| !line.starts_with("--- ")) {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let end = tokens
            .iter()
            .position(is_size)
            .map_or(tokens.len(), |at| at + 1);
        still.push(tokens[..end].join(" "));
    }
    still.join("\n")
}

/// The text a frame shows, from its `snapshot`: its rows of glyphs, each
/// without its trailing spaces, with no blank row at either end and one
/// blank row for several. Its colours and its overlays (where a sprite is
/// and which file it is from) stay out.
pub(crate) fn visible_text(snapshot: &str) -> String {
    let mut text = String::new();
    let mut gap = false;
    for row in snapshot.lines().take_while(|row| *row != "--- colours ---") {
        let row = row.trim_end();
        if row.is_empty() {
            gap = !text.is_empty();
            continue;
        }
        if !text.is_empty() {
            text.push_str(if gap { "\n\n" } else { "\n" });
        }
        text.push_str(row);
        gap = false;
    }
    text
}

/// Writes `reply` as one line after the mark.
fn send(out: &mut impl Write, reply: &Reply) -> std::io::Result<()> {
    let json = serde_json::to_string(reply).map_err(std::io::Error::other)?;
    writeln!(out, "{REPLY}{json}")?;
    out.flush()
}

/// The arguments of `preview-worker`: the most frames an action runs, the
/// pictures' scale and the game's start options, one of `options` each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Args {
    /// `--frames N`: the most frames an action runs before its picture.
    pub(crate) frames: u32,
    /// `--scale N`: each console pixel N × N pixels in a picture.
    pub(crate) scale: u32,
    /// The game's start options.
    pub(crate) start: Vec<GameArg>,
}

/// Reads `preview-worker`'s arguments, the game's start `options` among
/// them.
pub(crate) fn parse_args(args: &[String], options: &[GameOption]) -> Result<Args, String> {
    let mut parsed = Args {
        frames: FRAMES,
        scale: 1,
        start: Vec::new(),
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let most = match arg.as_str() {
            "--frames" => MAX_FRAMES,
            "--scale" => MAX_SCALE,
            _ => {
                let value = || iter.next().cloned().ok_or(format!("{arg} needs a value"));
                parsed.start.push(game_arg(arg, options, value)?);
                continue;
            }
        };
        let text = iter.next().ok_or_else(|| format!("{arg} needs a value"))?;
        let n = text
            .parse::<u32>()
            .ok()
            .filter(|n| (1..=most).contains(n))
            .ok_or_else(|| format!("{arg}: a whole number 1 to {most}, not {text}"))?;
        if arg == "--frames" {
            parsed.frames = n;
        } else {
            parsed.scale = n;
        }
    }
    Ok(parsed)
}

/// `preview-worker` with `args`: the game's session for one visitor,
/// served over `input` and `output` until `input` closes. The exit code: 0
/// when it closed, 1 if the game didn't start or a line was unreadable.
pub(crate) fn serve(
    game: &dyn GameTools,
    args: &[String],
    mut input: impl BufRead,
    mut output: impl Write,
) -> u8 {
    let started = parse_args(args, game.frame_options())
        .and_then(|args| Visit::start(game, &args.start, args.frames, args.scale));
    let mut visit = match started {
        Ok(visit) => visit,
        Err(e) => {
            let failed = Reply {
                failed: Some(e),
                ..Reply::default()
            };
            let _ = send(&mut output, &failed);
            return 1;
        }
    };
    if send(&mut output, &visit.shown()).is_err() {
        return 1;
    }
    loop {
        let mut line = Vec::new();
        match (&mut input).take(MAX_LINE).read_until(b'\n', &mut line) {
            Ok(0) => return 0,
            Ok(_) if line.ends_with(b"\n") => {}
            _ => return 1,
        }
        let reply = match serde_json::from_slice::<Act>(&line) {
            Ok(act) => visit.act(&act.action),
            Err(e) => Reply {
                refused: Some(format!("not an action: {e}")),
                ..Reply::default()
            },
        };
        if send(&mut output, &reply).is_err() {
            return 1;
        }
    }
}

#[cfg(test)]
mod tests;
