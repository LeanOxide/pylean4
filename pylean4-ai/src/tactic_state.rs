//! TacticState: a snapshot of a proof state.

use pyo3::prelude::*;

/// A snapshot of a Lean4 proof state.
///
/// Contains the current goals and hypotheses. Immutable — applying a
/// tactic produces a new `TacticState` rather than mutating this one.
///
/// # Example
///
/// ```python
/// state = env.reset()
/// print(state.goals)          # ["⊢ 1 + 1 = 2"]
/// result = state.apply("ring")
/// if result.success:
///     print("Proved!")
/// ```
#[pyclass(name = "TacticState")]
#[derive(Clone)]
pub struct TacticStatePy {
    // Will hold: LeanUnbound for the proof state
    // Plus cached goal/hypothesis strings
}

#[pymethods]
impl TacticStatePy {
    /// Get the list of current goals as strings.
    #[getter]
    fn goals(&self) -> Vec<String> {
        // TODO: extract goals from MetavarContext
        vec![]
    }

    /// Get the number of remaining goals.
    #[getter]
    pub fn num_goals(&self) -> usize {
        0
    }

    /// Check if the proof is complete (no remaining goals).
    #[getter]
    pub fn is_solved(&self) -> bool {
        self.num_goals() == 0
    }

    /// Apply a tactic string to this state.
    ///
    /// Returns a `TacticResult` with the new state (if successful)
    /// or an error message.
    pub fn apply(&self, _py: Python, _tactic: &str) -> PyResult<TacticResultPy> {
        // TODO: implement via MetaM
        Ok(TacticResultPy {
            success: false,
            error: Some("not yet implemented".to_string()),
            next_state: None,
        })
    }

    fn __repr__(&self) -> String {
        format!("TacticState(goals={})", self.num_goals())
    }
}

/// Result of applying a tactic.
#[pyclass(name = "TacticResult")]
#[derive(Clone)]
pub struct TacticResultPy {
    #[pyo3(get)]
    pub success: bool,
    #[pyo3(get)]
    pub error: Option<String>,
    #[pyo3(get)]
    pub next_state: Option<TacticStatePy>,
}

#[pymethods]
impl TacticResultPy {
    fn __repr__(&self) -> String {
        if self.success {
            "TacticResult(success=True)".to_string()
        } else {
            format!(
                "TacticResult(success=False, error={:?})",
                self.error.as_deref().unwrap_or("unknown")
            )
        }
    }

    fn __bool__(&self) -> bool {
        self.success
    }
}
