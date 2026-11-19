//! The pack as the page streams it (ADR-0128 §2, §3): a [`Source`] over a
//! buffer of the pack's whole length, made once its index has arrived and
//! filled in order, between frames, with the chunks the page received
//! ([`Stream::pump`]). A read past what has arrived is
//! [`SourceError::NotYet`]; once the page's fetch fails or its body ends
//! short, the stream is failed, and a read past what arrived is
//! [`SourceError::Failed`], so a load waiting on a folder reads it, gets the
//! missing file and fails with its error instead of waiting forever.
//!
//! The page is a [`Page`]: the plugin in `crates/cli/web/pack-file.js` on
//! the web, a fake in the tests, so all of this runs off the browser.

use std::sync::{Mutex, MutexGuard, PoisonError};

use shuyi_assets::pack::{Index, IndexError, PackError, Source, SourceError};

use super::PackFileError;

/// What the page holds of the pack it fetches.
pub(crate) trait Page {
    /// Writes the next bytes the page received into `into`, as many as it
    /// holds up to its length, and drops them: how many it wrote.
    fn take(&self, into: &mut [u8]) -> usize;
    /// Whether the whole body has been received (the page may still hold
    /// some of it).
    fn ended(&self) -> bool;
    /// Why the page has no pack, or why its fetch broke; `""` while it is
    /// fine.
    fn problem(&self) -> String;
}

/// The pack's bytes as they arrive: a [`Source`] the game's bundle reads
/// through its layer ([`Index::into_layer`]). One copy, in the program's
/// memory, kept for the run.
pub(crate) struct Stream {
    state: Mutex<Filled>,
}

/// A stream's buffer and how far it is filled.
struct Filled {
    /// The pack's whole length; its first `filled` bytes have arrived.
    bytes: Box<[u8]>,
    filled: usize,
    /// Why the rest won't arrive, once the page's fetch broke.
    failed: Option<String>,
}

impl core::fmt::Debug for Stream {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let state = self.state();
        f.debug_struct("Stream")
            .field("length", &state.bytes.len())
            .field("filled", &state.filled)
            .field("failed", &state.failed)
            .finish()
    }
}

impl Stream {
    /// A stream of a pack of `length` bytes, none arrived yet.
    pub(crate) fn new(length: usize) -> Self {
        Self {
            state: Mutex::new(Filled {
                bytes: vec![0; length].into_boxed_slice(),
                filled: 0,
                failed: None,
            }),
        }
    }

    /// Its state. A panic while it was held left nothing half written that
    /// matters: `filled` moves only once the bytes are in.
    fn state(&self) -> MutexGuard<'_, Filled> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Fills its next bytes with `take`, which writes at most the slice it
    /// is given and says how many it wrote.
    fn fill(&self, take: impl FnOnce(&mut [u8]) -> usize) {
        let mut state = self.state();
        let filled = state.filled;
        let Some(rest) = state.bytes.get_mut(filled..) else {
            return;
        };
        let wrote = take(rest).min(rest.len());
        state.filled = filled + wrote;
    }

    /// Adds `chunk` after the bytes arrived: those past the pack's length are
    /// left out.
    pub(crate) fn push(&self, chunk: &[u8]) {
        self.fill(|rest| {
            let count = chunk.len().min(rest.len());
            rest[..count].copy_from_slice(&chunk[..count]);
            count
        });
    }

    /// Marks it failed for `why`: the rest won't arrive. The first reason
    /// is kept.
    pub(crate) fn fail(&self, why: String) {
        self.state().failed.get_or_insert(why);
    }

    /// Whether nothing more will arrive: every byte has, or it failed.
    fn over(&self) -> bool {
        let state = self.state();
        state.failed.is_some() || state.filled == state.bytes.len()
    }

    /// Copies into it the chunks `page` received since, in order, and fails
    /// it if the page's fetch broke or its body ended short: what the
    /// backend does between frames.
    pub(crate) fn pump(&self, page: &impl Page) {
        if self.over() {
            return;
        }
        self.fill(|rest| page.take(rest));
        if self.over() {
            return;
        }
        let problem = page.problem();
        if !problem.is_empty() {
            self.fail(problem);
        } else if page.ended() {
            let state = self.state();
            let why = format!(
                "the page's fetch of the pack ended after {} of its {} bytes",
                state.filled,
                state.bytes.len()
            );
            drop(state);
            self.fail(why);
        }
    }
}

