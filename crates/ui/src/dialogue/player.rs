//! Playing a dialogue [`Scene`] one text box at a time. [`DialoguePlayer`]
//! is the state machine only (who stands where, what is said, which reply
//! choice is open, which music the script asked for, and what the game's
//! own lines have set: its [`DialogueHooks`]); a dialogue screen draws its
//! [`View`] and asks for the music.

use crate::prelude::*;
use alloc::borrow::Cow;
use alloc::sync::Arc;
use core::fmt::Debug;

use shuyi_assets::dialogue::{
    CharacterId, ChoiceOption, Directive, LineId, MusicLine, Present, Scene, Side, Step,
};

/// What a game adds to playing a scene: its own lines
/// ([`Directive`]s, such as a place behind the talk) and what they have set
/// so far (its [`Stage`](Self::Stage)), and the tokens it fills into the
/// text (the hero's name, say).
pub trait DialogueHooks: Clone + Debug + PartialEq + Eq {
    /// The game's own lines.
    type Directive: Directive;

    /// What the game's lines have set so far: nothing until the first.
    type Stage: Clone + Debug + Default + PartialEq + Eq;

    /// `text` as shown: the game's tokens filled in. The scene keeps them.
    fn fill<'a>(&self, text: &'a str) -> Cow<'a, str>;

    /// Plays the game's line `directive` on `stage`, when the scene reaches
    /// it (read or skipped).
    fn direct(&self, stage: &mut Self::Stage, directive: &Self::Directive);
}

/// The hooks of a game that adds nothing: no lines of its own, no tokens.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Plain;

impl DialogueHooks for Plain {
    type Directive = shuyi_assets::dialogue::NoDirective;
    type Stage = ();

    fn fill<'a>(&self, text: &'a str) -> Cow<'a, str> {
        Cow::Borrowed(text)
    }

    fn direct(&self, (): &mut (), directive: &Self::Directive) {
        match *directive {}
    }
}

/// A character standing on one side of the screen.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Placed {
    character: CharacterId,
    expression: String,
}

impl Placed {
    fn portrait(&self) -> Portrait<'_> {
        Portrait {
            character: &self.character,
            expression: &self.expression,
        }
    }
}

/// A portrait on screen: who, with which expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Portrait<'a> {
    /// The character.
    pub character: &'a CharacterId,
    /// Their current expression.
    pub expression: &'a str,
}

/// What the screen shows for the current text box, with the game's tokens
/// filled in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View<'a, S> {
    /// The left portrait, if any.
    pub left: Option<Portrait<'a>>,
    /// The right portrait, if any.
    pub right: Option<Portrait<'a>>,
    /// Which side is speaking (`None` for narration, or when finished).
    pub speaker: Option<Side>,
    /// The text box's text (`None` once the scene is finished). While a
    /// choice is open, the text box before it (the line being answered).
    pub text: Option<Cow<'a, str>>,
    /// The location/time caption, if one has been shown.
    pub caption: Option<Cow<'a, str>>,
    /// What the game's own lines have set so far.
    pub stage: &'a S,
    /// Whether the text is narration (no speaker).
    pub narration: bool,
    /// The lead's replies, in menu order, while a choice is open: pick
    /// one with [`DialoguePlayer::choose`].
    pub choices: Option<Vec<Cow<'a, str>>>,
}

/// Where a step is: in the scene, or in a reply's reaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct At {
    /// Index in the scene's steps (for a reaction: of its `Choice`).
    step: usize,
    /// For a reaction: the option and the step's index in it.
    reaction: Option<(usize, usize)>,
}

/// What is on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Showing {
    /// A text box: the speech or narration step at `At`.
    Text(At),
    /// The choice at this scene step, answering the text box `At` (if any).
    Choice(usize, Option<At>),
    /// The scene is over.
    Finished,
}

/// A reply being played: its reaction. When the reaction ends the scene
/// goes on after the choice, with the portraits as the reaction left them
/// (the script sets any expression change for the rejoin).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Reaction {
    /// The scene step of the `Choice`.
    choice: usize,
    /// The option picked.
    option: usize,
    /// The next reaction step.
    next: usize,
}

/// Plays a scene: each [`advance`](Self::advance) moves to the next text
/// box (speech or narration), applying the portrait and caption steps in
/// between. At a reply choice it waits for [`choose`](Self::choose), plays
/// that reply's reaction, then rejoins the scene after the choice. The
/// `@music` lines passed on the way are kept for [`take_music`](Self::take_music).
/// It plays the scene as it is for those present when it starts: the
/// lines of an `@if` block whose character isn't there are no part of it,
/// read or skipped. The game's own lines play on its stage as the scene
/// reaches them ([`DialogueHooks::direct`]).
/// Pure: no drawing, no clock, no sound. Cheap to clone (the scene is
/// shared), so a copy can be kept as the scene was at a line (a dialogue
/// log).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialoguePlayer<H: DialogueHooks> {
    /// The scene for those present: it has no `@if` blocks.
    scene: Arc<Scene<H::Directive>>,
    /// Fills in the game's tokens and plays its lines.
    hooks: H,
    /// Index of the next scene step (after the text box on screen, or the
    /// open choice's).
    next: usize,
    /// The reply being played, if any.
    reaction: Option<Reaction>,
    showing: Showing,
    /// The last text box shown, for an open choice.
    last_text: Option<At>,
    left: Option<Placed>,
    right: Option<Placed>,
    caption: Option<String>,
    /// The last `@music` line reached and not yet taken.
    music: Option<MusicLine>,
    /// What the game's lines have set so far.
    stage: H::Stage,
}

