//! The lookup over this test program's own debug information. Only on
//! Linux and Windows GNU, where a test build carries DWARF (the root
//! `Cargo.toml` keeps shuyi-cli's: `[profile.dev.package.shuyi-cli]`);
//! an MSVC build writes PDB files, and macOS keeps its DWARF apart.
#![cfg(any(target_os = "linux", all(windows, target_env = "gnu")))]

use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};

use object::{Object, ObjectSection, ObjectSymbol, SymbolKind};
use shuyi_platform::crash::{CodeFrame, CrashReport};

use super::Symbols;
use crate::contract::crash::{CrashInput, crash};

/// This test program: its own debug file.
fn this_program() -> PathBuf {
    std::env::current_exe().unwrap()
}

fn read(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap()
}

/// The offset of the function whose symbol's name holds `name`: its
/// address minus the image base, as the crash hook's offsets are.
fn offset_of(name: &str) -> u64 {
    let data = read(&this_program());
    let file = object::File::parse(&*data).unwrap();
    let mut found = file
        .symbols()
        .filter(|symbol| symbol.kind() == SymbolKind::Text)
        .filter(|symbol| symbol.name().is_ok_and(|s| s.contains(name)));
    let symbol = found.next().unwrap_or_else(|| panic!("no symbol {name}"));
    assert!(found.next().is_none(), "two symbols hold {name}");
    symbol.address() - file.relative_address_base()
}

const TARGET_FIRST: u32 = line!();
/// The function the first test names, by a name nothing else holds.
#[inline(never)]
fn probe_target_3441(n: u64) -> u64 {
    let doubled = black_box(n) * 2;
    black_box(doubled + 1)
}
const TARGET_LAST: u32 = line!();

#[test]
fn a_frame_a_few_bytes_into_a_function_is_named_with_its_file_and_line() {
    assert_eq!(probe_target_3441(black_box(3)), 7);
    let offset = offset_of("probe_target_3441") + 4;
    let dir = std::env::temp_dir().join(format!("shuyi-crash-symbols-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let mut report = CrashReport::new(1, "boom");
    report.frames = vec![CodeFrame::new(Some(offset)), CodeFrame::new(None)];
    fs::write(dir.join("c.ron"), report.to_text()).unwrap();
    let mut input = CrashInput::new("c.ron");
    input.symbols = Some(this_program().to_string_lossy().into_owned());
    let answer = crash(&input, &dir);
    assert!(answer.ok, "{answer:?}");
    assert!(answer.limits.is_empty(), "{:?}", answer.limits);
    let frames = answer.result.unwrap().frames;
    let [named, other] = &frames[..] else {
        panic!("{frames:?}")
    };
    assert_eq!(named.offset, Some(format!("{offset:016x}")));
    let function = named.function.as_deref().unwrap_or_default();
    assert!(
        function.ends_with("crash::symbols::tests::probe_target_3441"),
        "{named:?}"
    );
    let file = named.file.as_deref().unwrap_or_default().replace('\\', "/");
    assert!(file.ends_with("crash/symbols/tests.rs"), "{named:?}");
    let line = named.line.unwrap_or_default();
    assert!((TARGET_FIRST..TARGET_LAST).contains(&line), "{named:?}");
    assert_eq!(other.offset, None);
    fs::remove_dir_all(&dir).unwrap();
}

/// A function whose first byte is where a call made just before it would
/// return to: named there only if the lookup forgot to take 1 off.
#[inline(never)]
fn probe_after_3441(n: u64) -> u64 {
    black_box(n) ^ 0x5a
}

#[test]
fn a_frame_is_looked_up_one_byte_before_where_it_returns_to() {
    assert_eq!(probe_after_3441(black_box(0)), 0x5a);
    let symbols = Symbols::open(&this_program()).unwrap();
    let start = offset_of("probe_after_3441");
    let names = |offset: u64| -> Vec<String> {
        symbols
            .frames(offset)
            .into_iter()
            .filter_map(|frame| frame.function)
            .collect()
    };
    // A call that returns to the function's first byte was made from
    // whatever lies before it.
    let before = names(start);
    assert!(
        !before.iter().any(|name| name.ends_with("probe_after_3441")),
        "{before:?}"
    );
    let inside = names(start + 1);
    assert!(
        inside.iter().any(|name| name.ends_with("probe_after_3441")),
        "{inside:?}"
    );
}

/// Inlined into [`probe_outer_3441`] even unoptimised.
#[expect(
    clippy::inline_always,
    reason = "the test needs an inlined frame in a test build, which only inline(always) makes"
)]
#[inline(always)]
fn probe_inner_3441(n: u64) -> u64 {
    black_box(n).rotate_left(7) ^ black_box(0x3441)
}

