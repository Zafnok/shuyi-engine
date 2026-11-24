//! The helpers, streams, forks and bags: their exact draws, their exact
//! odds, and the golden values ADR-0143 freezes.

use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::{format, vec};

use proptest::prelude::*;

use super::*;

/// The first `n` outputs of `rng`.
fn outputs(rng: &mut SimRng, n: usize) -> Vec<u32> {
    (0..n).map(|_| rng.next_u32()).collect()
}

/// A source whose `roll_below` draws are `draws`.
fn scripted(draws: impl Into<Vec<u32>>) -> ScriptedRng {
    ScriptedRng::new([]).with_below(draws)
}

#[test]
fn mix64_and_mix_at_give_splitmix64s_published_outputs() {
    assert_eq!(mix_at(0, 0), 0xE220_A839_7B1D_CDAF);
    assert_eq!(mix_at(0, 1), 0x6E78_9E6A_A1B9_65F4);
    assert_eq!(mix64(0), 0xE220_A839_7B1D_CDAF);
    assert_eq!(mix64(GOLDEN_GAMMA), 0x6E78_9E6A_A1B9_65F4);
}

#[test]
fn range_draws_once_and_none_when_empty() {
    let mut rng = scripted([3]);
    assert_eq!(rng.range(10..20), 13);
    assert_eq!(rng.consumed_below(), 1);
    assert_eq!(rng.range(5..5), 5);
    assert_eq!(rng.range(Range { start: 7, end: 2 }), 7);
    assert_eq!(rng.consumed_below(), 1);
}

#[test]
fn range_spans_every_i32() {
    let mut rng = scripted([0, u32::MAX - 1]);
    assert_eq!(rng.range(i32::MIN..i32::MAX), i32::MIN);
    assert_eq!(rng.range(i32::MIN..i32::MAX), i32::MAX - 1);
    let mut sim = SimRng::new(1);
    for _ in 0..100 {
        assert!(sim.range(i32::MIN..i32::MAX) < i32::MAX);
    }
}

#[test]
fn range_gives_each_value_once_over_every_draw() {
    let got: Vec<i32> = (0..5).map(|d| scripted([d]).range(-2..3)).collect();
    assert_eq!(got, [-2, -1, 0, 1, 2]);
}

#[test]
fn chance_draws_once_whatever_the_odds() {
    let mut rng = scripted([4, 0, 4]);
    assert!(!rng.chance(0, 5));
    assert!(rng.chance(5, 5));
    assert!(rng.chance(9, 5));
    assert_eq!(rng.consumed_below(), 3);
    assert!(!rng.chance(1, 0));
    assert_eq!(rng.consumed_below(), 3);
}

#[test]
fn chance_has_exact_odds() {
    let hits = (0..7).filter(|&d| scripted([d]).chance(3, 7)).count();
    assert_eq!(hits, 3);
}

#[test]
fn index_and_pick_draw_once_and_none_for_nothing() {
    let mut rng = scripted([0, 2, 1]);
    assert_eq!(rng.index(0), None);
    assert_eq!(rng.index(1), Some(0));
    assert_eq!(rng.index(3), Some(2));
    assert_eq!(rng.pick::<u8>(&[]), None);
    assert_eq!(rng.consumed_below(), 2);
    assert_eq!(rng.pick(&['a', 'b', 'c']), Some(&'b'));
    assert_eq!(rng.consumed_below(), 3);
}

#[test]
fn shuffle_of_none_or_one_item_draws_nothing() {
    let mut rng = scripted([]);
    rng.shuffle::<u8>(&mut []);
    rng.shuffle(&mut [7]);
    assert_eq!(rng.consumed_below(), 0);
}

#[test]
fn shuffle_makes_len_minus_one_draws_as_fisher_yates() {
    // i = 3 swaps with 0, i = 2 with 2, i = 1 with 0.
    let mut rng = scripted([0, 2, 0]);
    let mut items = ['a', 'b', 'c', 'd'];
    rng.shuffle(&mut items);
    assert_eq!(items, ['b', 'd', 'c', 'a']);
    assert_eq!(rng.consumed_below(), 3);
}

#[test]
fn shuffle_gives_each_order_once_over_every_script() {
    let mut orders = BTreeSet::new();
    for a in 0..4 {
        for b in 0..3 {
            for c in 0..2 {
                let mut items = [0u8, 1, 2, 3];
                scripted([a, b, c]).shuffle(&mut items);
                orders.insert(items);
            }
        }
    }
    assert_eq!(orders.len(), 24);
}

#[test]
fn weighted_index_has_exact_odds() {
    let mut counts = [0; 3];
    for d in 0..8 {
        let mut rng = scripted([d]);
        counts[rng.weighted_index(&[3, 0, 5]).unwrap()] += 1;
        assert_eq!(rng.consumed_below(), 1);
    }
    assert_eq!(counts, [3, 0, 5]);
}

