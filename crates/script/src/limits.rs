//! How far a script may go: [`Limits`].

use rhai::Engine;

/// Where every run of a script stops, with an error of
/// [`ErrorKind::Limit`](crate::ErrorKind::Limit). Together they bound its
/// time (its operations) and its memory (each value's size, and how many
/// there can be). A limit of 0 is taken as 1.
///
/// ```
/// let mut limits = shuyi_script::Limits::default();
/// limits.operations = 1_000_000;
/// let runtime = shuyi_script::Runtime::new(limits);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Limits {
    /// The operations one run may do: each expression, statement and call
    /// counts. Also how many values a run can make, so it bounds memory.
    pub operations: u64,
    /// A string's bytes.
    pub text_bytes: usize,
    /// A list's items, counting the lists inside it.
    pub list_items: usize,
    /// A map's entries, counting the maps inside it.
    pub map_entries: usize,
    /// The variables in scope at once, [`VIEW`](crate::VIEW) among them.
    pub variables: usize,
    /// How deep calls go: a function calling itself stops here.
    pub call_depth: usize,
    /// How deep an expression nests, checked when it compiles.
    pub expression_depth: usize,
}

impl Limits {
    /// The defaults: room for a rule over a view of a few hundred things,
    /// a few milliseconds at most.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            operations: 100_000,
            text_bytes: 4_096,
            list_items: 1_024,
            map_entries: 256,
            variables: 256,
            call_depth: 32,
            expression_depth: 64,
        }
    }

    /// Sets them on `engine`, each at least 1: Rhai takes 0 as no limit.
    pub(crate) fn apply(self, engine: &mut Engine) {
        engine.set_max_operations(self.operations.max(1));
        engine.set_max_string_size(self.text_bytes.max(1));
        engine.set_max_array_size(self.list_items.max(1));
        engine.set_max_map_size(self.map_entries.max(1));
        engine.set_max_variables(self.variables.max(1));
        engine.set_max_call_levels(self.call_depth.max(1));
        let depth = self.expression_depth.max(1);
        engine.set_max_expr_depths(depth, depth);
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self::new()
    }
}
