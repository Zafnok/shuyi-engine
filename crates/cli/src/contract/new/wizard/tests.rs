//! The wizard's flags and targets.

use super::*;
use crate::template::built_in;

#[test]
fn the_engines_concerns_each_have_a_flag_and_time_has_none() {
    assert_eq!(
        concern_flag("--perspective").as_deref(),
        Some("perspective")
    );
    assert_eq!(concern_flag("--bodies").as_deref(), Some("bodies"));
    assert_eq!(concern_flag("--kind").as_deref(), Some("kind"));
    assert_eq!(concern_flag("--time"), None);
    assert_eq!(concern_flag("--nope"), None);
    assert_eq!(concern_flag("bodies"), None);
}

#[test]
fn the_targets_are_the_engines_profiles_and_default_to_the_templates() {
    assert_eq!(default_targets(&built_in().unwrap()), ["desktop", "web"]);
    let targets = engine_targets().unwrap();
    let ready = |id: &str| targets.iter().find(|t| t.id == id).map(|t| t.ready);
    assert_eq!(ready("desktop"), Some(true));
    assert_eq!(ready("playdate"), Some(true));
    assert_eq!(ready("switch"), Some(false));
}
