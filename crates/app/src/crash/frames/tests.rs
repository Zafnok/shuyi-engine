//! The executable's range in `/proc/self/maps`, offsets, and a frame
//! captured inside a function of this test program.

use super::*;

/// Lines of `/proc/self/maps`, the program's among others.
const MAPS: &str = "\
5581a2c00000-5581a2c3e000 r--p 00000000 08:01 1234                       /opt/my game/game
5581a2c3e000-5581a2f10000 r-xp 0003e000 08:01 1234                       /opt/my game/game
5581a2f10000-5581a2f9a000 r--p 00310000 08:01 1234                       /opt/my game/game
5581a3000000-5581a3021000 rw-p 00000000 00:00 0                          [heap]
7f3c10000000-7f3c10028000 r--p 00000000 08:01 99                         /usr/lib/libc.so.6
7f3c10028000-7f3c101bd000 r-xp 00028000 08:01 99                         /usr/lib/libc.so.6
7ffd5c2a1000-7ffd5c2c2000 rw-p 00000000 00:00 0                          [stack]
";

#[test]
fn the_executables_range_spans_its_own_lines_only() {
    assert_eq!(
        executable_range(MAPS, "/opt/my game/game"),
        Some((0x5581_a2c0_0000, 0x5581_a2f9_a000))
    );
    assert_eq!(
        executable_range(MAPS, "/usr/lib/libc.so.6"),
        Some((0x7f3c_1000_0000, 0x7f3c_101b_d000))
    );
    assert_eq!(executable_range(MAPS, "/opt/my game"), None);
    assert_eq!(executable_range("", "/opt/my game/game"), None);
    assert_eq!(
        executable_range("zz-10 r--p 0 0:0 1 /a\n10-20 r--p 0 0:0 1 /a\n", "/a"),
        Some((0x10, 0x20))
    );
}

#[test]
fn an_offset_is_from_the_base_and_only_inside_the_executable() {
    assert_eq!(offset_from(0x1234, 0x1000, true), Some(0x234));
    assert_eq!(offset_from(0x1000, 0x1000, true), Some(0));
    assert_eq!(offset_from(0x1234, 0x1000, false), None);
    assert_eq!(offset_from(0x0fff, 0x1000, true), None);
}

/// The frames as captured from inside this function.
#[inline(never)]
fn captured_here() -> Vec<CodeFrame> {
    std::hint::black_box(capture())
}

#[cfg(any(windows, all(target_os = "linux", target_env = "gnu")))]
#[test]
fn a_frame_lands_inside_the_function_that_captured_it() {
    note_executable();
    let frames = captured_here();
    assert!(!frames.is_empty() && frames.len() <= FRAMES);
    let here = captured_here as fn() -> Vec<CodeFrame> as usize;
    let Some(own) = os::offset(here) else {
        panic!("the test's own function is outside the executable");
    };
    // A return address into `captured_here`: a little past its start.
    let inside = |offset: u64| offset > own && offset < own + 4096;
    assert!(
        frames.iter().filter_map(|f| f.offset).any(inside),
        "no frame just past {own:#x}: {frames:?}"
    );
    // An address in no module has no offset.
    assert_eq!(os::offset(0), None);
}

#[cfg(not(any(windows, all(target_os = "linux", target_env = "gnu"))))]
#[test]
fn elsewhere_there_are_no_frames() {
    note_executable();
    assert_eq!(captured_here(), Vec::<CodeFrame>::new());
}
