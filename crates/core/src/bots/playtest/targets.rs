//! Playtest targets by tier: what each entrant should achieve on a stage
//! of each of the game's tiers, turned into the [`Check`]s of one stage.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use super::{Band, Check, Condition, Want};

/// What each entrant should achieve, over the game's own tier type `T`
/// (whatever its stages are labelled with). [`Targets::checks`] gives a
/// stage's checks from it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    deny_unknown_fields,
    bound(
        serialize = "T: Ord + Serialize",
        deserialize = "T: Ord + Deserialize<'de>"
    )
)]
pub struct Targets<T: Ord> {
    /// What makes a try a success, per entrant.
    pub success: BTreeMap<String, Vec<Condition>>,
    /// The share of successes wanted, per tier and entrant.
    #[serde(default)]
    pub bands: BTreeMap<T, BTreeMap<String, Band>>,
    /// The share under which a run is blocked, in percent, per entrant
    /// that has one.
    #[serde(default)]
    pub floors: BTreeMap<String, u32>,
    /// Medians compared between two entrants, on a stage of any tier.
    #[serde(default)]
    pub medians: Vec<MedianTarget>,
}

/// A [`Check::Median`] wanted wherever both entrants play.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MedianTarget {
    /// The entrant checked.
    pub entrant: String,
    /// The measure.
    pub measure: String,
    /// Which tries count.
    #[serde(default)]
    pub among: Vec<Condition>,
    /// Which way it should lean.
    pub want: Want,
    /// The entrant compared with.
    pub than: String,
}

impl<T: Ord> Targets<T> {
    /// The checks of a stage of `tier` that `entrants` play: a
    /// [`Check::Rate`] per entrant, in the order given, that has both a
    /// success rule and a band for `tier`, then each median, in file
    /// order, whose two entrants both play. Every check names only
    /// entrants of `entrants`.
    pub fn checks(&self, tier: &T, entrants: &[&str]) -> Vec<Check> {
        let bands = self.bands.get(tier);
        let rates = entrants.iter().filter_map(|&name| {
            let success = self.success.get(name)?;
            let band = bands?.get(name)?;
            Some(Check::Rate {
                entrant: name.into(),
                success: success.clone(),
                band: *band,
                floor: self.floors.get(name).copied(),
            })
        });
        let medians = self
            .medians
            .iter()
            .filter(|m| {
                entrants.contains(&m.entrant.as_str()) && entrants.contains(&m.than.as_str())
            })
            .map(|m| Check::Median {
                entrant: m.entrant.clone(),
                measure: m.measure.clone(),
                among: m.among.clone(),
                want: m.want,
                than: m.than.clone(),
            });
        rates.chain(medians).collect()
    }

    /// The names the targets use that aren't in `known`, sorted, each
    /// once: a misspelt entrant would otherwise just get no check.
    pub fn unknown_entrants(&self, known: &[&str]) -> Vec<String> {
        let named = self
            .success
            .keys()
            .chain(self.bands.values().flat_map(BTreeMap::keys))
            .chain(self.floors.keys())
            .chain(self.medians.iter().flat_map(|m| [&m.entrant, &m.than]));
        named
            .filter(|name| !known.contains(&name.as_str()))
            .cloned()
            .collect::<BTreeSet<String>>()
            .into_iter()
            .collect()
    }
}
