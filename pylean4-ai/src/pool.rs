//! BatchVerifier: parallel tactic verification with worker pool.

use crate::tactic_state::{TacticResultPy, TacticStatePy};
use pyo3::prelude::*;

/// High-throughput parallel tactic verifier.
///
/// Uses a Rust-side worker pool to verify multiple (state, tactic) pairs
/// concurrently. The GIL is released during verification, allowing
/// Python to continue other work.
///
/// # Performance Target
///
/// >10,000 tactic verifications per second on a modern multi-core machine.
///
/// # Example
///
/// ```python
/// verifier = BatchVerifier(num_workers=8)
///
/// states = [env.reset() for _ in range(batch_size)]
/// tactics = model.generate_batch(states)
///
/// results = verifier.verify_batch(states, tactics)
/// rewards = [1.0 if r.success else -0.1 for r in results]
/// ```
#[pyclass(name = "BatchVerifier")]
pub struct BatchVerifierPy {
    num_workers: usize,
}

#[pymethods]
impl BatchVerifierPy {
    #[new]
    #[pyo3(signature = (num_workers=8))]
    fn new(num_workers: usize) -> Self {
        Self { num_workers }
    }

    /// Verify a batch of (state, tactic) pairs in parallel.
    ///
    /// Releases the GIL and dispatches work to the Rust worker pool.
    /// Returns results in the same order as the inputs.
    fn verify_batch(
        &self,
        py: Python,
        states: Vec<TacticStatePy>,
        tactics: Vec<String>,
    ) -> PyResult<Vec<TacticResultPy>> {
        if states.len() != tactics.len() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "states and tactics must have the same length",
            ));
        }

        let _num_workers = self.num_workers;

        // Release GIL and run verification in parallel
        let results = py.allow_threads(|| {
            // TODO: use rayon thread pool with pre-initialized Lean threads
            states
                .iter()
                .zip(tactics.iter())
                .map(|(_state, _tactic)| TacticResultPy {
                    success: false,
                    error: Some("batch verification not yet implemented".to_string()),
                    next_state: None,
                })
                .collect::<Vec<_>>()
        });

        Ok(results)
    }

    /// Get the number of worker threads.
    #[getter]
    fn workers(&self) -> usize {
        self.num_workers
    }

    fn __repr__(&self) -> String {
        format!("BatchVerifier(num_workers={})", self.num_workers)
    }
}