impl Source for Stream {
    fn read(&self, at: u64, into: &mut [u8]) -> Result<(), SourceError> {
        let state = self.state();
        let range = usize::try_from(at)
            .ok()
            .and_then(|start| Some(start..start.checked_add(into.len())?));
        match range {
            Some(range) if range.end <= state.filled => {
                into.copy_from_slice(&state.bytes[range]);
                Ok(())
            }
            _ if state.failed.is_some() => {
                Err(SourceError::Failed(state.failed.clone().unwrap_or_default()))
            }
            Some(range) if range.end <= state.bytes.len() => Err(SourceError::NotYet),
            _ => Err(SourceError::Failed("past the end of the pack".to_owned())),
        }
    }

    /// Whether those bytes have arrived; once it failed, or past the pack's
    /// end, they never will, and a read of them fails: they count as
    /// arrived, as a file whose read failed does.
    fn arrived(&self, at: u64, len: u64) -> bool {
        let state = self.state();
        let end = at.saturating_add(len);
        end <= state.filled as u64 || state.failed.is_some() || end > state.bytes.len() as u64
    }
}

/// The pack `file` the page streams, opened with `stamp`: its index, read
/// from the first bytes the page holds, and the stream of its bytes, which
/// holds every byte the page has received.
///
/// # Errors
///
/// When the page has no pack, or holds fewer bytes than its header and
/// index, [`PackFileError::Missing`] with what its fetch said, else
/// [`PackFileError::Refused`]; when `Index::read` refuses its index,
/// [`PackFileError::Refused`].
pub(crate) fn open(
    page: &impl Page,
    file: &str,
    stamp: &str,
) -> Result<(Index, Stream), PackFileError> {
    let refused = |error| PackFileError::Refused {
        file: file.to_owned(),
        error,
    };
    let mut prefix = Vec::new();
    let index = loop {
        match Index::read(&prefix, stamp) {
            Ok(index) => break index,
            Err(IndexError::Short { needs }) => {
                let needs = usize::try_from(needs).map_err(|_| refused(PackError::Truncated))?;
                let had = prefix.len();
                prefix.resize(needs, 0);
                let got = page.take(&mut prefix[had..]);
                prefix.truncate(had + got.min(needs - had));
                if prefix.len() < needs {
                    let problem = page.problem();
                    if problem.is_empty() {
                        return Err(refused(PackError::Truncated));
                    }
                    return Err(PackFileError::Missing {
                        file: file.to_owned(),
                        looked: vec![format!("the page: {problem}")],
                    });
                }
            }
            Err(IndexError::Pack(error)) => return Err(refused(error)),
            Err(error) => return Err(refused(PackError::Broken(error.to_string()))),
        }
    };
    // The pack ends where its last file does, or with its index.
    let end = index
        .paths()
        .filter_map(|path| index.file_range(path))
        .map(|range| range.end)
        .fold(prefix.len() as u64, u64::max);
    let length = usize::try_from(end).map_err(|_| refused(PackError::Truncated))?;
    let stream = Stream::new(length);
    stream.push(&prefix);
    stream.pump(page);
    Ok((index, stream))
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::collections::{BTreeMap, VecDeque};

    use shuyi_assets::bundle::{Bundle, Layer};
    use shuyi_assets::pack::{Content, PackInfo, write_first};

    use super::*;

    const STAMP: &str = "demo 1";

    /// A page that has received `chunks`, hands them over in order, and may
    /// say its fetch ended or broke.
    #[derive(Default)]
    struct FakePage {
        chunks: RefCell<VecDeque<Vec<u8>>>,
        ended: Cell<bool>,
        problem: RefCell<String>,
    }

    impl FakePage {
        fn receive(&self, chunk: &[u8]) {
            self.chunks.borrow_mut().push_back(chunk.to_vec());
        }

        /// How many bytes it still holds.
        fn held(&self) -> usize {
            self.chunks.borrow().iter().map(Vec::len).sum()
        }
    }

    impl Page for FakePage {
        fn take(&self, into: &mut [u8]) -> usize {
            let mut chunks = self.chunks.borrow_mut();
            let mut wrote = 0;
            while wrote < into.len() {
                let Some(chunk) = chunks.front_mut() else { break };
                let count = chunk.len().min(into.len() - wrote);
                into[wrote..wrote + count].copy_from_slice(&chunk[..count]);
                wrote += count;
                chunk.drain(..count);
                if chunk.is_empty() {
                    chunks.pop_front();
                }
            }
            wrote
        }

        fn ended(&self) -> bool {
            self.ended.get()
        }

        fn problem(&self) -> String {
            self.problem.borrow().clone()
        }
    }

    /// 3,000 bytes that don't compress, so the pack keeps them as they are.
    fn big() -> Vec<u8> {
        let mut x: u32 = 2_463_534_242;
        (0..3000)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                x.to_le_bytes()[0]
            })
            .collect()
    }

    /// A pack with a title folder laid first and a big file after it.
    fn pack() -> Vec<u8> {
        let mut info = PackInfo::default();
        info.stamp = STAMP.to_owned();
        let files = BTreeMap::from([
            ("a.ron".to_owned(), Content::Raw(big())),
            ("title/logo.txt".to_owned(), Content::Raw(b"logo".to_vec())),
        ]);
        write_first(&info, &["title"], &files)
    }

    /// Where the bytes of `path` lie in `pack`.
    fn range(pack: &[u8], path: &str) -> core::ops::Range<usize> {
        let index = Index::read(pack, STAMP).unwrap();
        let range = index.file_range(path).unwrap();
        usize::try_from(range.start).unwrap()..usize::try_from(range.end).unwrap()
    }

    /// The bundle of the pack `stream` holds, read through `index`, its
    /// layer, and the stream, kept for the run as the backend keeps it.
    fn bundle(index: Index, stream: Stream) -> (Bundle, Layer, &'static Stream) {
        let source: &'static Stream = Box::leak(Box::new(stream));
        let layer = index.into_layer("files", source);
        (Bundle::new(vec![layer]), layer, source)
    }

    #[test]
    fn the_source_answers_the_bytes_arrived_and_not_yet_past_them() {
        let stream = Stream::new(10);
        stream.push(b"0123");
        let mut four = [0; 4];
        assert_eq!(stream.read(0, &mut four), Ok(()));
        assert_eq!(&four, b"0123");
        let mut two = [0; 2];
        assert_eq!(stream.read(2, &mut two), Ok(()));
        assert_eq!(&two, b"23");
        assert_eq!(stream.read(3, &mut two), Err(SourceError::NotYet));
        assert!(stream.arrived(0, 4));
        assert!(!stream.arrived(3, 2));
        // Chunks go in in order, and none past the pack's length.
        stream.push(b"456");
        stream.push(b"789abc");
        assert_eq!(stream.read(3, &mut two), Ok(()));
        assert_eq!(&two, b"34");
        let mut all = [0; 10];
        assert_eq!(stream.read(0, &mut all), Ok(()));
        assert_eq!(&all, b"0123456789");
        assert!(stream.arrived(0, 10));
        // Past the end it never arrives: a read of it fails.
        assert!(stream.arrived(9, 2));
        assert_eq!(
            stream.read(9, &mut two),
            Err(SourceError::Failed("past the end of the pack".to_owned()))
        );
        assert!(stream.read(u64::MAX, &mut two).is_err());
    }

    #[test]
    fn a_stream_marked_failed_answers_failed_past_what_arrived_and_arrived_true() {
        let stream = Stream::new(10);
        stream.push(b"01234");
        stream.fail("game.pack: network error".to_owned());
        stream.fail("a second reason".to_owned());
        let mut two = [0; 2];
        assert_eq!(stream.read(3, &mut two), Ok(()));
        assert_eq!(&two, b"34");
        assert_eq!(
            stream.read(4, &mut two),
            Err(SourceError::Failed("game.pack: network error".to_owned()))
        );
        assert!(stream.arrived(4, 6));
        assert!(format!("{stream:?}").contains("network error"));
    }

    #[test]
    fn the_index_is_read_from_the_page_and_the_rest_pumped_in_order() {
        let pack = pack();
        let page = FakePage::default();
        // The page started the game once the header and the index arrived.
        let index_end = range(&pack, "title/logo.txt").start;
        page.receive(&pack[..10]);
        page.receive(&pack[10..index_end]);
        let (index, stream) = open(&page, "demo.pack", STAMP).unwrap();
        assert_eq!(index.info().stamp, STAMP);
        let mut byte = [0];
        assert_eq!(
            stream.read(index_end as u64, &mut byte),
            Err(SourceError::NotYet)
        );
        // The title's bytes arrive, then the rest, in chunks of any size.
        let title_end = range(&pack, "title/logo.txt").end;
        page.receive(&pack[index_end..title_end + 7]);
        stream.pump(&page);
        assert_eq!(page.held(), 0, "the page keeps nothing it handed over");
        let (bundle, layer, source) = bundle(index, stream);
        assert!(bundle.arrived("title"));
        assert_eq!(bundle.file("title/logo.txt").as_deref(), Some("logo"));
        assert!(!bundle.arrived(""));
        assert_eq!(bundle.bytes("a.ron"), None);
        assert!(bundle.has("a.ron"));
        for chunk in pack[title_end + 7..].chunks(1000) {
            page.receive(chunk);
            source.pump(&page);
        }
        assert!(bundle.arrived(""));
        assert_eq!(bundle.bytes("a.ron").as_deref(), Some(&big()[..]));
        assert!(layer.problems().is_empty());
        // Once full, nothing more is taken.
        page.ended.set(true);
        source.pump(&page);
        assert!(layer.problems().is_empty());
    }

    #[test]
    fn a_stream_that_breaks_half_way_fails_and_its_files_read_as_missing() {
        let pack = pack();
        let half = range(&pack, "a.ron").start + 1500;
        for broken in [true, false] {
            let page = FakePage::default();
            page.receive(&pack[..half]);
            let (index, stream) = open(&page, "demo.pack", STAMP).unwrap();
            let (bundle, layer, source) = bundle(index, stream);
            assert!(!bundle.arrived(""));
            source.pump(&page);
            assert!(!bundle.arrived(""), "nothing more, and nothing wrong");
            if broken {
                *page.problem.borrow_mut() = "demo.pack: network error".to_owned();
            } else {
                page.ended.set(true);
            }
            source.pump(&page);
            // Whatever waited on the rest goes on, and reads it as missing.
            assert!(bundle.arrived(""));
            assert_eq!(bundle.file("title/logo.txt").as_deref(), Some("logo"));
            assert_eq!(bundle.bytes("a.ron"), None);
            assert_eq!(bundle.bytes("a.ron"), None);
            let problems = layer.problems();
            assert_eq!(problems.len(), 1, "{problems:?}");
            let problem = problems[0].to_string();
            let why = if broken {
                "demo.pack: network error".to_owned()
            } else {
                format!(
                    "the page's fetch of the pack ended after {half} of its {} bytes",
                    pack.len()
                )
            };
            assert_eq!(problem, format!("a.ron: its source failed: {why}"));
        }
    }

    #[test]
    fn a_page_with_no_pack_or_too_few_bytes_says_why() {
        let page = FakePage::default();
        *page.problem.borrow_mut() = "demo.pack: HTTP 404".to_owned();
        let error = open(&page, "demo.pack", STAMP).unwrap_err();
        assert_eq!(
            error,
            PackFileError::Missing {
                file: "demo.pack".to_owned(),
                looked: vec!["the page: demo.pack: HTTP 404".to_owned()],
            }
        );
        let pack = pack();
        // Its body ended inside the index.
        let page = FakePage::default();
        page.receive(&pack[..30]);
        page.ended.set(true);
        let error = open(&page, "demo.pack", STAMP).unwrap_err();
        assert!(
            matches!(
                error,
                PackFileError::Refused {
                    error: PackError::Truncated,
                    ..
                }
            ),
            "{error:?}"
        );
        // Not a pack, or another game's.
        let page = FakePage::default();
        page.receive(b"<!DOCTYPE html> a page, not a pack");
        assert!(matches!(
            open(&page, "demo.pack", STAMP),
            Err(PackFileError::Refused {
                error: PackError::NotAPack,
                ..
            })
        ));
        let page = FakePage::default();
        page.receive(&pack);
        assert!(matches!(
            open(&page, "demo.pack", "demo 2"),
            Err(PackFileError::Refused {
                error: PackError::Stamp { .. },
                ..
            })
        ));
    }
}
