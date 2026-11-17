//! What stays in memory when a game holds its pack, measured (ticket 2906,
//! `docs/research/asset-memory.md`).
//!
//! Each pack is held in each of the ways a game can hold one, under the
//! heap counter (`shuyi-alloc-count`):
//!
//! 1. **embedded:** the pack is in the program (as `include_bytes!` puts
//!    it), opened and laid in a bundle ([`Pack::into_layer`]): only the
//!    files it unpacks take heap;
//! 2. **read whole, copied:** the pack read into the heap, every file
//!    copied out ([`Pack::into_owned`]) and the read dropped;
//! 3. **read whole, kept:** the pack read into the heap and kept, its
//!    stored files borrowing it, as `shuyi_app::pack_beside_program` and
//!    the Playdate's `read_pack` hold it today;
//! 4. **on first read:** each file read from the pack when the load asks
//!    for it, and dropped once decoded. Nothing in the engine reads so, so
//!    it is computed from the parts measured above, its arithmetic printed.
//!
//! For each: the most heap at once while the pack is opened and the game
//! loaded from it (the peak), and what stays after (resident), split into
//! the files' bytes by class, the pack's table and paths, and the decoded
//! values the load keeps. A pack's own bytes count as the heap's only
//! where the way reads it into the heap: an embedded one is the program's.
//!
//! The packs: Hello, Grid's, baked as `cargo shuyi bake` bakes it; with
//! `--folder <dir>`, a folder of a game's files, every one in as its own
//! bytes and loaded with the engine's own kinds (each picture's size, each
//! sound decoded whole, as the backend decodes a sound at start); and
//! Hello, Grid's files with synthetic files added up to the `web` and the
//! `playdate` profiles' `package_bytes`, in the proportions of the game's
//! public assets. `--write-synthetic <target> <file>` writes the
//! synthetic pack of a target's budget to a file, for a run in a browser.
//!
//! Run it with `cargo run -p hello-grid --example asset_memory --release`,
//! and `-- --folder <dir>` for a folder. A release build counts with a
//! counter of its own; a debug build reads the global one's memory tags
//! (ADR-0046 §9).

use std::any::Any;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

use hello_grid::PACK_STAMP;
use hello_grid::tools::HelloTools;
use shuyi::assets::bundle::{Bundle, Layer};
use shuyi::assets::format::Formats;
use shuyi::assets::image::ImageTable;
use shuyi::assets::pack::{Content, Pack, PackFile, PackInfo, write};
use shuyi::assets::target::TargetProfile;
use shuyi::core::{ContentHash, SimRng};
use shuyi_alloc_count::{global, tags};
use shuyi_cli::GameTools;

/// A release build's counter: there the global one is compiled out
/// (`global::COUNTING`), and a binary has one global allocator.
#[cfg(not(debug_assertions))]
#[global_allocator]
static HEAP: shuyi_alloc_count::CountingAlloc<std::alloc::System> =
    shuyi_alloc_count::CountingAlloc::new(std::alloc::System);

/// The class of a game's span, which a measurement's tag is: as
/// `shuyi-app`'s spans are counted.
const SPAN_CLASS: u8 = 2;

/// The stamp of a pack made of a folder.
const FOLDER_STAMP: &str = "asset-memory";

/// The seed of the synthetic files.
const SEED: u64 = 2906;

/// The public assets of Visions of Shuyi at its commit `38553774`
/// (2026-11-16), by class: the mean bytes of a file, and the class's share
/// of all their bytes in thousandths. Only these numbers are kept here,
/// never a file of the game.
const PROPORTIONS: [(Class, u64, u64); 3] = [
    (Class::Png, 706, 28),
    (Class::Sound, 20_110, 499),
    (Class::Other, 6_458, 473),
];

/// The targets whose budgets the synthetic packs fill, with their
/// profiles.
const TARGETS: [(&str, &str); 2] = [
    ("web", include_str!("../../../targets/web.ron")),
    ("playdate", include_str!("../../../targets/playdate.ron")),
];

/// What one step took of the heap, in bytes asked for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Usage {
    /// The most bytes it had live at once.
    peak: u64,
    /// The bytes it left live.
    kept: u64,
}

