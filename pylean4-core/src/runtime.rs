//! Lean runtime lifecycle management.
//!
//! Handles initialization of the Lean4 runtime and provides the
//! thread-local context needed for all Lean operations.

use pyo3::prelude::*;
use std::sync::Once;

use crate::object::{LeanObject, LeanTypeTag};
use leo3::instance::LeanAny;

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

    /// Create an empty Lean environment.
    fn empty_environment(&self) -> PyResult<LeanObject> {
        let result = leo3::with_lean(|lean| {
            let env = leo3::meta::LeanEnvironment::empty(lean, 0)?;
            Ok::<_, leo3::LeanError>(env.cast::<LeanAny>().unbind_mt())
        });

        match result {
            Ok(inner) => Ok(LeanObject::new(inner, LeanTypeTag::Environment)),
            Err(e) => Err(crate::error::lean_to_py_err(e)),
        }
    }

    fn __repr__(&self) -> String {
        format!("pylean4.Runtime(initialized={})", self.initialized)
    }
}
