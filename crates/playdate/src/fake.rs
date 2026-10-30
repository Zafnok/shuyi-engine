//! A fake of the SDK, for the backend's tests: what a test says the device
//! does, and what the backend asked of it, with files that can lose the power.

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::sync::atomic::{AtomicU64, Ordering};

use shuyi_ui::frame::onebit::{FRAME_BYTES, Frame};
use shuyi_ui::perf::HeapCounts;

use crate::sdk::Sdk;

/// What a rename onto a file that's there does: the SDK's docs don't say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rename {
    /// It replaces it.
    #[default]
    Replaces,
    /// It fails.
    Refuses,
}

/// A file on the fake's medium.
#[derive(Debug, Clone, Default)]
struct File {
    /// What a read gives now.
    bytes: Vec<u8>,
    /// What's left after a power cut: flushed, and only that.
    durable: Vec<u8>,
}

#[derive(Debug)]
struct State {
    millis: u32,
    buttons: VecDeque<(u32, u32)>,
    crank: VecDeque<f32>,
    docked: bool,
    frame: Box<Frame>,
    marked: Vec<(i32, i32)>,
    texts: Vec<(String, bool)>,
    logs: Vec<String>,
    refresh_rate: Option<u16>,
    menu: Vec<(String, u8)>,
    menu_full: bool,
    frame_uses: u32,
    data: BTreeMap<String, File>,
    folders: BTreeSet<String>,
    bundled: BTreeMap<String, Vec<u8>>,
    file_calls: Vec<String>,
    rename: Rename,
    /// Operations of the medium left before the power goes, if it is going.
    ops_left: Option<u32>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            millis: 0,
            buttons: VecDeque::new(),
            crank: VecDeque::new(),
            docked: false,
            frame: Box::new([0; FRAME_BYTES]),
            marked: Vec::new(),
            texts: Vec::new(),
            logs: Vec::new(),
            refresh_rate: None,
            menu: Vec::new(),
            menu_full: false,
            frame_uses: 0,
            data: BTreeMap::new(),
            folders: BTreeSet::new(),
            bundled: BTreeMap::new(),
            file_calls: Vec::new(),
            rename: Rename::default(),
            ops_left: None,
        }
    }
}

impl State {
    /// One operation of the medium: it fails once the power has gone.
    fn op(&mut self) -> Result<(), String> {
        match &mut self.ops_left {
            Some(0) => Err("the power is off".to_owned()),
            Some(left) => {
                *left -= 1;
                Ok(())
            }
            None => Ok(()),
        }
    }
}

/// The fake: a clone is the same device.
#[derive(Debug, Clone, Default)]
pub struct FakeSdk(Rc<RefCell<State>>);

impl FakeSdk {
    /// The next frame's clock reads `millis`.
    pub fn set_millis(&self, millis: u32) {
        self.0.borrow_mut().millis = millis;
    }

    /// The next read of the buttons gives bits `current` held and `pushed`
    /// pressed; after the queue, nothing.
    pub fn queue_buttons(&self, current: u32, pushed: u32) {
        self.0.borrow_mut().buttons.push_back((current, pushed));
    }

    /// The next read of the crank gives `degrees`; after the queue, none.
    pub fn queue_crank(&self, degrees: f32) {
        self.0.borrow_mut().crank.push_back(degrees);
    }

    /// Whether the crank is docked.
    pub fn set_docked(&self, docked: bool) {
        self.0.borrow_mut().docked = docked;
    }

    /// The rows marked since the last call.
    pub fn take_marked(&self) -> Vec<(i32, i32)> {
        core::mem::take(&mut self.0.borrow_mut().marked)
    }

    /// The text drawn since the last call, each with whether it was on white.
    pub fn take_texts(&self) -> Vec<(String, bool)> {
        core::mem::take(&mut self.0.borrow_mut().texts)
    }

    /// The console's lines.
    pub fn logs(&self) -> Vec<String> {
        self.0.borrow().logs.clone()
    }

    /// The display's rate, as last set.
    pub fn refresh_rate(&self) -> Option<u16> {
        self.0.borrow().refresh_rate
    }

    /// The menu's items, in the order added.
    pub fn menu(&self) -> Vec<(String, u8)> {
        self.0.borrow().menu.clone()
    }

    /// The system menu refuses every item from now on.
    pub fn fill_menu(&self) {
        self.0.borrow_mut().menu_full = true;
    }

    /// How many times the frame buffer has been touched.
    pub fn frame_uses(&self) -> u32 {
        self.0.borrow().frame_uses
    }

    /// The frame buffer, now.
    pub fn frame(&self) -> Box<Frame> {
        self.0.borrow().frame.clone()
    }

    /// Scribbles on the frame buffer, as the system menu may.
    pub fn scribble_on_frame(&self) {
        self.0.borrow_mut().frame.fill(0x55);
    }

    /// Puts a file in the `.pdx`.
    pub fn put_bundled(&self, path: &str, bytes: &[u8]) {
        self.0
            .borrow_mut()
            .bundled
            .insert(path.to_owned(), bytes.to_vec());
    }

    /// Puts a file in the data folder, durable.
    pub fn put_file(&self, path: &str, bytes: &[u8]) {
        let file = File {
            bytes: bytes.to_vec(),
            durable: bytes.to_vec(),
        };
        self.0.borrow_mut().data.insert(path.to_owned(), file);
    }

    /// The data folder's file `path`, now.
    pub fn file(&self, path: &str) -> Option<Vec<u8>> {
        self.0
            .borrow()
            .data
            .get(path)
            .map(|file| file.bytes.clone())
    }

