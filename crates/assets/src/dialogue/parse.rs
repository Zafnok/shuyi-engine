//! The `.dlg` parser: lines → scenes. Reports syntax and structure errors
//! (unknown directives, bad ids, missing `@end`, blocks left open, non-ASCII
//! text); checks that need the whole scene or other files are in `check`.
use crate::prelude::*;

use super::{CharacterId, ChoiceOption, Directive, LineId, MusicLine, Scene, Side, Step};
use crate::error::ContentError;

/// A scene as written in a file, with the source lines the checks point at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedScene<D = super::NoDirective> {
    /// The scene.
    pub scene: Scene<D>,
    /// The file it is in (for error messages).
    pub file: String,
    /// 1-based line of its `@scene`.
    pub line: u32,
    /// Where its steps are.
    pub lines: Lines,
}

/// Where the steps of one list of steps (a scene's, a reaction's, one part
/// of an `@if` block) are in the file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Lines {
    /// 1-based line where each step starts, in the order of the steps.
    pub steps: Vec<u32>,
    /// The parts of each block among the steps (a `Step::Choice` or a
    /// `Step::If`), in the order of the blocks: a choice's options; an
    /// `@if` block's two parts, the steps played with the character there
    /// and those played without.
    pub blocks: Vec<Vec<PartLines>>,
}

/// Where one part of a block is: an option of a `@choice`, or one of the
/// two parts of an `@if` block.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PartLines {
    /// 1-based line that starts it: the option's `* tone: text`, the
    /// `@if`, or the `@else` (the `@if`'s line when it has no `@else`).
    pub line: u32,
    /// Where its steps are.
    pub lines: Lines,
}

/// A block being read.
enum Open<D> {
    /// `@choice`, until its `@endchoice`.
    Choice {
        /// Line of its `@choice`.
        line: u32,
        options: Vec<ChoiceOption<D>>,
        parts: Vec<PartLines>,
    },
    /// `@if`, until its `@endif`.
    If {
        /// Line of its `@if`.
        line: u32,
        character: CharacterId,
        then: (Vec<Step<D>>, Lines),
        /// The line of its `@else` and the steps after it, once read.
        otherwise: Option<(u32, Vec<Step<D>>, Lines)>,
    },
}

impl<D> Open<D> {
    /// The error for a block still open where its scene or reaction ends.
    fn unclosed(&self) -> (u32, &'static str) {
        match self {
            Open::Choice { line, .. } => (*line, "@choice has no @endchoice"),
            Open::If { line, .. } => (*line, "@if has no @endif"),
        }
    }
}

/// How far reaction lines are indented under their option.
const REACTION_INDENT: usize = 2;

/// Parses dialogue `source` (errors attributed to `file`) into its scenes,
/// in file order, and every syntax error found. A scene with errors in it
/// is still returned (without the broken lines), so later checks can run.
/// A directive the engine doesn't know is read by the game's `D`.
pub fn parse_dlg<D: Directive>(
    file: &str,
    source: &str,
) -> (Vec<ParsedScene<D>>, Vec<ContentError>) {
    let mut p = Parser {
        file,
        scenes: Vec::new(),
        open: None,
        blocks: Vec::new(),
        continuable: false,
        errors: Vec::new(),
    };
    for (i, line) in source.split('\n').enumerate() {
        p.line(u32::try_from(i + 1).unwrap_or(u32::MAX), line);
    }
    p.close_unclosed();
    if let Some(open) = p.open.take() {
        p.no_end(&open);
        p.finish(open);
    }
    (p.scenes, p.errors)
}

