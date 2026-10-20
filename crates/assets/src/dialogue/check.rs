//! Checks on parsed scenes: who is on screen, known characters and
//! expressions, text length, tokens and names written out, reply choices,
//! `@if` blocks, music cues, line ids that clash, and scene ids unique
//! across files. What the game knows (its characters, portraits, tokens and
//! cues) comes from its [`SceneRules`].
use crate::prelude::*;

use alloc::collections::BTreeMap;

use shuyi_core::text::{Part, split_tokens};

use super::line_id::line_hash;
use super::parse::{Lines, ParsedScene, PartLines};
use super::{
    CharacterId, ChoiceOption, DialogueLimits, Directive, MusicLine, NARRATION_SPEAKER,
    REPLY_SPEAKER, Side, Step,
};
use crate::error::ContentError;
use crate::lang::text_width;
use crate::names::{Names, is_name_id, name_token};
use crate::text::word_wrap;

/// The `{tokens}` a game fills in itself, besides the names table's
/// (`{n:<id>}`, [`Names`]): a player-named hero's name, say.
pub trait Tokens {
    /// The most cells `{token}` can become, or `None` if it isn't one of
    /// the game's tokens.
    fn longest(&self, token: &str) -> Option<usize>;

    /// What a `{token}` that is neither the game's nor a name token is
    /// told.
    fn unknown(&self, token: &str) -> String {
        format!("unknown token \"{{{token}}}\"; use a name, {{n:<id>}}")
    }

    /// What the name token `{token}` (naming `id`) is told before the
    /// table is asked: a name the game fills in some other way. `None`
    /// for most ids.
    fn reserved_name(&self, _id: &str, _token: &str) -> Option<String> {
        None
    }

    /// What the name token `{token}` is told when `id` isn't in the names
    /// table.
    fn unknown_name(&self, id: &str, token: &str) -> String {
        format!("unknown name id \"{id}\" in {{{token}}}")
    }

    /// What a display name written out in a line (`form`, of name `id`)
    /// is told instead of the usual advice, if the game fills that name in
    /// some other way.
    fn written_name(&self, _id: &str, _form: &str) -> Option<String> {
        None
    }
}

/// The tokens of a game that fills in nothing but names.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoTokens;

impl Tokens for NoTokens {
    fn longest(&self, _token: &str) -> Option<usize> {
        None
    }
}

/// What [`check_scene`] checks a scene against: the game's limits, tokens
/// and names, and whatever it knows of its characters, portraits and
/// music. A check the game has nothing for passes.
pub trait SceneRules<D> {
    /// How much text the box and the menu hold.
    fn limits(&self) -> &DialogueLimits;

    /// The game's own `{tokens}`.
    fn tokens(&self) -> &dyn Tokens;

    /// The names table, when it loaded: name tokens and names written out
    /// are checked against it, and every name token counts as its longest
    /// name.
    fn names(&self) -> Option<&Names> {
        None
    }

    /// Whether `character` may be placed and speak.
    fn can_speak(&self, _character: &CharacterId) -> bool {
        true
    }

    /// What is wrong with `character` showing `expression`, if anything.
    fn expression_problem(&self, _character: &CharacterId, _expression: &str) -> Option<String> {
        None
    }

    /// What is wrong with `speaker` saying a line of `cells` cells (at
    /// most, tokens filled in), if anything, beyond the box's limit.
    fn speech_problem(&self, _speaker: &CharacterId, _cells: usize) -> Option<String> {
        None
    }

    /// What is wrong with `@music <cue>`, if anything.
    fn music_problem(&self, _cue: &str) -> Option<String> {
        None
    }

    /// The game's own checks on `parsed`, which
    /// [`scripts_from_sources`](super::scripts_from_sources) runs after
    /// [`check_scene`].
    fn scene_problems(&self, _parsed: &ParsedScene<D>) -> Vec<ContentError> {
        Vec::new()
    }
}

/// Who stands on the left and right.
type Screen<'a> = [Option<&'a CharacterId>; 2];