/// How many steps have been measured: each gets a tag of its own.
static STEPS: AtomicU32 = AtomicU32::new(0);

/// Runs `step` under the heap counter: its value, and what it took, or
/// `None` in a build that counts nothing (a debug build with `--cfg
/// shuyi_no_heap_count`).
fn measure<T>(step: impl FnOnce() -> T) -> (T, Option<Usage>) {
    if !global::COUNTING {
        return own_counter(step);
    }
    let number = STEPS.fetch_add(1, Ordering::Relaxed);
    let name: &'static str = format!("asset_memory step {number}").leak();
    let tag = tags::tag(SPAN_CLASS, name);
    let before = tags::enter(tag);
    let value = step();
    tags::leave(before);
    let row = global::rows().and_then(|rows| rows.into_iter().find(|row| row.tag == tag));
    let usage = row.map(|row| Usage {
        peak: row.peak_bytes,
        kept: row.live_bytes,
    });
    (value, usage)
}

/// [`measure`] on this program's own counter, in a release build.
#[cfg(not(debug_assertions))]
fn own_counter<T>(step: impl FnOnce() -> T) -> (T, Option<Usage>) {
    let wide = |bytes: usize| u64::try_from(bytes).unwrap_or(u64::MAX);
    let start = HEAP.live_bytes();
    HEAP.reset_peak();
    let value = step();
    let usage = Usage {
        peak: wide(HEAP.peak_bytes().saturating_sub(start)),
        kept: wide(HEAP.live_bytes().saturating_sub(start)),
    };
    (value, Some(usage))
}

/// A debug build without the global counter counts nothing.
#[cfg(debug_assertions)]
fn own_counter<T>(step: impl FnOnce() -> T) -> (T, Option<Usage>) {
    (step(), None)
}

/// Whether `path` ends with extension `extension`, in any case.
fn has_extension(path: &str, extension: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|found| found.eq_ignore_ascii_case(extension))
}

/// What a file of a pack is, for the split of the bytes kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    /// A baked file: assets in their binary form.
    Baked,
    /// A picture.
    Png,
    /// A sound.
    Sound,
    /// Anything else, kept as its own bytes.
    Other,
}

impl Class {
    /// Every class, in the order they are printed.
    const ALL: [Self; 4] = [Self::Baked, Self::Png, Self::Sound, Self::Other];

    /// The class of `file`: a synthetic file's by its folder.
    fn of(file: &PackFile<'_>) -> Self {
        let path = file.path.as_str();
        if !file.kinds.is_empty() {
            Self::Baked
        } else if has_extension(path, "png") || path.starts_with("synthetic/png/") {
            Self::Png
        } else if ["ogg", "wav"].iter().any(|end| has_extension(path, end))
            || path.starts_with("synthetic/sound/")
        {
            Self::Sound
        } else {
            Self::Other
        }
    }

    /// Its name in the tables.
    fn name(self) -> &'static str {
        match self {
            Self::Baked => "baked data",
            Self::Png => "PNG files",
            Self::Sound => "sounds",
            Self::Other => "other files",
        }
    }

    /// Its place in [`Class::ALL`].
    fn at(self) -> usize {
        self as usize
    }

    /// The folder a synthetic file of the class is in.
    fn synthetic_folder(self) -> &'static str {
        match self {
            Self::Baked => "synthetic/baked",
            Self::Png => "synthetic/png",
            Self::Sound => "synthetic/sound",
            Self::Other => "synthetic/data",
        }
    }
}

/// The bytes of one class of a pack's files.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ClassBytes {
    /// How many files.
    files: u64,
    /// The bytes the pack keeps of them.
    stored: u64,
    /// Their bytes unpacked.
    unpacked: u64,
    /// The unpacked bytes of those the pack compressed: opening the pack
    /// unpacks them into memory of their own.
    compressed: u64,
}

/// A pack, as its files are.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Shape {
    /// The pack's bytes.
    length: u64,
    /// Its header and index: the bytes before its files'.
    index: u64,
    /// Its files by class ([`Class::ALL`]).
    classes: [ClassBytes; 4],
    /// The file with the most bytes stored and unpacked: its path, its
    /// stored bytes and its unpacked ones.
    largest: (String, u64, u64),
}

