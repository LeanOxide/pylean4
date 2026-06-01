//! ProofEnvironment: Gym-style interface for RL training.

use _core::{LeanObject, LeanTypeTag};
use crate::tactic_state::{MetaContextState, TacticResultPy, TacticStatePy};
use leo3::instance::LeanAny;
use leo3::meta::{LeanConstantInfo, LeanEnvironment, LeanName, MetaMContext};
use leo3::LeanUnbound;
use pyo3::prelude::*;

/// Gym-style proof environment for reinforcement learning.
///
/// Provides `reset()` and `step()` methods compatible with standard
/// RL training loops.
///
/// # Example
///
/// ```python
/// env_obj = runtime.load_module("Mathlib.Tactic.Ring")
/// env = ProofEnvironment(env_obj, "one_plus_one")
/// state = env.reset()
///
/// for tactic in model.generate(state):
///     state, reward, done, info = env.step(state, tactic)
///     if done:
///         break
/// ```
#[pyclass(name = "ProofEnvironment")]
pub struct ProofEnvironmentPy {
    env: LeanUnbound<LeanAny>,
    theorem_name: String,
}

#[pymethods]
impl ProofEnvironmentPy {
    #[new]
    fn new(env: &LeanObject, theorem_name: String) -> PyResult<Self> {
        if env.type_tag != LeanTypeTag::Environment {
            return Err(pyo3::exceptions::PyTypeError::new_err(format!(
                "expected LeanObject of type Environment, got {}",
                env.type_tag,
            )));
        }
        Ok(Self {
            env: env.inner.clone(),
            theorem_name,
        })
    }

    /// Reset to the initial proof state for the configured theorem.
    ///
    /// Looks up the theorem by name in the environment, retrieves its type
    /// (the proposition to prove), creates a MetaMContext, and produces an
    /// initial TacticState with a single goal matching the theorem's type.
    fn reset(&self, py: Python) -> PyResult<TacticStatePy> {
        let env_unbound = self.env.clone();
        let theorem_name = self.theorem_name.clone();

        let result: Result<_, leo3::LeanError> = py.allow_threads(move || {
            leo3::with_lean(|lean| {
                // Bind the environment
                let env = env_unbound.into_bound(lean).cast::<LeanEnvironment>();

                // Look up the theorem by name
                let name = LeanName::from_str(lean, &theorem_name)?;
                let cinfo = LeanEnvironment::find(&env, &name)?
                    .ok_or_else(|| leo3::LeanError::other(&format!(
                        "theorem '{}' not found in environment",
                        theorem_name
                    )))?;

                // Get the theorem's type (the proposition to prove)
                let thm_type = LeanConstantInfo::type_(&cinfo)?;

                // Create a MetaMContext from the environment
                let mut ctx = MetaMContext::new(lean, env)?;

                // Create an initial goal (metavariable) with the theorem's type
                let goal = ctx.mk_goal(&thm_type)?;

                // Extract the MetaContextState for storage
                let (env, core_ctx, core_state, meta_ctx, meta_state) = ctx.into_parts();
                let meta_state = MetaContextState {
                    env: env.cast::<LeanAny>().unbind_mt(),
                    core_ctx: core_ctx.cast::<LeanAny>().unbind_mt(),
                    core_state: core_state.cast::<LeanAny>().unbind_mt(),
                    meta_ctx: meta_ctx.cast::<LeanAny>().unbind_mt(),
                    meta_state: meta_state.cast::<LeanAny>().unbind_mt(),
                };

                let goal_unbound = goal.cast::<LeanAny>().unbind_mt();

                Ok((vec![goal_unbound], meta_state))
            })
        });

        match result {
            Ok((goals, meta_state)) => Ok(TacticStatePy {
                goals,
                meta_state: Some(meta_state),
            }),
            Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(format!("{e}"))),
        }
    }

    /// Apply a tactic string and return (next_state, reward, done, info).
    ///
    /// Convenience wrapper that dispatches to the DSL methods.
    /// For structured access, use TacticState methods directly.
    fn step(
        &self,
        py: Python,
        state: &TacticStatePy,
        tactic: &str,
    ) -> PyResult<(TacticStatePy, f64, bool, PyObject)> {
        // Simple dispatch: only support intro and rfl via string for now
        let result = if tactic.trim() == "rfl" {
            state.rfl(py)?
        } else if let Some(name) = tactic.trim().strip_prefix("intro ") {
            state.intro(py, name.trim())?
        } else {
            TacticResultPy {
                success: false,
                error: Some(format!("unsupported tactic string: {tactic}")),
                next_state: None,
            }
        };

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
            "ProofEnvironment(theorem={:?})",
            self.theorem_name
        )
    }
}
