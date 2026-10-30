//! The device (`--cfg shuyi_playdate_sdk`): [`Sdk`] on the real SDK through
//! `shim/shim.c`, the allocator, the panic handler and what the game's
//! entry points call. The only `unsafe` in the crate.
#![allow(unsafe_code)]

mod alloc_sdk;
mod panic;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::ffi::{CStr, c_char, c_float, c_int, c_uint, c_void};

use shuyi_platform::Start;
use shuyi_ui::GameData;
use shuyi_ui::frame::onebit::Frame;
use shuyi_ui::perf::HeapCounts;

use crate::driver::Driver;
use crate::sdk::{Sdk, SdkEvent};
use crate::text::c_string;

// One wrapper per SDK call, in `shim/shim.c`.
unsafe extern "C" {
    fn pd_log(text: *const c_char);
    fn pd_millis() -> c_uint;
    fn pd_elapsed() -> c_float;
    fn pd_buttons(current: *mut c_uint, pushed: *mut c_uint);
    fn pd_crank_change() -> c_float;
    fn pd_crank_docked() -> c_int;
    fn pd_add_menu_item(title: *const c_char, slot: c_int) -> c_int;
    fn pd_set_refresh_rate(rate: c_float);
    fn pd_frame() -> *mut u8;
    fn pd_mark_updated_rows(start: c_int, end: c_int);
    fn pd_draw_text(text: *const c_char, len: usize, white: c_int);
    fn pd_file_open_data(name: *const c_char) -> *mut c_void;
    fn pd_file_open_bundled(name: *const c_char) -> *mut c_void;
    fn pd_file_open_write(name: *const c_char) -> *mut c_void;
    fn pd_file_read(file: *mut c_void, buf: *mut c_void, len: c_uint) -> c_int;
    fn pd_file_write(file: *mut c_void, buf: *const c_void, len: c_uint) -> c_int;
    fn pd_file_flush(file: *mut c_void) -> c_int;
    fn pd_file_close(file: *mut c_void) -> c_int;
    fn pd_file_error() -> *const c_char;
    fn pd_file_is_file(name: *const c_char) -> c_int;
    fn pd_file_list(
        folder: *const c_char,
        each: extern "C" fn(name: *const c_char, userdata: *mut c_void),
        userdata: *mut c_void,
    ) -> c_int;
    fn pd_file_mkdir(path: *const c_char) -> c_int;
    fn pd_file_unlink(name: *const c_char) -> c_int;
    fn pd_file_rename(from: *const c_char, to: *const c_char) -> c_int;
}

/// How many bytes a read asks the SDK for at a time.
const CHUNK: usize = 4096;

/// The SDK's message for the last file call that failed.
fn last_error() -> String {
    let error = unsafe { pd_file_error() };
    if error.is_null() {
        // check-text: not player text (for the log)
        return String::from("unknown file error");
    }
    // SAFETY: the SDK hands back a C string it keeps.
    String::from(unsafe { CStr::from_ptr(error) }.to_string_lossy())
}

/// `Ok` if the SDK call returned 0, else its message.
fn ok(result: c_int) -> Result<(), String> {
    if result == 0 {
        Ok(())
    } else {
        Err(last_error())
    }
}

/// Reads all of the open `file`, and closes it.
fn read_all(file: *mut c_void) -> Result<Vec<u8>, String> {
    if file.is_null() {
        return Err(last_error());
    }
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; CHUNK];
    let result = loop {
        let len = c_uint::try_from(chunk.len()).unwrap_or(c_uint::MAX);
        let read = unsafe { pd_file_read(file, chunk.as_mut_ptr().cast(), len) };
        match usize::try_from(read) {
            Ok(0) => break Ok(()),
            Ok(read) => bytes.extend_from_slice(&chunk[..read.min(CHUNK)]),
            Err(_) => break Err(last_error()),
        }
    };
    unsafe { pd_file_close(file) };
    result.map(|()| bytes)
}