impl Shape {
    /// The shape of `pack`, opened from its `length` bytes.
    fn of(pack: &Pack<'_>, length: u64) -> Self {
        let mut shape = Self {
            length,
            ..Self::default()
        };
        for file in pack.files() {
            let stored = wide(file.stored);
            let unpacked = wide(file.bytes.len());
            let class = &mut shape.classes[Class::of(file).at()];
            class.files += 1;
            class.stored += stored;
            class.unpacked += unpacked;
            if matches!(file.bytes, Cow::Owned(_)) {
                class.compressed += unpacked;
            }
            if stored + unpacked > shape.largest.1 + shape.largest.2 {
                shape.largest = (file.path.clone(), stored, unpacked);
            }
        }
        let stored: u64 = shape.classes.iter().map(|class| class.stored).sum();
        shape.index = length.saturating_sub(stored);
        shape
    }
}

/// One way of holding a pack, measured.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Way {
    /// Its name.
    name: &'static str,
    /// Getting the pack's layer: reading it, if the way does, and opening
    /// it.
    open: Usage,
    /// Loading the game from the layer.
    load: Usage,
    /// The files' bytes the heap holds after, by class.
    files: [u64; 4],
    /// The pack's bytes in the program, not the heap.
    in_program: u64,
}

impl Way {
    /// The most heap at once while the pack was opened and the game loaded.
    fn peak(&self) -> u64 {
        self.open.peak.max(self.open.kept + self.load.peak)
    }

    /// The heap that stays after.
    fn kept(&self) -> u64 {
        self.open.kept + self.load.kept
    }

    /// The files' bytes the heap holds after.
    fn file_bytes(&self) -> u64 {
        self.files.iter().sum()
    }

    /// The rest of the open's heap: the pack's table and paths (and its
    /// index where the read is kept).
    fn table(&self) -> u64 {
        self.open.kept.saturating_sub(self.file_bytes())
    }
}

/// Reading each file when the load asks for it, computed from the
/// embedded way's parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FirstRead {
    /// The pack's table and paths, kept to find each file.
    table: u64,
    /// The largest file's bytes stored, read while it is decoded.
    largest_stored: u64,
    /// Its bytes unpacked.
    largest_unpacked: u64,
    /// The load's own peak, with the decoded values it builds.
    load_peak: u64,
    /// The decoded values the load keeps.
    decoded: u64,
}

impl FirstRead {
    /// The most heap at once: the table, the largest file read and
    /// unpacked, on top of the load's own peak.
    fn peak(&self) -> u64 {
        self.table + self.largest_stored + self.largest_unpacked + self.load_peak
    }

    /// The heap that stays: the table and the decoded values.
    fn kept(&self) -> u64 {
        self.table + self.decoded
    }
}

/// Everything measured of one pack.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Report {
    shape: Shape,
    ways: Vec<Way>,
    first_read: FirstRead,
}

/// A load of a game from a bundle: what it keeps, held until measured.
type Loader = fn(&Bundle) -> Result<Box<dyn Any>, String>;

/// Every problem in `errors`, one a line.
fn lines(errors: &[impl ToString]) -> String {
    let lines: Vec<String> = errors.iter().map(ToString::to_string).collect();
    lines.join("\n")
}

/// `bytes` as a `u64`.
fn wide(bytes: usize) -> u64 {
    u64::try_from(bytes).unwrap_or(u64::MAX)
}

/// Hello, Grid's load: its data, as `Hello::from_pack` reads it.
fn hello_load(bundle: &Bundle) -> Result<Box<dyn Any>, String> {
    let hello = hello_grid::Hello::load_from(bundle, &hello_grid::formats())
        .map_err(|errors| lines(&errors))?;
    Ok(Box::new(hello))
}

