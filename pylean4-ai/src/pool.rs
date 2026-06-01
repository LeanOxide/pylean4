//! BatchVerifier: parallel tactic verification with worker pool.

use crate::tactic_state::{MetaContextState, TacticResultPy, TacticStatePy};
use leo3::instance::LeanAny;
use leo3::prelude::*;
use pyo3::prelude::*;
use rayon::prelude::*;

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
    pool: rayon::ThreadPool,
    num_workers: usize,
}

#[pymethods]
impl BatchVerifierPy {
    #[new]
    #[pyo3(signature = (num_workers=8))]
    fn new(num_workers: usize) -> PyResult<Self> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_workers)
            .build()
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "failed to create thread pool: {e}"
                ))
            })?;
        Ok(Self { pool, num_workers })
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

        // Collect work items: pair each state with its tactic string
        let work: Vec<_> = states.into_iter().zip(tactics.into_iter()).collect();

        // Release GIL and run verification in parallel on the rayon pool
        let results = py.allow_threads(|| {
            self.pool.install(|| {
                work.into_par_iter()
                    .map(|(state, tactic)| verify_single(state, &tactic))
                    .collect::<Vec<_>>()
            })
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

// ============================================================================
// Internal: single tactic verification (runs on a rayon worker thread)
// ============================================================================

/// Verify a single (state, tactic) pair inside a leo3 thread context.
///
/// This is called from a rayon worker thread. `leo3::with_lean` ensures
/// the thread is properly initialized for Lean runtime access.
fn verify_single(state: TacticStatePy, tactic: &str) -> TacticResultPy {
    let Some(meta) = state.meta_state.clone() else {
        return TacticResultPy {
            success: false,
            error: Some("TacticState has no MetaM context".into()),
            next_state: None,
        };
    };

    if state.goals.is_empty() {
        return TacticResultPy {
            success: false,
            error: Some("no goals".into()),
            next_state: None,
        };
    }

    let goals: Vec<_> = state.goals.iter().map(|g| g.clone()).collect();
    let meta_for_result = state.meta_state.clone();

    // Each worker thread gets its own Lean thread context via with_lean
    let result = leo3::with_lean(|lean| {
        let mut ctx = rebuild_ctx(lean, meta);
        let bound_goals: Vec<_> = goals
            .into_iter()
            .map(|g| g.into_bound(lean).cast())
            .collect();
        let tactic_state = leo3::meta::tactic::TacticState::new(bound_goals);

        let tactic_result = dispatch_tactic(&mut ctx, tactic_state, lean, tactic);

        match tactic_result {
            leo3::meta::tactic::TacticResult::Success(new_state) => {
                let new_goals: Vec<_> = new_state
                    .into_goals()
                    .into_iter()
                    .map(|g| g.cast::<LeanAny>().unbind_mt())
                    .collect();
                Ok(new_goals)
            }
            leo3::meta::tactic::TacticResult::Failure(e) => Err(format!("{e}")),
        }
    });

    match result {
        Ok(new_goals) => TacticResultPy {
            success: true,
            error: None,
            next_state: Some(TacticStatePy {
                goals: new_goals,
                meta_state: meta_for_result,
            }),
        },
        Err(msg) => TacticResultPy {
            success: false,
            error: Some(msg),
            next_state: None,
        },
    }
}

/// Dispatch a tactic string to the appropriate leo3 tactic function.
fn dispatch_tactic<'l>(
    ctx: &mut leo3::meta::MetaMContext<'l>,
    state: leo3::meta::tactic::TacticState<'l>,
    lean: leo3::Lean<'l>,
    tactic: &str,
) -> leo3::meta::tactic::TacticResult<'l> {
    let tactic = tactic.trim();

    if tactic == "rfl" {
        leo3::meta::tactic::rfl(ctx, state)
    } else if let Some(name) = tactic.strip_prefix("intro ") {
        let name = name.trim();
        match leo3::meta::LeanName::from_str(lean, name) {
            Ok(lean_name) => leo3::meta::tactic::intro(ctx, state, &lean_name),
            Err(e) => leo3::meta::tactic::TacticResult::Failure(e),
        }
    } else if tactic == "assumption" {
        // assumption without explicit hypotheses — pass empty list
        // (full local context iteration not yet available)
        leo3::meta::tactic::assumption(ctx, state, &[])
    } else {
        leo3::meta::tactic::TacticResult::Failure(leo3::LeanError::other(
            &format!("unsupported tactic: {tactic}"),
        ))
    }
}

/// Rebuild a MetaMContext from stored unbound parts.
fn rebuild_ctx<'l>(
    lean: leo3::Lean<'l>,
    meta: MetaContextState,
) -> leo3::meta::MetaMContext<'l> {
    unsafe {
        leo3::meta::MetaMContext::from_parts(
            lean,
            meta.env.into_bound(lean).cast(),
            meta.core_ctx.into_bound(lean).cast(),
            meta.core_state.into_bound(lean).cast(),
            meta.meta_ctx.into_bound(lean).cast(),
            meta.meta_state.into_bound(lean).cast(),
        )
    }
}