    /// The data folder's writes and renames since the last call.
    pub fn take_file_calls(&self) -> Vec<String> {
        core::mem::take(&mut self.0.borrow_mut().file_calls)
    }

    /// What a rename onto a file that's there does.
    pub fn set_rename(&self, rename: Rename) {
        self.0.borrow_mut().rename = rename;
    }

    /// The power goes after `ops` more operations of the medium.
    pub fn cut_power_after(&self, ops: u32) {
        self.0.borrow_mut().ops_left = Some(ops);
    }

    /// The power comes back: the medium has what it had made durable.
    pub fn restore_power(&self) {
        let mut state = self.0.borrow_mut();
        state.ops_left = None;
        for file in state.data.values_mut() {
            file.bytes.clone_from(&file.durable);
        }
    }
}

/// The fake clock's next reading.
static CLOCK_US: AtomicU64 = AtomicU64::new(0);

impl Sdk for FakeSdk {
    fn millis(&self) -> u32 {
        self.0.borrow().millis
    }

    fn clock_us() -> u64 {
        CLOCK_US.fetch_add(100, Ordering::Relaxed)
    }

    fn heap() -> Option<HeapCounts> {
        Some(HeapCounts::new(1024, 2048, 3))
    }

    fn buttons(&self) -> (u32, u32) {
        self.0.borrow_mut().buttons.pop_front().unwrap_or((0, 0))
    }

    fn crank_change(&self) -> f32 {
        self.0.borrow_mut().crank.pop_front().unwrap_or(0.0)
    }

    fn crank_docked(&self) -> bool {
        self.0.borrow().docked
    }

    fn add_menu_item(&self, title: &str, slot: u8) -> bool {
        let mut state = self.0.borrow_mut();
        if state.menu_full {
            return false;
        }
        state.menu.push((title.to_owned(), slot));
        true
    }

    fn set_refresh_rate(&self, fps: u16) {
        self.0.borrow_mut().refresh_rate = Some(fps);
    }

    fn with_frame(&self, draw: impl FnOnce(&mut Frame)) {
        let mut state = self.0.borrow_mut();
        state.frame_uses += 1;
        draw(&mut state.frame);
    }

    fn mark_rows(&self, first: i32, last: i32) {
        self.0.borrow_mut().marked.push((first, last));
    }

    fn draw_text(&self, text: &str, white: bool) {
        let mut state = self.0.borrow_mut();
        state.texts.push((text.to_owned(), white));
        state.frame.fill(if white { u8::MAX } else { 0 });
    }

    fn log(&self, line: &str) {
        self.0.borrow_mut().logs.push(line.to_owned());
    }

    fn is_file(&self, path: &str) -> bool {
        self.0.borrow().data.contains_key(path)
    }

    fn read_data(&self, path: &str) -> Result<Vec<u8>, String> {
        let file = self.0.borrow().data.get(path).cloned();
        file.map(|file| file.bytes)
            .ok_or_else(|| format!("no file {path}"))
    }

    fn read_bundled(&self, path: &str) -> Result<Vec<u8>, String> {
        let file = self.0.borrow().bundled.get(path).cloned();
        file.ok_or_else(|| format!("no file {path}"))
    }

    /// As the real call does: the file emptied at the open, the bytes
    /// written, then flushed. Each is an operation, and only what is
    /// flushed is durable.
    fn write_data(&self, path: &str, bytes: &[u8]) -> Result<(), String> {
        let mut state = self.0.borrow_mut();
        state.op()?;
        state.file_calls.push(format!("write {path}"));
        state.data.insert(path.to_owned(), File::default());
        state.op()?;
        let half = bytes.len() / 2;
        if let Some(file) = state.data.get_mut(path) {
            file.bytes = bytes.to_vec();
            file.durable = bytes[..half].to_vec();
        }
        state.op()?;
        if let Some(file) = state.data.get_mut(path) {
            file.durable = bytes.to_vec();
        }
        Ok(())
    }

    fn rename_data(&self, from: &str, to: &str) -> Result<(), String> {
        let mut state = self.0.borrow_mut();
        state.op()?;
        if state.rename == Rename::Refuses && state.data.contains_key(to) {
            return Err(format!("{to} is there"));
        }
        let file = state
            .data
            .remove(from)
            .ok_or_else(|| format!("no file {from}"))?;
        state.data.insert(to.to_owned(), file);
        state.file_calls.push(format!("rename {from} {to}"));
        Ok(())
    }

    fn remove_data(&self, path: &str) -> Result<(), String> {
        let mut state = self.0.borrow_mut();
        state.op()?;
        state.data.remove(path);
        state.file_calls.push(format!("remove {path}"));
        Ok(())
    }

    fn data_names(&self, folder: &str) -> Result<Vec<String>, String> {
        let state = self.0.borrow();
        if state.ops_left == Some(0) {
            return Err("the power is off".to_owned());
        }
        let prefix = if folder.is_empty() {
            String::new()
        } else {
            if !state.folders.contains(folder) {
                return Err(format!("no folder {folder}"));
            }
            format!("{folder}/")
        };
        let files = state.data.keys().filter_map(|path| {
            let name = path.strip_prefix(&prefix)?;
            (!name.contains('/')).then(|| name.to_owned())
        });
        let folders = state
            .folders
            .iter()
            .filter(|_| folder.is_empty())
            .map(|name| format!("{name}/"));
        Ok(files.chain(folders).collect())
    }

    fn make_data_folder(&self, folder: &str) -> Result<(), String> {
        if self.0.borrow_mut().folders.insert(folder.to_owned()) {
            Ok(())
        } else {
            Err(format!("{folder} is there"))
        }
    }
}
