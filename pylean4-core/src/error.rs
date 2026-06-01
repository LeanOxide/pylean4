//! Error mapping between Lean and Python exceptions.

use leo3::err::LeanError;
use pyo3::exceptions::{PyRuntimeError, PyTypeError, PyValueError};
use pyo3::PyErr;

/// Convert a LeanError into a Python exception.
pub fn lean_to_py_err(err: LeanError) -> PyErr {
    match err {
        LeanError::Exception { message, .. } => {
            PyRuntimeError::new_err(format!("Lean exception: {message}"))
        }
        LeanError::NullPointer { operation } => {
            PyRuntimeError::new_err(format!("Lean null pointer in: {operation}"))
        }
        LeanError::Conversion(msg) => PyTypeError::new_err(format!("Lean conversion: {msg}")),
        LeanError::OutOfBounds { index, len } => {
            PyValueError::new_err(format!("index {index} out of bounds (len={len})"))
        }
        _ => PyRuntimeError::new_err(format!("Lean error: {err}")),
    }
}

/// Convert a type mismatch into a Python TypeError.
pub fn type_mismatch(expected: &str, got: &str) -> PyErr {
    PyTypeError::new_err(format!("expected {expected}, got {got}"))
}

/// Convert an invalid value into a Python ValueError.
#[allow(dead_code)]
pub fn invalid_value(msg: impl Into<String>) -> PyErr {
    PyValueError::new_err(msg.into())
}
