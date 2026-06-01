//! Lean runtime lifecycle management.
//!
//! Handles initialization of the Lean4 runtime and provides the
//! thread-local context needed for all Lean operations.

use pyo3::prelude::*;
use std::sync::Once;

static INIT: Once = Once::new();

/// Manages the Lean4 runtime lifecycle.
///
/// Only one `LeanRuntime` should exist per process. Creating it initializes
/// the Lean4 runtime (worker thread, module system, etc.). The runtime
/// remains active until the process exits.
///
/// # Example
///
/// ```python
/// import pylean4
/// rt = pylean4.Runtime()
/// ```
#[pyclass(name = "Runtime")]
pub struct LeanRuntime {
    initialized: bool,
}

#[pymethods]
impl LeanRuntime {
    #[new]
    fn new() -> PyResult<Self> {
        INIT.call_once(|| {
            leo3::prepare_freethreaded_lean();
        });

        Ok(Self { initialized: true })
    }

    /// Check if the runtime is initialized.
    #[getter]
    fn is_initialized(&self) -> bool {
        self.initialized
    }

    fn __repr__(&self) -> String {
        format!("pylean4.Runtime(initialized={})", self.initialized)
    }
}
