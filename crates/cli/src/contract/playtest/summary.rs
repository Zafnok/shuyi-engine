//! A plan's summary items ([`Summary`]): the few numbers of an entrant's
//! tries a designer reads first, each over the game's named measures, as
//! data a game may keep in a file.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use shuyi_core::bots::playtest::Tried;

/// One number of an entrant's tries, as `playtest` prints it after the
/// entrant's name. A try missing the measure counts it as 0. `label`
/// defaults to the measure's name (`"most"` for [`Most`](Summary::Most)).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub enum Summary {
    /// "won 83/100": the sum of `measure` over the tries, of how many.
    #[non_exhaustive]
    Sum {
        /// The measure.
        measure: String,
        /// The name it is printed with.
        #[serde(default)]
        label: Option<String>,
    },
    /// "deaths per try 0: 30  1: 21  2: 25  3+: 24": tries by value, the
    /// last bucket open.
    #[non_exhaustive]
    Spread {
        /// The measure.
        measure: String,
        /// The value of the last bucket, which takes it and more.
        up_to: u32,
        /// The name it is printed with.
        #[serde(default)]
        label: Option<String>,
    },
    /// "most: scout 41×": among keys `"<prefix><key>"` set to 1 or more
    /// (keys with a further `skip_suffix` left out), the key set in the
    /// most tries, then the first by name; "nobody" if none.
    #[non_exhaustive]
    Most {
        /// What the keys start with.
        prefix: String,
        /// What a key left out ends with.
        #[serde(default)]
        skip_suffix: Option<String>,
        /// The name it is printed with.
        #[serde(default)]
        label: Option<String>,
    },
    /// "items median 1": the lower middle value, 0 for no tries.
    #[non_exhaustive]
    Median {
        /// The measure.
        measure: String,
        /// The name it is printed with.
        #[serde(default)]
        label: Option<String>,
    },
    /// "items max 3": the greatest value, 0 for no tries.
    #[non_exhaustive]
    Max {
        /// The measure.
        measure: String,
        /// The name it is printed with.
        #[serde(default)]
        label: Option<String>,
    },
}

impl Summary {
    /// A [`Sum`](Summary::Sum) of `measure`.
    pub fn sum(measure: impl Into<String>) -> Self {
        Summary::Sum {
            measure: measure.into(),
            label: None,
        }
    }

    /// A [`Spread`](Summary::Spread) of `measure`, its last bucket
    /// `up_to` and more.
    pub fn spread(measure: impl Into<String>, up_to: u32) -> Self {
        Summary::Spread {
            measure: measure.into(),
            up_to,
            label: None,
        }
    }

    /// A [`Most`](Summary::Most) of the keys starting with `prefix`, none
    /// left out ([`with_skip_suffix`](Self::with_skip_suffix) leaves some
    /// out).
    pub fn most(prefix: impl Into<String>) -> Self {
        Summary::Most {
            prefix: prefix.into(),
            skip_suffix: None,
            label: None,
        }
    }

    /// A [`Median`](Summary::Median) of `measure`.
    pub fn median(measure: impl Into<String>) -> Self {
        Summary::Median {
            measure: measure.into(),
            label: None,
        }
    }

    /// A [`Max`](Summary::Max) of `measure`.
    pub fn max(measure: impl Into<String>) -> Self {
        Summary::Max {
            measure: measure.into(),
            label: None,
        }
    }

    /// The item printed with `label` instead of its default.
    #[must_use]
    pub fn with_label(mut self, label: Option<String>) -> Self {
        match &mut self {
            Summary::Sum { label: l, .. }
            | Summary::Spread { label: l, .. }
            | Summary::Most { label: l, .. }
            | Summary::Median { label: l, .. }
            | Summary::Max { label: l, .. } => *l = label,
        }
        self
    }

    /// The item leaving out keys ending with `skip_suffix`, if it is a
    /// [`Most`](Summary::Most); any other item as it was.
    #[must_use]
    pub fn with_skip_suffix(mut self, skip_suffix: Option<String>) -> Self {
        if let Summary::Most { skip_suffix: s, .. } = &mut self {
            *s = skip_suffix;
        }
        self
    }

    /// The item over `tries`, as its doc comment shows it.
    pub fn text(&self, tries: &[Tried]) -> String {
        let named = |label: &Option<String>, measure: &str| -> String {
            label.clone().unwrap_or_else(|| measure.to_owned())
        };
        match self {
            Summary::Sum { measure, label } => {
                let sum = values(tries, measure).fold(0i64, i64::saturating_add);
                format!("{} {sum}/{}", named(label, measure), tries.len())
            }
            Summary::Spread {
                measure,
                up_to,
                label,
            } => {
                let mut buckets: BTreeMap<u32, usize> = (0..=*up_to).map(|b| (b, 0)).collect();
                for value in values(tries, measure) {
                    let bucket = u32::try_from(value.max(0)).unwrap_or(u32::MAX).min(*up_to);
                    *buckets.entry(bucket).or_default() += 1;
                }
                let spread: Vec<String> = buckets
                    .iter()
                    .map(|(&b, n)| {
                        let open = if b == *up_to { "+" } else { "" };
                        format!("{b}{open}: {n}")
                    })
                    .collect();
                format!("{} per try {}", named(label, measure), spread.join("  "))
            }
            Summary::Most {
                prefix,
                skip_suffix,
                label,
            } => {
                let mut set: BTreeMap<&str, u32> = BTreeMap::new();
                for t in tries {
                    let keys = t
                        .measures
                        .iter()
                        .filter(|&(_, &value)| value >= 1)
                        .filter_map(|(key, _)| key.strip_prefix(prefix.as_str()))
                        .filter(|key| skip_suffix.as_ref().is_none_or(|s| !key.ends_with(s)));
                    for key in keys {
                        *set.entry(key).or_default() += 1;
                    }
                }
                // The most tries, then the first by name.
                let most = set
                    .iter()
                    .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
                    .map_or_else(|| "nobody".to_owned(), |(key, n)| format!("{key} {n}×"));
                format!("{}: {most}", named(label, "most"))
            }
            Summary::Median { measure, label } => {
                let mut sorted: Vec<i64> = values(tries, measure).collect();
                sorted.sort_unstable();
                let median = sorted
                    .get(sorted.len().saturating_sub(1) / 2)
                    .copied()
                    .unwrap_or(0);
                format!("{} median {median}", named(label, measure))
            }
            Summary::Max { measure, label } => {
                let max = values(tries, measure).max().unwrap_or(0);
                format!("{} max {max}", named(label, measure))
            }
        }
    }
}

/// `measure` in each of `tries`, 0 where it is missing.
fn values<'a>(tries: &'a [Tried], measure: &'a str) -> impl Iterator<Item = i64> + 'a {
    tries
        .iter()
        .map(move |t| t.measures.get(measure).copied().unwrap_or(0))
}
