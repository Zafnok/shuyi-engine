//! The assets crate as a plugin crate uses it: a format of its own,
//! registered beside the engine's, and the code that asks for an asset
//! left as it was (ADR-0007). One test program; each file is a module.

mod hot;
mod txtimg;
