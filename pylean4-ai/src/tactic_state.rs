//! TacticState: a snapshot of a proof state.
//!
//! Wraps leo3's `TacticState` for Python, storing goals as unbound
//! references that get rebound on each tactic application.

use leo3::instance::LeanAny;
use leo3::prelude::*;
use leo3::LeanUnbound;
use pyo3::prelude::*;

/// A snapshot of a Lean4 proof state.
///
/// Contains the current goals (metavariable expressions) and a reference
/// to the MetaM context needed to execute tactics. Immutable — applying
/// a tactic produces a new `TacticState`.
///
/// # Example
///
/// ```python
/// state = env.reset()
/// print(state.num_goals)
/// result = state.apply("ring")
/// if result.success:
///     print("Proved!")
/// ```
#[pyclass(name = "TacticState")]
#[derive(Clone)]
pub struct TacticStatePy {
    /// Goals stored as MT-safe unbound references.
    pub(crate) goals: Vec<LeanUnbound<LeanAny>>,
    /// Shared MetaM context state (cloned on each tactic call).
    pub(crate) meta_state: Option<MetaContextState>,
}

#[pymethods]
impl TacticStatePy {
    /// Get the number of remaining goals.
    #[getter]
    pub fn num_goals(&self) -> usize {
        self.goals.len()
    }

    /// Check if the proof is complete (no remaining goals).
    #[getter]
    pub fn is_solved(&self) -> bool {
        self.goals.is_empty()
    }

    /// Get goal types as human-readable strings.
    #[getter]
    fn goals_pp(&self) -> Vec<String> {
        // Pretty-print each goal via MetaM
        let Some(ref state) = self.meta_state else {
            return self.goals.iter().map(|_| "<no context>".to_string()).collect();
        };

        let goals_clone: Vec<_> = self.goals.iter().map(|g| g.clone()).collect();
        let meta = state.clone();

        leo3::with_lean(|lean| {
            let mut ctx = unsafe {
                leo3::meta::MetaMContext::from_parts(
                    lean,
                    meta.env.into_bound(lean).cast(),
                    meta.core_ctx.into_bound(lean).cast(),
                    meta.core_state.into_bound(lean).cast(),
                    meta.meta_ctx.into_bound(lean).cast(),
                    meta.meta_state.into_bound(lean).cast(),
                )
            };

            goals_clone
                .into_iter()
                .map(|g| {
                    let bound = g.into_bound(lean).cast();
                    match leo3::meta::tactic::goal_type(&mut ctx, &bound) {
                        Ok(ty) => match leo3::meta::LeanExpr::dbg_to_string(&ty) {
                            Ok(s) => format!("⊢ {s}"),
                            Err(_) => "⊢ <pp failed>".to_string(),
                        },
                        Err(_) => "⊢ <unknown>".to_string(),
                    }
                })
                .collect()
        })
    }

    /// Apply a tactic by name to this state.
    ///
    /// Supported tactics: "intro <name>", "exact <expr>", "rfl", "assumption"
    ///
    /// Returns a `TacticResult` with the new state (if successful)
    /// or an error message.
    pub fn apply(&self, py: Python, tactic: &str) -> PyResult<TacticResultPy> {
        let Some(ref meta) = self.meta_state else {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "TacticState has no MetaM context",
            ));
        };

        if self.goals.is_empty() {
            return Ok(TacticResultPy {
                success: false,
                error: Some("no goals".to_string()),
                next_state: None,
            });
        }

        let goals_clone: Vec<_> = self.goals.iter().map(|g| g.clone()).collect();
        let meta_clone = meta.clone();
        let tactic_str = tactic.to_string();

        // Release GIL during Lean computation
        let result = py.allow_threads(move || {
            leo3::with_lean(|lean| {
                let mut ctx = unsafe {
                    leo3::meta::MetaMContext::from_parts(
                        lean,
                        meta_clone.env.into_bound(lean).cast(),
                        meta_clone.core_ctx.into_bound(lean).cast(),
                        meta_clone.core_state.into_bound(lean).cast(),
                        meta_clone.meta_ctx.into_bound(lean).cast(),
                        meta_clone.meta_state.into_bound(lean).cast(),
                    )
                };

                // Rebind goals
                let bound_goals: Vec<_> = goals_clone
                    .into_iter()
                    .map(|g| g.into_bound(lean).cast())
                    .collect();
                let state = leo3::meta::tactic::TacticState::new(bound_goals);

                // Parse and dispatch tactic
                let tactic_result = dispatch_tactic(&mut ctx, state, &tactic_str, lean);

                // Convert result back to unbound
                match tactic_result {
                    leo3::meta::tactic::TacticResult::Success(new_state) => {
                        let new_goals: Vec<_> = new_state
                            .into_goals()
                            .into_iter()
                            .map(|g| g.cast::<LeanAny>().unbind_mt())
                            .collect();
                        Ok(new_goals)
                    }
                    leo3::meta::tactic::TacticResult::Failure(e) => Err(e),
                }
            })
        });

        match result {
            Ok(new_goals) => Ok(TacticResultPy {
                success: true,
                error: None,
                next_state: Some(TacticStatePy {
                    goals: new_goals,
                    meta_state: self.meta_state.clone(),
                }),
            }),
            Err(e) => Ok(TacticResultPy {
                success: false,
                error: Some(format!("{e}")),
                next_state: None,
            }),
        }
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

/// Stored MetaM context state (all MT-safe, lifetime-erased).
/// Shared between TacticState instances via Clone.
#[derive(Clone)]
pub(crate) struct MetaContextState {
    pub env: LeanUnbound<LeanAny>,
    pub core_ctx: LeanUnbound<LeanAny>,
    pub core_state: LeanUnbound<LeanAny>,
    pub meta_ctx: LeanUnbound<LeanAny>,
    pub meta_state: LeanUnbound<LeanAny>,
}

/// Parse a tactic string and dispatch to the appropriate leo3 tactic.
fn dispatch_tactic<'l>(
    ctx: &mut leo3::meta::MetaMContext<'l>,
    state: leo3::meta::tactic::TacticState<'l>,
    tactic: &str,
    lean: leo3::Lean<'l>,
) -> leo3::meta::tactic::TacticResult<'l> {
    let tactic = tactic.trim();

    if tactic == "rfl" {
        return leo3::meta::tactic::rfl(ctx, state);
    }

    if tactic == "assumption" {
        // assumption requires hypotheses list — not yet supported in string-based API
        return leo3::meta::tactic::TacticResult::Failure(leo3::LeanError::Other(
            "assumption tactic requires hypotheses (use structured API)".to_string(),
        ));
    }

    if let Some(name) = tactic.strip_prefix("intro ") {
        let name = name.trim();
        match leo3::meta::LeanName::from_str(lean, name) {
            Ok(lean_name) => return leo3::meta::tactic::intro(ctx, state, &lean_name),
            Err(e) => return leo3::meta::tactic::TacticResult::Failure(e),
        }
    }

    // Fallback: unsupported tactic
    leo3::meta::tactic::TacticResult::Failure(leo3::LeanError::Other(format!(
        "unsupported tactic: {tactic}"
    )))
}
