//! What the in-memory network does to messages: [`Faults`].

/// A per-mille chance's whole range.
pub(crate) const PER_MILLE: u16 = 1_000;

/// What a [`MemoryNetwork`](crate::MemoryNetwork) does to messages: loss,
/// duplicates and reordering of `Unreliable` messages, and a delay of
/// every delivery, drawn from one `SimRng` seeded with [`Faults::seed`].
///
/// Chances are per mille (0 to 1,000; more counts as 1,000). Delays
/// count steps from a send to its delivery: 1 is the next
/// [`step`](crate::MemoryNetwork::step), the soonest there is. A delay of
/// 0 counts as 1, and a `delay_max` below `delay_min` as `delay_min`.
/// [`Faults::none`] (and `Default`) has both at 1: everything is
/// delivered at the next step, whole, once and in order.
///
/// ```
/// use shuyi_net::Faults;
///
/// let faults = Faults::none()
///     .with_seed(7)
///     .with_loss(50)
///     .with_delay_min(2)
///     .with_delay_max(4);
/// assert_eq!((faults.loss(), faults.duplicate()), (50, 0));
/// assert_eq!((faults.delay_min(), faults.delay_max()), (2, 4));
/// assert_eq!(Faults::none().with_reorder(1_500).reorder(), 1_000);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Faults {
    seed: u64,
    loss: u16,
    duplicate: u16,
    reorder: u16,
    delay_min: u32,
    delay_max: u32,
}

impl Faults {
    /// No faults: nothing lost, duplicated or reordered, and every
    /// delivery at the next step. The seed is 0.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            seed: 0,
            loss: 0,
            duplicate: 0,
            reorder: 0,
            delay_min: 1,
            delay_max: 1,
        }
    }

    /// The seed of the draws.
    #[must_use]
    pub const fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// The chance, per mille, that an `Unreliable` message is lost.
    #[must_use]
    pub const fn with_loss(mut self, per_mille: u16) -> Self {
        self.loss = clamp_per_mille(per_mille);
        self
    }

    /// The chance, per mille, that an `Unreliable` message that isn't lost
    /// arrives twice, the copy with a delay of its own.
    #[must_use]
    pub const fn with_duplicate(mut self, per_mille: u16) -> Self {
        self.duplicate = clamp_per_mille(per_mille);
        self
    }

    /// The chance, per mille, that an `Unreliable` message that isn't lost
    /// is held behind the other deliveries of its step (its copy too), so
    /// the later messages that arrive at that step come first.
    #[must_use]
    pub const fn with_reorder(mut self, per_mille: u16) -> Self {
        self.reorder = clamp_per_mille(per_mille);
        self
    }

    /// The fewest steps from a send to its delivery.
    #[must_use]
    pub const fn with_delay_min(mut self, steps: u32) -> Self {
        self.delay_min = steps;
        self
    }

    /// The most steps from a send to its delivery (a `Reliable` message
    /// waits longer only behind an earlier one on its link).
    #[must_use]
    pub const fn with_delay_max(mut self, steps: u32) -> Self {
        self.delay_max = steps;
        self
    }

    /// The seed of the draws.
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    /// The chance of a loss, per mille.
    pub const fn loss(&self) -> u16 {
        self.loss
    }

    /// The chance of a duplicate, per mille.
    pub const fn duplicate(&self) -> u16 {
        self.duplicate
    }

    /// The chance of a reordering, per mille.
    pub const fn reorder(&self) -> u16 {
        self.reorder
    }

    /// The fewest steps from a send to its delivery, as set.
    pub const fn delay_min(&self) -> u32 {
        self.delay_min
    }

    /// The most steps from a send to its delivery, as set.
    pub const fn delay_max(&self) -> u32 {
        self.delay_max
    }

    /// The delays as used: at least 1, and the most at least the fewest.
    pub(crate) fn delays(&self) -> (u64, u64) {
        let min = self.delay_min.max(1);
        (u64::from(min), u64::from(self.delay_max.max(min)))
    }
}

impl Default for Faults {
    fn default() -> Self {
        Self::none()
    }
}

/// `per_mille`, at most 1,000.
const fn clamp_per_mille(per_mille: u16) -> u16 {
    if per_mille > PER_MILLE {
        PER_MILLE
    } else {
        per_mille
    }
}
