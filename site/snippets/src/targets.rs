//! Chapter 9, targets and export: the project file a game ships with
//! (`project/shuyi.ron` here, `shuyi.ron` in a game's folder), its
//! targets and its web build, checked against the engine's target
//! profiles. The link argument the web build needs is this crate's
//! `build.rs`.

#[cfg(test)]
mod tests {
    use std::path::Path;

    use shuyi_cli::contract::target::{Level, engine_profiles};
    use shuyi_cli::project::Config;

    #[test]
    fn the_project_file_reads_and_names_the_engines_targets() {
        let config = Config::parse(include_str!("../project/shuyi.ron")).unwrap();
        let profiles = engine_profiles().unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(config.targets.len(), 2);
        for id in config.targets.keys() {
            assert!(profiles.contains_key(id), "{id}");
        }
        let web = config.web.unwrap();
        assert_eq!(web.dist, "dist/web");
    }

    #[test]
    fn who_the_game_is_warns_of_the_placeholder_id() {
        let config = Config::parse(include_str!("../project/shuyi.ron")).unwrap();
        let app = config.app.unwrap();
        // The snippets write no `code` here, so it is out of date.
        let found = shuyi_cli::app_info::problems(&app, Path::new("."), "cargo shuyi");
        let settings: Vec<&str> = found.iter().map(|p| p.setting.as_str()).collect();
        assert_eq!(settings, ["app.id", "app.code"]);
        assert_eq!(found[0].level, Level::Warning);
        // The quick start's game is this one.
        let quick = crate::quick_start::APP;
        assert_eq!((quick.name, quick.id), (app.name.as_str(), app.id.as_str()));
        assert_eq!(Some(quick.version), app.version.as_deref());
    }
}
