//! Lean Environment queries.

use pyo3::prelude::*;

/// Python wrapper for a Lean4 Environment.
///
/// Provides read-only access to the Lean environment: looking up
/// definitions, types, instances, and constants.
#[pyclass(name = "Environment")]
pub struct LeanEnvironmentPy {
    // Will hold LeanUnbound<LeanEnvironment>
}

#[pymethods]
impl LeanEnvironmentPy {
    fn __repr__(&self) -> &str {
        "pylean4.Environment()"
    }
}
