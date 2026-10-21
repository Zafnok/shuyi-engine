use super::*;

/// Every pattern: each base, plain and inverted.
fn every_pattern() -> Vec<Pattern> {
    Base::ALL
        .iter()
        .flat_map(|&base| [Pattern::new(base), Pattern::new(base).inverted()])
        .collect()
}

/// How many pixels of `rows` are black.
fn black(rows: Rows) -> u32 {
    rows.iter().map(|row| row.count_zeros()).sum()
}

#[test]
fn each_patterns_black_pixels_match_its_density() {
    let densities = [
        (Base::Black, 64),
        (Base::White, 0),
        (Base::Dither25, 16),
        (Base::Dither50, 32),
        (Base::Dither75, 48),
        (Base::DiagonalUp, 16),
        (Base::DiagonalDown, 16),
        (Base::HLines, 16),
        (Base::VLines, 16),
        (Base::Checker2, 32),
    ];
    for (base, n) in densities {
        assert_eq!(base.black_pixels(), n, "{}", base.name());
        assert_eq!(black(base.rows()), n, "{}", base.name());
        let inverted = Pattern::new(base).inverted();
        assert_eq!(inverted.black_pixels(), 64 - n);
        assert_eq!(black(inverted.rows()), 64 - n);
    }
    for pattern in every_pattern() {
        assert_eq!(black(pattern.rows()), pattern.black_pixels(), "{pattern}");
    }
}

#[test]
fn the_patterns_are_drawn_as_named() {
    assert_eq!(Base::Black.rows(), [0; 8]);
    assert_eq!(Base::White.rows(), [0xff; 8]);
    // Half Bayer is a checker of single pixels.
    assert_eq!(
        Base::Dither50.rows(),
        [0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa]
    );
    assert_eq!(
        Base::Checker2.rows(),
        [0x33, 0x33, 0xcc, 0xcc, 0x33, 0x33, 0xcc, 0xcc]
    );
    assert_eq!(
        Base::HLines.rows(),
        [0, 0xff, 0xff, 0xff, 0, 0xff, 0xff, 0xff]
    );
    assert_eq!(Base::VLines.rows(), [0x77; 8]);
    // A line rising to the right: one pixel left on the row below.
    assert_eq!(Base::DiagonalUp.rows()[..4], [0x77, 0xee, 0xdd, 0xbb]);
    assert_eq!(Base::DiagonalDown.rows()[..4], [0x77, 0xbb, 0xdd, 0xee]);
    // The falling diagonal is the game's `Diagonals` ink.
    for y in 0..8u8 {
        for x in 0..8u8 {
            let white = Base::DiagonalDown.rows()[usize::from(y)] >> (7 - x) & 1 == 1;
            let ink = (i32::from(x) - i32::from(y)).rem_euclid(4) != 0;
            assert_eq!(white, ink, "({x}, {y})");
        }
    }
    // A quarter dither's black pixels are a half dither's, and those a
    // three-quarter's: each darker level only adds black.
    let darker = |a: Base, b: Base| a.rows().iter().zip(b.rows()).all(|(a, b)| b & !a == 0);
    assert!(darker(Base::Dither25, Base::Dither50));
    assert!(darker(Base::Dither50, Base::Dither75));
}

#[test]
fn the_luminance_dither_falls_evenly_through_the_named_dithers() {
    assert_eq!(dither(0), Base::White.rows());
    assert_eq!(dither(255), Base::Black.rows());
    assert_eq!(dither(64), Base::Dither25.rows());
    assert_eq!(dither(128), Base::Dither50.rows());
    assert_eq!(dither(192), Base::Dither75.rows());
    let mut last = dither(0);
    for darkness in 1..=255u8 {
        let rows = dither(darkness);
        // Never lighter than the level before, and one more black pixel
        // every 4 levels.
        assert!(
            last.iter().zip(rows).all(|(a, b)| b & !a == 0),
            "{darkness}"
        );
        assert_eq!(black(rows), (u32::from(darkness) + 1) / 4, "{darkness}");
        last = rows;
    }
}

#[test]
fn patterns_look_alike_when_one_is_the_other_moved() {
    let p = |base| Pattern::new(base);
    for pattern in every_pattern() {
        assert!(pattern.looks_like(pattern), "{pattern}");
    }
    assert!(p(Base::Dither50).looks_like(p(Base::Dither50).inverted()));
    assert!(p(Base::Checker2).looks_like(p(Base::Checker2).inverted()));
    assert!(Pattern::BLACK.looks_like(Pattern::WHITE.inverted()));
    assert!(!p(Base::DiagonalUp).looks_like(p(Base::DiagonalDown)));
    assert!(!p(Base::HLines).looks_like(p(Base::VLines)));
    assert!(!p(Base::Dither25).looks_like(p(Base::Dither25).inverted()));
    assert!(!p(Base::Dither50).looks_like(p(Base::Checker2)));
    assert!(!Pattern::BLACK.looks_like(Pattern::WHITE));
    // Of the 20 patterns, only these pairs look alike: a solid and the
    // other inverted, a dither and the opposite one inverted (Bayer's
    // order is the same upside down, moved), and the two checkers
    // inverted.
    let all = every_pattern();
    let alike: Vec<String> = all
        .iter()
        .enumerate()
        .flat_map(|(i, a)| {
            all[i + 1..]
                .iter()
                .filter(move |b| a.looks_like(**b))
                .map(move |b| format!("{a} ~ {b}"))
        })
        .collect();
    assert_eq!(
        alike,
        [
            "Black ~ Inverted(White)",
            "Inverted(Black) ~ White",
            "Dither25 ~ Inverted(Dither75)",
            "Inverted(Dither25) ~ Dither75",
            "Dither50 ~ Inverted(Dither50)",
            "Checker2 ~ Inverted(Checker2)",
        ]
    );
}

