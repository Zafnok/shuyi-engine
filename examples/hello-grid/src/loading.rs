//! The game's data read a file a step, so its loading card counts the files
//! (ADR-0128 §4): [`DataLoad`] reads them, and [`in_steps`] makes a
//! platform's load in steps of it (`Start::in_steps`). Where the platform
//! gives a slice (the Playdate), a frame runs as many files as fit and the
//! card shows how many are read; elsewhere they all load in one frame. Where
//! the files arrive in time (the web, which streams the pack), the load
//! first waits for what its first frame reads ([`FIRST`]), and the card
//! counts it as it arrives.

use alloc::collections::BTreeMap;
use core::borrow::Borrow;

use crate::prelude::*;

use shuyi::assets::ContentError;
use shuyi::assets::bundle::Bundle;
use shuyi::assets::dialogue::{DialogueFile, DialogueTable};
use shuyi::assets::font::FontAtlasDef;
use shuyi::assets::format::Formats;
use shuyi::assets::grid_map::GridMap;
use shuyi::assets::image::ImageTable;
use shuyi::assets::keymap::KeymapDef;
use shuyi::assets::lang::Lang;
use shuyi::assets::lighting::Lighting;
use shuyi::assets::palette::PaletteDef;
use shuyi::core::TerrainId;
use shuyi::platform::Progress;
use shuyi::ui::Game;
use shuyi::ui::text::check_key_placeholders;

use crate::rules::{self, Feature};
use crate::{
    Action, CONSOLE, DIALOGUE, English, FIRST, FONT_PNG, FONT_RON, Hello, dialogue_text, gather,
    sound,
};

/// The game's data as it is read, a file a [`step`](Self::step): the
/// text, the keys, the palette, the scene, the stage, the font atlas, then
/// the pictures. A file that doesn't load adds its problems, and the others
/// are read all the same, so [`finish`](Self::finish) says every problem.
#[derive(Default)]
pub struct DataLoad {
    english: Option<English>,
    keymap: Option<KeymapDef<Action>>,
    palette: Option<PaletteDef>,
    dialogue: Option<DialogueTable>,
    stage_map: Option<GridMap<TerrainId, Feature>>,
    font: Option<FontAtlasDef>,
    images: Option<ImageTable>,
    errors: Vec<ContentError>,
    /// The files read so far.
    read: u32,
}

impl DataLoad {
    /// The files the game's data is read from: the units of its load.
    pub const FILES: u32 = 7;

    /// The data of every file in `bundle`, read with `formats` at once.
    ///
    /// # Errors
    ///
    /// Every problem found ([`finish`](Self::finish)).
    pub fn all(bundle: &Bundle, formats: &Formats) -> Result<Hello, Vec<ContentError>> {
        let mut data = Self::default();
        for _ in 0..Self::FILES {
            data.step(bundle, formats);
        }
        data.finish()
    }

    /// Reads the next file from `bundle` with `formats`, if one is left:
    /// the files read so far.
    pub fn step(&mut self, bundle: &Bundle, formats: &Formats) -> u32 {
        let errors = &mut self.errors;
        match self.read {
            0 => {
                self.english = gather(errors, || {
                    let english = formats.load::<English>(bundle, "en.ron")?;
                    check_key_placeholders::<Action>(bundle, "en.ron", &english.0)?;
                    Ok(english)
                });
            }
            1 => {
                self.keymap = gather(errors, || {
                    formats.load::<KeymapDef<Action>>(bundle, "keymap.ron")
                });
            }
            2 => {
                self.palette = gather(errors, || formats.load::<PaletteDef>(bundle, "palette.ron"));
            }
            3 => self.dialogue = gather(errors, || dialogue(bundle, formats)),
            4 => {
                self.stage_map = gather(errors, || {
                    let map = formats.load::<GridMap<TerrainId, Feature>>(bundle, "stage.map")?;
                    rules::Stage::new(&map, 0).map_err(|e| {
                        vec![ContentError::new(bundle.display_path("stage.map"), e)]
                    })?;
                    Ok(map)
                });
            }
            5 => {
                self.font = gather(errors, || formats.load::<FontAtlasDef>(bundle, FONT_RON));
            }
            6 => {
                self.images = gather(errors, || {
                    ImageTable::load_with(bundle, formats, &[FONT_PNG])
                });
            }
            _ => return self.read,
        }
        self.read += 1;
        self.read
    }