/// Whether `s` is a valid scene, character or expression id: lowercase
/// ASCII letters, digits and `_`, not empty.
pub fn is_id(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// What is wrong with character `c` in dialogue text, if anything. Text
/// must be printable ASCII (the font may lack anything else); common
/// typographic punctuation gets its ASCII spelling suggested.
pub fn char_problem(c: char) -> Option<String> {
    if (' '..='~').contains(&c) {
        return None;
    }
    let ascii = match c {
        '\u{2018}' | '\u{2019}' => "'",
        '\u{201C}' | '\u{201D}' => "\"",
        '\u{2026}' => "...",
        '\u{2014}' => "--",
        '\u{2013}' => "-",
        _ => {
            return Some(format!(
                "unsupported character {c:?}; dialogue text is plain ASCII"
            ));
        }
    };
    Some(format!("non-ASCII punctuation {c:?}; use {ascii} instead"))
}

/// What an id that isn't one ([`is_id`]) is told.
pub fn bad_id(id: &str) -> String {
    format!("\"{id}\" is not a valid id; use lowercase letters, digits and _")
}

struct Parser<'a, D> {
    file: &'a str,
    scenes: Vec<ParsedScene<D>>,
    /// The scene being read (after `@scene`, before `@end`).
    open: Option<ParsedScene<D>>,
    /// The blocks being read inside `open`, the innermost last. At most one
    /// is a `@choice`.
    blocks: Vec<Open<D>>,
    /// Whether the last line was speech or narration, so an indented line
    /// continues it.
    continuable: bool,
    errors: Vec<ContentError>,
}

