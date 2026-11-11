//! [`Bare`]: a backend that implements only the required methods (the
//! seven of ADR-0024 §2, and `facts`), as a private console's backend
//! may, and runs a frame of the toy game.
//!
//! A method added to `Platform` without a default fails to compile here:
//! give it a default instead (ADR-0040 §7), so no backend breaks. The one
//! exception is `facts`: a backend always says which target it is
//! (ADR-0060).

use shuyi_platform::platform::{NO_FILES, NO_PICKER};
use shuyi_platform::{App, Audio, DisplayInfo, Frame, FrameOutcome, Millis, Platform};
use shuyi_platform::{ChosenFile, Offered, Silent, SystemEvent, TextInput};
use shuyi_ui::player_files::Folder;
use shuyi_ui::storage::{MemoryStorage, Storage};
use shuyi_ui::toy;
use shuyi_ui::{PlatformFacts, RawInputEvent};

/// The fewest methods a backend can have: no sound, no system keyboard,
/// storage in memory.
#[derive(Debug, Default)]
struct Bare {
    audio: Silent,
    /// How many frames it was shown.
    presented: u32,
}

impl Platform for Bare {
    fn display(&self) -> DisplayInfo {
        DisplayInfo::new(400, 240)
    }

    fn present(&mut self, _: &Frame<'_>) {
        self.presented += 1;
    }

    fn poll_input(&mut self, _: &mut Vec<RawInputEvent>) {}

    fn system_events(&mut self, _: &mut Vec<SystemEvent>) {}

    fn storage(&mut self) -> Box<dyn Storage> {
        Box::new(MemoryStorage::new())
    }

    fn audio(&mut self) -> &mut dyn Audio {
        &mut self.audio
    }

    fn text_input(&mut self) -> Option<&mut dyn TextInput> {
        None
    }

    fn facts(&self) -> PlatformFacts {
        PlatformFacts::default()
    }
}

#[test]
fn a_backend_of_the_seven_methods_runs_a_frame() {
    let mut bare = Bare::default();
    let mut app = App::running(toy::game());
    assert_eq!(app.frame(&mut bare, Millis(0)), FrameOutcome::Continue);
    assert_eq!(bare.presented, 1);
    // The defaults: no clock, so nothing measured, and no memory read.
    assert!(bare.clock().is_none());
    assert_eq!(bare.memory(), None);
    assert_eq!(bare.heap_snapshot(), None);
    assert_eq!(app.heap_snapshot(&mut bare), None);
    assert_eq!(app.perf().records().len(), 0);
    // No folder watched in every build, so nothing asked of the files.
    assert!(!bare.watches_files());
}

#[test]
fn a_backend_without_files_refuses_them_and_app_says_why() {
    let mut bare = Bare::default();
    let mut app = App::running(toy::game());
    let ctx = app.game_mut().map(shuyi_ui::Game::ctx_mut);
    let ctx = ctx.unwrap_or_else(|| panic!("the toy runs"));
    ctx.offer_file("a.ron", "x");
    ctx.ask_for_file(&[]);
    ctx.open_storage_folder(Folder::Storage);
    let _ = app.frame(&mut bare, Millis(0));
    let warned: Vec<String> = app.log().lines().map(|line| line.text.clone()).collect();
    assert_eq!(
        warned,
        [format!("the Storage folder wasn't opened: {NO_FILES}")]
    );
    let heard = app.game().map(|game| game.ctx().platform.clone());
    assert!(heard.is_some_and(|facts| !facts.can_pick_files()));
    // The answers are the next frame's events: the hook hears them.
    let mut app = App::running(toy::game());
    app.set_system_hook(|game, event| {
        let _ = game.ctx_mut().storage.write("heard", &format!("{event:?}"));
    });
    if let Some(game) = app.game_mut() {
        game.ctx_mut().ask_for_file(&[]);
    }
    let _ = app.frame(&mut bare, Millis(0));
    let _ = app.frame(&mut bare, Millis(16));
    let heard = app
        .game()
        .and_then(|game| game.ctx().storage.read("heard").ok().flatten());
    let refused = SystemEvent::FileChosen(ChosenFile::Refused(NO_PICKER.into()));
    assert_eq!(heard, Some(format!("{refused:?}")));
    assert!(matches!(
        bare.offer_file("a.ron", "x"),
        Some(Offered::Failed(why)) if why == NO_FILES
    ));
}