impl<H: DialogueHooks> DialoguePlayer<H> {
    /// Starts `scene` as it plays for those `present`, at its first text
    /// box, with `hooks` filling its text and playing the game's lines. A
    /// scene with nothing to say for them is finished at once.
    pub fn new(scene: &Scene<H::Directive>, hooks: H, present: &Present) -> Self {
        let mut player = Self {
            scene: Arc::new(scene.resolved(present)),
            hooks,
            next: 0,
            reaction: None,
            showing: Showing::Finished,
            last_text: None,
            left: None,
            right: None,
            caption: None,
            music: None,
            stage: H::Stage::default(),
        };
        player.step_on();
        player
    }

    /// What fills the text and plays the game's lines.
    pub fn hooks(&self) -> &H {
        &self.hooks
    }

    /// What the game's lines have set so far.
    pub fn stage(&self) -> &H::Stage {
        &self.stage
    }

    /// What the game's lines have set so far, for the game to take what it
    /// has played from it.
    pub fn stage_mut(&mut self) -> &mut H::Stage {
        &mut self.stage
    }

    /// `text` as shown: the game's tokens filled in. The scene keeps them.
    fn fill<'a>(&self, text: &'a str) -> Cow<'a, str> {
        self.hooks.fill(text)
    }

    /// The id of the scene being played.
    pub fn scene_id(&self) -> &str {
        &self.scene.id
    }

    /// The scene being played: the one it was started with, as it is for
    /// those present.
    pub fn scene(&self) -> &Scene<H::Directive> {
        &self.scene
    }

    /// The step at `at`.
    fn step(&self, at: At) -> Option<&Step<H::Directive>> {
        let step = self.scene.steps.get(at.step)?;
        match (step, at.reaction) {
            (_, None) => Some(step),
            (Step::Choice { options }, Some((option, i))) => options.get(option)?.steps.get(i),
            _ => None,
        }
    }

    /// The options of the choice at scene step `index`.
    fn options(&self, index: usize) -> &[ChoiceOption<H::Directive>] {
        match self.scene.steps.get(index) {
            Some(Step::Choice { options }) => options,
            _ => &[],
        }
    }

    /// What to show now.
    pub fn current(&self) -> View<'_, H::Stage> {
        let (text_at, choices) = match self.showing {
            Showing::Text(at) => (Some(at), None),
            Showing::Choice(index, before) => {
                let options = self.options(index);
                let texts = options.iter().map(|o| self.fill(&o.text));
                (before, Some(texts.collect()))
            }
            Showing::Finished => (None, None),
        };
        let step = text_at.and_then(|at| self.step(at));
        let speaker = match step {
            Some(Step::Say { speaker, .. }) => self.side_of(speaker),
            _ => None,
        };
        View {
            left: self.left.as_ref().map(Placed::portrait),
            right: self.right.as_ref().map(Placed::portrait),
            speaker,
            text: step.and_then(Step::text).map(|t| self.fill(t)),
            caption: self.caption.as_deref().map(|c| self.fill(c)),
            stage: &self.stage,
            narration: matches!(step, Some(Step::Narrate { .. })),
            choices,
        }
    }

    /// The id of the line in the text box (while a choice is open, of the
    /// line being answered); `None` once the scene is finished.
    pub fn line(&self) -> Option<&LineId> {
        let at = match self.showing {
            Showing::Text(at) => Some(at),
            Showing::Choice(_, before) => before,
            Showing::Finished => None,
        };
        match at.and_then(|at| self.step(at)) {
            Some(Step::Say { line, .. } | Step::Narrate { line, .. }) => Some(line),
            _ => None,
        }
    }

    /// The id of reply `option` of the open choice, if a choice is open and
    /// has it.
    pub fn reply_line(&self, option: usize) -> Option<&LineId> {
        let Showing::Choice(index, _) = self.showing else {
            return None;
        };
        self.options(index).get(option).map(|o| &o.line)
    }

    /// The music the scene has asked for and [`take_music`](Self::take_music)
    /// hasn't taken yet, without taking it.
    pub fn pending_music(&self) -> Option<&MusicLine> {
        self.music.as_ref()
    }

    /// Moves to the next text box, applying every portrait, caption and
    /// place step before it. After the last one the scene is finished; advancing a
    /// finished scene does nothing. While a choice is open this does
    /// nothing either: [`choose`](Self::choose) a reply.
    pub fn advance(&mut self) {
        if matches!(self.showing, Showing::Text(_)) {
            self.step_on();
        }
    }

    /// Picks reply `option` of the open choice: plays its reaction, then
    /// the scene after the choice. Does nothing if no choice is open or
    /// there is no such option.
    pub fn choose(&mut self, option: usize) {
        let Showing::Choice(index, _) = self.showing else {
            return;
        };
        if option >= self.options(index).len() {
            return;
        }
        self.reaction = Some(Reaction {
            choice: index,
            option,
            next: 0,
        });
        self.step_on();
    }

    /// Advances until a choice is open or the scene is over (skipping a
    /// scene stops at each choice).
    pub fn skip_to_choice(&mut self) {
        while matches!(self.showing, Showing::Text(_)) {
            let before = (self.next, self.reaction);
            self.step_on();
            // Each text box moves the position on; never spin in place.
            if (self.next, self.reaction) == before {
                break;
            }
        }
    }

    /// The music the scene has asked for since the last call: the last
    /// `@music` line reached, whether the lines before it were read or
    /// skipped. `None` if no `@music` line was reached since.
    pub fn take_music(&mut self) -> Option<MusicLine> {
        self.music.take()
    }

    /// Whether a reply choice is waiting for [`choose`](Self::choose).
    pub fn is_choosing(&self) -> bool {
        matches!(self.showing, Showing::Choice(..))
    }

    /// Whether every text box has been shown and advanced past.
    pub fn is_finished(&self) -> bool {
        self.showing == Showing::Finished
    }

    /// Applies steps up to the next text box or choice.
    fn step_on(&mut self) {
        if let Some(r) = self.reaction {
            let steps = self
                .options(r.choice)
                .get(r.option)
                .map_or(&[][..], |o| &o.steps[..]);
            let rest: Vec<Step<H::Directive>> = steps.iter().skip(r.next).cloned().collect();
            for (i, step) in (r.next..).zip(rest) {
                let at = At {
                    step: r.choice,
                    reaction: Some((r.option, i)),
                };
                if self.apply(&step, at) {
                    self.reaction = Some(Reaction { next: i + 1, ..r });
                    return;
                }
            }
            // The reaction is over: on with the scene.
            self.reaction = None;
        }
        let rest: Vec<(usize, Step<H::Directive>)> = self
            .scene
            .steps
            .iter()
            .cloned()
            .enumerate()
            .skip(self.next)
            .collect();
        for (index, step) in rest {
            if let Step::Choice { options } = &step {
                // A choice without replies (which the validator rejects)
                // is passed over rather than waiting forever.
                if !options.is_empty() {
                    self.showing = Showing::Choice(index, self.last_text);
                    self.next = index + 1;
                    return;
                }
                continue;
            }
            let at = At {
                step: index,
                reaction: None,
            };
            if self.apply(&step, at) {
                self.next = index + 1;
                return;
            }
        }
        self.showing = Showing::Finished;
    }

    /// Applies `step` (found at `at`). Returns whether it is a text box,
    /// which is now shown.
    fn apply(&mut self, step: &Step<H::Directive>, at: At) -> bool {
        match step {
            Step::Caption { text } => self.caption = Some(text.clone()),
            Step::Place {
                side,
                character,
                expression,
            } => {
                let placed = Some(Placed {
                    character: character.clone(),
                    expression: expression.clone(),
                });
                match side {
                    Side::Left => self.left = placed,
                    Side::Right => self.right = placed,
                }
            }
            Step::Clear { side: Side::Left } => self.left = None,
            Step::Clear { side: Side::Right } => self.right = None,
            Step::Say {
                speaker,
                expression,
                ..
            } => {
                if let Some(expression) = expression {
                    for p in [&mut self.left, &mut self.right].into_iter().flatten() {
                        if &p.character == speaker {
                            p.expression.clone_from(expression);
                        }
                    }
                }
            }
            Step::Music(music) => self.music = Some(music.clone()),
            Step::Directive(directive) => self.hooks.direct(&mut self.stage, directive),
            // The scene was resolved for those present: it has no blocks.
            Step::Narrate { .. } | Step::Choice { .. } | Step::If { .. } => {}
        }
        if step.text().is_some() {
            self.showing = Showing::Text(at);
            self.last_text = Some(at);
            return true;
        }
        false
    }

    /// The side `character` stands on.
    fn side_of(&self, character: &CharacterId) -> Option<Side> {
        let on = |p: &Option<Placed>| p.as_ref().is_some_and(|p| &p.character == character);
        if on(&self.left) {
            Some(Side::Left)
        } else if on(&self.right) {
            Some(Side::Right)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests;