/// Checks one scene against `rules`, replaying it to know who is on
/// screen at each line.
pub fn check_scene<D: Directive>(
    parsed: &ParsedScene<D>,
    rules: &impl SceneRules<D>,
) -> Vec<ContentError> {
    let mut c = Checker {
        file: &parsed.file,
        rules,
        limits: *rules.limits(),
        tokens: rules.tokens(),
        names: rules.names(),
        hashes: BTreeMap::new(),
        errors: Vec::new(),
    };
    let mut state = State::default();
    c.steps(&parsed.scene.steps, &parsed.lines, &mut state);
    if !parsed.scene.steps.iter().any(Step::has_text) {
        c.err(
            parsed.line,
            format!("scene \"{}\" has no speech or narration", parsed.scene.id),
        );
    }
    c.errors
}

/// The most speech and narration lines `steps` can play: an `@if` block
/// counts as its longer part.
fn texts<D>(steps: &[Step<D>]) -> usize {
    let count = |step: &Step<D>| match step {
        Step::If {
            then, otherwise, ..
        } => texts(then).max(texts(otherwise)),
        _ => usize::from(step.text().is_some()),
    };
    steps.iter().map(count).sum()
}

/// What the replay knows at a point of the scene: who is on screen, the
/// caption, and the game's directives in force, by kind
/// ([`Directive::kind`]).
#[derive(Debug, Clone, PartialEq, Eq)]
struct State<'a, D> {
    screen: Screen<'a>,
    caption: Option<&'a str>,
    in_force: BTreeMap<&'static str, &'a D>,
}

impl<D> Default for State<'_, D> {
    fn default() -> Self {
        Self {
            screen: [None; 2],
            caption: None,
            in_force: BTreeMap::new(),
        }
    }
}

impl<D: PartialEq> State<'_, D> {
    /// The first kind (by name) of directive that `self` and `other` leave
    /// different ones of in force.
    fn other_directive(&self, other: &Self) -> Option<&'static str> {
        let mut kinds: Vec<&'static str> = self
            .in_force
            .keys()
            .chain(other.in_force.keys())
            .copied()
            .collect();
        kinds.sort_unstable();
        kinds
            .into_iter()
            .find(|kind| self.in_force.get(kind) != other.in_force.get(kind))
    }
}

struct Checker<'a, R> {
    file: &'a str,
    rules: &'a R,
    limits: DialogueLimits,
    tokens: &'a dyn Tokens,
    names: Option<&'a Names>,
    /// The first line of the scene with each line-id hash: its source
    /// line, speaker and text.
    hashes: BTreeMap<String, (u32, String, String)>,
    errors: Vec<ContentError>,
}

/// Cells `text` can take once every token is filled in, at most
/// ([`text_width`]): the game's `tokens` at their longest
/// ([`Tokens::longest`]) and every name token as `longest_name` (the
/// longest name in the table, so a rename can't push the line over its
/// limit). Unknown tokens, and name tokens without a table, count as
/// written.
pub fn longest_width(text: &str, tokens: &dyn Tokens, longest_name: Option<usize>) -> usize {
    split_tokens(text)
        .map(|part| match part {
            Part::Text(t) | Part::Unclosed(t) => text_width(t),
            Part::Token(t) => token_cells(t, tokens, longest_name),
        })
        .sum()
}

/// Cells `{token}` takes at its longest, as [`longest_width`] counts it.
fn token_cells(token: &str, tokens: &dyn Tokens, longest_name: Option<usize>) -> usize {
    match (name_token(token), longest_name) {
        (Some(_), Some(longest)) => longest,
        _ => tokens.longest(token).unwrap_or(text_width(token) + 2),
    }
}

/// Lines `text` takes in a box `width` cells wide ([`word_wrap`]) once
/// every token is filled in as its longest: each token is written out as
/// that many letters, with the tokens and names counted as in
/// [`longest_width`]. A line that fits the box here fits it with any
/// player's name.
pub fn lines_at_longest(
    text: &str,
    tokens: &dyn Tokens,
    longest_name: Option<usize>,
    width: usize,
) -> usize {
    let filled: String = split_tokens(text)
        .map(|part| match part {
            Part::Text(t) | Part::Unclosed(t) => t.to_owned(),
            Part::Token(t) => "W".repeat(token_cells(t, tokens, longest_name)),
        })
        .collect();
    word_wrap(&filled, width).len()
}

