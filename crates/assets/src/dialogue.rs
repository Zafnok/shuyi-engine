//! Dialogue scripts (`.dlg` files): scenes of portraits, speech, narration,
//! reply choices and music changes. [`parse_dlg`] turns a file into scenes,
//! [`check_scene`] and [`check_duplicates`] validate them against the
//! game's [`SceneRules`], and [`print_scene`] writes a scene back out.
//! Every line that shows text has a [`LineId`]. Lines in an `@if` block
//! play only while a character is there ([`Present`]).
//!
//! A game adds directives of its own (a place behind the talk, a cut to a
//! map) with a [`Directive`] type: the parser hands it every `@name` it
//! doesn't know, and a scene keeps the steps it makes in order.
//!
//! As a format (ADR-0007), [`Dlg`] reads a file's scenes
//! ([`DialogueFile`]), and [`scripts_from_parsed`] checks them.
//!
//! The format, for a game that adds no directive:
//!
//! ```text
//! @scene gate_talk
//! @caption The east gate, dusk
//! @left mentor neutral
//! @right scout happy
//! mentor: Is the road clear?
//! scout[sad]: Not since noon.
//!   Riders came over the hill.
//! > Rain begins to fall.
//! @music storm
//! @choice
//! * earnest: Then we wait.
//!   mentor: Agreed.
//! * wry: Then we get wet.
//! @endchoice
//! @if scout
//! scout: I'll keep watch.
//! @else
//! > Nobody keeps watch.
//! @endif
//! @end
//! ```
use crate::prelude::*;
use serde::{Deserialize, Serialize};

mod check;
mod line_id;
mod parse;
mod present;

use alloc::collections::BTreeMap;
use core::fmt;
use core::marker::PhantomData;

pub use shuyi_core::CharacterId;

pub use check::{
    NoTokens, SceneRules, Tokens, check_duplicates, check_scene, lines_at_longest, longest_width,
    token_problems,
};
pub use line_id::{
    Line, LineId, NARRATION_SPEAKER, REPLY_SPEAKER, caption_key, line_hash, scene_of,
};
pub use parse::{Lines, ParsedScene, PartLines, bad_id, char_problem, is_id, parse_dlg};
pub use present::Present;

use crate::error::ContentError;
use crate::format::{AssetKind, AssetPath, DecodeCx, Format, TEXT_MAX_BYTES, text};

/// How much text a game's dialogue box and reply menu hold: the limits the
/// checks keep a script to, counted in cells ([`text_width`]).
///
/// [`text_width`]: crate::lang::text_width
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogueLimits {
    /// Cells per line of the text box.
    pub box_width: usize,
    /// Lines of text the text box shows at once: one page.
    pub box_lines: usize,
    /// Longest text of one speech or narration line.
    pub max_text: usize,
    /// Longest text of a reply choice: it must fit the menu.
    pub max_option: usize,
    /// Fewest replies a `@choice` offers.
    pub min_options: usize,
    /// Most replies a `@choice` offers.
    pub max_options: usize,
    /// Most text steps (speech or narration) in one reply's reaction, so
    /// the scene rejoins quickly.
    pub max_reaction_texts: usize,
}

/// A step a game adds to the format: `@<name> <args>`, for a name the
/// engine's own directives don't use. The parser reads it with
/// [`Directive::parse`], a scene keeps it in order as
/// [`Step::Directive`], and the game plays it.
pub trait Directive: Clone + fmt::Debug + PartialEq + Eq {
    /// Reads the line `@<name> <args>` (`args` trimmed): `None` if none of
    /// the game's directives is called `name`; otherwise the step, if one
    /// could be made, and every problem with the line.
    fn parse(name: &str, args: &str) -> Option<Parsed<Self>>;

    /// The line as written, without its `@` (`place market 3,12`): what
    /// [`print_scene`] writes. Parsing it gives the directive back.
    fn print(&self) -> String;

    /// For a directive that stays in force until the next one of its kind
    /// (a place drawn behind the talk), that kind's name, written as the
    /// directive is (`place`): every part of a `@choice` or an `@if` block
    /// must then leave the same one in force. `None` for one that doesn't
    /// stay.
    fn kind(&self) -> Option<&'static str> {
        None
    }
}

/// What [`Directive::parse`] read from one line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed<D> {
    /// The step, if the line makes one.
    pub step: Option<D>,
    /// What is wrong with the line, if anything.
    pub errors: Vec<String>,
}