/// A load of the engine's own kinds from a folder of a game's files: each
/// picture's size (the image table), and each sound decoded whole, as the
/// backend decodes a sound at start.
fn engine_load(bundle: &Bundle) -> Result<Box<dyn Any>, String> {
    let images = ImageTable::load_with(bundle, &Formats::standard(), &[]).map_err(|e| lines(&e))?;
    let limits = shuyi_sound::Limits::default();
    let mut sounds = Vec::new();
    for path in bundle.files_under("") {
        let bytes = bundle.bytes(path).unwrap_or_default();
        let pcm = if has_extension(path, "wav") {
            let mut source = shuyi_sound::wav::WavSource::new(bytes, &limits)
                .map_err(|e| format!("{path}: {e}"))?;
            shuyi_sound::read_all(&mut source, &limits)
        } else if has_extension(path, "ogg") {
            let mut source = shuyi_sound::vorbis::VorbisSource::new(bytes, &limits)
                .map_err(|e| format!("{path}: {e}"))?;
            shuyi_sound::read_all(&mut source, &limits)
        } else {
            continue;
        };
        sounds.push(pcm.map_err(|e| format!("{path}: {e}"))?);
    }
    Ok(Box::new((images, sounds)))
}

/// The heap a counted step took, or why there is none.
fn counted(usage: Option<Usage>) -> Result<Usage, String> {
    usage.ok_or_else(|| "this build counts no heap".to_owned())
}

/// Holds the pack `bytes` (kept for the run, as a program keeps what it
/// embeds) in every way, loading the game from it with `load`: the
/// report, or `None` in a build that counts no heap.
fn measure_pack(bytes: &'static [u8], stamp: &str, load: Loader) -> Result<Option<Report>, String> {
    if !global::COUNTING && cfg!(debug_assertions) {
        return Ok(None);
    }
    let pack = Pack::open(bytes, stamp).map_err(|e| e.to_string())?;
    let shape = Shape::of(&pack, wide(bytes.len()));
    drop(pack);
    let by_class = |part: fn(&ClassBytes) -> u64| shape.classes.map(|class| part(&class));
    let ways = [
        (
            "embedded",
            by_class(|class| class.compressed),
            wide(bytes.len()),
        ),
        ("read whole, copied", by_class(|class| class.unpacked), 0),
        (
            "read whole, kept",
            by_class(|class| class.stored + class.compressed),
            0,
        ),
    ];
    let mut measured = Vec::new();
    for (at, (name, files, in_program)) in ways.into_iter().enumerate() {
        let (layer, open) = measure(|| -> Result<Layer, String> {
            let pack = match at {
                0 => Pack::open(bytes, stamp).map_err(|e| e.to_string())?,
                1 => {
                    let read = bytes.to_vec();
                    let pack = Pack::open(&read, stamp).map_err(|e| e.to_string())?;
                    pack.into_owned()
                }
                _ => {
                    let read: &'static [u8] = bytes.to_vec().leak();
                    Pack::open(read, stamp).map_err(|e| e.to_string())?
                }
            };
            Ok(pack.into_layer("pack"))
        });
        let layer = layer?;
        let (kept, load) = measure(|| load(&Bundle::new(vec![layer])));
        let kept = kept?;
        measured.push(Way {
            name,
            open: counted(open)?,
            load: counted(load)?,
            files,
            in_program,
        });
        drop(kept);
    }
    let embedded = &measured[0];
    let first_read = FirstRead {
        table: embedded.table(),
        largest_stored: shape.largest.1,
        largest_unpacked: shape.largest.2,
        load_peak: embedded.load.peak,
        decoded: embedded.load.kept,
    };
    Ok(Some(Report {
        shape,
        ways: measured,
        first_read,
    }))
}

/// Hello, Grid's pack, as `cargo shuyi bake` bakes it, with `extra` files
/// laid over its own.
fn hello_pack(extra: BTreeMap<String, Vec<u8>>) -> Result<Vec<u8>, String> {
    let game = HelloTools.bake(None)?;
    let mut layers = vec![hello_grid::layer()];
    if !extra.is_empty() {
        let over = Layer::owned("synthetic".to_owned(), extra).map_err(|e| e.to_string())?;
        layers.insert(0, over);
    }
    let bundle = Bundle::new(layers);
    let baked = game
        .bake
        .bake(&game.formats, &bundle, None)
        .map_err(|errors| lines(&errors))?;
    Ok(baked.bytes)
}