/// Adds the name the SDK lists to the `Vec<String>` behind `names`.
extern "C" fn listed(name: *const c_char, names: *mut c_void) {
    // SAFETY: `data_names` passes its own `Vec<String>` and the SDK a C
    // string, both alive for the call.
    let names = unsafe { &mut *names.cast::<Vec<String>>() };
    let name = unsafe { CStr::from_ptr(name) };
    names.push(String::from(name.to_string_lossy()));
}

/// The SDK itself: a value of no size, since the shim holds its handle.
#[derive(Debug, Clone, Copy, Default)]
pub struct Device;

/// Microseconds in a second, for the timer that gives seconds.
const MICROS: f32 = 1_000_000.0;

/// The highest time [`Device::clock_us`] has given.
struct Highest(UnsafeCell<u64>);

// SAFETY: the Playdate runs the game on one thread (see `Game`).
unsafe impl Sync for Highest {}

static CLOCK_US: Highest = Highest(UnsafeCell::new(0));

impl Sdk for Device {
    fn millis(&self) -> u32 {
        unsafe { pd_millis() }
    }

    /// `getElapsedTime`, seconds as a float since the game started (nothing
    /// resets it): whole microseconds, held from going back by the float's
    /// rounding.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn clock_us() -> u64 {
        let now = (unsafe { pd_elapsed() } * MICROS) as u64;
        // SAFETY: one thread (see `Game`), and nothing here calls back.
        let highest = unsafe { &mut *CLOCK_US.0.get() };
        *highest = (*highest).max(now);
        *highest
    }

    fn heap() -> Option<HeapCounts> {
        alloc_sdk::heap()
    }

    fn buttons(&self) -> (u32, u32) {
        let (mut current, mut pushed) = (0, 0);
        unsafe { pd_buttons(&raw mut current, &raw mut pushed) };
        (current, pushed)
    }

    fn crank_change(&self) -> f32 {
        unsafe { pd_crank_change() }
    }

    fn crank_docked(&self) -> bool {
        unsafe { pd_crank_docked() != 0 }
    }

    fn add_menu_item(&self, title: &str, slot: u8) -> bool {
        // The SDK copies the title.
        let title = c_string(title);
        unsafe { pd_add_menu_item(title.as_ptr().cast(), c_int::from(slot)) != 0 }
    }

    fn set_refresh_rate(&self, fps: u16) {
        unsafe { pd_set_refresh_rate(f32::from(fps)) };
    }

    fn with_frame(&self, draw: impl FnOnce(&mut Frame)) {
        // SAFETY: the SDK's frame buffer is `LCD_ROWSIZE` × `LCD_ROWS` bytes,
        // which `shim.c` checks is a `Frame`, and nothing else holds it
        // during the update callback.
        draw(unsafe { &mut *pd_frame().cast::<Frame>() });
    }

    fn mark_rows(&self, first: i32, last: i32) {
        unsafe { pd_mark_updated_rows(first, last) };
    }

    fn draw_text(&self, text: &str, white: bool) {
        unsafe { pd_draw_text(text.as_ptr().cast(), text.len(), c_int::from(white)) };
    }

    fn log(&self, line: &str) {
        let line = c_string(line);
        unsafe { pd_log(line.as_ptr().cast()) };
    }

    fn is_file(&self, path: &str) -> bool {
        let path = c_string(path);
        unsafe { pd_file_is_file(path.as_ptr().cast()) != 0 }
    }

    fn read_data(&self, path: &str) -> Result<Vec<u8>, String> {
        let name = c_string(path);
        read_all(unsafe { pd_file_open_data(name.as_ptr().cast()) })
    }

    fn read_bundled(&self, path: &str) -> Result<Vec<u8>, String> {
        let name = c_string(path);
        read_all(unsafe { pd_file_open_bundled(name.as_ptr().cast()) })
    }

    fn write_data(&self, path: &str, bytes: &[u8]) -> Result<(), String> {
        let name = c_string(path);
        let file = unsafe { pd_file_open_write(name.as_ptr().cast()) };
        if file.is_null() {
            return Err(last_error());
        }
        // check-text: not player text (for the log)
        let len = c_uint::try_from(bytes.len()).map_err(|_| String::from("the file is too big"))?;
        let written = unsafe { pd_file_write(file, bytes.as_ptr().cast(), len) };
        let all = usize::try_from(written).ok() == Some(bytes.len());
        let flushed = all && unsafe { pd_file_flush(file) } >= 0;
        let error = (!flushed).then(last_error);
        let closed = ok(unsafe { pd_file_close(file) });
        match error {
            Some(error) => Err(error),
            None => closed,
        }
    }

    fn rename_data(&self, from: &str, to: &str) -> Result<(), String> {
        let (from, to) = (c_string(from), c_string(to));
        ok(unsafe { pd_file_rename(from.as_ptr().cast(), to.as_ptr().cast()) })
    }

    fn remove_data(&self, path: &str) -> Result<(), String> {
        let path = c_string(path);
        ok(unsafe { pd_file_unlink(path.as_ptr().cast()) })
    }

    fn data_names(&self, folder: &str) -> Result<Vec<String>, String> {
        let folder = c_string(folder);
        let mut names: Vec<String> = Vec::new();
        let names_ptr: *mut Vec<String> = &raw mut names;
        ok(unsafe { pd_file_list(folder.as_ptr().cast(), listed, names_ptr.cast()) })?;
        Ok(names)
    }

    fn make_data_folder(&self, folder: &str) -> Result<(), String> {
        let folder = c_string(folder);
        ok(unsafe { pd_file_mkdir(folder.as_ptr().cast()) })
    }
}