impl<D> Parsed<D> {
    /// A good line that makes `step`.
    pub fn step(step: D) -> Self {
        Self {
            step: Some(step),
            errors: Vec::new(),
        }
    }

    /// A line that makes no step, for `error`.
    pub fn error(error: impl Into<String>) -> Self {
        Self {
            step: None,
            errors: vec![error.into()],
        }
    }
}

/// The directives of a game that adds none: there are no such steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoDirective {}

impl Directive for NoDirective {
    fn parse(_name: &str, _args: &str) -> Option<Parsed<Self>> {
        None
    }

    fn print(&self) -> String {
        match *self {}
    }
}

/// A side of the dialogue screen, where one portrait stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Side {
    /// The left portrait.
    Left,
    /// The right portrait.
    Right,
}

impl Side {
    /// The side as written in a script: `left` or `right`.
    pub fn name(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
        }
    }

    /// The other side.
    #[must_use]
    pub fn other(self) -> Self {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

/// One step of a scene, in script order. `D` is the game's own
/// directives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Step<D = NoDirective> {
    /// `@caption <text>`: a location/time caption, shown until the next one.
    Caption {
        /// The caption.
        text: String,
    },
    /// `@left <id> <expression>`: a character enters (or replaces whoever
    /// is) on `side`.
    Place {
        /// Where.
        side: Side,
        /// Who.
        character: CharacterId,
        /// With which expression.
        expression: String,
    },
    /// `@left clear`: the portrait on `side` leaves.
    Clear {
        /// Which side empties.
        side: Side,
    },
    /// `id: text` or `id[expression]: text`: an on-screen character speaks,
    /// first changing expression if one is given.
    Say {
        /// Who speaks.
        speaker: CharacterId,
        /// The speaker's new expression, if it changes.
        expression: Option<String>,
        /// What they say.
        text: String,
        /// The line's id.
        line: LineId,
    },
    /// `> text`: narration, no speaker.
    Narrate {
        /// The narration.
        text: String,
        /// The line's id.
        line: LineId,
    },
    /// `@choice` … `@endchoice`: the replies the player picks from, each
    /// with its own reaction steps. Every option rejoins the scene after
    /// the block.
    Choice {
        /// The options, in menu order.
        options: Vec<ChoiceOption<D>>,
    },
    /// `@music <cue>` or `@music stop`: the music changes when playback
    /// reaches this line, and stays so after the scene.
    Music(MusicLine),
    /// A directive of the game's ([`Directive`]).
    Directive(D),
    /// `@if <character>` … `@else` … `@endif`: steps that play only if the
    /// character is there when the scene plays ([`Present`]), and the
    /// steps that play instead if they aren't.
    If {
        /// Who must be there.
        character: CharacterId,
        /// The steps played if they are.
        then: Vec<Step<D>>,
        /// The steps played if they aren't (none without an `@else`).
        otherwise: Vec<Step<D>>,
    },
}

/// What a `@music` line asks for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MusicLine {
    /// `@music <cue>`: switch to this music cue.
    Cue(String),
    /// `@music stop`: fade the music out.
    Stop,
}

impl MusicLine {
    /// What follows `@music` for [`MusicLine::Stop`].
    pub const STOP: &'static str = "stop";

    /// What follows `@music` in a script: the cue id, or `stop`.
    pub fn arg(&self) -> &str {
        match self {
            MusicLine::Cue(cue) => cue,
            MusicLine::Stop => Self::STOP,
        }
    }
}

/// One reply the player can pick: `* <tone>: <text>`, then its reaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChoiceOption<D = NoDirective> {
    /// The reply's tone (`earnest`, `wry`…), for writers; not shown.
    pub tone: String,
    /// What is said, shown in the menu.
    pub text: String,
    /// The reply's id.
    pub line: LineId,
    /// The reaction: steps played after picking this option (no choices).
    pub steps: Vec<Step<D>>,
}

impl<D> Step<D> {
    /// The text of a text step (one text box): a `Say` or `Narrate`.
    pub fn text(&self) -> Option<&str> {
        match self {
            Step::Say { text, .. } | Step::Narrate { text, .. } => Some(text),
            Step::Caption { .. }
            | Step::Place { .. }
            | Step::Clear { .. }
            | Step::Choice { .. }
            | Step::Music(_)
            | Step::Directive(_)
            | Step::If { .. } => None,
        }
    }

