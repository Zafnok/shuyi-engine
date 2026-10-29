//! Chapter 9, targets and export: the project file a game ships with
//! (`project/shuyi.ron` here, `shuyi.ron` in a game's folder), its
//! targets and its web build, checked against the engine's target
//! profiles. The link argument the web build needs is this crate's
//! `build.rs`.

#[cfg(test)]
mod tests {
    use shuyi_cli::contract::target::engine_profiles;
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
}
