//! A bundle's fingerprint ([`Bundle::fingerprint`]).

use shuyi_core::ContentHash;

use super::Bundle;
use crate::fnv::Hasher;
use crate::prelude::*;

impl Bundle {
    /// A fingerprint of the content that ships, the same on every
    /// platform: what a record or a save is stamped with
    /// (`shuyi_core::Made`). Any byte, name or file changed in a layer
    /// that ships changes it; the test layers never count, whatever the
    /// thread ([`story_only`](super::story_only) changes nothing).
    ///
    /// The layers that ship are taken from the last read to the first
    /// (the base layer first), each file of a layer in path order, as its
    /// path's length and bytes then its contents' length and bytes, into
    /// the word-wise FNV-1a hash of the pack's checksum. A layer that
    /// [stands for](super::Layer::standing_for) a bundle (a pack's)
    /// replaces what was taken so far with that bundle's fingerprint. So a
    /// build that ships one pack has the fingerprint of the files it was
    /// baked from, and a layer laid over it counts as it would over them.
    ///
    /// It reads every byte that ships, without telling the
    /// [`watch`](super::watch)er: compute it once and keep it.
    pub fn fingerprint(&self) -> ContentHash {
        let mut hash = Hasher::new();
        for layer in self.shipped().iter().rev() {
            if let Some(content) = layer.stands_for {
                hash = Hasher(content.0);
                continue;
            }
            let mut paths = Vec::new();
            layer.files.list("", true, &mut paths);
            paths.sort_unstable();
            for path in paths {
                hash.sized(path.as_bytes());
                hash.sized(layer.files.get(path).unwrap_or_default());
            }
        }
        ContentHash(hash.0)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use alloc::collections::BTreeMap;

    use super::super::{Layer, story_only};
    use super::*;

    static BASE: &[(&str, &[u8])] = &[
        ("data/a.ron", b"base a"),
        ("data/b.ron", b"base b"),
        ("top.txt", b"top"),
    ];

    static OVER: &[(&str, &[u8])] = &[("data/a.ron", b"over a"), ("new.txt", b"new")];

    /// A layer of `files`, kept for the run.
    fn layer(files: &[(&str, &[u8])]) -> Layer {
        let files = files
            .iter()
            .map(|(path, bytes)| ((*path).to_owned(), bytes.to_vec()))
            .collect::<BTreeMap<_, _>>();
        Layer::owned("layer".to_owned(), files).unwrap()
    }

    /// The fingerprint of one layer of `files`.
    fn of(files: &[(&str, &[u8])]) -> ContentHash {
        Bundle::new(vec![layer(files)]).fingerprint()
    }

    /// A fixed bundle of two layers. Its fingerprint is the row
    /// `fingerprint` of `tests/golden/formats.ron` (ticket 2116): a change
    /// there changes every stamp a game wrote, whose records then read as
    /// made with other content.
    pub(crate) fn fixed() -> Bundle {
        Bundle::new(vec![Layer::table("over", OVER), Layer::table("base", BASE)])
    }

    #[test]
    fn no_layer_is_the_offset_basis() {
        // No layer, or an empty one, is FNV-1a's offset basis.
        assert_eq!(
            Bundle::new(Vec::new()).fingerprint(),
            ContentHash(0xcbf2_9ce4_8422_2325)
        );
        assert_eq!(of(&[]), ContentHash(0xcbf2_9ce4_8422_2325));
    }

    #[test]
    fn a_byte_a_name_or_a_file_changed_changes_it() {
        let base = of(BASE);
        assert_eq!(of(BASE), base);
        let byte: &[(&str, &[u8])] = &[
            ("data/a.ron", b"base A"),
            ("data/b.ron", b"base b"),
            ("top.txt", b"top"),
        ];
        let name: &[(&str, &[u8])] = &[
            ("data/a.ron", b"base a"),
            ("data/c.ron", b"base b"),
            ("top.txt", b"top"),
        ];
        let added: &[(&str, &[u8])] = &[
            ("data/a.ron", b"base a"),
            ("data/b.ron", b"base b"),
            ("data/c.ron", b""),
            ("top.txt", b"top"),
        ];
        // A byte moved from a path to its contents.
        let moved: &[(&str, &[u8])] = &[
            ("data/a.ro", b"nbase a"),
            ("data/b.ron", b"base b"),
            ("top.txt", b"top"),
        ];
        for changed in [byte, name, added, moved] {
            assert_ne!(of(changed), base, "{changed:?}");
        }
    }

    #[test]
    fn the_layers_count_base_first_and_the_test_layers_never() {
        let base = Layer::table("base", BASE);
        let over = Layer::table("over", OVER);
        let both = Bundle::new(vec![over, base]).fingerprint();
        assert_ne!(both, Bundle::new(vec![base, over]).fingerprint());
        assert_ne!(both, of(BASE));
        // A test layer, read or not, counts for nothing.
        let tests = Bundle::new(vec![over, base, Layer::table("tests", BASE).test()]);
        assert_eq!(tests.fingerprint(), both);
        assert_eq!(story_only(|| tests.fingerprint()), both);
        let changed = Bundle::new(vec![over, base, Layer::table("tests", OVER).test()]);
        assert_eq!(changed.fingerprint(), both);
        // A private layer ships, and counts.
        let private = Bundle::new(vec![over.private(), base]).fingerprint();
        assert_eq!(private, both);
        // Whatever kind of layer holds the files, they count the same.
        assert_eq!(
            Bundle::new(vec![layer(OVER), layer(BASE)]).fingerprint(),
            both
        );
    }

    #[test]
    fn a_layer_standing_for_a_bundle_takes_its_fingerprint_and_hides_those_under_it() {
        let base = Layer::table("base", BASE);
        let over = Layer::table("over", OVER);
        let loose = Bundle::new(vec![base]).fingerprint();
        // A pack of `BASE` alone: its own files don't count.
        let pack = Layer::table("pack", OVER).standing_for(loose);
        assert_eq!(Bundle::new(vec![pack]).fingerprint(), loose);
        // A loose layer over it counts as over the files it stands for.
        assert_eq!(
            Bundle::new(vec![over, pack]).fingerprint(),
            Bundle::new(vec![over, base]).fingerprint()
        );
        // A layer under it doesn't count: the pack holds every file.
        assert_eq!(
            Bundle::new(vec![pack, over]).fingerprint(),
            Bundle::new(vec![pack]).fingerprint()
        );
        // A test layer standing for something counts for nothing either.
        let test = Layer::table("tests", OVER)
            .standing_for(ContentHash(1))
            .test();
        assert_eq!(Bundle::new(vec![base, test]).fingerprint(), loose);
    }

    proptest::proptest! {
        /// One byte changed in one file of any bundle changes its
        /// fingerprint.
        #[test]
        fn one_byte_changed_anywhere_changes_it(
            layers in proptest::collection::vec(
                proptest::collection::btree_map(
                    "[ab]{1,2}(/[ab]{1,2}){0,2}",
                    proptest::collection::vec(proptest::num::u8::ANY, 1..20),
                    1..6,
                ),
                1..4,
            ),
            which in proptest::num::usize::ANY,
            at in proptest::num::usize::ANY,
            flip in 1..=u8::MAX,
        ) {
            let make = |layers: &[BTreeMap<String, Vec<u8>>]| {
                let layers = layers
                    .iter()
                    .enumerate()
                    .map(|(i, files)| Layer::owned(format!("layer{i}"), files.clone()).unwrap())
                    .collect();
                Bundle::new(layers).fingerprint()
            };
            let before = make(&layers);
            let mut changed = layers.clone();
            let files: Vec<(usize, String)> = changed
                .iter()
                .enumerate()
                .flat_map(|(i, files)| files.keys().map(move |path| (i, path.clone())))
                .collect();
            let (layer, path) = &files[which % files.len()];
            let bytes = changed[*layer].get_mut(path).unwrap();
            let len = bytes.len();
            bytes[at % len] ^= flip;
            proptest::prop_assert_ne!(make(&changed), before);
        }
    }
}