/// What is wrong with name token `{token}`, if anything: its id must be in
/// the names table, and not one the game fills in itself.
fn name_token_problem(token: &str, tokens: &dyn Tokens, names: Option<&Names>) -> Option<String> {
    let id = name_token(token).map_or("", |t| t.id);
    if let Some(problem) = tokens.reserved_name(id, token) {
        Some(problem)
    } else if !is_name_id(id) || names.is_some_and(|n| n.get(id).is_none()) {
        Some(tokens.unknown_name(id, token))
    } else {
        None
    }
}

/// What is wrong with the `{...}` of `text`, in order: each must be one of
/// the game's `tokens` or a name token with an id in `names` (any valid id
/// without a table), and every `{` must close. The rule for a script and
/// for every language pack's dialogue text.
pub fn token_problems(text: &str, tokens: &dyn Tokens, names: Option<&Names>) -> Vec<String> {
    split_tokens(text)
        .filter_map(|part| match part {
            Part::Token(t) if name_token(t).is_some() => name_token_problem(t, tokens, names),
            Part::Token(t) if tokens.longest(t).is_none() => Some(tokens.unknown(t)),
            Part::Unclosed(_) => Some("\"{\" has no closing \"}\"".to_owned()),
            Part::Text(_) | Part::Token(_) => None,
        })
        .collect()
}

fn slot(side: Side) -> usize {
    match side {
        Side::Left => 0,
        Side::Right => 1,
    }
}

impl<R> Checker<'_, R> {
    fn err(&mut self, line: u32, message: String) {
        self.errors
            .push(ContentError::new(self.file, message).at(line, None));
    }

    /// Reports a line (`speaker` saying `text`) whose id hash an earlier,
    /// different line of the scene has: their ids would depend on their
    /// order. A repeat of the same line is fine (it gets `_2`).
    fn line_id(&mut self, line: u32, speaker: &str, text: &str) {
        let hash = line_hash(speaker, text);
        match self.hashes.get(&hash) {
            None => {
                self.hashes
                    .insert(hash, (line, speaker.to_owned(), text.to_owned()));
            }
            Some((_, s, t)) if s == speaker && t == text => {}
            Some((first, _, t)) => {
                let message = format!(
                    "this line and line {first} (\"{t}\") get the same line id hash \
                     {hash}; reword one of them"
                );
                self.err(line, message);
            }
        }
    }

    /// [`lines_at_longest`] of `text`, with the names of the table.
    fn pages(&self, text: &str) -> usize {
        let name = self.names.map(Names::longest);
        lines_at_longest(text, self.tokens, name, self.limits.box_width)
    }

    /// [`longest_width`] of `text`, with the names of the table.
    fn len(&self, text: &str) -> usize {
        longest_width(text, self.tokens, self.names.map(Names::longest))
    }

    /// Reports a display name from the names table written out in `text`
    /// instead of its token, which a rename would leave behind.
    fn literal_names(&mut self, line: u32, text: &str) {
        let Some(names) = self.names else {
            return;
        };
        let Some((id, form)) = names.literal_in(text) else {
            return;
        };
        let message = self.tokens.written_name(id, form).unwrap_or_else(|| {
            let name = names.get(id).unwrap_or(form);
            format!(
                "\"{form}\" is written out; write {{n:{id}}} (\"{name}\") so a rename reaches this line"
            )
        });
        self.err(line, message);
    }

    /// Reports every `{...}` in `text` that isn't one of the game's tokens
    /// or a name token with a known id ([`token_problems`]).
    fn tokens(&mut self, line: u32, text: &str) {
        for message in token_problems(text, self.tokens, self.names) {
            self.err(line, message);
        }
    }
}

