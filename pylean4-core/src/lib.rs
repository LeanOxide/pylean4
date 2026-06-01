//! pylean4-core: Python bindings for Lean4 via leo3.
//!
//! This crate provides the low-level Python ↔ Lean4 bridge using PyO3.
//! All Lean objects are wrapped in thread-safe `LeanObject` handles that
//! manage Lean's reference counting automatically.

use pyo3::prelude::*;

mod runtime;
mod object;
mod types;
mod closure;
mod environment;
mod meta;
mod module;
mod conversion;
mod error;

pub use object::{LeanObject, LeanTypeTag};
pub use runtime::LeanRuntime;

/// The pylean4._core Python module.
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<LeanRuntime>()?;
    m.add_class::<LeanObject>()?;
    m.add_class::<environment::LeanEnvironmentPy>()?;
    m.add_class::<meta::MetaContextPy>()?;
    Ok(())
}
