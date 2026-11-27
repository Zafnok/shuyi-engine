//! `crash --symbols`: a crash report's frames named from the debug file
//! of its build. The debug file is the program as built
//! before it was stripped, with its line tables (DWARF), in a PE or ELF
//! file; `addr2line`'s [`Loader`] reads it.
//!
//! A frame's offset is its return address minus the program's load
//! address (`shuyi_platform::crash::CodeFrame`). The address the debug
//! file knows is the image base it was linked at (Windows; 0 on Linux)
//! plus the offset, and 1 less: a return address is the instruction after
//! the call, which may be on the next line or in the next function.
//! `backtrace` and debuggers take 1 off the same way.

use std::borrow::Cow;
use std::path::Path;

use addr2line::Loader;

use super::NamedFrame;

/// A debug file, read.
pub(super) struct Symbols {
    loader: Loader,
    /// The image base the file was linked at: `relative_address_base`.
    base: u64,
    /// Where the program's code is, `[begin, end)` as the file's
    /// addresses: its `.text` section, if it has one.
    code: Option<(u64, u64)>,
}

impl Symbols {
    /// The debug file at `path`, or why it doesn't read as a program.
    pub(super) fn open(path: &Path) -> Result<Self, String> {
        let loader = Loader::new(path).map_err(|e| e.to_string())?;
        let base = loader.relative_address_base();
        let code = loader
            .get_section_range(b".text")
            .map(|range| (range.begin, range.end));
        Ok(Self { loader, base, code })
    }

    /// The address the file knows for a frame at `offset`: the call's own
    /// instruction, 1 before where it returns to. `None` for an offset
    /// that can't be one (0, or past the end of the address space).
    fn probe(&self, offset: u64) -> Option<u64> {
        self.base.checked_add(offset)?.checked_sub(1)
    }

    /// The frames at `offset`, the innermost first: an inlined function
    /// gives a frame of its own, then the function it was inlined into, at
    /// the line of the call. Empty when the file knows nothing of the
    /// offset; a frame with no function when it knows only its line.
    ///
    /// A function's name is the debug information's; where it gives none,
    /// the symbol table's ([`Loader::find_symbol`]), if the offset is in
    /// the program's code: the symbol table's lookup gives the nearest
    /// symbol before any address, however far.
    pub(super) fn frames(&self, offset: u64) -> Vec<NamedFrame> {
        let Some(probe) = self.probe(offset) else {
            return Vec::new();
        };
        let mut named = Vec::new();
        if let Ok(mut frames) = self.loader.find_frames(probe) {
            while let Ok(Some(frame)) = frames.next() {
                let mut one = NamedFrame::new(Some(offset));
                one.function = frame
                    .function
                    .and_then(|function| function.demangle().ok().map(Cow::into_owned));
                if let Some(location) = frame.location {
                    one.file = location.file.map(str::to_owned);
                    one.line = location.line;
                }
                named.push(one);
            }
        }
        if !named.iter().any(|frame| frame.function.is_some())
            && let Some(name) = self.symbol(probe)
        {
            if named.is_empty() {
                named.push(NamedFrame::new(Some(offset)));
            }
            named[0].function = Some(name);
        }
        named
    }

    /// The symbol table's name for `probe`, demangled, if `probe` is in
    /// the program's code.
    fn symbol(&self, probe: u64) -> Option<String> {
        let (begin, end) = self.code?;
        if !(begin..end).contains(&probe) {
            return None;
        }
        let name = self.loader.find_symbol(probe)?;
        Some(addr2line::demangle_auto(Cow::Borrowed(name), None).into_owned())
    }
}

#[cfg(test)]
mod tests;
