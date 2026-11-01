//! The game's palette reloading as its file changes: a small module of the
//! game's own, which keeps a [`Hot`] value as its state, refreshes it when
//! files change and hands the screens the new colours on the context. With
//! `cargo shuyi run --watch`, a colour saved in `files/palette.ron` shows
//! within two seconds, and a broken file shows its error over the frame
//! while the old colours stay.

use std::sync::Arc;

use shuyi::assets::ContentError;
use shuyi::assets::bundle::Bundle;
use shuyi::assets::hot::{Hot, Reload};
use shuyi::assets::palette::PaletteDef;
use shuyi::ui::Ctx;
use shuyi::ui::frame::{EngineColor, Palette};
use shuyi::ui::module::{Hook, Install, Module};

use crate::prelude::*;
use crate::{Hello, formats};

/// The module.
#[derive(Debug)]
pub struct HotPalette {
    palette: Hot<PaletteDef>,
}

impl HotPalette {
    /// The palette of `bundle`, or every problem with it.
    pub fn new(bundle: &Bundle) -> Result<Self, Vec<ContentError>> {
        let palette = Hot::load(&Arc::new(formats()), bundle, "palette.ron")?;
        Ok(Self { palette })
    }
}

impl Module<Hello> for HotPalette {
    fn name(&self) -> &'static str {
        "hot_palette"
    }

    fn install(&mut self, install: &mut Install<'_, Hello>) {
        install.hook(Hook::Reloaded);
    }

    fn reloaded(&mut self, ctx: &mut Ctx<Hello>, changed: &[String]) {
        let reload = self.palette.refresh(changed);
        // The game's format checks that it has every colour.
        if reload == Reload::Reloaded
            && let Ok(palette) = Palette::new::<EngineColor>(&self.palette)
        {
            ctx.palette = palette;
        }
        ctx.note_reload(self.palette.id(), &reload);
    }
}