#[test]
fn a_pattern_is_spelled_as_its_base_or_inverted() {
    for pattern in every_pattern() {
        let spelled = pattern.to_string();
        let read: Pattern = ron::from_str(&spelled).unwrap();
        assert_eq!(read, pattern, "{spelled}");
        assert_eq!(ron::to_string(&pattern).unwrap(), spelled);
    }
    assert_eq!(
        Pattern::new(Base::Dither50).inverted().to_string(),
        "Inverted(Dither50)"
    );
    let twice: Pattern = ron::from_str("Inverted(Inverted(HLines))").unwrap();
    assert_eq!(twice, Pattern::from(Base::HLines));
    assert!(!twice.is_inverted());
    assert!(Pattern::from(Base::HLines).inverted().is_inverted());
    assert_eq!(twice.base(), Base::HLines);
    assert!(Pattern::WHITE.is_solid());
    assert!(Pattern::WHITE.inverted().is_solid());
    assert!(!Pattern::new(Base::Dither75).is_solid());
    assert!(ron::from_str::<Pattern>("Grey").is_err());
}

/// The example map of the module's doc.
const MAP: &str = r#"(
    roles: {
        "text": (fg: Black, bg: White),
        "team.player": (fg: White),
        "team.enemy": (fg: Dither50),
        "team.ally": (fg: Inverted(DiagonalUp)),
    },
    distinct: [["team.player", "team.enemy", "team.ally"]],
    text: ["text"],
)"#;

fn palette(roles: &[(&str, [u8; 3])]) -> PaletteDef {
    PaletteDef {
        colors: roles.iter().map(|&(n, c)| (n.to_owned(), c)).collect(),
    }
}

/// The palette [`MAP`] is for.
fn teams() -> PaletteDef {
    palette(&[
        ("text", [250, 250, 250]),
        ("team.player", [0, 0, 255]),
        ("team.enemy", [255, 0, 0]),
        ("team.ally", [0, 255, 0]),
    ])
}

fn map(source: &str) -> PatternMapDef {
    PatternMapDef::from_source("p@onebit.ron", source).unwrap_or_else(|e| panic!("{e:?}"))
}

#[test]
fn a_pattern_map_reads_roles_groups_and_text_roles() {
    let read = map(MAP);
    assert_eq!(read.roles.len(), 4);
    let player = read.get("team.player").unwrap();
    assert_eq!(player.fg, Pattern::WHITE);
    assert_eq!(player.bg, None);
    // No background: the foreground again.
    assert_eq!(player.background(), Pattern::WHITE);
    assert_eq!(
        read.get("text"),
        Some(&RolePatterns::new(Pattern::BLACK, Pattern::WHITE))
    );
    assert_eq!(
        read.get("team.ally").unwrap().fg,
        Pattern::new(Base::DiagonalUp).inverted()
    );
    assert_eq!(read.distinct, [["team.player", "team.enemy", "team.ally"]]);
    assert_eq!(read.text, ["text"]);
    assert!(read.get("nope").is_none());
    // Only roles are needed.
    assert_eq!(map("(roles: {})"), PatternMapDef::default());
}