/// What the entry points drive, whatever the game's type.
trait Running {
    fn update(&mut self) -> bool;
    fn event(&mut self, event: SdkEvent);
    fn menu_chosen(&mut self, slot: u8);
}

impl<G: GameData> Running for Driver<G, Device> {
    fn update(&mut self) -> bool {
        Driver::update(self)
    }

    fn event(&mut self, event: SdkEvent) {
        Driver::event(self, event);
    }

    fn menu_chosen(&mut self, slot: u8) {
        Driver::menu_chosen(self, slot);
    }
}

/// The running game. The SDK calls in on its only thread, so this is never
/// touched from two places at once.
struct Game(UnsafeCell<Option<Box<dyn Running>>>);

// SAFETY: the Playdate runs the game on one thread (see `Game`).
unsafe impl Sync for Game {}

static GAME: Game = Game(UnsafeCell::new(None));

/// The running game, once [`init`] has made it.
fn running() -> Option<&'static mut Box<dyn Running>> {
    // SAFETY: one thread (see `Game`), and no entry point calls another.
    unsafe { (*GAME.0.get()).as_mut() }
}

/// The SDK's first event: makes the game `start` says.
pub fn init<G: GameData>(start: Start<G>) {
    if cfg!(debug_assertions) {
        crate::storage::check(Device);
        // check-text: not player text (for the console)
        Device.log("storage: the conformance suite passed");
    }
    let driver: Box<dyn Running> = Box::new(Driver::new(start, Device));
    // SAFETY: one thread (see `Game`), before any update.
    unsafe { *GAME.0.get() = Some(driver) };
}

/// The SDK's update callback; 1 when the screen changed.
pub fn update() -> c_int {
    running().map_or(0, |game| c_int::from(game.update()))
}

/// The SDK's other events, by number.
pub fn event(event: c_int) {
    if let (Some(game), Some(event)) = (running(), SdkEvent::from_raw(event)) {
        game.event(event);
    }
}

/// The player chose the system menu's item in `slot`.
pub fn menu_chosen(slot: c_int) {
    if let (Some(game), Ok(slot)) = (running(), u8::try_from(slot)) {
        game.menu_chosen(slot);
    }
}
