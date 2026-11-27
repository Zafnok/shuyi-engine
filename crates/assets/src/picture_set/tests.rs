use super::*;

/// The names these tests require.
const REQUIRED: &[&str] = &["neutral", "happy", "upset"];
/// The rules of these tests: those names, each picture at most 64×48.
const RULES: PictureRules<'static> = PictureRules::new(REQUIRED, (64, 48));

/// A good set: the required names out of order, and two others.
const ANA: &str = r#"(
    id: "ana",
    pictures: {
        "wink": "ana/wink.png",
        "upset": "ana/frown.png",
        "neutral": "ana/neutral.png",
        "bored": "ana/bored.png",
        "happy": "ana/smile.png",
    },
)"#;

/// A bad set: no `upset`, the id `bo`, `happy` not in the table and
/// `neutral` too big.
const BAD: &str = r#"(
    id: "bo",
    pictures: {
        "neutral": "ana/huge.png",
        "happy": "ana/gone.png",
    },
)"#;

fn images() -> ImageTable {
    let size = ImageInfo::new(64, 48);
    ImageTable::new(
        [
            ("portraits/ana/wink.png", size),
            ("portraits/ana/frown.png", size),
            ("portraits/ana/neutral.png", ImageInfo::new(32, 32)),
            ("portraits/ana/bored.png", size),
            ("portraits/ana/smile.png", size),
            ("portraits/ana/huge.png", ImageInfo::new(64, 49)),
        ]
        .into(),
    )
}

fn texts(errors: &[ContentError]) -> Vec<String> {
    errors.iter().map(ToString::to_string).collect()
}

fn ana() -> PictureSet {
    PictureSetDef::from_source("portraits/ana.ron", "ana", ANA, &RULES)
        .and_then(|def| def.resolve("portraits/ana.ron", "portraits", &images(), None, &RULES))
        .unwrap_or_else(|e| panic!("{e:?}"))
}

#[test]
fn a_good_set_lists_the_required_names_first_then_the_others_by_name() {
    let set = ana();
    assert_eq!(set.id, "ana");
    let names: Vec<&str> = set.pictures.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["neutral", "happy", "upset", "bored", "wink"]);
    let happy = set.get("happy").unwrap_or_else(|| panic!("no happy"));
    assert_eq!(happy.image.path(), "portraits/ana/smile.png");
    assert_eq!(happy.size, ImageInfo::new(64, 48));
    assert_eq!(
        set.get("neutral").map(|p| p.size),
        Some(ImageInfo::new(32, 32))
    );
    assert_eq!(set.get("sad"), None);
}

#[test]
fn a_bad_set_reports_each_problem_the_images_at_their_lines() {
    // The file alone: the id and the missing name.
    let file = "portraits/ana.ron";
    let errors = PictureSetDef::from_source(file, "ana", BAD, &RULES).err();
    assert_eq!(
        texts(&errors.unwrap_or_default()),
        [
            "portraits/ana.ron: id \"bo\" isn't the file's name \"ana\"",
            "portraits/ana.ron: missing required picture \"upset\"",
        ]
    );
    // Its pictures, against the table: the missing image and the big one.
    let def = PictureSetDef::from_source(file, "bo", BAD, &PictureRules::new(&[], (64, 48)))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let errors = def
        .resolve(file, "portraits", &images(), None, &RULES)
        .err();
    let errors = errors.unwrap_or_default();
    assert_eq!(
        texts(&errors),
        [
            "portraits/ana.ron:5: picture \"happy\": portraits/ana/gone.png is not an image in the assets",
            "portraits/ana.ron:4: picture \"neutral\": portraits/ana/huge.png is 64×49 px; a picture must fit 64×48 px",
        ]
    );
    let codes: Vec<_> = errors.iter().map(|e| e.code).collect();
    assert_eq!(
        codes,
        [Some(codes::MISSING_FILE), Some(codes::OUT_OF_RANGE)]
    );
}

#[test]
fn a_bundle_names_the_images_as_it_shows_them() {
    let def = PictureSetDef::from_source("a.ron", "bo", BAD, &PictureRules::new(&[], (64, 48)))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let bundle = crate::format::builtin_tests::bundle(vec![]);
    let errors = def.resolve("a.ron", "portraits", &images(), Some(bundle), &RULES);
    let text = texts(&errors.err().unwrap_or_default());
    assert!(
        text[0].contains(" base/portraits/ana/gone.png "),
        "{text:?}"
    );
}

#[test]
fn a_name_in_the_id_line_does_not_take_a_pictures_line() {
    // The set's id is also a picture's name: its line is the picture's.
    let source = "(\n    id: \"happy\",\n    pictures: {\n        \"happy\": \"x.png\",\n    },\n)";
    let def = PictureSetDef::from_source(
        "happy.ron",
        "happy",
        source,
        &PictureRules::new(&[], (1, 1)),
    )
    .unwrap_or_else(|e| panic!("{e:?}"));
    let errors = def.resolve("happy.ron", "", &ImageTable::default(), None, &RULES);
    assert_eq!(errors.err().unwrap_or_default()[0].line, Some(4));
}

#[test]
fn a_file_that_is_not_a_set_is_one_error() {
    let unknown = "(id: \"ana\", pictures: {}, poses: {})";
    for source in ["(id: \"ana\"", unknown] {
        let errors = PictureSetDef::from_source("a.ron", "ana", source, &RULES).err();
        assert_eq!(errors.map(|e| e.len()), Some(1), "{source}");
    }
}

#[test]
fn a_set_built_by_hand_resolves_without_lines() {
    let def = PictureSetDef::new("ana".into(), [("happy".into(), "gone.png".into())].into());
    let errors = def.resolve("a.ron", "", &images(), None, &RULES);
    assert_eq!(errors.err().unwrap_or_default()[0].line, None);
}

#[test]
fn name_problem_names_the_first_set_without_the_name() {
    let ana = ana();
    let other = PictureSet::new("bo", Vec::new());
    assert_eq!(name_problem(&[&ana], "happy", &[]), None);
    assert_eq!(
        name_problem(&[&ana, &other], "happy", &[]),
        Some(NameProblem::NotInSet(&other))
    );
    assert_eq!(
        name_problem(&[&other, &ana], "sad", &[]),
        Some(NameProblem::NotInSet(&other))
    );
    let problem = name_problem(&[&ana], "sad", &[]).map(|p| p.to_string());
    assert_eq!(
        problem.as_deref(),
        Some("\"ana\" has no picture of that name; it has: neutral, happy, upset, bored, wink")
    );
}

#[test]
fn name_problem_without_a_set_reads_the_standard_list() {
    const STANDARD: &[&str] = &["neutral", "happy"];
    assert_eq!(name_problem(&[], "happy", STANDARD), None);
    assert_eq!(
        name_problem(&[], "sad", STANDARD),
        Some(NameProblem::NotAllowed(STANDARD))
    );
    assert_eq!(
        name_problem(&[], "sad", STANDARD)
            .map(|p| p.to_string())
            .as_deref(),
        Some("with no picture set, the name must be one of: neutral, happy")
    );
    // A set in hand, the standard list doesn't count.
    let ana = ana();
    assert_eq!(name_problem(&[&ana], "wink", &[]), None,);
    assert!(name_problem(&[&PictureSet::new("bo", Vec::new())], "happy", STANDARD).is_some());
}
