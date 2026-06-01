//! MetaM / TacticM execution context.

use pyo3::prelude::*;

/// Python wrapper for a MetaM execution context.
///
/// Provides the ability to run MetaM computations, which includes
/// type checking, elaboration, and tactic execution.
#[pyclass(name = "MetaContext")]
pub struct MetaContextPy {
    // Will hold MetaMContext state
}

#[pymethods]
impl MetaContextPy {
    fn __repr__(&self) -> &str {
        "pylean4.MetaContext()"
    }
}
