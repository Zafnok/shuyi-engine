//! Pictures a skin paints at run time through the Harness (ticket 2399,
//! ADR-0077): the toy's plasma stage paints an RGBA, an indexed and a
//! one-bit picture from its step count. What happened is read from the
//! stage's view and the snapshot's lines, never from pixels: each picture
//! is one line, its revision moving only when the plasma does.

use std::fmt::Write as _;

use insta::assert_snapshot;
use shuyi_platform::harness::{FRAME_DT, Harness};
use shuyi_ui::frame::painted::{Painted, PaintedKind};
use shuyi_ui::toy::plasma::{self, INDEXED, ONE_BIT, PlasmaStage, RGBA, STEPS_A_PHASE};
use shuyi_ui::toy::{self, Toy};

fn stage() -> Harness<Toy> {
    Harness::new(plasma::game_on(toy::ctx()))
}

fn view(h: &Harness<Toy>) -> &PlasmaStage {
    h.screen::<PlasmaStage>()
        .unwrap_or_else(|| panic!("no plasma stage: {:?}", h.screens()))
}

/// The snapshot's lines of painted items.
fn painted_lines(h: &Harness<Toy>) -> Vec<String> {
    h.snapshot()
        .lines()
        .filter(|line| line.contains(" painted "))
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_plasma_over_ten_steps() {
    let mut h = stage();
    let mut text = String::new();
    for _ in 0..=10 {
        let stage = view(&h);
        let _ = writeln!(text, "step {} phase {}", stage.step, stage.phase());
        for line in painted_lines(&h) {
            let _ = writeln!(text, "  {line}");
        }
        h.frame(&[], FRAME_DT);
    }
    assert_snapshot!("plasma_over_ten_steps", text);
}

#[test]
fn each_picture_is_its_kind_and_moves_only_with_the_phase() {
    let mut h = stage();
    let kinds = |h: &Harness<Toy>| {
        let pictures = h.game().buffer().pictures();
        [RGBA, INDEXED, ONE_BIT].map(|id| pictures.get(id).map(|p| (p.kind(), p.size())))
    };
    assert_eq!(
        kinds(&h),
        [
            Some((PaintedKind::Rgba, plasma::SIZE)),
            Some((PaintedKind::Indexed, plasma::SIZE)),
            Some((PaintedKind::OneBit, plasma::SIZE)),
        ]
    );
    let revisions = |h: &Harness<Toy>| {
        let pictures = h.game().buffer().pictures();
        [RGBA, INDEXED, ONE_BIT].map(|id| pictures.get(id).map(Painted::revision))
    };
    let first = revisions(&h);
    for _ in 1..STEPS_A_PHASE {
        h.frame(&[], FRAME_DT);
        assert_eq!(revisions(&h), first, "step {}", view(&h).step);
    }
    h.frame(&[], FRAME_DT);
    assert_eq!(view(&h).phase(), 1);
    let moved = revisions(&h);
    for (before, after) in first.iter().zip(&moved) {
        assert_ne!(before, after);
    }
    // Three items, each over its own picture, all `Under`.
    assert_eq!(h.game().buffer().painted_sprites().len(), 3);
}
