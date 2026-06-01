//! ProofEnvironment: Gym-style interface for RL training.

use crate::tactic_state::{TacticResultPy, TacticStatePy};
use pyo3::prelude::*;

/// Gym-style proof environment for reinforcement learning.
///
/// Provides `reset()` and `step()` methods compatible with standard
/// RL training loops.
///
/// # Example
///
/// ```python
/// env = ProofEnvironment("Mathlib.Tactic.Ring", "one_plus_one")
/// state = env.reset()
///
/// for tactic in model.generate(state):
///     state, reward, done, info = env.step(state, tactic)
///     if done:
///         break
/// ```
#[pyclass(name = "ProofEnvironment")]
pub struct ProofEnvironmentPy {
    module_name: String,
    theorem_name: String,
}

#[pymethods]
impl ProofEnvironmentPy {
    #[new]
    fn new(module_name: String, theorem_name: String) -> Self {
        Self {
            module_name,
            theorem_name,
        }
    }

    /// Reset to the initial proof state for the configured theorem.
    fn reset(&self) -> PyResult<TacticStatePy> {
        // TODO: load module, find theorem, create initial state
        Ok(TacticStatePy {
            goals: vec![],
            meta_state: None,
        })
    }

    /// Apply a tactic and return (next_state, reward, done, info).
    ///
    /// - reward: 1.0 if proof complete, 0.0 if progress, -0.1 if error
    /// - done: True if proof complete or unrecoverable error
    /// - info: dict with additional metadata
    fn step(
        &self,
        py: Python,
        state: &TacticStatePy,
        tactic: &str,
    ) -> PyResult<(TacticStatePy, f64, bool, PyObject)> {
        let result = state.apply(py, tactic)?;

        let (next_state, reward, done) = if result.success {
            let ns = result.next_state.unwrap_or_else(|| state.clone());
            if ns.is_solved() {
                (ns, 1.0, true)
            } else {
                (ns, 0.0, false)
            }
        } else {
            (state.clone(), -0.1, false)
        };

        let info = pyo3::types::PyDict::new(py);
        info.set_item("error", result.error)?;

        Ok((next_state, reward, done, info.into_any().unbind()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ProofEnvironment(module={:?}, theorem={:?})",
            self.module_name, self.theorem_name
        )
    }
}
