//! [`Playdate`]: the [`Platform`] on the SDK ([`Sdk`]).

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use shuyi_assets::keymap::{ActionSet, Button};
use shuyi_assets::target::TargetProfile;
use shuyi_platform::{Audio, DisplayInfo, Frame, Platform, Silent, Storage, SystemEvent, TextInput};
use shuyi_ui::frame_cap::{FrameCap, Pacing};
use shuyi_ui::input::pad::PadKind;
use shuyi_ui::input::state::Device;
use shuyi_ui::perf::{MemoryReading, PerfClock};
use shuyi_ui::{Game, GameData, PlatformFacts, RawInputEvent};

use crate::display::Screen;
use crate::input::PadInput;
use crate::sdk::{PACK, Sdk, SdkEvent};
use crate::storage::SdkStorage;

/// The target's profile, embedded: the numbers here are its, never typed
/// twice.
const PROFILE: &str = include_str!("../../../targets/playdate.ron");

/// The text key of a system menu item's title: this, then the name of the
/// action on it (`system_menu.Map`). Without it, the action's name.
pub const MENU_TITLE: &str = "system_menu.";

/// The profile of the target, read once.
fn profile() -> TargetProfile {
    TargetProfile::from_source("targets/playdate.ron", "playdate", PROFILE)
        .unwrap_or_else(|errors| panic!("targets/playdate.ron: {errors:?}"))
}

/// The game's baked pack in the `.pdx` of `sdk`, read whole into the heap and
/// kept for as long as the game runs.
pub fn read_pack(sdk: &impl Sdk) -> Result<&'static [u8], String> {
    let bytes = sdk.read_bundled(PACK)?;
    Ok(Vec::leak(bytes))
}

/// The Playdate's [`Platform`].
#[derive(Debug)]
pub struct Playdate<S: Sdk> {
    sdk: S,
    facts: PlatformFacts,
    /// The rate the update callback runs at: the profile's.
    rate: u16,
    /// The most items the system menu holds: the profile's.
    menu_slots: u8,
    pad: PadInput,
    /// Whether the crank was docked when the game was last told.
    docked: Option<bool>,
    /// The slots of the items chosen since the last poll.
    chosen: Vec<u8>,
    screen: Screen,
    audio: Silent,
    /// The system's events since the last call.
    events: Vec<SystemEvent>,
    /// How many of the system's holds are on: the lock and the menu may
    /// overlap, and the game runs again when both are off.
    holds: u8,
    /// The frame cap's serial on the last frame presented.
    serial: u32,
}

impl<S: Sdk> Playdate<S> {
    /// The backend on `sdk`: it sets the display's rate from the profile.
    pub fn new(sdk: S) -> Self {
        let profile = profile();
        let rate = profile.frame_rate.target;
        sdk.set_refresh_rate(rate);
        Self {
            sdk,
            facts: PlatformFacts::from_profile(&profile)
                .with_device(Device::Pad(PadKind::Playdate)),
            rate,
            menu_slots: profile.input.system_menu_slots,
            pad: PadInput::new(shuyi_assets::keymap::CrankDef::default()),
            docked: None,
            chosen: Vec::new(),
            screen: Screen::default(),
            audio: Silent,
            events: Vec::new(),
            holds: 0,
            serial: 0,
        }
    }

    /// The SDK.
    pub fn sdk(&self) -> &S {
        &self.sdk
    }

    /// The game has loaded: reads the crank's notch from its keymap, adds the
    /// system menu's items its optional actions are bound to, and lets it
    /// hear of the crank from the next frame.
    pub fn start<G: GameData>(&mut self, game: &Game<G>) {
        let data = &game.ctx().game;
        self.pad.set_crank(data.keymap_def().crank);
        self.docked = None;
        for slot in 0..self.menu_slots {
            let bound = Button::system_menu(slot)
                .and_then(|button| game.input().keymap().pad_action(button));
            let Some(action) = bound else {
                continue;
            };
            let name = action.name();
            let key = alloc::format!("{MENU_TITLE}{name}");
            let title = if data.lang().has(&key) {
                game.ctx().text(&key)
            } else {
                name
            };
            if !self.sdk.add_menu_item(title, slot) {
                // check-text: not player text (for the log)
                self.sdk.log("the system menu refused an item");
            }
        }
    }

