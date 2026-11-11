//! Files the player keeps, on the Harness: a screen offers a file and asks
//! for one, the headless backend keeps the offers and answers the asks from
//! a queue, and the game's system hook hears each answer in the next frame.

use shuyi_platform::SystemEvent;
use shuyi_platform::harness::{FRAME_DT, Harness};
use shuyi_platform::player_files::{ChosenFile, MAX_CHOSEN_BYTES, Offered};
use shuyi_ui::frame::glyph_buffer::GlyphBuffer;
use shuyi_ui::log::{LogCategory, LogLevel};
use shuyi_ui::player_files::Folder;
use shuyi_ui::screen::{FrameInput, Screen, Transition};
use shuyi_ui::toy::{self, Action, Toy, ToyCtx};
use shuyi_ui::{Game, StoragePlace};

/// What a [`Files`] screen does on its next update.
#[derive(Debug, Clone, Default)]
pub enum Do {
    /// Nothing.
    #[default]
    Nothing,
    /// Offers `a.ron` holding `x`.
    Offer(&'static str),
    /// Asks for a `.ron` file.
    Ask,
    /// Opens the exports folder.
    Open,
}

/// A screen that does what it is told once, and keeps what the system's
/// hook heard of the player's files. Confirm asks for a file too, and
/// Cancel offers `farm.ron`: a recorded run is driven by keys, since
/// reaching into the game between frames stops its record.
#[derive(Debug, Default)]
pub struct Files {
    pub next: Do,
    pub heard: Vec<SystemEvent>,
}

impl Screen<Toy> for Files {
    fn name(&self) -> &'static str {
        "files"
    }

    fn actions(&self) -> Option<Vec<Action>> {
        Some(vec![Action::Confirm, Action::Cancel])
    }

    fn update(&mut self, ctx: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        for action in &input.actions {
            match action {
                Action::Confirm => self.next = Do::Ask,
                Action::Cancel => self.next = Do::Offer("farm.ron"),
                _ => {}
            }
        }
        match core::mem::take(&mut self.next) {
            Do::Nothing => {}
            Do::Offer(name) => ctx.offer_file(name, "x"),
            Do::Ask => ctx.ask_for_file(&[".ron"]),
            Do::Open => ctx.open_storage_folder(Folder::Exports),
        }
        Transition::None
    }

    fn draw(&self, _: &ToyCtx, _: &mut GlyphBuffer) {}

    fn as_any(&self) -> Option<&dyn core::any::Any> {
        Some(self)
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn core::any::Any> {
        Some(self)
    }
}

/// The game's system hook: hands the screen what it heard of the files.
pub fn hear(game: &mut Game<Toy>, event: SystemEvent) {
    if matches!(
        event,
        SystemEvent::FileOffered(_) | SystemEvent::FileChosen(_)
    ) && let Some(files) = game.screen_mut::<Files>()
    {
        files.heard.push(event);
    }
}

fn harness() -> Harness<Toy> {
    let game = Game::new(toy::ctx(), Box::new(Files::default()), toy::hooks());
    Harness::new(game).with_system_hook(hear)
}

/// Tells the screen to `act` in the next frame, and runs it and one more.
fn act(h: &mut Harness<Toy>, act: Do) -> &[SystemEvent] {
    if let Some(files) = h.screen_mut::<Files>() {
        files.next = act;
    }
    h.wait(FRAME_DT).wait(FRAME_DT);
    h.screen::<Files>()
        .map_or(&[], |files| files.heard.as_slice())
}

fn heard(h: &Harness<Toy>) -> Vec<SystemEvent> {
    h.screen::<Files>()
        .map(|files| files.heard.clone())
        .unwrap_or_default()
}

#[test]
fn an_offered_file_is_kept_and_its_place_heard_in_the_next_frame() {
    let mut h = harness();
    // The screen offers in its update; the answer is the next frame's.
    if let Some(files) = h.screen_mut::<Files>() {
        files.next = Do::Offer("a.ron");
    }
    h.wait(FRAME_DT);
    assert_eq!(h.offered(), [("a.ron".to_owned(), "x".to_owned())]);
    assert_eq!(heard(&h), []);
    h.wait(FRAME_DT);
    let written = SystemEvent::FileOffered(Offered::Written("<memory>/exports/a.ron".into()));
    assert_eq!(heard(&h), core::slice::from_ref(&written));
    // A second offer of the name never writes over the first.
    act(&mut h, Do::Offer("a.ron"));
    let second = SystemEvent::FileOffered(Offered::Written("<memory>/exports/a (2).ron".into()));
    assert_eq!(heard(&h), [written, second]);
    assert_eq!(h.offered()[1], ("a (2).ron".to_owned(), "x".to_owned()));
}

#[test]
fn a_name_with_a_folder_in_it_fails_before_the_platform_sees_it() {
    let mut h = harness();
    let heard = act(&mut h, Do::Offer("../a.ron")).to_vec();
    assert!(
        matches!(&heard[..], [SystemEvent::FileOffered(Offered::Failed(why))] if why.contains("one name")),
        "{heard:?}"
    );
    assert_eq!(h.offered(), []);
}

#[test]
fn a_queued_answer_is_heard_after_the_ask() {
    let mut h = harness();
    h.answer_next_ask(ChosenFile::picked("farm.ron", "(hay: 3)"));
    let heard = act(&mut h, Do::Ask).to_vec();
    assert_eq!(
        heard,
        [SystemEvent::FileChosen(ChosenFile::picked(
            "farm.ron", "(hay: 3)"
        ))]
    );
    assert_eq!(h.platform().asks(), [vec![".ron".to_owned()]]);
}

#[test]
fn a_cancelled_ask_is_heard_and_so_is_one_with_no_answer_queued() {
    let mut h = harness();
    h.answer_next_ask(ChosenFile::Cancelled);
    act(&mut h, Do::Ask);
    act(&mut h, Do::Ask);
    let cancelled = SystemEvent::FileChosen(ChosenFile::Cancelled);
    assert_eq!(heard(&h), [cancelled.clone(), cancelled]);
}

#[test]
fn a_file_not_utf8_or_too_big_is_refused() {
    let mut h = harness();
    h.answer_next_ask(ChosenFile::from_bytes("a.png", vec![0x89, 0xff]));
    h.answer_next_ask(ChosenFile::from_bytes(
        "big.ron",
        vec![b' '; MAX_CHOSEN_BYTES + 1],
    ));
    act(&mut h, Do::Ask);
    act(&mut h, Do::Ask);
    let heard = heard(&h);
    let reasons: Vec<&str> = heard
        .iter()
        .filter_map(|event| match event {
            SystemEvent::FileChosen(ChosenFile::Refused(why)) => Some(why.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(reasons.len(), 2, "{heard:?}");
    assert!(reasons[0].contains("isn't UTF-8"), "{}", reasons[0]);
    assert!(reasons[1].contains("more than"), "{}", reasons[1]);
}

#[test]
fn a_folder_is_opened_and_the_storage_is_said_to_be_in_memory() {
    let mut h = harness();
    act(&mut h, Do::Open);
    assert_eq!(h.platform().opened_folders(), [Folder::Exports]);
    let facts = shuyi_platform::Platform::facts(h.platform());
    assert_eq!(
        facts.storage_place,
        StoragePlace::Folder(shuyi_platform::headless::MEMORY_FOLDER.into())
    );
    let warned = h
        .log()
        .lines()
        .any(|line| line.category == LogCategory::Platform && line.level == LogLevel::Warn);
    assert!(!warned);
}