/// Synthetic files of `sources` bytes in all, in the proportions of the
/// game's public assets ([`PROPORTIONS`]), the same for the same `seed`:
/// pictures and sounds of bytes that don't compress, and data that
/// compresses about as the game's does (to about half): runs of random
/// letters between runs of repeated text.
fn synthetic_files(sources: u64, seed: u64) -> BTreeMap<String, Vec<u8>> {
    /// The text repeated between a data file's random letters.
    const LINE: &[u8] = b"(name: \"a row\", count: 12, kind: Plain),
";
    let mut rng = SimRng::new(seed);
    let mut files = BTreeMap::new();
    for (class, mean, share) in PROPORTIONS {
        let mut left = sources * share / 1000;
        let mut number = 0;
        while left > 0 {
            let size = left.min(mean);
            left -= size;
            let size = usize::try_from(size).unwrap_or(usize::MAX);
            let mut bytes = Vec::with_capacity(size + LINE.len());
            while bytes.len() < size {
                if class == Class::Other {
                    let letters = (0..LINE.len())
                        .map(|_| b'a' + u8::try_from(rng.roll_below(26)).unwrap_or(0));
                    bytes.extend(letters);
                    bytes.extend_from_slice(LINE);
                } else {
                    bytes.extend_from_slice(&rng.next_u32().to_le_bytes());
                }
            }
            bytes.truncate(size);
            let path = format!("{}/{number:05}", class.synthetic_folder());
            files.insert(path, bytes);
            number += 1;
        }
    }
    files
}

/// Hello, Grid's pack with synthetic files ([`synthetic_files`]) added
/// until the pack is about `budget` bytes: made once with `budget` bytes of
/// sources, then again with the sources scaled by how much the first
/// fell short.
fn synthetic_pack(budget: u64, seed: u64) -> Result<Vec<u8>, String> {
    let first = hello_pack(synthetic_files(budget, seed))?;
    let made = wide(first.len()).max(1);
    if made >= budget {
        return Ok(first);
    }
    hello_pack(synthetic_files(budget * budget / made, seed))
}

/// A pack of every file in folder `dir`, each as its own bytes.
fn folder_pack(dir: &Path) -> Result<Vec<u8>, String> {
    let mut files = BTreeMap::new();
    let mut folders = vec![dir.to_path_buf()];
    while let Some(folder) = folders.pop() {
        let entries =
            std::fs::read_dir(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        for entry in entries {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.is_dir() {
                folders.push(path);
                continue;
            }
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let relative = path.strip_prefix(dir).map_err(|e| e.to_string())?;
            let parts: Vec<String> = relative
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect();
            files.insert(parts.join("/"), Content::Raw(bytes));
        }
    }
    let info = PackInfo::new(FOLDER_STAMP, None, ContentHash::default());
    Ok(write(&info, &files))
}

/// The `package_bytes` budget of each target in [`TARGETS`].
fn budgets() -> Result<Vec<(&'static str, u64)>, String> {
    TARGETS
        .iter()
        .map(|&(id, source)| {
            let profile = TargetProfile::from_source(&format!("targets/{id}.ron"), id, source)
                .map_err(|errors| lines(&errors))?;
            let bytes = profile
                .budgets
                .package_bytes
                .ok_or_else(|| format!("targets/{id}.ron has no package_bytes"))?;
            Ok((id, bytes))
        })
        .collect()
}

/// Prints `report` of the pack named `title`, as Markdown tables.
fn print(out: &mut impl Write, title: &str, report: &Report) -> io::Result<()> {
    let shape = &report.shape;
    writeln!(out, "## {title}\n")?;
    writeln!(
        out,
        "The pack: {} bytes, of which {} bytes are its header and index.\n",
        shape.length, shape.index
    )?;
    writeln!(
        out,
        "| Class | Files | Stored bytes | Unpacked bytes | Unpacked from compressed |"
    )?;
    writeln!(
        out,
        "| ----- | ----- | ------------ | -------------- | ------------------------ |"
    )?;
    for class in Class::ALL {
        let bytes = shape.classes[class.at()];
        writeln!(
            out,
            "| {} | {} | {} | {} | {} |",
            class.name(),
            bytes.files,
            bytes.stored,
            bytes.unpacked,
            bytes.compressed
        )?;
    }
    writeln!(out)?;
    let heads: Vec<&str> = Class::ALL.iter().map(|class| class.name()).collect();
    writeln!(
        out,
        "| Way | Peak heap | Resident heap | {} | Table and paths | Decoded values | Pack in the program |",
        heads.join(" | ")
    )?;
    writeln!(
        out,
        "| --- | --------- | ------------- | --- | --- | --- | --- | --- | --- | --- |"
    )?;
    for way in &report.ways {
        let files: Vec<String> = way.files.iter().map(ToString::to_string).collect();
        writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} |",
            way.name,
            way.peak(),
            way.kept(),
            files.join(" | "),
            way.table(),
            way.load.kept,
            way.in_program
        )?;
    }
    let first = &report.first_read;
    writeln!(
        out,
        "| on first read | {} | {} | 0 | 0 | 0 | 0 | {} | {} | 0 |\n",
        first.peak(),
        first.kept(),
        first.table,
        first.decoded
    )?;
    writeln!(
        out,
        "On first read, computed: peak = table {} + the largest file ({}) stored {} + unpacked {} \
         + the load's own peak {} = {}; resident = table {} + decoded values {} = {}.\n",
        first.table,
        shape.largest.0,
        first.largest_stored,
        first.largest_unpacked,
        first.load_peak,
        first.peak(),
        first.table,
        first.decoded,
        first.kept()
    )?;
    let embedded = &report.ways[0];
    writeln!(
        out,
        "The embedded way's parts: opening took {} bytes at its peak and kept {}; the load took {} \
         at its peak and kept {}.\n",
        embedded.open.peak, embedded.open.kept, embedded.load.peak, embedded.load.kept
    )
}