    /// The player chose the system menu's item in `slot`: a press of its
    /// button at the next poll, so it goes where input goes.
    pub fn menu_chosen(&mut self, slot: u8) {
        if slot < self.menu_slots {
            self.chosen.push(slot);
        }
    }

    /// The SDK's event, as the [`SystemEvent`]s the next frame reads. True
    /// when it should be heard now: no update comes until the system gives
    /// the game back, so what it saves must be saved before.
    pub fn event(&mut self, event: SdkEvent) -> bool {
        match event {
            SdkEvent::Lock | SdkEvent::Pause => {
                self.holds = self.holds.saturating_add(1);
                if self.holds > 1 {
                    return false;
                }
                self.events.push(SystemEvent::Suspend);
                true
            }
            SdkEvent::Unlock | SdkEvent::Resume => {
                self.holds = self.holds.saturating_sub(1);
                if self.holds == 0 {
                    self.events.push(SystemEvent::Resume);
                    self.screen.invalidate();
                }
                false
            }
            SdkEvent::Terminate => {
                self.events.push(SystemEvent::QuitRequested);
                true
            }
            SdkEvent::LowPower => {
                self.events.push(SystemEvent::LowBattery);
                true
            }
        }
    }

    /// Whether anything reached the screen since the last call: the update
    /// callback returns it.
    pub fn take_changed(&mut self) -> bool {
        self.screen.take_changed()
    }

    /// What it applied, for `App::set_pacing`: the device runs at the
    /// profile's rate whatever a game asks, so a request is confirmed.
    pub fn pacing(&self, fps: u16) -> Pacing {
        let mut pacing = Pacing::new(FrameCap::Fps(self.rate), self.serial);
        pacing.fps = fps;
        pacing
    }
}

impl<S: Sdk> Platform for Playdate<S> {
    fn display(&self) -> DisplayInfo {
        DisplayInfo::new(400, 240).one_bit()
    }

    fn present(&mut self, frame: &Frame<'_>) {
        self.serial = frame.frame_cap_serial;
        self.screen.present(&self.sdk, frame);
    }

    fn poll_input(&mut self, out: &mut Vec<RawInputEvent>) {
        let (current, pushed) = self.sdk.buttons();
        let turned = self.sdk.crank_change();
        let docked = self.sdk.crank_docked();
        out.extend(self.pad.poll(current, pushed, turned, docked));
        for slot in self.chosen.drain(..) {
            if let Some(button) = Button::system_menu(slot) {
                out.push(RawInputEvent::PadDown(button, PadKind::Playdate));
                out.push(RawInputEvent::PadUp(button));
            }
        }
    }

    fn system_events(&mut self, out: &mut Vec<SystemEvent>) {
        out.append(&mut self.events);
        let docked = self.sdk.crank_docked();
        if self.docked != Some(docked) {
            self.docked = Some(docked);
            out.push(if docked {
                SystemEvent::CrankDocked
            } else {
                SystemEvent::CrankUndocked
            });
        }
    }

    fn storage(&mut self) -> Box<dyn Storage> {
        Box::new(SdkStorage::new(self.sdk.clone()))
    }

    fn audio(&mut self) -> &mut dyn Audio {
        &mut self.audio
    }

    fn text_input(&mut self) -> Option<&mut dyn TextInput> {
        None
    }

    fn facts(&self) -> PlatformFacts {
        self.facts.clone()
    }

    fn clock(&self) -> Option<PerfClock> {
        cfg!(debug_assertions).then_some(PerfClock::Real(S::clock_us))
    }

    fn memory(&mut self) -> Option<MemoryReading> {
        S::heap().map(MemoryReading::new)
    }
}