#[test]
fn weighted_index_refuses_a_total_of_zero_or_past_u32_without_a_draw() {
    let mut rng = scripted([]);
    assert_eq!(rng.weighted_index(&[]), None);
    assert_eq!(rng.weighted_index(&[0, 0]), None);
    assert_eq!(rng.weighted_index(&[u32::MAX, 1]), None);
    assert_eq!(rng.weighted_index(&[u32::MAX, u32::MAX, u32::MAX]), None);
    assert_eq!(rng.consumed_below(), 0);
    let mut rng = scripted([u32::MAX - 1]);
    assert_eq!(rng.weighted_index(&[u32::MAX - 1, 1]), Some(1));
}

#[test]
fn pick_weighted_picks_by_the_weight_beside_each_item() {
    let items = [(1, "common"), (0, "never"), (2, "rare")];
    let got: Vec<&str> = (0..3)
        .filter_map(|d| scripted([d]).pick_weighted(&items).copied())
        .collect();
    assert_eq!(got, ["common", "rare", "rare"]);
    assert_eq!(scripted([]).pick_weighted(&[(0, 'x')]), None);
}

#[test]
fn a_dyn_random_source_has_the_non_generic_helpers() {
    let mut sim = SimRng::new(3);
    let rng: &mut dyn RandomSource = &mut sim;
    let value = rng.range(-5..5);
    assert!((-5..5).contains(&value));
    let _ = (rng.chance(1, 2), rng.index(4), rng.weighted_index(&[1, 1]));
}

#[test]
fn fork_moves_its_parent_by_exactly_two_draws() {
    let mut parent = SimRng::new(2026);
    let mut twin = parent.clone();
    let mut child = parent.fork("level");
    for _ in 0..100 {
        child.next_u32();
    }
    twin.next_u32();
    twin.next_u32();
    assert_eq!(parent, twin);
}

#[test]
fn a_draw_on_one_stream_leaves_another_as_it_was() {
    let mut combat = SimRng::stream(9, "combat");
    let mut loot = SimRng::stream(9, "loot");
    let before = outputs(&mut SimRng::stream(9, "loot"), 8);
    combat.next_u32();
    assert_eq!(outputs(&mut loot, 8), before);

    // With one shared generator, the same extra draw moves the loot.
    let mut shared = SimRng::new(9);
    let before = outputs(&mut shared.clone(), 8);
    shared.next_u32();
    assert_ne!(outputs(&mut shared, 8), before);
}

#[test]
fn a_thousand_names_share_no_first_two_outputs() {
    let mut seen = BTreeSet::new();
    for i in 0..1_000 {
        let mut rng = SimRng::stream(2026, &format!("name {i}"));
        assert!(seen.insert(outputs(&mut rng, 2)), "name {i} repeats");
    }
}

#[test]
fn streams_and_forks_resume_after_a_round_trip_through_ron() {
    let mut stream = SimRng::stream(5, "loot");
    stream.next_u32();
    let mut fork = SimRng::new(5).fork("level");
    fork.next_u32();
    for rng in [&mut stream, &mut fork] {
        let mut copy: SimRng = ron::from_str(&ron::to_string(rng).unwrap()).unwrap();
        assert_eq!(outputs(&mut copy, 16), outputs(rng, 16));
    }
}

#[test]
fn an_empty_bag_draws_nothing() {
    let mut rng = scripted([]);
    let mut bag: Bag<u8> = Bag::new(vec![]);
    assert_eq!(bag.draw(&mut rng), None);
    assert_eq!((bag.left(), rng.consumed_below()), (0, 0));
}

#[test]
fn a_bag_of_one_gives_it_every_time_with_no_draw() {
    let mut rng = scripted([]);
    let mut bag = Bag::new(vec!['x']);
    for _ in 0..5 {
        assert_eq!(bag.draw(&mut rng), Some(&'x'));
    }
    assert_eq!(rng.consumed_below(), 0);
    assert_eq!(bag.items(), ['x']);
}

#[test]
fn a_bag_shuffles_as_it_refills_and_deals_from_the_end() {
    // Each refill shuffles 0..3 with two draws: [0, 0] makes [1, 2, 0].
    let mut rng = scripted([0, 0, 2, 1]);
    let mut bag = Bag::new(vec!['a', 'b', 'c']);
    let first: String = (0..3).map(|_| *bag.draw(&mut rng).unwrap()).collect();
    assert_eq!(first, "acb");
    assert_eq!((bag.left(), rng.consumed_below()), (0, 2));
    assert_eq!(bag.draw(&mut rng), Some(&'c'));
    assert_eq!((bag.left(), rng.consumed_below()), (2, 4));
}

