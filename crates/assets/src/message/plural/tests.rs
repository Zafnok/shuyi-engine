use alloc::vec::Vec;

use proptest::prelude::*;

use super::*;

/// CLDR's integer samples: each line of `plural_samples.txt` as its tag,
/// kind, category and numbers.
fn samples() -> Vec<(&'static str, Kind, Category, Vec<u64>)> {
    let text = include_str!("../plural_samples.txt");
    let mut rows = Vec::new();
    for line in text
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let (head, numbers) = line.split_once(": ").expect(line);
        let mut head = head.split(' ');
        let (Some(tag), Some(kind), Some(category)) = (head.next(), head.next(), head.next())
        else {
            panic!("{line}");
        };
        let kind = match kind {
            "cardinal" => Kind::Cardinal,
            "ordinal" => Kind::Ordinal,
            _ => panic!("{line}"),
        };
        let category = Category::from_name(category).expect(line);
        let mut all = Vec::new();
        for item in numbers.split(", ") {
            if let Some((from, to)) = item.split_once('~') {
                all.extend(from.parse::<u64>().unwrap()..=to.parse().unwrap());
            } else {
                all.push(item.parse().unwrap());
            }
        }
        rows.push((tag, kind, category, all));
    }
    rows
}

#[test]
fn every_cldr_sample_gets_its_category() {
    let rows = samples();
    assert!(rows.len() > 40, "{}", rows.len());
    for (tag, kind, category, numbers) in rows {
        for n in numbers {
            let got = match kind {
                Kind::Cardinal => cardinal(tag, n),
                Kind::Ordinal => ordinal(tag, n),
            };
            assert_eq!(got, category, "{tag} {kind:?} {n}");
        }
    }
}

#[test]
fn every_language_of_the_table_has_samples_of_each_of_its_categories() {
    let rows = samples();
    for &(tag, ..) in TABLE {
        for kind in [Kind::Cardinal, Kind::Ordinal] {
            // `other` of ru, uk and pl holds decimals only: no whole
            // number is `other` there.
            let whole = categories(tag, kind)
                .iter()
                .filter(|c| **c != Category::Other);
            for &category in whole {
                let found = rows
                    .iter()
                    .any(|r| r.0 == tag && r.1 == kind && r.2 == category);
                assert!(found, "{tag} {kind:?} {category:?}: no samples");
            }
        }
    }
}

#[test]
fn russian_picks_one_few_and_many_and_french_counts_zero_as_one() {
    let ru = |n| cardinal("ru", n);
    assert_eq!((ru(1), ru(21)), (Category::One, Category::One));
    assert_eq!((ru(2), ru(22)), (Category::Few, Category::Few));
    assert_eq!((ru(5), ru(11)), (Category::Many, Category::Many));
    assert_eq!(cardinal("fr", 0), Category::One);
    assert_eq!(cardinal("fr", 1), Category::One);
    assert_eq!(cardinal("en", 0), Category::Other);
}

#[test]
fn a_tag_uses_its_own_row_then_its_languages_then_other_only() {
    assert_eq!(cardinal("pt-BR", 0), Category::One);
    assert_eq!(cardinal("pt-PT", 0), Category::Other);
    assert_eq!(cardinal("ru-UA", 21), Category::One);
    assert_eq!(cardinal("nl", 1), Category::Other);
    assert_eq!(categories("nl", Kind::Cardinal), [Category::Other]);
    assert_eq!(categories("zh-Hans", Kind::Ordinal), [Category::Other]);
}

#[test]
fn a_category_is_named_as_cldr_names_it() {
    for &category in Category::ALL {
        assert_eq!(Category::from_name(category.name()), Some(category));
    }
    assert_eq!(Category::from_name("several"), None);
}

proptest! {
    #[test]
    fn a_number_falls_in_a_category_its_language_has(n: u64, row in 0..TABLE.len() + 1) {
        let tag = TABLE.get(row).map_or("nl", |r| r.0);
        prop_assert!(categories(tag, Kind::Cardinal).contains(&cardinal(tag, n)));
        prop_assert!(categories(tag, Kind::Ordinal).contains(&ordinal(tag, n)));
        prop_assert_eq!(categories(tag, Kind::Cardinal).last(), Some(&Category::Other));
    }
}
