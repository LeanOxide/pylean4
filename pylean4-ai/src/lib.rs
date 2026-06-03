//! pylean4-ai: High-performance AI/RL acceleration layer for Lean4.
//!
//! This crate provides:
//! - `TacticState`: proof state with tactic DSL methods
//! - `Expr`: expression construction DSL
//! - `ProofEnvironment`: Gym-style step interface for RL training
//! - `BatchVerifier`: parallel tactic verification (>10k/sec target)

use pyo3::prelude::*;

pub mod tactic_state;
mod meta_context;
mod proof_env;
mod pool;

pub use tactic_state::{TacticStatePy, TacticResultPy, ExprPy};
pub use meta_context::MetaContextPy;
pub use proof_env::ProofEnvironmentPy;
pub use pool::BatchVerifierPy;

/// The pylean4._ai Python module.
#[pymodule]
fn _ai(m: &Bound<'_, PyModule>) -> PyResult<()> {
    _core::register_core_base(m)?;
    m.add_class::<MetaContextPy>()?;
    m.add_class::<TacticStatePy>()?;
    m.add_class::<TacticResultPy>()?;
    m.add_class::<ExprPy>()?;
    m.add_class::<ProofEnvironmentPy>()?;
    m.add_class::<BatchVerifierPy>()?;
    Ok(())
}
