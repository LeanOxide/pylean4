//! MetaM context exposed by the combined Python extension.

use crate::tactic_state::{error_to_py, ExprPy, MetaContextState, TacticStatePy};
use _core::{LeanObject, LeanTypeTag};
use leo3::instance::LeanAny;
use pyo3::prelude::*;

/// Python wrapper for a Lean MetaM execution context.
#[pyclass(name = "MetaContext")]
pub struct MetaContextPy {
    state: Option<MetaContextState>,
}

#[pymethods]
impl MetaContextPy {
    /// Create a new MetaContext from an environment LeanObject.
    #[new]
    fn new(env_obj: &LeanObject) -> PyResult<Self> {
        if env_obj.type_tag != LeanTypeTag::Environment {
            return Err(pyo3::exceptions::PyTypeError::new_err(format!(
                "expected Environment, got {}",
                env_obj.type_tag
            )));
        }

        let env_unbound = env_obj.inner.clone();
        let state = leo3::with_lean(|lean| {
            let env = env_unbound.bind(lean);
            let core_ctx = leo3::meta::context::CoreContext::mk_default(lean)?;
            let core_state =
                leo3::meta::context::CoreState::mk_core_state(lean, &env.clone().cast())?;
            let meta_ctx = leo3::meta::context::MetaContext::mk_default(lean)?;
            let meta_state = leo3::meta::context::MetaState::mk_meta_state(lean)?;

            Ok::<_, leo3::LeanError>(MetaContextState {
                env: env.unbind_mt(),
                core_ctx: core_ctx.cast::<LeanAny>().unbind_mt(),
                core_state: core_state.cast::<LeanAny>().unbind_mt(),
                meta_ctx: meta_ctx.cast::<LeanAny>().unbind_mt(),
                meta_state: meta_state.cast::<LeanAny>().unbind_mt(),
            })
        });

        match state {
            Ok(state) => Ok(Self { state: Some(state) }),
            Err(e) => Err(error_to_py(e)),
        }
    }

    /// Create an initial tactic state with one goal of the given type.
    fn mk_goal(&self, py: Python, goal_type: &ExprPy) -> PyResult<TacticStatePy> {
        let state = self
            .state
            .clone()
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("MetaContext is closed"))?;
        let goal_type = goal_type.inner.clone();

        let result = py.allow_threads(move || {
            leo3::with_lean(|lean| {
                let mut ctx = rebuild_ctx(lean, state);
                let goal_type = goal_type.into_bound(lean).cast();
                let goal = ctx.mk_goal(&goal_type)?;
                let (env, core_ctx, core_state, meta_ctx, meta_state) = ctx.into_parts();
                let meta_state = MetaContextState {
                    env: env.cast::<LeanAny>().unbind_mt(),
                    core_ctx: core_ctx.cast::<LeanAny>().unbind_mt(),
                    core_state: core_state.cast::<LeanAny>().unbind_mt(),
                    meta_ctx: meta_ctx.cast::<LeanAny>().unbind_mt(),
                    meta_state: meta_state.cast::<LeanAny>().unbind_mt(),
                };

                Ok::<_, leo3::LeanError>((goal.cast::<LeanAny>().unbind_mt(), meta_state))
            })
        });

        match result {
            Ok((goal, meta_state)) => Ok(TacticStatePy {
                goals: vec![goal],
                meta_state: Some(meta_state),
            }),
            Err(e) => Err(error_to_py(e)),
        }
    }

    /// Check if this context is still usable.
    #[getter]
    fn is_valid(&self) -> bool {
        self.state.is_some()
    }

    fn __repr__(&self) -> String {
        if self.state.is_some() {
            "pylean4.MetaContext(valid=True)".to_string()
        } else {
            "pylean4.MetaContext(valid=False)".to_string()
        }
    }
}

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
