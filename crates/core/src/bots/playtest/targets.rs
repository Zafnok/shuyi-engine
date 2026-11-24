//! Playtest targets by tier: what each entrant should achieve on a stage
//! of each of the game's tiers, turned into the [`Check`]s of one stage.
//!
//! A stage may also be of kinds the game names (a time trial, a hold):
//! each kind's bands lie over the tier's, entrant by entrant, and a kind
//! can leave the medians out, for a stage that lasts the same however it
//! is played ([`Targets::checks_of`]).

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
    /// What changes on a stage of each kind the game names, by its name.
    #[serde(default)]
    pub kinds: BTreeMap<String, KindTargets<T>>,
}

/// What changes on a stage of one kind ([`Targets::kinds`]): its own
/// bands, laid over the tier's entrant by entrant, and whether its medians
/// are checked. Build it with [`KindTargets::new`] (ADR-0039).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    deny_unknown_fields,
    bound(
        serialize = "T: Ord + Serialize",
        deserialize = "T: Ord + Deserialize<'de>"
    )
)]
#[non_exhaustive]
pub struct KindTargets<T: Ord> {
    /// The share of successes wanted, per tier and entrant: an entrant or
    /// a tier it leaves out keeps the tier's band.
    #[serde(default)]
    pub bands: BTreeMap<T, BTreeMap<String, Band>>,
    /// Whether a stage of this kind checks the medians; true when the file
    /// leaves it out.
    #[serde(default = "checked")]
    pub medians: bool,
}

/// The default of [`KindTargets::medians`].
const fn checked() -> bool {
    true
}

impl<T: Ord> KindTargets<T> {
    /// A kind with its own `bands`, checking the medians if `medians`.
    pub fn new(bands: BTreeMap<T, BTreeMap<String, Band>>, medians: bool) -> Self {
        Self { bands, medians }
    }
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
        self.checks_of(tier, &[], entrants)
    }

    /// The checks of a stage of `tier` and of `kinds`, in that order, that
    /// `entrants` play, as [`checks`](Self::checks) gives them but with
    /// each kind's bands for `tier` over the tier's, entrant by entrant (a
    /// later kind wins), and no median if any of `kinds` leaves them out.
    /// A kind the targets don't name changes nothing.
    pub fn checks_of(&self, tier: &T, kinds: &[&str], entrants: &[&str]) -> Vec<Check> {
        let named = || kinds.iter().filter_map(|&kind| self.kinds.get(kind));
        let mut bands: BTreeMap<&str, Band> = BTreeMap::new();
        for over in core::iter::once(self.bands.get(tier))
            .chain(named().map(|kind| kind.bands.get(tier)))
            .flatten()
        {
            bands.extend(over.iter().map(|(name, band)| (name.as_str(), *band)));
        }
        let rates = entrants.iter().filter_map(|&name| {
            let success = self.success.get(name)?;
            let band = bands.get(name)?;
            Some(Check::Rate {
                entrant: name.into(),
                success: success.clone(),
                band: *band,
                floor: self.floors.get(name).copied(),
            })
        });
        let checked = named().all(|kind| kind.medians);
        let medians = self
            .medians
            .iter()
            .filter(|_| checked)
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
    /// once: a misspelt entrant would otherwise just get no check. It
    /// lists every unknown name at once, for a lint over the whole file;
    /// [`judge`](super::judge) refuses only the first, with
    /// [`UnknownEntrant`](super::UnknownEntrant).
    pub fn unknown_entrants(&self, known: &[&str]) -> Vec<String> {
        let named = self
            .success
            .keys()
            .chain(self.bands.values().flat_map(BTreeMap::keys))
            .chain(
                self.kinds
                    .values()
                    .flat_map(|kind| kind.bands.values().flat_map(BTreeMap::keys)),
            )
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