    /// Whether the step is, or has inside it, a speech or narration line.
    pub fn has_text(&self) -> bool {
        let any = |steps: &[Step<D>]| steps.iter().any(Step::has_text);
        match self {
            Step::Choice { options } => options.iter().any(|o| any(&o.steps)),
            Step::If {
                then, otherwise, ..
            } => any(then) || any(otherwise),
            _ => self.text().is_some(),
        }
    }
}

/// One scene: `@scene <id>` … `@end`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scene<D = NoDirective> {
    /// Scene id, unique across every dialogue file.
    pub id: String,
    /// Its steps, in order.
    pub steps: Vec<Step<D>>,
}

impl<D> Default for Scene<D> {
    fn default() -> Self {
        Self {
            id: String::new(),
            steps: Vec::new(),
        }
    }
}

/// Every scene of every dialogue file, by id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogueTable<D = NoDirective> {
    /// Scenes by id.
    pub scenes: BTreeMap<String, Scene<D>>,
    /// The file each scene was written in, by scene id (empty for a table
    /// not read from files).
    pub files: BTreeMap<String, String>,
}

impl<D> Default for DialogueTable<D> {
    fn default() -> Self {
        Self {
            scenes: BTreeMap::new(),
            files: BTreeMap::new(),
        }
    }
}

impl<D> DialogueTable<D> {
    /// The scene with id `id`.
    pub fn get(&self, id: &str) -> Option<&Scene<D>> {
        self.scenes.get(id)
    }
}

/// Every scene of every dialogue file, with where each was written: what
/// checks that need more of the game's data run on once that is loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scripts<D = NoDirective> {
    /// The scenes, by id.
    pub table: DialogueTable<D>,
    /// The scenes as written, in file order.
    pub parsed: Vec<ParsedScene<D>>,
}

impl<D> Default for Scripts<D> {
    fn default() -> Self {
        Self {
            table: DialogueTable::default(),
            parsed: Vec::new(),
        }
    }
}

/// The scenes of one `.dlg` file as written, each with where it is: what
/// the `.dlg` format ([`Dlg`]) reads. [`scripts_from_parsed`] checks them
/// against the game's rules, once its characters, names and music are
/// loaded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogueFile<D = NoDirective> {
    /// Its scenes, in file order.
    pub scenes: Vec<ParsedScene<D>>,
}

impl<D: 'static> AssetKind for DialogueFile<D> {
    const NAME: &'static str = "dialogue";
}

/// The `.dlg` format (`dlg`, `.dlg`) of a game whose own directives are
/// `D`: a file's scenes, with every syntax and structure error
/// [`parse_dlg`] finds. The checks that need the game's rules are
/// [`scripts_from_parsed`]'s.
pub struct Dlg<D = NoDirective>(PhantomData<fn() -> D>);

impl<D> Dlg<D> {
    /// The format, for a game's directives `D`.
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<D> Default for Dlg<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D> fmt::Debug for Dlg<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Dlg")
    }
}

impl<D: Directive + 'static> Format for Dlg<D> {
    type Asset = DialogueFile<D>;

    fn id(&self) -> &'static str {
        "dlg"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["dlg"]
    }

    fn max_bytes(&self) -> usize {
        TEXT_MAX_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        _cx: &DecodeCx<'_>,
    ) -> Result<DialogueFile<D>, Vec<ContentError>> {
        let (scenes, errors) = parse_dlg(file.shown(), text(file, bytes)?);
        if errors.is_empty() {
            Ok(DialogueFile { scenes })
        } else {
            Err(errors)
        }
    }
}

/// Like [`scripts_from_sources`], for files given as `(file name,
/// source)` where a `None` source is a file that isn't valid UTF-8 (an
/// error).
pub fn scripts_from_files<D: Directive>(
    files: &[(String, Option<&str>)],
    rules: &impl SceneRules<D>,
) -> Result<Scripts<D>, Vec<ContentError>> {
    let mut sources = Vec::new();
    let mut errors = Vec::new();
    for (file, source) in files {
        match source {
            Some(source) => sources.push((file, *source)),
            None => errors.push(ContentError::new(file, "file is not valid UTF-8")),
        }
    }
    match scripts_from_sources(&sources, rules) {
        Ok(scripts) if errors.is_empty() => Ok(scripts),
        Ok(_) => Err(errors),
        Err(e) => {
            errors.extend(e);
            Err(errors)
        }
    }
}