#[inline(never)]
fn probe_outer_3441(n: u64) -> u64 {
    let m = black_box(n) + 1;
    probe_inner_3441(m) + 2
}

#[test]
fn an_inlined_function_is_a_frame_of_its_own_before_its_caller() {
    assert_ne!(probe_outer_3441(black_box(5)), 0);
    let symbols = Symbols::open(&this_program()).unwrap();
    let start = offset_of("probe_outer_3441");
    let inlined = (1..512).map(|n| symbols.frames(start + n)).find(|frames| {
        frames.len() >= 2
            && frames[0]
                .function
                .as_deref()
                .is_some_and(|f| f.ends_with("probe_inner_3441"))
    });
    let frames = inlined.expect("no address of probe_outer_3441 inlines probe_inner_3441");
    assert!(
        frames[1]
            .function
            .as_deref()
            .is_some_and(|f| f.ends_with("probe_outer_3441")),
        "{frames:?}"
    );
    assert_eq!(frames[0].offset, frames[1].offset);
    // The caller's line is the call's, in this file.
    assert!(frames[1].line.is_some(), "{frames:?}");
}

#[test]
fn the_symbol_table_names_a_function_the_debug_information_doesnt() {
    // Dependencies build without debug information
    // (`[profile.dev.package."*"]`), so some function of theirs in this
    // program has a symbol and no DWARF.
    let path = this_program();
    let data = read(&path);
    let file = object::File::parse(&*data).unwrap();
    let base = file.relative_address_base();
    let text = file.section_by_name(".text").unwrap().index();
    let loader = addr2line::Loader::new(&path).unwrap();
    let symbols = Symbols::open(&path).unwrap();
    let undescribed = file
        .symbols()
        .filter(|symbol| symbol.is_definition() && symbol.section_index() == Some(text))
        .filter(|symbol| symbol.kind() == SymbolKind::Text && symbol.address() > base)
        .map(|symbol| symbol.address() - base)
        .find(|&offset| {
            let mut frames = loader.find_frames(base + offset).unwrap();
            let named = std::iter::from_fn(|| frames.next().ok().flatten())
                .any(|frame| frame.function.is_some());
            !named && loader.find_symbol(base + offset).is_some()
        })
        .expect("every function has debug information");
    // One byte in: the frame is looked up at its first byte.
    let frames = symbols.frames(undescribed + 1);
    let name = frames.first().and_then(|frame| frame.function.clone());
    let symbol = loader.find_symbol(base + undescribed).unwrap();
    let expected = addr2line::demangle_auto(symbol.into(), None).into_owned();
    assert_eq!(name, Some(expected), "{frames:?}");
}

