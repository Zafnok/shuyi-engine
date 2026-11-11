//! Rules written as text: a game's modders write a small rule in
//! [Rhai](https://rhai.rs), the game compiles it once from its bundle, and
//! its rules call it with a read-only view, reading back plain values they
//! turn into their own effects. A script never changes the game's state.
//!
//! - [`Runtime`]: the configured language, with its [`Limits`];
//!   [`Runtime::compile`] makes a [`Script`], and [`Runtime::run`] and
//!   [`Runtime::call`] run one on a view.
//! - [`Value`]: the plain values a script gives back, when a game reads
//!   them without a type of its own.
//! - [`error`]: what can go wrong, [`ScriptError`].
//!
//! # What a script can't do
//!
//! The language is cut to what a rule needs (ADR-0093): integers only
//! (`i64`, checked: an overflow is an error), no clock, no random numbers,
//! no files (`import` doesn't compile), no network, no output (`print`,
//! `debug` and `eval` don't compile). Every run stops at the
//! [`Limits`]: a number of operations, a string's bytes, a list's items, a
//! map's entries, variables, the call depth and the expression depth. The
//! same script on the same view gives the same values on every platform,
//! so replays, rewinds and lockstep stay exact.
//!
//! # Views and values
//!
//! A view is a type of the game's own, made from its state for one side,
//! with the luck to come and what the side can't see left out (ADR-0084
//! §4). It goes in through its `Serialize` derive: a struct becomes a map
//! the script reads by field (`view.growth`), an enum is externally tagged,
//! as serde's default is. What comes back is read with the game's own
//! `Deserialize` type, or as a [`Value`]. No type is registered with the
//! runtime, so none is named in it: a view is a map in the script.
//!
//! ```
//! use shuyi_script::{Limits, Runtime};
//!
//! #[derive(serde::Serialize)]
//! struct Crop {
//!     growth: i64,
//!     rain: bool,
//! }
//!
//! let runtime = Runtime::new(Limits::default());
//! let script = runtime
//!     .compile("mods/rain.rhai", "fn grows(crop) { if crop.rain { crop.growth * 2 } else { crop.growth } }")
//!     .unwrap_or_else(|e| panic!("{e}"));
//! let grows: i64 = runtime
//!     .call(&script, "grows", &Crop { growth: 3, rain: true })
//!     .unwrap_or_else(|e| panic!("{e}"));
//! assert_eq!(grows, 6);
//! ```

#![deny(clippy::float_arithmetic)]

pub mod error;
mod limits;
mod value;

#[cfg(test)]
mod tests;

pub use error::{ErrorKind, Limit, ScriptError};
pub use limits::Limits;
pub use value::Value;

use rhai::packages::{BasicArrayPackage, BasicMapPackage, CorePackage, LogicPackage, Package};
use rhai::{AST, CallFnOptions, Dynamic, Engine, Scope};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// The name a script reads its view by in [`Runtime::run`].
pub const VIEW: &str = "view";

/// The words a script can't use: output and running text as code.
const DISABLED: [&str; 3] = ["print", "debug", "eval"];

/// The script language, configured once: what a script may do, and its
/// [`Limits`]. A game makes one and keeps it beside its compiled scripts.
///
/// It isn't `Send`: a thread that runs scripts makes its own.
pub struct Runtime {
    engine: Engine,
}

impl Runtime {
    /// The language with `limits`. A limit of 0 is taken as 1: no limit is
    /// ever off.
    #[must_use]
    pub fn new(limits: Limits) -> Self {
        let mut engine = Engine::new_raw();
        // The standard package without its bit fields, byte blobs, extra
        // string functions and maths (ADR-0093 §2): what a rule reads and
        // returns.
        CorePackage::new().register_into_engine(&mut engine);
        LogicPackage::new().register_into_engine(&mut engine);
        BasicArrayPackage::new().register_into_engine(&mut engine);
        BasicMapPackage::new().register_into_engine(&mut engine);
        for symbol in DISABLED {
            engine.disable_symbol(symbol);
        }
        // A misspelt name is an error, not `()`.
        engine.set_strict_variables(true);
        engine.set_fail_on_invalid_map_property(true);
        limits.apply(&mut engine);
        Self { engine }
    }

    /// Compiles `source`, named `name`: the bundle's display path of its
    /// file, which every error names. Top-level code may read the view as
    /// [`VIEW`] (for [`run`](Self::run)); functions read what they are
    /// called with (for [`call`](Self::call)).
    pub fn compile(&self, name: &str, source: &str) -> Result<Script, ScriptError> {
        // `view` is declared as a variable, not a constant, so strict
        // variables accept it and the optimizer doesn't fold it.
        let mut scope = Scope::new();
        scope.push(VIEW, Dynamic::UNIT);
        let mut ast = self
            .engine
            .compile_with_scope(&scope, source)
            .map_err(|e| ScriptError::from_parse(name, &e))?;
        ast.set_source(name);
        Ok(Script {
            name: name.into(),
            ast,
        })
    }

    /// Runs `script`'s top-level code with `view` as the constant
    /// [`VIEW`], and reads back what it ends with as an `R`.
    pub fn run<V, R>(&self, script: &Script, view: &V) -> Result<R, ScriptError>
    where
        V: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let view = script.view(view)?;
        let mut scope = Scope::new();
        scope.push_constant_dynamic(VIEW, view);
        let out = self
            .engine
            .eval_ast_with_scope::<Dynamic>(&mut scope, &script.ast)
            .map_err(|e| ScriptError::from_eval(&script.name, *e))?;
        script.read(&out)
    }

    /// Calls `script`'s function `function` with `view` as its one
    /// argument, and reads back what it returns as an `R`. The script's
    /// top-level code doesn't run.
    pub fn call<V, R>(&self, script: &Script, function: &str, view: &V) -> Result<R, ScriptError>
    where
        V: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let view = script.view(view)?;
        let options = CallFnOptions::new().eval_ast(false).rewind_scope(true);
        let out = self
            .engine
            .call_fn_with_options::<Dynamic>(
                options,
                &mut Scope::new(),
                &script.ast,
                function,
                (view,),
            )
            .map_err(|e| ScriptError::from_eval(&script.name, *e))?;
        script.read(&out)
    }
}

impl Default for Runtime {
    /// The language with [`Limits::default`].
    fn default() -> Self {
        Self::new(Limits::default())
    }
}

impl core::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Runtime").finish_non_exhaustive()
    }
}

/// A compiled script, made by [`Runtime::compile`]: run it with the
/// runtime that compiled it.
#[derive(Debug, Clone)]
pub struct Script {
    name: String,
    ast: AST,
}

impl Script {
    /// The name it was compiled with: its file's display path.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether it defines a function `name`, with any number of
    /// parameters.
    #[must_use]
    pub fn has_function(&self, name: &str) -> bool {
        self.ast.iter_functions().any(|f| f.name == name)
    }

    /// `view` in the script's values.
    fn view<V: Serialize + ?Sized>(&self, view: &V) -> Result<Dynamic, ScriptError> {
        rhai::serde::to_dynamic(view).map_err(|e| ScriptError::view(&self.name, &e))
    }

    /// What a run gave, as an `R`.
    fn read<R: DeserializeOwned>(&self, out: &Dynamic) -> Result<R, ScriptError> {
        rhai::serde::from_dynamic(out).map_err(|e| ScriptError::value(&self.name, &e))
    }
}