#[test]
fn a_bad_pattern_map_is_refused_with_every_error() {
    let errors = |source| {
        PatternMapDef::from_source("p@onebit.ron", source)
            .err()
            .unwrap_or_default()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        errors(r#"(roles: {}, distinct: [["a"], ["b", "c", "b", "b"]])"#),
        [
            "p@onebit.ron: distinct group 1 [a] needs two roles or more",
            "p@onebit.ron: distinct group 2 names \"b\" twice",
            "p@onebit.ron: distinct group 2 names \"b\" twice",
        ]
    );
    let unknown = errors("(\n  roles: {\"a\": (fg: Grey)},\n)");
    assert_eq!(unknown.len(), 1);
    assert!(unknown[0].starts_with("p@onebit.ron:2:"), "{unknown:?}");
    assert_eq!(errors("(roles: {}, colours: [])").len(), 1);
}

#[test]
fn the_format_reads_a_map_as_from_source_does() {
    let file = AssetPath::new("skin/palette@onebit.ron");
    let read = PatternMapRon.decode(&file, MAP.as_bytes(), &DecodeCx::new());
    assert_eq!(read, PatternMapDef::from_source(file.shown(), MAP));
    assert_eq!(PatternMapRon.id(), "pattern-map");
    assert_eq!(PatternMapRon.extensions(), ["ron"]);
    assert_eq!(PatternMapRon.max_bytes(), TEXT_MAX_BYTES);
    assert!(
        PatternMapRon
            .decode(&file, &[0xff], &DecodeCx::new())
            .is_err()
    );
}

#[test]
fn a_palettes_map_is_beside_it() {
    assert_eq!(map_path("skin/palette.ron"), "skin/palette@onebit.ron");
    assert_eq!(map_path("palette.ron"), "palette@onebit.ron");
    assert_eq!(map_path("a/b/colours"), "a/b/colours@onebit.ron");
}

#[test]
fn a_map_that_keeps_everything_apart_has_no_problems() {
    assert_eq!(check(&teams(), &map(MAP)), []);
}

#[test]
fn two_roles_of_a_group_on_one_pattern_fail_naming_both() {
    let same = MAP.replace("(fg: White)", "(fg: Dither50)");
    let problems = check(&teams(), &map(&same));
    assert_eq!(problems.len(), 1, "{problems:?}");
    let problem = &problems[0];
    assert_eq!(problem.severity, Severity::Error);
    assert_eq!(problem.rule, Rule::Distinct);
    assert_eq!(problem.roles, ["team.player", "team.enemy"]);
    assert_eq!(
        problem.message,
        "roles \"team.player\" and \"team.enemy\" must stay distinct, but their foregrounds look \
         the same in one bit (both Dither50)"
    );
    // A pattern moved is the same pattern.
    let moved = MAP.replace("(fg: White)", "(fg: Inverted(Dither50))");
    let problems = check(&teams(), &map(&moved));
    assert_eq!(problems.len(), 1);
    assert!(
        problems[0]
            .message
            .ends_with("(Inverted(Dither50) and Dither50 are the same pattern moved)"),
        "{}",
        problems[0].message
    );
    // Backgrounds may match: only foregrounds tell things apart.
    let backgrounds = MAP.replace("(fg: White)", "(fg: White, bg: Dither50)");
    assert_eq!(check(&teams(), &map(&backgrounds)), []);
}

#[test]
fn every_role_needs_patterns_and_every_named_role_a_colour() {
    let mut palette = teams();
    palette.colors.insert("terrain.water".into(), [0, 0, 90]);
    palette.colors.remove("team.ally");
    let shown: Vec<(Rule, Vec<String>)> = check(&palette, &map(MAP))
        .into_iter()
        .map(|p| (p.rule, p.roles))
        .collect();
    assert_eq!(
        shown,
        [
            (Rule::Missing, vec!["terrain.water".to_owned()]),
            (Rule::Unknown, vec!["team.ally".to_owned()]),
            (Rule::Unknown, vec!["team.ally".to_owned()]),
        ]
    );
    let messages: Vec<String> = check(&palette, &map(MAP))
        .into_iter()
        .map(|p| p.message)
        .collect();
    assert_eq!(
        messages,
        [
            "role \"terrain.water\" has no patterns",
            "the map names role \"team.ally\", which the palette doesn't have",
            "a distinct group names role \"team.ally\", which the palette doesn't have",
        ]
    );
}

#[test]
fn roles_of_one_colour_must_share_their_patterns() {
    let mut palette = teams();
    palette.colors.insert("team.ally".into(), [255, 0, 0]);
    let problems = check(&palette, &map(MAP));
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].rule, Rule::SharedColour);
    assert_eq!(problems[0].roles, ["team.ally", "team.enemy"]);
    assert!(problems[0].message.contains("both #ff0000 in the palette"));
    // The same patterns are fine, the background said or not.
    let same = MAP.replace("(fg: Inverted(DiagonalUp))", "(fg: Dither50, bg: Dither50)");
    let mut same = map(&same);
    same.distinct.clear();
    assert_eq!(check(&palette, &same), []);
}

#[test]
fn text_is_black_on_white_or_white_on_black_or_a_warning() {
    let inverse = MAP.replace("(fg: Black, bg: White)", "(fg: White, bg: Black)");
    assert_eq!(check(&teams(), &map(&inverse)), []);
    for (patterns, shown) in [
        ("(fg: Black)", "Black on Black"),
        ("(fg: Dither75, bg: White)", "Dither75 on White"),
    ] {
        let source = MAP.replace("(fg: Black, bg: White)", patterns);
        let problems = check(&teams(), &map(&source));
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert_eq!(problems[0].severity, Severity::Warning);
        assert_eq!(problems[0].rule, Rule::Text);
        assert_eq!(problems[0].roles, ["text"]);
        assert_eq!(
            problems[0].message,
            format!(
                "text role \"text\" is {shown}: text reads best Black on White or White on Black"
            )
        );
    }
}