#[test]
fn an_offset_outside_every_function_is_counted_not_an_error() {
    let dir = std::env::temp_dir().join(format!("shuyi-crash-outside-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let mut report = CrashReport::new(1, "boom");
    let far = 0x7fff_0000_0000;
    report.frames = vec![
        CodeFrame::new(Some(offset_of("probe_target_3441") + 4)),
        CodeFrame::new(Some(far)),
        CodeFrame::new(Some(0)),
        CodeFrame::new(None),
    ];
    assert_eq!(probe_target_3441(black_box(1)), 3);
    fs::write(dir.join("c.ron"), report.to_text()).unwrap();
    let mut input = CrashInput::new("c.ron");
    input.symbols = Some(this_program().to_string_lossy().into_owned());
    let answer = crash(&input, &dir);
    assert!(answer.ok, "{answer:?}");
    assert_eq!(
        answer.limits,
        ["2 of 3 frames fall outside every function: the debug file may be from another build"]
    );
    let frames = answer.result.unwrap().frames;
    assert_eq!(frames.len(), 4, "{frames:?}");
    assert_eq!(frames[1].offset.as_deref(), Some("00007fff00000000"));
    assert_eq!(frames[1].function, None);
    assert_eq!(frames[2].offset.as_deref(), Some("0000000000000000"));
    fs::remove_dir_all(&dir).unwrap();
}

/// The range `[low, high)` this program's lines cover in
/// `/proc/self/maps`, as the crash hook reads it.
#[cfg(target_os = "linux")]
fn program_range(_frames: &[(u64, Option<u64>)]) -> (u64, u64) {
    let exe = fs::read_link("/proc/self/exe").unwrap();
    let exe = exe.to_string_lossy();
    let maps = fs::read_to_string("/proc/self/maps").unwrap();
    let mut range: Option<(u64, u64)> = None;
    for line in maps.lines() {
        let mut fields = line.splitn(6, ' ');
        let span = fields.next().unwrap();
        if fields.nth(4).map(str::trim_start) != Some(&*exe) {
            continue;
        }
        let (start, end) = span.split_once('-').unwrap();
        let start = u64::from_str_radix(start, 16).unwrap();
        let end = u64::from_str_radix(end, 16).unwrap();
        range = Some(range.map_or((start, end), |(low, high)| (low.min(start), high.max(end))));
    }
    range.expect("no line of /proc/self/maps names this program")
}

/// The offsets of the calling thread's frames in this program, as the
/// crash hook takes them: each return address minus the program's load
/// address, the frames of other modules left out.
#[inline(never)]
fn stack_probe_3441() -> Vec<u64> {
    let mut frames: Vec<(u64, Option<u64>)> = Vec::new();
    backtrace::trace(|frame| {
        #[cfg(windows)]
        let base = frame
            .module_base_address()
            .and_then(|base| u64::try_from(base as usize).ok());
        #[cfg(target_os = "linux")]
        let base = None;
        if let Ok(ip) = u64::try_from(frame.ip() as usize) {
            frames.push((ip, base));
        }
        true
    });
    let (low, high) = program_range(&frames);
    frames
        .into_iter()
        .filter(|&(ip, base)| {
            (low..high).contains(&ip) && (cfg!(target_os = "linux") || base == Some(low))
        })
        .map(|(ip, _)| ip - low)
        .collect()
}

/// Windows: where the program starts, from the bases of the frames'
/// modules: the highest base at or below this test's own code, which only
/// the program's own can be, as modules don't overlap.
#[cfg(windows)]
fn program_range(frames: &[(u64, Option<u64>)]) -> (u64, u64) {
    let ours = u64::try_from(stack_probe_3441 as *const () as usize).unwrap();
    let base = frames
        .iter()
        .filter_map(|&(_, base)| base)
        .filter(|&base| base <= ours)
        .max()
        .expect("no frame's module holds this program");
    (base, u64::MAX)
}

#[test]
fn a_frame_of_this_programs_own_stack_is_named() {
    let offsets = black_box(stack_probe_3441());
    assert_ne!(offsets.len(), 0);
    let symbols = Symbols::open(&this_program()).unwrap();
    let names: Vec<String> = offsets
        .iter()
        .flat_map(|&offset| symbols.frames(offset))
        .filter_map(|frame| frame.function)
        .collect();
    for name in [
        "stack_probe_3441",
        "a_frame_of_this_programs_own_stack_is_named",
    ] {
        assert!(
            names.iter().any(|named| named.ends_with(name)),
            "no {name} in {names:?}"
        );
    }
}