#[test]
fn a_saved_bag_resumes_the_same() {
    let mut rng = SimRng::new(11);
    let mut bag = Bag::new((0..7).collect::<Vec<u8>>());
    for _ in 0..3 {
        bag.draw(&mut rng);
    }
    let text = ron::to_string(&(&bag, &rng)).unwrap();
    let (mut copy, mut copy_rng): (Bag<u8>, SimRng) = ron::from_str(&text).unwrap();
    assert_eq!(copy, bag);
    for _ in 0..20 {
        assert_eq!(copy.draw(&mut copy_rng), bag.draw(&mut rng));
    }
}

#[test]
fn a_bad_saved_index_is_refused() {
    let past = ron::from_str::<Bag<u8>>("(items: [1, 2], left: [0, 2])").unwrap_err();
    assert!(
        past.to_string()
            .contains("index 2 is not below its 2 items"),
        "{past}"
    );
    let twice = ron::from_str::<Bag<u8>>("(items: [1, 2], left: [1, 1])").unwrap_err();
    assert!(
        twice.to_string().contains("index 1 is left twice"),
        "{twice}"
    );
    assert!(ron::from_str::<Bag<u8>>("(items: [1, 2], left: [1, 0])").is_ok());
}

/// The values ADR-0143 freezes. A change here is a breaking change
/// (`Breaking:` line): a changed helper is a new function, never this one.
mod golden {
    use super::*;

    #[test]
    fn a_named_stream() {
        let got = outputs(&mut SimRng::stream(2026, "loot"), 8);
        let want = [
            0x8875_447c,
            0x0395_bf48,
            0x8afc_19f8,
            0x15b0_5de4,
            0xda31_1120,
            0xbf0d_9307,
            0xc646_62ef,
            0xf635_c066,
        ];
        assert_eq!(got, want);
    }

    #[test]
    fn a_fork_and_its_parent() {
        let mut parent = SimRng::new(2026);
        let mut child = parent.fork("level");
        let child_want = [
            0x1206_223c,
            0xf94b_4800,
            0xd02a_2f26,
            0xf8fd_4d2d,
            0xc54e_78c4,
            0x2572_7d13,
            0xa956_0909,
            0x859c_1781,
        ];
        let parent_want = [
            0x64b8_189d,
            0x8b04_58fd,
            0x70f1_3104,
            0x035a_0dfb,
            0xcb78_1dc3,
            0xd23a_6eb5,
            0xdea9_d284,
            0x0b6c_1610,
        ];
        assert_eq!(outputs(&mut child, 8), child_want);
        assert_eq!(outputs(&mut parent, 8), parent_want);
    }

    #[test]
    fn a_shuffle_a_bag_and_weighted_picks() {
        let mut items: Vec<u8> = (0..10).collect();
        SimRng::new(2026).shuffle(&mut items);
        assert_eq!(items, [7, 0, 1, 6, 8, 9, 3, 5, 2, 4]);

        let mut rng = SimRng::new(2026);
        let mut bag = Bag::new((0..7).collect::<Vec<u8>>());
        let dealt: Vec<u8> = (0..14).map(|_| *bag.draw(&mut rng).unwrap()).collect();
        assert_eq!(dealt, [5, 2, 0, 1, 3, 6, 4, 5, 1, 3, 0, 6, 2, 4]);

        let mut rng = SimRng::new(2026);
        let picks: Vec<usize> = (0..10)
            .map(|_| rng.weighted_index(&[3, 0, 5]).unwrap())
            .collect();
        assert_eq!(picks, [0, 0, 2, 2, 2, 2, 2, 2, 2, 0]);
    }
}

proptest! {
    #[test]
    fn range_stays_in_its_range(seed: u64, a: i32, b: i32) {
        let (lo, hi) = (a.min(b), a.max(b));
        let value = SimRng::new(seed).range(lo..hi);
        prop_assert!(lo == hi || (lo..hi).contains(&value));
    }

    #[test]
    fn shuffle_gives_a_permutation(seed: u64, len in 0usize..50) {
        let mut items: Vec<usize> = (0..len).collect();
        SimRng::new(seed).shuffle(&mut items);
        items.sort_unstable();
        prop_assert_eq!(items, (0..len).collect::<Vec<_>>());
    }

    #[test]
    fn every_len_draws_of_a_bag_hold_each_item_once(seed: u64, len in 1usize..12) {
        let mut rng = SimRng::new(seed);
        let mut bag = Bag::new((0..len).collect::<Vec<_>>());
        for _ in 0..3 {
            let mut round: Vec<usize> = (0..len).map(|_| *bag.draw(&mut rng).unwrap()).collect();
            round.sort_unstable();
            prop_assert_eq!(round, (0..len).collect::<Vec<_>>());
        }
    }

    #[test]
    fn two_names_give_two_streams(seed: u64, a in "[a-z]{1,8}", b in "[a-z]{1,8}") {
        prop_assume!(a != b);
        let first = outputs(&mut SimRng::stream(seed, &a), 16);
        prop_assert_ne!(first, outputs(&mut SimRng::stream(seed, &b), 16));
    }
}
