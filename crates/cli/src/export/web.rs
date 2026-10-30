//! The web exporter: `shuyi web`'s build ([`crate::web`]), with no
//! feature, copied into `<base>/` in the export's folder, and that folder
//! zipped with `index.html` at the zip's root, as itch.io and other
//! portals take it. It needs the game's `web` part, as `shuyi web` does.

use super::{ExportCx, ExportError, ExportReport, Exporter, Signing, archive, fresh, paths};
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
        build::run(rules, &cx.root, &Self::options(cx), &cx.game).map_err(|e| {
            if e.starts_with(paths::FOUND) {
                ExportError::BuildPath(e)
            } else {
                ExportError::Build(e)
            }
        })?;
        pack(
            cx,
            &cx.root.join(&rules.dist),
            &format!("{}.wasm", rules.bin),
        )
    }
}

/// Copies the web build in `dist` into `<base>/` and zips it, `wasm`
/// being its binary.
pub fn pack(
    cx: &ExportCx,
    dist: &std::path::Path,
    wasm: &str,
) -> Result<ExportReport, ExportError> {
    let base = cx.base();
    let folder = cx.out.join(&base);
    fresh(&folder)?;
    build::copy_tree(dist, &folder, &|_| true).map_err(ExportError::Build)?;
    let entries = archive::entries(&folder, "", &|_| false).map_err(ExportError::Build)?;
    let zip = format!("{base}.zip");
    archive::zip(&cx.out.join(&zip), &entries).map_err(ExportError::Build)?;
    let sizes = vec![cx.size(&cx.out.join(&zip))?, cx.size(&folder.join(wasm))?];
    Ok(cx.report(&zip, sizes, Signing::None, None))
}
