//! Deterministic gameplay captures of the 2912 fixtures for the clip study.
//!
//! A fixture's text frame is laid out on a console and painted by the
//! engine's software raster, so the same game, seed and actions give the
//! same PNG bytes on every run. This crate is research tooling outside the
//! engine's release path.

use cloning_fixture::{courier, workshop};
use shuyi_assets::font::FontAtlasDef;
use shuyi_assets::image::ImageTable;
use shuyi_cli::codec::encode_png;
use shuyi_cli::frame_png::Painter;
use shuyi_core::TurnRules;
use shuyi_ui::frame::Rgb;
use shuyi_ui::frame::glyph_buffer::{Cell, GlyphBuffer};

/// Console columns: with [`ROWS`] and [`SCALE`], a picture of 1280 × 736.
pub const COLS: u16 = 80;
/// Console rows.
pub const ROWS: u16 = 23;
/// Picture pixels per console pixel.
pub const SCALE: u32 = 2;

const ATLAS_PNG: &[u8] = include_bytes!("../../../../../examples/hello-grid/files/font/atlas.png");
const ATLAS_RON: &str = include_str!("../../../../../examples/hello-grid/files/font/atlas.ron");

const BACKGROUND: Rgb = Rgb::new(0x13, 0x20, 0x2a);
const TEXT: Rgb = Rgb::new(0xef, 0xf5, 0xe8);

/// The colour a map tile is drawn in; everything else is plain text.
const fn tile_color(tile: char) -> Rgb {
    match tile {
        '@' => Rgb::new(0x5c, 0xd6, 0xe8),
        'p' => Rgb::new(0xf2, 0xc9, 0x4c),
        'D' => Rgb::new(0x7b, 0xd8, 0x6a),
        '=' => Rgb::new(0xc8, 0xa8, 0x7a),
        '!' => Rgb::new(0xf0, 0x5d, 0x5e),
        '#' => Rgb::new(0x71, 0x82, 0x8e),
        '.' => Rgb::new(0x3e, 0x50, 0x5c),
        _ => TEXT,
    }
}

/// True for a line of map tiles rather than words.
fn is_map_row(line: &str) -> bool {
    !line.is_empty() && line.chars().all(|tile| "@pD=!#.".contains(tile))
}

/// The text frames of a game after each action, beginning with the first.
///
/// A refused or unknown action leaves its frame unchanged, as the fixture's
/// own oracle does.
pub fn text_frames(game: &str, seed: u64, actions: &[&str]) -> Result<Vec<String>, String> {
    match game {
        "courier" => {
            let mut state = courier::Courier::new(seed);
            let mut frames = vec![state.frame()];
            for action in actions {
                if let Some(parsed) = courier::Action::parse(action) {
                    let _refusal_keeps_the_state = state.apply(&parsed);
                }
                frames.push(state.frame());
            }
            Ok(frames)
        }
        "workshop" => {
            let mut state = workshop::Workshop::new(seed);
            let mut frames = vec![state.frame()];
            for action in actions {
                if let Some(parsed) = workshop::Action::parse(action) {
                    let _refusal_keeps_the_state = state.apply(&parsed);
                }
                frames.push(state.frame());
            }
            Ok(frames)
        }
        _ => Err(format!("unknown game: {game}")),
    }
}

/// A text frame laid out in the middle of the console.
pub fn layout(frame: &str) -> GlyphBuffer {
    let fill = Cell {
        glyph: ' ',
        fg: TEXT,
        bg: BACKGROUND,
        see_through: false,
    };
    let mut buffer = GlyphBuffer::new(COLS, ROWS, fill);
    let lines: Vec<&str> = frame.lines().collect();
    let widest = lines.iter().map(|line| line.len()).max().unwrap_or(0);
    let left = (i32::from(COLS) - i32::try_from(widest).unwrap_or(0)) / 2;
    let top = (i32::from(ROWS) - i32::try_from(lines.len()).unwrap_or(0)) / 2;
    for (y, line) in (top..).zip(&lines) {
        if is_map_row(line) {
            for (x, tile) in (left..).zip(line.chars()) {
                buffer.print(x, y, &tile.to_string(), tile_color(tile), BACKGROUND);
            }
        } else {
            buffer.print(left, y, line, TEXT, BACKGROUND);
        }
    }
    buffer
}