impl<D: Directive> Parser<'_, D> {
    fn err(&mut self, line: u32, message: impl Into<String>) {
        self.errors
            .push(ContentError::new(self.file, message).at(line, None));
    }

    fn no_end(&mut self, open: &ParsedScene<D>) {
        self.err(
            open.line,
            format!("scene \"{}\" has no @end", open.scene.id),
        );
    }

    /// Adds the complete scene `open` to the scenes read, giving its lines
    /// their ids (their text is final only now: continuation lines add to
    /// it).
    fn finish(&mut self, mut open: ParsedScene<D>) {
        open.scene.assign_line_ids();
        self.scenes.push(open);
    }

    /// Reads line `n` (1-based).
    fn line(&mut self, n: u32, raw: &str) {
        let line = raw.trim_end();
        let body = line.trim_start_matches(' ');
        if body.is_empty() || body.starts_with('#') {
            return;
        }
        let indent = line.len() - body.len();
        if self.in_choice() {
            match indent {
                0 => self.choice_line(n, line, body),
                1..REACTION_INDENT => self.err(n, "reaction lines are indented by two spaces"),
                REACTION_INDENT => self.statement(n, line, body),
                _ => self.continuation(n, line, body),
            }
            return;
        }
        if indent > 0 {
            self.continuation(n, line, body);
            return;
        }
        if body.starts_with('*') {
            self.err(
                n,
                "an option (\"* tone: text\") must be inside a @choice block",
            );
            return;
        }
        self.statement(n, line, body);
    }

    /// A step line (`body` is `line` without its indentation): directive,
    /// narration or speech.
    fn statement(&mut self, n: u32, line: &str, body: &str) {
        self.continuable = false;
        if let Some(rest) = body.strip_prefix('@') {
            self.directive(n, line, rest);
        } else if let Some(rest) = body.strip_prefix('>') {
            let text = rest.trim();
            if text.is_empty() {
                self.err(n, "narration has no text");
            } else {
                self.check_chars(n, line, text);
                self.push(
                    n,
                    Step::Narrate {
                        text: text.into(),
                        line: LineId::default(),
                    },
                );
                self.continuable = self.open.is_some();
            }
        } else {
            self.speech(n, line, body);
        }
    }

    /// An unindented line inside a `@choice` block: an option,
    /// `@endchoice`, or a line that ends the block early.
    fn choice_line(&mut self, n: u32, line: &str, body: &str) {
        self.continuable = false;
        let name = body.split(' ').next().unwrap_or("");
        let option = body.strip_prefix('*');
        let ends = matches!(name, "@endchoice" | "@choice" | "@end" | "@scene");
        if option.is_none() && !ends {
            self.err(
                n,
                "inside @choice, a line is an option (\"* tone: text\"), an indented reaction line or @endchoice",
            );
            return;
        }
        // The reaction above is over: an `@if` it opened must be closed.
        self.close_reaction_ifs();
        match option {
            Some(rest) => self.option(n, line, rest),
            None => self.directive(n, line, &body[1..]),
        }
    }

    /// Whether a `@choice` block is open.
    fn in_choice(&self) -> bool {
        self.blocks.iter().any(|b| matches!(b, Open::Choice { .. }))
    }

    /// Reports and closes every `@if` block still open inside the open
    /// `@choice`: its reaction ends here.
    fn close_reaction_ifs(&mut self) {
        let is_if = |block: &mut Open<D>| matches!(block, Open::If { .. });
        while let Some(block) = self.blocks.pop_if(is_if) {
            let (line, message) = block.unclosed();
            self.err(line, message);
            self.add_block(block);
        }
    }

    /// `* <tone>: <text>` on line `n`; `rest` follows the `*`.
    fn option(&mut self, n: u32, line: &str, rest: &str) {
        let parsed = rest
            .strip_prefix(' ')
            .and_then(|r| r.split_once(':'))
            .map(|(tone, text)| (tone.trim(), text.trim()))
            .filter(|(tone, text)| !tone.is_empty() && !text.is_empty());
        let Some((tone, text)) = parsed else {
            self.err(n, "an option needs a tone and text: \"* tone: text\"");
            return;
        };
        if !is_id(tone) {
            self.bad_id(n, tone);
        }
        self.check_chars(n, line, text);
        if let Some(Open::Choice { options, parts, .. }) = self.blocks.last_mut() {
            options.push(ChoiceOption {
                tone: tone.into(),
                text: text.into(),
                line: LineId::default(),
                steps: Vec::new(),
            });
            parts.push(PartLines {
                line: n,
                lines: Lines::default(),
            });
        }
    }

    /// `@choice` on line `n`.
    fn open_choice(&mut self, n: u32, args: &str) {
        if !args.is_empty() {
            self.err(n, "@choice takes nothing after it");
        }
        if self.in_choice() {
            self.err(n, "@choice can't be nested inside another @choice");
            return;
        }
        if self.open.is_none() {
            self.err(n, "line is outside a scene; start one with @scene <id>");
            return;
        }
        self.blocks.push(Open::Choice {
            line: n,
            options: Vec::new(),
            parts: Vec::new(),
        });
    }

    /// `@endchoice` on line `n`.
    fn end_choice(&mut self, n: u32, args: &str) {
        if !args.is_empty() {
            self.err(n, "@endchoice takes nothing after it");
        }
        if !self.in_choice() {
            self.err(n, "@endchoice without an open @choice");
            return;
        }
        self.close_reaction_ifs();
        if let Some(choice) = self.blocks.pop() {
            self.add_block(choice);
        }
    }

    /// `@if <character>` on line `n`.
    fn open_if(&mut self, n: u32, args: &str) {
        let words: Vec<&str> = args.split_whitespace().collect();
        let character = if let [character] = words[..] {
            if !is_id(character) {
                self.bad_id(n, character);
            }
            character
        } else {
            self.err(n, "@if needs one character id");
            ""
        };
        if self.target().is_none() {
            self.no_target(n);
            return;
        }
        self.blocks.push(Open::If {
            line: n,
            character: CharacterId(character.into()),
            then: (Vec::new(), Lines::default()),
            otherwise: None,
        });
    }

    /// `@else` on line `n`.
    fn else_if(&mut self, n: u32, args: &str) {
        if !args.is_empty() {
            self.err(n, "@else takes nothing after it");
        }
        match self.blocks.last_mut() {
            Some(Open::If {
                otherwise: Some((first, ..)),
                ..
            }) => {
                let message = format!("@if already has an @else, on line {first}");
                self.err(n, message);
            }
            Some(Open::If { otherwise, .. }) => {
                *otherwise = Some((n, Vec::new(), Lines::default()));
            }
            _ => self.err(n, "@else without an open @if"),
        }
    }

    /// `@endif` on line `n`.
    fn end_if(&mut self, n: u32, args: &str) {
        if !args.is_empty() {
            self.err(n, "@endif takes nothing after it");
        }
        match self.blocks.pop_if(|block| matches!(block, Open::If { .. })) {
            Some(block) => self.add_block(block),
            None => self.err(n, "@endif without an open @if"),
        }
    }

    /// Adds `block`, just closed (taken off the open blocks), as one step
    /// to what it was in.
    fn add_block(&mut self, block: Open<D>) {
        let (line, step, parts) = match block {
            Open::Choice {
                line,
                options,
                parts,
            } => (line, Step::Choice { options }, parts),
            Open::If {
                line,
                character,
                then,
                otherwise,
            } => {
                let (else_line, otherwise, else_lines) =
                    otherwise.unwrap_or((line, Vec::new(), Lines::default()));
                let step = Step::If {
                    character,
                    then: then.0,
                    otherwise,
                };
                let part = |line, lines| PartLines { line, lines };
                (
                    line,
                    step,
                    vec![part(line, then.1), part(else_line, else_lines)],
                )
            }
        };
        if let Some((steps, lines)) = self.target() {
            steps.push(step);
            lines.steps.push(line);
            lines.blocks.push(parts);
        }
    }

    /// Reports every block still open (its scene or file ends first) and
    /// closes it.
    fn close_unclosed(&mut self) {
        while let Some(block) = self.blocks.pop() {
            let (line, message) = block.unclosed();
            self.err(line, message);
            self.add_block(block);
        }
    }

    /// Where the next step goes: the part being read of the innermost open
    /// block (an `@if` block's steps, or its `@else`'s; a `@choice`'s last
    /// option's reaction), or else the open scene. `None` outside a scene
    /// and in a `@choice` before its first option.
    fn target(&mut self) -> Option<(&mut Vec<Step<D>>, &mut Lines)> {
        match self.blocks.last_mut() {
            Some(Open::If {
                otherwise: Some((_, steps, lines)),
                ..
            }) => Some((steps, lines)),
            Some(Open::If { then, .. }) => Some((&mut then.0, &mut then.1)),
            Some(Open::Choice { options, parts, .. }) => {
                match (options.last_mut(), parts.last_mut()) {
                    (Some(option), Some(part)) => Some((&mut option.steps, &mut part.lines)),
                    _ => None,
                }
            }
            None => self
                .open
                .as_mut()
                .map(|open| (&mut open.scene.steps, &mut open.lines)),
        }
    }

    /// Reports that the line `n` has nowhere to go ([`Self::target`]).
    fn no_target(&mut self, n: u32) {
        let message = if self.blocks.is_empty() {
            "line is outside a scene; start one with @scene <id>"
        } else {
            "a reaction line needs an option above it: \"* tone: text\""
        };
        self.err(n, message);
    }

    /// The step an indented line would continue: the last one read.
    fn last_step_mut(&mut self) -> Option<&mut Step<D>> {
        self.target().and_then(|(steps, _)| steps.last_mut())
    }

    /// An indented line: more text for the speech or narration above.
    fn continuation(&mut self, n: u32, line: &str, body: &str) {
        if body.starts_with('@') {
            self.err(n, "directives can't be indented");
            return;
        }
        let continuable = self.continuable;
        let last = self.last_step_mut().filter(|_| continuable);
        match last {
            Some(Step::Say { text, .. } | Step::Narrate { text, .. }) => {
                text.push(' ');
                text.push_str(body);
                self.check_chars(n, line, body);
            }
            _ => self.err(
                n,
                "indented line doesn't continue a speech or narration line",
            ),
        }
    }

    /// `@<rest>` on line `n`.
    fn directive(&mut self, n: u32, line: &str, rest: &str) {
        let (name, args) = rest
            .split_once(' ')
            .map_or((rest, ""), |(name, args)| (name, args.trim()));
        match name {
            "scene" => self.scene(n, args),
            "end" => self.end(n, args),
            "caption" if args.is_empty() => self.err(n, "@caption has no text"),
            "caption" => {
                self.check_chars(n, line, args);
                self.push(n, Step::Caption { text: args.into() });
            }
            "left" => self.place(n, Side::Left, args),
            "right" => self.place(n, Side::Right, args),
            "music" => self.music(n, args),
            "choice" => self.open_choice(n, args),
            "endchoice" => self.end_choice(n, args),
            "if" => self.open_if(n, args),
            "else" => self.else_if(n, args),
            "endif" => self.end_if(n, args),
            _ => match D::parse(name, args) {
                Some(parsed) => self.game_directive(n, parsed),
                None => self.err(n, format!("unknown directive \"@{name}\"")),
            },
        }
    }

    /// A directive of the game's on line `n`, as [`Directive::parse`] read
    /// it.
    fn game_directive(&mut self, n: u32, parsed: super::Parsed<D>) {
        for error in parsed.errors {
            self.err(n, error);
        }
        if let Some(step) = parsed.step {
            self.push(n, Step::Directive(step));
        }
    }

    fn scene(&mut self, n: u32, args: &str) {
        self.close_unclosed();
        if let Some(open) = self.open.take() {
            self.no_end(&open);
            self.finish(open);
        }
        let ok = args.split_whitespace().count() == 1;
        if !ok {
            self.err(n, "@scene needs one scene id");
        } else if !is_id(args) {
            self.bad_id(n, args);
        }
        self.open = Some(ParsedScene {
            scene: Scene {
                id: args.into(),
                steps: Vec::new(),
            },
            file: self.file.into(),
            line: n,
            lines: Lines::default(),
        });
    }

    fn end(&mut self, n: u32, args: &str) {
        if !args.is_empty() {
            self.err(n, "@end takes nothing after it");
        }
        self.close_unclosed();
        match self.open.take() {
            Some(open) => self.finish(open),
            None => self.err(n, "@end without an open @scene"),
        }
    }

    /// `@left`/`@right` with `args`: `<id> <expression>` or `clear`.
    fn place(&mut self, n: u32, side: Side, args: &str) {
        let args: Vec<&str> = args.split_whitespace().collect();
        let step = match args[..] {
            ["clear"] => Step::Clear { side },
            [character, expression] if character != "clear" => {
                if !is_id(character) {
                    self.bad_id(n, character);
                }
                if !is_id(expression) {
                    self.bad_id(n, expression);
                }
                Step::Place {
                    side,
                    character: CharacterId(character.into()),
                    expression: expression.into(),
                }
            }
            _ => {
                self.err(
                    n,
                    format!(
                        "@{} needs a character id and an expression, or \"clear\"",
                        side.name()
                    ),
                );
                return;
            }
        };
        self.push(n, step);
    }

    /// `@music` with `args`: one music cue id, or `stop`.
    fn music(&mut self, n: u32, args: &str) {
        let args: Vec<&str> = args.split_whitespace().collect();
        let [arg] = args[..] else {
            self.err(n, "@music needs one music cue id, or \"stop\"");
            return;
        };
        let music = if arg == MusicLine::STOP {
            MusicLine::Stop
        } else {
            if !is_id(arg) {
                self.bad_id(n, arg);
            }
            MusicLine::Cue(arg.into())
        };
        self.push(n, Step::Music(music));
    }

    /// `id: text` or `id[expression]: text` on line `n`.
    fn speech(&mut self, n: u32, line: &str, body: &str) {
        let Some((head, text)) = body.split_once(':').filter(|(h, _)| !h.contains(' ')) else {
            self.err(
                n,
                "unrecognised line; expected \"@directive\", \"> narration\", \"speaker: text\" or \"# comment\"",
            );
            return;
        };
        let (speaker, expression) = match head.strip_suffix(']').and_then(|h| h.split_once('[')) {
            Some((speaker, expression)) => (speaker, Some(expression)),
            None => (head, None),
        };
        for id in core::iter::once(speaker).chain(expression) {
            if !is_id(id) {
                self.bad_id(n, id);
            }
        }
        let text = text.trim();
        if text.is_empty() {
            self.err(n, format!("\"{head}:\" has no text"));
            return;
        }
        self.check_chars(n, line, text);
        self.push(
            n,
            Step::Say {
                speaker: CharacterId(speaker.into()),
                expression: expression.map(Into::into),
                text: text.into(),
                line: LineId::default(),
            },
        );
        self.continuable = self.open.is_some();
    }

    fn bad_id(&mut self, n: u32, id: &str) {
        self.err(n, bad_id(id));
    }

    /// Adds `step` (from line `n`) to where steps go now
    /// ([`Self::target`]).
    fn push(&mut self, n: u32, step: Step<D>) {
        match self.target() {
            Some((steps, lines)) => {
                steps.push(step);
                lines.steps.push(n);
            }
            None => self.no_target(n),
        }
    }

    /// Reports every unsupported character of `text`, a suffix of `line`,
    /// with its column.
    fn check_chars(&mut self, n: u32, line: &str, text: &str) {
        let before = line[..line.len() - text.len()].chars().count();
        for (i, c) in text.chars().enumerate() {
            if let Some(message) = char_problem(c) {
                let column = u32::try_from(before + i + 1).unwrap_or(u32::MAX);
                self.errors
                    .push(ContentError::new(self.file, message).at(n, Some(column)));
            }
        }
    }
}