    /// The game's data, once every file is read, or every problem found.
    ///
    /// # Errors
    ///
    /// Each file's problems; before every file is read, those so far and how
    /// many files aren't read yet.
    pub fn finish(self) -> Result<Hello, Vec<ContentError>> {
        let Self {
            english,
            keymap,
            palette,
            dialogue,
            stage_map,
            font,
            images,
            mut errors,
            read,
        } = self;
        if read < Self::FILES {
            let left = format!("{} of its files not read", Self::FILES - read);
            errors.push(ContentError::new("the game's data", left));
            return Err(errors);
        }
        match (english, keymap, palette, dialogue, stage_map, font, images) {
            (
                Some(english),
                Some(keymap),
                Some(palette),
                Some(dialogue),
                Some(stage_map),
                Some(font),
                Some(images),
            ) => Ok(Hello {
                console: CONSOLE,
                lang: Lang::with_source(english.0, dialogue_text(&dialogue), BTreeMap::new()),
                keymap,
                audio: sound::manifest(),
                palette,
                dialogue,
                stage_map,
                font,
                images,
                lights: Lighting::default(),
                seed: 0,
                one_bit: None,
            }),
            _ => Err(errors),
        }
    }
}

/// The scene's table, from [`DIALOGUE`].
fn dialogue(bundle: &Bundle, formats: &Formats) -> Result<DialogueTable, Vec<ContentError>> {
    let file = formats.load::<DialogueFile>(bundle, DIALOGUE)?;
    let scenes = file
        .scenes
        .into_iter()
        .map(|p| (p.scene.id.clone(), p.scene));
    let mut table = DialogueTable::new(scenes.collect());
    table.files = table
        .scenes
        .keys()
        .map(|id| (id.clone(), DIALOGUE.to_owned()))
        .collect();
    Ok(table)
}

/// The steps of the game's load (`Start::in_steps`): the first `open`s the
/// files its data is read from (or says why it can't); until each of
/// [`FIRST`] has arrived (`Bundle::arrived`), a step gives the card how
/// many have and the next waits for the platform ([`Progress::Wait`]), a
/// frame each; then a step reads each file ([`DataLoad::step`]). Each of
/// [`FIRST`] arrived and each file read is a unit of the card's count
/// ([`Progress::Of`]), and the last step `finish`es the game from the data,
/// the files and their formats. A step after that says the load ran past
/// its end. Should the files stop arriving (the page's fetch broke), each
/// counts as arrived, and the load reads the missing one and says so.
pub fn in_steps<B: Borrow<Bundle>>(
    open: impl FnOnce() -> Result<(B, Formats), String>,
    finish: impl FnOnce(Result<Hello, Vec<ContentError>>, &B, &Formats) -> Result<Game<Hello>, String>,
) -> impl FnMut() -> Progress<Hello> {
    let mut open = Some(open);
    let mut finish = Some(finish);
    let mut files = None;
    let mut data = DataLoad::default();
    let first = u32::try_from(FIRST.len()).unwrap_or(u32::MAX);
    let of = first + DataLoad::FILES;
    // One step a file, counted here, so the load ends whatever a step says.
    let mut unread = 0..DataLoad::FILES;
    // Whether the last step gave the count of what has arrived, and the
    // next one waits.
    let mut counted = false;
    move || {
        if let Some(open) = open.take() {
            return match open() {
                Ok(opened) => {
                    files = Some(opened);
                    Progress::Of { done: 0, of }
                }
                Err(why) => Progress::done(Err(why)),
            };
        }
        let past = || Progress::done(Err("the game's load ran past its end".to_owned()));
        let Some((bundle, formats)) = &files else {
            return past();
        };
        let arrived = FIRST
            .iter()
            .filter(|first| bundle.borrow().arrived(first))
            .count();
        if arrived < FIRST.len() {
            counted = !counted;
            if !counted {
                return Progress::Wait;
            }
            let done = u32::try_from(arrived).unwrap_or(first);
            return Progress::Of { done, of };
        }
        if unread.next().is_some() {
            let done = first + data.step(bundle.borrow(), formats);
            return Progress::Of { done, of };
        }
        let Some(finish) = finish.take() else {
            return past();
        };
        Progress::done(finish(core::mem::take(&mut data).finish(), bundle, formats))
    }
}