impl<R> Checker<'_, R> {
    /// Checks `steps`, written at `lines`, applying them to `state`.
    fn steps<'s, D: Directive>(
        &mut self,
        steps: &'s [Step<D>],
        lines: &Lines,
        state: &mut State<'s, D>,
    ) where
        R: SceneRules<D>,
    {
        let mut blocks = lines.blocks.iter();
        for (step, &line) in steps.iter().zip(&lines.steps) {
            match step {
                Step::Choice { options } => {
                    if let Some(parts) = blocks.next() {
                        self.choice(line, options, parts, state);
                    }
                }
                Step::If {
                    character,
                    then,
                    otherwise,
                } => {
                    if let Some(parts) = blocks.next() {
                        self.branch(line, character, [then, otherwise], parts, state);
                    }
                }
                _ => self.step(step, line, state),
            }
        }
    }

    /// Checks an `@if` block on `line` and applies it to `state`: its
    /// character can speak, and its two parts leave the screen, the
    /// caption and the directives in force the same.
    fn branch<'s, D: Directive>(
        &mut self,
        line: u32,
        character: &CharacterId,
        branches: [&'s [Step<D>]; 2],
        parts: &[PartLines],
        state: &mut State<'s, D>,
    ) where
        R: SceneRules<D>,
    {
        if !self.rules.can_speak(character) {
            self.err(line, format!("unknown character \"{}\"", character.0));
        }
        let mut after = [state.clone(), state.clone()];
        for ((steps, part), after) in branches.into_iter().zip(parts).zip(&mut after) {
            self.steps(steps, &part.lines, after);
        }
        let [with, without] = after;
        // Reported where the part without them starts, if it is written.
        let at = parts.get(1).map_or(line, |p| p.line);
        if with.screen != without.screen {
            self.err(
                at,
                format!(
                    "with \"{}\" here {}; without, {}; an @if block must leave the same \
                     characters on screen either way",
                    character.0,
                    describe(with.screen),
                    describe(without.screen)
                ),
            );
        } else if with.caption != without.caption {
            self.err(
                at,
                format!(
                    "this @if block leaves a different caption with \"{}\" here and without; \
                     it must leave the same caption either way",
                    character.0
                ),
            );
        } else if let Some(kind) = with.other_directive(&without) {
            self.err(
                at,
                format!(
                    "this @if block leaves a different @{kind} with \"{}\" here and without; \
                     it must leave the same {kind} either way",
                    character.0
                ),
            );
        }
        *state = with;
    }

    /// Checks one step (not a block) on `line` and applies it to `state`.
    fn step<'s, D: Directive>(&mut self, step: &'s Step<D>, line: u32, state: &mut State<'s, D>)
    where
        R: SceneRules<D>,
    {
        if let Some(text) = step.text() {
            self.tokens(line, text);
            self.literal_names(line, text);
            let len = self.len(text);
            let limit = self.limits.max_text;
            if len > limit {
                self.err(
                    line,
                    format!("text is {len} characters; the limit is {limit}"),
                );
            } else if self.pages(text) > self.limits.box_lines {
                let lines = self.pages(text);
                let box_lines = self.limits.box_lines;
                self.err(
                    line,
                    format!(
                        "text takes {lines} lines of {} characters; the box shows {box_lines}",
                        self.limits.box_width
                    ),
                );
            }
        }
        match step {
            Step::Caption { text } => {
                self.tokens(line, text);
                self.literal_names(line, text);
                state.caption = Some(text);
            }
            Step::Place {
                side,
                character,
                expression,
            } => {
                if !self.rules.can_speak(character) {
                    self.err(line, format!("unknown character \"{}\"", character.0));
                }
                if let Some(message) = self.rules.expression_problem(character, expression) {
                    self.err(line, message);
                }
                if state.screen[slot(side.other())] == Some(character) {
                    self.err(
                        line,
                        format!(
                            "\"{}\" is already on the {}; a character can't be on both sides",
                            character.0,
                            side.other().name()
                        ),
                    );
                }
                state.screen[slot(*side)] = Some(character);
            }
            Step::Clear { side } => state.screen[slot(*side)] = None,
            Step::Say {
                speaker,
                expression,
                text,
                ..
            } => {
                self.line_id(line, &speaker.0, text);
                if !self.rules.can_speak(speaker) {
                    self.err(line, format!("unknown character \"{}\"", speaker.0));
                } else if !state.screen.contains(&Some(speaker)) {
                    self.err(
                        line,
                        format!(
                            "\"{}\" speaks but is not on screen; place them with @left or @right first",
                            speaker.0
                        ),
                    );
                }
                let problem = expression
                    .as_deref()
                    .and_then(|e| self.rules.expression_problem(speaker, e));
                if let Some(message) = problem {
                    self.err(line, message);
                }
                let len = self.len(text);
                if let Some(message) = self.rules.speech_problem(speaker, len) {
                    self.err(line, message);
                }
            }
            Step::Music(MusicLine::Cue(cue)) => {
                if let Some(message) = self.rules.music_problem(cue) {
                    self.err(line, message);
                }
            }
            Step::Narrate { text, .. } => self.line_id(line, NARRATION_SPEAKER, text),
            Step::Directive(directive) => {
                if let Some(kind) = directive.kind() {
                    state.in_force.insert(kind, directive);
                }
            }
            Step::Choice { .. } | Step::If { .. } | Step::Music(MusicLine::Stop) => {}
        }
    }

    /// Checks a `@choice` block on `line` and applies it to `state`: every
    /// option must leave the screen as the first one does.
    fn choice<'s, D: Directive>(
        &mut self,
        line: u32,
        options: &'s [ChoiceOption<D>],
        parts: &[PartLines],
        state: &mut State<'s, D>,
    ) where
        R: SceneRules<D>,
    {
        let DialogueLimits {
            min_options: min,
            max_options: max,
            max_option,
            max_reaction_texts,
            ..
        } = self.limits;
        let n = options.len();
        if !(min..=max).contains(&n) {
            let needs = match max - min {
                0 => format!("{min}"),
                1 => format!("{min} or {max}"),
                _ => format!("{min} to {max}"),
            };
            self.err(line, format!("@choice has {n} options; it needs {needs}"));
        }
        let mut first: Option<State<'s, D>> = None;
        for (option, part) in options.iter().zip(parts) {
            let at = part.line;
            self.line_id(at, REPLY_SPEAKER, &option.text);
            self.tokens(at, &option.text);
            self.literal_names(at, &option.text);
            let len = self.len(&option.text);
            if len > max_option {
                self.err(
                    at,
                    format!(
                        "option text is {len} characters; the limit is {max_option}, so it fits the menu"
                    ),
                );
            }
            let texts = texts(&option.steps);
            if texts > max_reaction_texts {
                self.err(
                    at,
                    format!(
                        "reaction has {texts} speech or narration lines; the limit is \
                         {max_reaction_texts}, so the scene rejoins quickly"
                    ),
                );
            }
            let mut after = state.clone();
            self.steps(&option.steps, &part.lines, &mut after);
            let Some(f) = &first else {
                first = Some(after);
                continue;
            };
            if f.screen != after.screen {
                self.err(
                    at,
                    format!(
                        "after this reaction {}; after the first option's {}; every option \
                         must leave the same characters on screen",
                        describe(after.screen),
                        describe(f.screen)
                    ),
                );
            } else if f.caption != after.caption {
                self.err(
                    at,
                    "this reaction leaves a different caption from the first option's; every \
                     option must leave the same caption"
                        .to_owned(),
                );
            } else if let Some(kind) = f.other_directive(&after) {
                self.err(
                    at,
                    format!(
                        "this reaction leaves a different @{kind} from the first option's; \
                         every option must leave the same {kind}"
                    ),
                );
            }
        }
        if let Some(f) = first {
            *state = f;
        }
    }
}

/// Who is on `screen`, in words: `scout is on the left and nobody on the
/// right`.
fn describe(screen: Screen) -> String {
    fn name(c: Option<&CharacterId>) -> &str {
        c.map_or("nobody", |c| c.0.as_str())
    }
    format!(
        "{} is on the left and {} on the right",
        name(screen[0]),
        name(screen[1])
    )
}

/// One error for each scene whose id an earlier scene (in any file)
/// already has.
pub fn check_duplicates<D>(scenes: &[ParsedScene<D>]) -> Vec<ContentError> {
    let mut first: BTreeMap<&str, &ParsedScene<D>> = BTreeMap::new();
    let mut errors = Vec::new();
    for s in scenes {
        if let Some(f) = first.get(s.scene.id.as_str()) {
            errors.push(
                ContentError::new(
                    &s.file,
                    format!(
                        "duplicate scene id \"{}\"; first used at {}:{}",
                        s.scene.id, f.file, f.line
                    ),
                )
                .at(s.line, None),
            );
        } else {
            first.insert(&s.scene.id, s);
        }
    }
    errors
}