/// Measures and prints the pack `bytes` (kept for the run) named `title`.
fn run(
    out: &mut impl Write,
    title: &str,
    bytes: Vec<u8>,
    stamp: &str,
    load: Loader,
) -> Result<(), String> {
    let bytes: &'static [u8] = bytes.leak();
    let Some(report) = measure_pack(bytes, stamp, load)? else {
        return Err("this build counts no heap: run it as a release build".to_owned());
    };
    print(out, title, &report).map_err(|e| e.to_string())
}

/// What the program is told to do.
enum Ask {
    /// Measure Hello, Grid, the synthetic packs, and a folder if one is
    /// named.
    Measure(Option<String>),
    /// Write the synthetic pack of a target's budget to a file.
    Write { target: String, file: String },
}

/// What `args` ask, or how to call the program.
fn ask(args: &[String]) -> Result<Ask, String> {
    match args {
        [] => Ok(Ask::Measure(None)),
        [flag, dir] if flag == "--folder" => Ok(Ask::Measure(Some(dir.clone()))),
        [flag, target, file] if flag == "--write-synthetic" => Ok(Ask::Write {
            target: target.clone(),
            file: file.clone(),
        }),
        _ => Err(
            "usage: asset_memory [--folder <dir> | --write-synthetic <web|playdate> <file>]"
                .to_owned(),
        ),
    }
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let budgets = budgets()?;
    let stdout = io::stdout();
    let mut out = stdout.lock();
    match ask(&args)? {
        Ask::Write { target, file } => {
            let &(_, budget) = budgets
                .iter()
                .find(|(id, _)| *id == target)
                .ok_or_else(|| format!("no synthetic pack for {target}"))?;
            let bytes = synthetic_pack(budget, SEED)?;
            std::fs::write(&file, &bytes).map_err(|e| format!("{file}: {e}"))?;
            writeln!(out, "{file}: {} bytes", bytes.len()).map_err(|e| e.to_string())
        }
        Ask::Measure(folder) => {
            run(
                &mut out,
                "Hello, Grid",
                hello_pack(BTreeMap::new())?,
                PACK_STAMP,
                hello_load,
            )?;
            if let Some(dir) = folder {
                let title = format!("The folder {dir}");
                run(
                    &mut out,
                    &title,
                    folder_pack(Path::new(&dir))?,
                    FOLDER_STAMP,
                    engine_load,
                )?;
            }
            for (id, budget) in budgets {
                let title = format!("Synthetic, the {id} budget ({budget} bytes)");
                run(
                    &mut out,
                    &title,
                    synthetic_pack(budget, SEED)?,
                    PACK_STAMP,
                    hello_load,
                )?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small synthetic pack, as the budgets' are made.
    const SMALL: u64 = 64 << 10;

    #[test]
    fn the_synthetic_pack_is_the_same_for_the_same_seed_and_opens() {
        let first = synthetic_pack(SMALL, SEED).unwrap();
        assert_eq!(first, synthetic_pack(SMALL, SEED).unwrap());
        assert_ne!(first, synthetic_pack(SMALL, SEED + 1).unwrap());
        let pack = Pack::open(&first, PACK_STAMP).unwrap();
        let shape = Shape::of(&pack, wide(first.len()));
        // Every class is there, and the data's text compresses.
        for class in Class::ALL {
            assert!(shape.classes[class.at()].files > 0, "{class:?}");
        }
        let data = shape.classes[Class::Other.at()];
        assert!(data.stored < data.unpacked, "{data:?}");
        let sounds = shape.classes[Class::Sound.at()];
        assert_eq!(sounds.stored, sounds.unpacked);
    }

    #[test]
    fn every_way_holds_a_small_synthetic_pack() {
        let bytes: &'static [u8] = synthetic_pack(SMALL, SEED).unwrap().leak();
        let Some(report) = measure_pack(bytes, PACK_STAMP, hello_load).unwrap() else {
            return;
        };
        let names: Vec<&str> = report.ways.iter().map(|way| way.name).collect();
        assert_eq!(
            names,
            ["embedded", "read whole, copied", "read whole, kept"]
        );
        for way in &report.ways {
            assert!(way.kept() <= way.peak(), "{way:?}");
            assert!(way.file_bytes() <= way.open.kept, "{way:?}");
            assert!(way.load.kept > 0, "{way:?}");
        }
        let [embedded, copied, kept] = &report.ways[..] else {
            panic!("three ways");
        };
        // Only the embedded pack is in the program, and the ways that read
        // it hold more than the one that doesn't.
        assert_eq!(embedded.in_program, wide(bytes.len()));
        assert!(embedded.kept() < copied.kept(), "{report:?}");
        assert!(copied.kept() <= kept.kept(), "{report:?}");
        assert!(copied.peak() >= wide(bytes.len()), "{report:?}");
        let first = report.first_read;
        assert!(first.kept() <= first.peak(), "{first:?}");
        assert!(first.kept() <= embedded.kept(), "{report:?}");
        let mut printed = Vec::new();
        print(&mut printed, "small", &report).unwrap();
        let printed = String::from_utf8(printed).unwrap();
        for part in [
            "embedded",
            "on first read",
            "decoded values",
            "sounds",
            "baked data",
        ] {
            assert!(printed.to_lowercase().contains(part), "{part}: {printed}");
        }
    }

    #[test]
    fn a_folder_packs_every_file_and_the_engine_loads_it() {
        let dir = std::env::temp_dir().join(format!("asset-memory-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("art")).unwrap();
        let png = hello_grid::file("tiles.png").unwrap();
        std::fs::write(dir.join("art/tiles.png"), png).unwrap();
        std::fs::write(dir.join("notes.txt"), "notes").unwrap();
        let bytes: &'static [u8] = folder_pack(&dir).unwrap().leak();
        std::fs::remove_dir_all(&dir).unwrap();
        let pack = Pack::open(bytes, FOLDER_STAMP).unwrap();
        assert_eq!(pack.get("art/tiles.png"), Some(png));
        assert_eq!(pack.get("notes.txt"), Some(&b"notes"[..]));
        if let Some(report) = measure_pack(bytes, FOLDER_STAMP, engine_load).unwrap() {
            assert_eq!(report.shape.classes[Class::Png.at()].files, 1);
            assert!(report.ways[0].load.kept > 0, "{report:?}");
        }
    }

    #[test]
    fn the_budgets_are_the_profiles_and_the_arguments_read() {
        let budgets = budgets().unwrap();
        assert_eq!(budgets.len(), 2);
        assert!(
            budgets.iter().all(|&(_, bytes)| bytes > 1 << 20),
            "{budgets:?}"
        );
        assert!(matches!(ask(&[]), Ok(Ask::Measure(None))));
        let folder = ["--folder".to_owned(), "assets".to_owned()];
        assert!(matches!(ask(&folder), Ok(Ask::Measure(Some(dir))) if dir == "assets"));
        assert!(ask(&["--what".to_owned()]).is_err());
    }
}