/// The painter of every capture: the example game's font on the study's
/// background colour.
pub fn painter() -> Result<Painter, String> {
    let atlas = FontAtlasDef::from_source("atlas.ron", ATLAS_RON, "atlas.png", Some(ATLAS_PNG), "")
        .map_err(|errors| format!("font atlas: {errors:?}"))?;
    Painter::new(
        atlas,
        ATLAS_PNG,
        &ImageTable::new(Default::default()),
        |_| None,
        BACKGROUND,
    )
}

/// The PNG files of a capture, one per text frame, in order.
pub fn capture(game: &str, seed: u64, actions: &[&str]) -> Result<Vec<Vec<u8>>, String> {
    let painter = painter()?;
    text_frames(game, seed, actions)?
        .iter()
        .map(|frame| {
            let picture = painter.render(&layout(frame), SCALE);
            encode_png(picture.width, picture.height, &picture.rgba)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const COURIER: [&str; 3] = ["down", "right", "up"];

    fn first_picture() -> Vec<u8> {
        capture("courier", 11, &[])
            .unwrap_or_else(|error| panic!("capture: {error}"))
            .remove(0)
    }

    #[test]
    fn a_capture_repeats_byte_for_byte() {
        let left = capture("courier", 11, &COURIER);
        let right = capture("courier", 11, &COURIER);
        assert_eq!(left, right);
    }

    #[test]
    fn a_picture_is_1280_by_736() {
        let (width, height, _) = shuyi_cli::codec::decode_png(&first_picture())
            .unwrap_or_else(|error| panic!("decode: {error}"));
        assert_eq!((width, height), (1280, 736));
    }

    #[test]
    fn each_action_adds_a_frame_and_a_move_changes_the_picture() {
        let pictures =
            capture("courier", 11, &COURIER).unwrap_or_else(|error| panic!("capture: {error}"));
        assert_eq!(pictures.len(), COURIER.len() + 1);
        assert_ne!(pictures[0], pictures[1]);
    }

    #[test]
    fn a_refused_action_repeats_its_frame() {
        let frames = text_frames("courier", 11, &["not-an-action", "left"])
            .unwrap_or_else(|error| panic!("frames: {error}"));
        assert_eq!(frames[0], frames[1]);
        assert_eq!(frames[0], frames[2]);
    }

    #[test]
    fn another_seed_can_show_another_game() {
        let seeds: std::collections::BTreeSet<String> = (0..16)
            .filter_map(|seed| text_frames("courier", seed, &[]).ok())
            .map(|mut frames| frames.remove(0))
            .collect();
        assert!(seeds.len() > 1);
    }

    #[test]
    fn workshop_captures_too_and_an_unknown_game_does_not() {
        assert!(capture("workshop", 11, &["gather"]).is_ok_and(|p| p.len() == 2));
        assert!(capture("elsewhere", 11, &[]).is_err());
    }

    #[test]
    fn only_tile_lines_count_as_a_map() {
        assert!(is_map_row("..@#..D"));
        assert!(!is_map_row("COURIER  turn 0"));
        assert!(!is_map_row(""));
    }

    #[test]
    fn every_tile_has_its_own_colour_and_words_are_text() {
        let tiles = "@pD=!#.";
        for (index, tile) in tiles.chars().enumerate() {
            assert_ne!(tile_color(tile), TEXT, "{tile}");
            for other in tiles.chars().skip(index + 1) {
                assert_ne!(tile_color(tile), tile_color(other), "{tile} {other}");
            }
        }
        assert_eq!(tile_color('W'), TEXT);
    }
}
