//! pylean4-ai: High-performance AI/RL acceleration layer for Lean4.
//!
//! This crate provides:
//! - `TacticState`: proof state snapshots with goal/hypothesis access
//! - `ProofEnvironment`: Gym-style step interface for RL training
//! - `BatchVerifier`: parallel tactic verification (>10k/sec target)

use pyo3::prelude::*;

mod tactic_state;
mod proof_env;
mod pool;

pub use tactic_state::{TacticStatePy, TacticResultPy};
pub use proof_env::ProofEnvironmentPy;
pub use pool::BatchVerifierPy;

/// The pylean4._ai Python module.
#[pymodule]
fn _ai(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<TacticStatePy>()?;
    m.add_class::<tactic_state::TacticResultPy>()?;
    m.add_class::<ProofEnvironmentPy>()?;
    m.add_class::<BatchVerifierPy>()?;
    Ok(())
}
