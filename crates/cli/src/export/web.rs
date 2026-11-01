//! The web exporter: `shuyi web`'s build ([`crate::web`]), with the features
//! of `shuyi.ron`'s `export` part, copied into `<base>/` in the export's
//! folder with what that part ships beside the program, and that folder
//! zipped with `index.html` at the zip's root, as itch.io and other
//! portals take it. It needs the game's `web` part, as `shuyi web` does.

use std::fs;

use super::{ExportCx, ExportError, ExportReport, Exporter, Signing, archive, fresh, paths};
use crate::project::Web;
use crate::web::{self as build, Options};

/// The web exporter.
pub static WEB: WebExporter = WebExporter;

/// The web exporter's type.
pub struct WebExporter;

impl WebExporter {
    /// The web build's options: release or not, and the `export` part's
    /// features.
    fn options(cx: &ExportCx) -> Options {
        Options {
            release: cx.release,
            features: cx.turned_on.clone(),
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
    let beside = cx.copy_beside(&|_| folder.clone())?;
    let entries = archive::entries(&folder, "", &|_| false).map_err(ExportError::Build)?;
    archive::zip(&cx.out.join(&zip), &entries).map_err(ExportError::Build)?;
    let wasm = format!("{}.wasm", rules.bin);
    let sizes = vec![cx.size(&cx.out.join(&zip))?, cx.size(&folder.join(&wasm))?];
    let mut report = cx.report(&zip, sizes, Signing::None, None);
    report.beside = beside;
    Ok(report)
}
