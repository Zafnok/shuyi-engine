//! The web exporter: `shuyi web`'s build ([`crate::web`]), with no
//! feature, copied into `<base>/` in the export's folder, and that folder
//! zipped with `index.html` at the zip's root, as itch.io and other
//! portals take it. It needs the game's `web` part, as `shuyi web` does.

use std::fs;

use super::disclosure::{FileKind, Shipped};
use super::{ExportCx, ExportError, ExportReport, Exporter, Signing, archive, fresh, paths};
use crate::project::Web;
use crate::web::{self as build, Options};

/// The web exporter.
pub static WEB: WebExporter = WebExporter;

/// The web exporter's type.
pub struct WebExporter;

impl WebExporter {
    /// The web build's options: release or not, and never a feature.
    fn options(cx: &ExportCx) -> Options {
        Options {
            release: cx.release,
            features: Vec::new(),
        }
    }
}

impl Exporter for WebExporter {
    fn id(&self) -> &'static str {
        "web"
    }

    fn profile(&self) -> &'static str {
        "web"
    }

    fn triples(&self) -> &'static [&'static str] {
        &[build::TARGET]
    }

    fn cargo_args(&self, cx: &ExportCx) -> Vec<Vec<String>> {
        cx.web
            .iter()
            .map(|rules| build::cargo_args(rules, &Self::options(cx)))
            .collect()
    }

    fn export(&self, cx: &ExportCx) -> Result<ExportReport, ExportError> {
        let rules = cx.web.as_ref().ok_or_else(|| {
            ExportError::Setup(format!("{} has no `web` part", crate::project::FILE))
        })?;
        super::installed(build::TARGET)?;
        for args in self.cargo_args(cx) {
            super::no_live_link(&args)?;
        }
        let dist = cx.root.join(&rules.dist);
        if build::is_under(&cx.out, &dist) {
            return Err(ExportError::Setup(format!(
                "the export's folder {} is in `web.dist`, `{}`, which each web build replaces: \
                 give the web build a folder of its own, such as `dist/web`",
                cx.shown, rules.dist
            )));
        }
        build::run(rules, &cx.root, &Self::options(cx), &cx.game).map_err(|e| {
            if e.starts_with(paths::FOUND) {
                ExportError::BuildPath(e)
            } else {
                ExportError::Build(e)
            }
        })?;
        pack(cx, rules, &dist)
    }

    /// The shell, the binary and the notices, which it writes; the music and
    /// voice files, which it copies; and `web.dist`, which holds them too.
    fn shipped(&self, cx: &ExportCx, report: &ExportReport) -> Shipped {
        let mut shipped = Shipped::new(
            cx.out.join(&report.folder),
            Some(cx.out.join(&report.package)),
            "",
        );
        let Some(rules) = &cx.web else {
            return shipped;
        };
        shipped.public.push(cx.root.join(&rules.dist));
        let game = build::game_files(rules, &cx.root).unwrap_or_default();
        for file in build::current_files(rules, &cx.root).unwrap_or_default() {
            if game.contains(&file) {
                let music = rules.music.as_deref().is_some_and(|music| {
                    file.starts_with(&format!("{}/", build::package_path(music)))
                });
                let part = if music { "music" } else { "voice" };
                let source = format!("web.{part}: {file}");
                shipped.source(file, FileKind::LooseAsset, source);
            } else if file == format!("{}.wasm", rules.bin) {
                shipped.source(file, FileKind::Program, "the built program");
            } else {
                shipped.source(file, FileKind::Written, "written by the export");
            }
        }
        shipped
    }
}

/// Copies the web build in `dist` into `<base>/` and zips it. A file there
/// that this build doesn't make is refused by its path, with no package
/// left behind.
pub fn pack(
    cx: &ExportCx,
    rules: &Web,
    dist: &std::path::Path,
) -> Result<ExportReport, ExportError> {
    let base = cx.base();
    let folder = cx.out.join(&base);
    let zip = format!("{base}.zip");
    fresh(&folder)?;
    build::copy_tree(dist, &folder, &|_| true).map_err(ExportError::Build)?;
    let entries = archive::entries(&folder, "", &|_| false).map_err(ExportError::Build)?;
    let current = build::current_files(rules, &cx.root).map_err(ExportError::Build)?;
    let unexpected: Vec<&str> = entries
        .iter()
        .map(|entry| entry.name.as_str())
        .filter(|name| !current.contains(*name))
        .collect();
    if !unexpected.is_empty() {
        let refused = ExportError::Build(format!(
            "the web export holds {}, which this build didn't make: remove it, or build again",
            unexpected.join(", ")
        ));
        let _ = fs::remove_dir_all(&folder);
        let _ = fs::remove_file(cx.out.join(&zip));
        return Err(refused);
    }
    archive::zip(&cx.out.join(&zip), &entries).map_err(ExportError::Build)?;
    let wasm = format!("{}.wasm", rules.bin);
    let sizes = vec![cx.size(&cx.out.join(&zip))?, cx.size(&folder.join(&wasm))?];
    Ok(cx.report(&zip, sizes, Signing::None, None))
}
