//! The web exporter: `shuyi web`'s build ([`crate::web`]), with the features
//! of `shuyi.ron`'s `export` part, copied into `<base>/` in the export's
//! folder with what that part ships beside the program, and that folder
//! zipped with `index.html` at the zip's root, as itch.io and other
//! portals take it. It needs the game's `web` part, as `shuyi web` does.

use std::fs;

use super::disclosure::{FileKind, Shipped};
use super::{
    ExportCx, ExportError, ExportReport, Exporter, Host, InCi, Signing, archive, fresh, paths,
};
use crate::game::GameTools;
use crate::project::{Project, WasmOptRule, Web};
use crate::web::{self as build, Options};

/// The web exporter.
pub static WEB: WebExporter = WebExporter;

/// The web exporter's type.
pub struct WebExporter;

impl WebExporter {
    /// The web build's options: release or not, the `export` part's
    /// features (or the edition's) with `packed` for a packed build, whether
    /// `wasm-opt` must run (unless the part says `wasm_opt: optional`), the
    /// edition's default features and content root, and the pack the bake
    /// wrote for a packed build.
    fn options(cx: &ExportCx) -> Options {
        Options {
            release: cx.release,
            features: cx.features_on(),
            require_wasm_opt: cx.wasm_opt == WasmOptRule::Required,
            no_default_features: !cx.default_features(),
            content: cx.content(),
            pack: cx.packs().then(|| cx.baked_pack()),
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

    fn host(&self) -> Host {
        Host::Linux
    }

    fn container(&self) -> Option<&'static str> {
        None
    }

    fn ci(&self) -> InCi {
        InCi::Build
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

    /// Bakes the pack a packed build reads, and proves it equal to its
    /// sources ([`ExportCx::bake_pack`]).
    fn prepare(
        &self,
        cx: &ExportCx,
        project: &Project,
        game: Option<&dyn GameTools>,
    ) -> Result<(), ExportError> {
        cx.bake_pack(project, game)
    }

    fn export(&self, cx: &ExportCx) -> Result<ExportReport, ExportError> {
        let rules = cx.web.as_ref().ok_or_else(|| {
            ExportError::Setup(format!("{} has no `web` part", crate::project::FILE))
        })?;
        super::installed(build::TARGET)?;
        for args in self.cargo_args(cx) {
            super::no_debug_features(&args)?;
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
            } else if e.starts_with(build::NO_WASM_OPT) {
                ExportError::MissingTool(e)
            } else {
                ExportError::Build(e)
            }
        })?;
        pack(cx, rules, &dist)
    }

    /// A release build's, if the binary it shipped holds no `name` section.
    fn strips_symbols(&self, cx: &ExportCx) -> bool {
        let Some(rules) = cx.web.as_ref().filter(|_| cx.release) else {
            return false;
        };
        let wasm = cx.out.join(cx.base()).join(format!("{}.wasm", rules.bin));
        fs::read(wasm)
            .ok()
            .and_then(|bytes| build::custom_sections(&bytes).ok())
            .is_some_and(|names| !names.iter().any(|name| name == "name"))
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
        cx.mark_beside(report, &mut shipped);
        cx.mark_pack(report, &mut shipped);
        shipped
    }
}

/// Copies the web build in `dist` into `<base>/` and zips it, with the pack
/// beside the page for a packed build. A file there that this build doesn't
/// make is refused by its path, with no package left behind.
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
    let mut current = build::current_files(rules, &cx.root).map_err(ExportError::Build)?;
    let pack = cx.packs().then(|| build::pack_file(rules));
    current.extend(pack.clone());
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
    let pack = match pack {
        Some(name) => {
            let file = folder.join(name);
            if !file.is_file() {
                return Err(ExportError::Build(format!(
                    "the web build has no {}, which its packed program reads",
                    build::pack_file(rules)
                )));
            }
            cx.search_pack(&file)?;
            let _ = fs::remove_file(cx.baked_pack());
            Some(cx.size(&file)?)
        }
        None => None,
    };
    let beside = cx.copy_beside(&|_| folder.clone())?;
    let entries = archive::entries(&folder, "", &|_| false).map_err(ExportError::Build)?;
    archive::zip(&cx.out.join(&zip), &entries).map_err(ExportError::Build)?;
    let wasm = format!("{}.wasm", rules.bin);
    let sizes = vec![cx.size(&cx.out.join(&zip))?, cx.size(&folder.join(&wasm))?];
    let mut report = cx.report(&zip, sizes, Signing::None, None);
    report.beside = beside;
    report.pack = pack;
    Ok(report)
}