/// Parses and validates dialogue files given as `(file name, source)`
/// pairs, each scene with [`check_scene`] and the game's own
/// [`SceneRules::scene_problems`], then the scene ids across files.
/// Reports every problem, ordered by file and line.
pub fn scripts_from_sources<D: Directive, F: AsRef<str>>(
    files: &[(F, &str)],
    rules: &impl SceneRules<D>,
) -> Result<Scripts<D>, Vec<ContentError>> {
    let mut scenes = Vec::new();
    let mut errors = Vec::new();
    for (file, source) in files {
        let (parsed, e) = parse_dlg(file.as_ref(), source);
        errors.extend(e);
        scenes.extend(parsed);
    }
    checked(scenes, errors, rules)
}

/// Validates scenes already parsed (`files`, from the `.dlg` format
/// [`Dlg`], in file order) as [`scripts_from_sources`] does: each scene
/// with [`check_scene`] and the game's own [`SceneRules::scene_problems`],
/// then the scene ids across files.
pub fn scripts_from_parsed<D: Directive>(
    files: impl IntoIterator<Item = DialogueFile<D>>,
    rules: &impl SceneRules<D>,
) -> Result<Scripts<D>, Vec<ContentError>> {
    let scenes = files.into_iter().flat_map(|file| file.scenes).collect();
    checked(scenes, Vec::new(), rules)
}

/// [`Scripts`] of `scenes`, or every problem with them added to `errors`
/// (those found parsing them), ordered by file and line.
fn checked<D: Directive>(
    scenes: Vec<ParsedScene<D>>,
    mut errors: Vec<ContentError>,
    rules: &impl SceneRules<D>,
) -> Result<Scripts<D>, Vec<ContentError>> {
    for s in &scenes {
        errors.extend(check_scene(s, rules));
        errors.extend(rules.scene_problems(s));
    }
    errors.extend(check_duplicates(&scenes));
    if errors.is_empty() {
        let table = DialogueTable {
            scenes: scenes
                .iter()
                .map(|p| (p.scene.id.clone(), p.scene.clone()))
                .collect(),
            files: scenes
                .iter()
                .map(|p| (p.scene.id.clone(), p.file.clone()))
                .collect(),
        };
        Ok(Scripts {
            table,
            parsed: scenes,
        })
    } else {
        errors.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
        Err(errors)
    }
}

/// Writes `scene` in `.dlg` format (one line per step, reactions indented
/// under their option, an `@if` block's lines as far in as its `@if`),
/// ending with `@end` and a newline. Parsing the result gives `scene`
/// back.
pub fn print_scene<D: Directive>(scene: &Scene<D>) -> String {
    let mut out = format!("@scene {}\n", scene.id);
    for step in &scene.steps {
        print_step(&mut out, step, "");
    }
    out.push_str("@end\n");
    out
}

/// Appends `step`'s line(s) to `out`, each starting with `indent`.
fn print_step<D: Directive>(out: &mut String, step: &Step<D>, indent: &str) {
    let line = match step {
        Step::Caption { text } => format!("@caption {text}"),
        Step::Place {
            side,
            character,
            expression,
        } => format!("@{} {} {expression}", side.name(), character.0),
        Step::Clear { side } => format!("@{} clear", side.name()),
        Step::Say {
            speaker,
            expression: Some(e),
            text,
            ..
        } => format!("{}[{e}]: {text}", speaker.0),
        Step::Say {
            speaker,
            expression: None,
            text,
            ..
        } => format!("{}: {text}", speaker.0),
        Step::Narrate { text, .. } => format!("> {text}"),
        Step::Music(music) => format!("@music {}", music.arg()),
        Step::Directive(directive) => format!("@{}", directive.print()),
        Step::Choice { options } => {
            out.push_str("@choice\n");
            for o in options {
                for part in ["* ", &o.tone, ": ", &o.text, "\n"] {
                    out.push_str(part);
                }
                for s in &o.steps {
                    print_step(out, s, "  ");
                }
            }
            "@endchoice".to_owned()
        }
        Step::If {
            character,
            then,
            otherwise,
        } => {
            for part in [indent, "@if ", &character.0, "\n"] {
                out.push_str(part);
            }
            for s in then {
                print_step(out, s, indent);
            }
            if !otherwise.is_empty() {
                out.push_str(indent);
                out.push_str("@else\n");
            }
            for s in otherwise {
                print_step(out, s, indent);
            }
            "@endif".to_owned()
        }
    };
    out.push_str(indent);
    out.push_str(&line);
    out.push('\n');
}

#[cfg(test)]
pub(crate) mod tests;
