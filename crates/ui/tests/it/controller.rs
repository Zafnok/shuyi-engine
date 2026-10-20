//! Playing on a controller through the Harness (ticket 1111, game ADR-0034):
//! buttons act through the keymap, the context knows which device and
//! kind of pad was used last, and help text names that device's keys.

use shuyi_ui::input::{Device, PadKind};

use crate::toy;

/// The gallery of pictures, opened with the Debug key.
fn gallery() -> shuyi_platform::harness::Harness<shuyi_ui::toy::Toy> {
    let mut h = toy();
    h.keys("F2");
    assert_eq!(h.screens(), ["home", "gallery"]);
    h
}

/// The gallery's help line, the frame's last row.
fn help(h: &shuyi_platform::harness::Harness<shuyi_ui::toy::Toy>) -> String {
    let rows = h.game().buffer().height();
    (0..h.game().buffer().width())
        .filter_map(|x| h.game().buffer().get(x.into(), i32::from(rows) - 1))
        .map(|c| c.glyph)
        .collect::<String>()
        .trim_end()
        .to_owned()
}

#[test]
fn buttons_act_through_the_keymap() {
    let mut h = toy();
    h.pad("South");
    assert_eq!(h.screens(), ["home", "menu"]);
    h.pad("East");
    assert_eq!(h.screens(), ["home"]);
}

#[test]
fn the_device_used_last_is_the_one_pressed_last() {
    let mut h = toy();
    assert_eq!(h.ctx().device, Device::Keyboard);
    // An unbound button: still the keyboard.
    h.pad("RightStickPress");
    assert_eq!(h.ctx().device, Device::Keyboard);
    h.pad("DpadDown");
    assert_eq!(h.ctx().device, Device::Pad(PadKind::Generic));
    h.use_pad(PadKind::PlayStation).pad("DpadDown");
    assert_eq!(h.ctx().device, Device::Pad(PadKind::PlayStation));
    h.use_pad(PadKind::Nintendo).hold_pad("DpadDown", 0.0);
    assert_eq!(h.ctx().device, Device::Pad(PadKind::Nintendo));
    h.keys("Up");
    assert_eq!(h.ctx().device, Device::Keyboard);
}

#[test]
fn help_text_names_the_button_after_a_controller_press() {
    let mut h = gallery();
    assert_eq!(help(&h), "d back");
    h.use_pad(PadKind::Xbox).pad("DpadDown");
    assert_eq!(help(&h), "B back");
    h.keys("Up");
    assert_eq!(help(&h), "d back");
    h.pad("East");
    assert_eq!(h.screens(), ["home"]);
}

#[test]
fn snapshot_as_only_looks() {
    let mut h = gallery();
    let keys = h.snapshot();
    h.use_pad(PadKind::Xbox).pad("DpadDown");
    let pad = h.snapshot();
    assert_ne!(pad, keys);
    assert_eq!(h.snapshot_as(Device::Keyboard), keys);
    // Looking changes neither the device nor the frame.
    assert_eq!(h.ctx().device, Device::Pad(PadKind::Xbox));
    assert_eq!(h.snapshot(), pad);
    assert_eq!(h.snapshot_as(Device::Pad(PadKind::Xbox)), pad);
}
