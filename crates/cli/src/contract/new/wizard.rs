//! The new-game wizard's answers by flag (ADR-0013 §1, §2): each concern
//! of the catalog (`docs/modules.ron`) that `shuyi new` asks, all but
//! `time`, has a flag of its id ([`concern_flag`]: `--perspective`,
//! `--bodies`…), and `--targets` takes the engine's target profiles
//! ([`engine_targets`]), the template's always-on parts' targets by
//! default ([`default_targets`]). The questions themselves, listed for an
//! agent, are ticket 3424's; asked in a terminal, 3423's.

use crate::catalog::Concerns;
use crate::template::{self, Template};

/// A target an engine profile adds, as `--targets` takes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// The profile's id (`playdate`).
    pub id: String,
    /// Whether the engine runs on it yet: a planned one is never picked.
    pub ready: bool,
}

/// The targets `template` ships every game on: its parts' that always
/// apply.
pub fn default_targets(template: &Template) -> Vec<String> {
    let mut targets: Vec<String> = Vec::new();
    for part in template
        .parts
        .iter()
        .filter(|p| p.when == template::When::Always)
    {
        for target in &part.targets {
            if !targets.contains(target) {
                targets.push(target.clone());
            }
        }
    }
    targets
}

/// The engine's target profiles, by id.
pub fn engine_targets() -> Result<Vec<Target>, String> {
    let profiles = crate::contract::target::engine_profiles().map_err(|errors| {
        let said: Vec<String> = errors.iter().map(ToString::to_string).collect();
        said.join("; ")
    })?;
    Ok(profiles
        .into_iter()
        .map(|(id, p)| Target {
            id,
            ready: p.profile.status == shuyi_assets::target::Status::Supported,
        })
        .collect())
}

/// The concern `flag` (`--bodies`) answers, if it is one `shuyi new` asks
/// of the engine's catalog as this `shuyi` embeds it.
pub fn concern_flag(flag: &str) -> Option<String> {
    let id = flag.strip_prefix("--")?;
    let concerns = Concerns::embedded().ok()?;
    template::asked(&concerns, id).map(|c| c.id.clone())
}

#[cfg(test)]
mod tests;
