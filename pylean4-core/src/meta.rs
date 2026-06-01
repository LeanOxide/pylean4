//! MetaM / TacticM execution context.
//!
//! Wraps leo3's `MetaMContext` for Python, handling the lifetime erasure
//! by storing unbound references and rebinding them on each call.

use crate::error::lean_to_py_err;
use crate::object::{LeanObject, LeanTypeTag};
use leo3::instance::LeanAny;
use leo3::prelude::*;
use leo3::LeanUnbound;
use pyo3::prelude::*;

/// Stored state for a MetaM context (all MT-safe, lifetime-erased).
struct MetaContextState {
    env: LeanUnbound<LeanAny>,
    core_ctx: LeanUnbound<LeanAny>,
    core_state: LeanUnbound<LeanAny>,
    meta_ctx: LeanUnbound<LeanAny>,
    meta_state: LeanUnbound<LeanAny>,
}

/// Python wrapper for a MetaM execution context.
///
/// Provides the ability to run MetaM computations, which includes
/// type checking, elaboration, and tactic execution.
///
/// # Example
///
/// ```python
/// rt = pylean4.Runtime()
/// env = rt.empty_environment()
/// ctx = pylean4.MetaContext(env)
/// result = ctx.run(some_computation)
/// ```
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
            return Err(crate::error::type_mismatch(
                "Environment",
                &env_obj.type_tag.to_string(),
            ));
        }

        let env_unbound = env_obj.inner.clone();

        // Initialize all context/state objects
        let state = leo3::with_lean(|lean| {
            let env = env_unbound.bind(lean);

            // Use the meta module to create context components
            let core_ctx = leo3::meta::context::CoreContext::mk_default(lean)?;
            let env_for_state = env.clone();
            let core_state =
                leo3::meta::context::CoreState::mk_core_state(lean, &env_for_state.cast())?;
            let meta_ctx = leo3::meta::context::MetaContext::mk_default(lean)?;
            let meta_state = leo3::meta::context::MetaState::mk_meta_state(lean)?;

            Ok::<MetaContextState, leo3::LeanError>(MetaContextState {
                env: env.unbind_mt(),
                core_ctx: core_ctx.cast::<LeanAny>().unbind_mt(),
                core_state: core_state.cast::<LeanAny>().unbind_mt(),
                meta_ctx: meta_ctx.cast::<LeanAny>().unbind_mt(),
                meta_state: meta_state.cast::<LeanAny>().unbind_mt(),
            })
        });

        match state {
            Ok(s) => Ok(Self { state: Some(s) }),
            Err(e) => Err(lean_to_py_err(e)),
        }
    }

    /// Run a MetaM computation and return the result.
    ///
    /// The computation should be a LeanObject representing a `MetaM α` value.
    /// Returns the result as a LeanObject.
    fn run(&mut self, py: Python, computation: &LeanObject) -> PyResult<LeanObject> {
        let state = self
            .state
            .as_ref()
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("MetaContext is closed"))?;

        let comp_unbound = computation.inner.clone();
        let env = state.env.clone();
        let core_ctx = state.core_ctx.clone();
        let core_state = state.core_state.clone();
        let meta_ctx = state.meta_ctx.clone();
        let meta_state = state.meta_state.clone();

        // Release GIL during Lean computation
        let result = py.allow_threads(move || {
            leo3::with_lean(|lean| {
                let mut ctx = unsafe {
                    // Rebind all state to the current lifetime
                    leo3::meta::MetaMContext::from_parts(
                        lean,
                        env.into_bound(lean).cast(),
                        core_ctx.into_bound(lean).cast(),
                        core_state.into_bound(lean).cast(),
                        meta_ctx.into_bound(lean).cast(),
                        meta_state.into_bound(lean).cast(),
                    )
                };

                let comp_bound = comp_unbound.into_bound(lean);
                let result = ctx.run(comp_bound)?;
                Ok::<LeanUnbound<LeanAny>, leo3::LeanError>(result.cast::<LeanAny>().unbind_mt())
            })
        });

        match result {
            Ok(unbound) => Ok(LeanObject::new(unbound, LeanTypeTag::Other)),
            Err(e) => Err(lean_to_py_err(e)),
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
