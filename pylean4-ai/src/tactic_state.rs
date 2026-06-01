//! TacticState: Python DSL for tactic-style proof construction.
//!
//! Instead of parsing tactic strings, each tactic is a direct method call.
//! Expressions are constructed via the `Expr` Python class.

use leo3::instance::LeanAny;
use leo3::prelude::*;
use leo3::LeanUnbound;
use pyo3::prelude::*;

// ============================================================================
// MetaContextState — shared context for all TacticState instances
// ============================================================================

/// Stored MetaM context state (all MT-safe, lifetime-erased).
#[derive(Clone)]
pub(crate) struct MetaContextState {
    pub env: LeanUnbound<LeanAny>,
    pub core_ctx: LeanUnbound<LeanAny>,
    pub core_state: LeanUnbound<LeanAny>,
    pub meta_ctx: LeanUnbound<LeanAny>,
    pub meta_state: LeanUnbound<LeanAny>,
}

// ============================================================================
// TacticState
// ============================================================================

/// A proof state snapshot. Immutable — each tactic returns a new state.
///
/// # Example
///
/// ```python
/// state = ctx.mk_goal(prop_type)
/// state = state.intro("h")
/// state = state.rfl()
/// assert state.is_solved
/// ```
#[pyclass(name = "TacticState")]
#[derive(Clone)]
pub struct TacticStatePy {
    pub(crate) goals: Vec<LeanUnbound<LeanAny>>,
    pub(crate) meta_state: Option<MetaContextState>,
}

#[pymethods]
impl TacticStatePy {
    /// Number of remaining goals.
    #[getter]
    pub fn num_goals(&self) -> usize {
        self.goals.len()
    }

    /// Whether the proof is complete.
    #[getter]
    pub fn is_solved(&self) -> bool {
        self.goals.is_empty()
    }

    /// Pretty-printed goal types.
    #[getter]
    fn goals_pp(&self) -> Vec<String> {
        let Some(ref meta) = self.meta_state else {
            return vec!["<no context>".into(); self.goals.len()];
        };
        let goals: Vec<_> = self.goals.iter().map(|g| g.clone()).collect();
        let meta = meta.clone();

        leo3::with_lean(|lean| {
            let mut ctx = rebuild_ctx(lean, meta);
            goals
                .into_iter()
                .map(|g| pp_goal(&mut ctx, g.into_bound(lean).cast()))
                .collect()
        })
    }

    // ====================================================================
    // Tactic DSL methods
    // ====================================================================

    /// Introduce a hypothesis from a ∀/Π goal.
    ///
    /// ```python
    /// state = state.intro("h")
    /// ```
    pub fn intro(&self, py: Python, name: &str) -> PyResult<TacticResultPy> {
        self.run_tactic(py, |ctx, state, lean| {
            let lean_name = leo3::meta::LeanName::from_str(lean, name)?;
            Ok(leo3::meta::tactic::intro(ctx, state, &lean_name))
        })
    }

    /// Close the goal with an exact proof term.
    ///
    /// ```python
    /// proof = Expr.app(Expr.const("Eq.refl"), nat, zero)
    /// state = state.exact(proof)
    /// ```
    fn exact(&self, py: Python, expr: &ExprPy) -> PyResult<TacticResultPy> {
        let expr_unbound = expr.inner.clone();
        self.run_tactic(py, move |ctx, state, lean| {
            let bound_expr = expr_unbound.into_bound(lean).cast();
            Ok(leo3::meta::tactic::exact(ctx, state, &bound_expr))
        })
    }

    /// Close a reflexivity goal (a = a).
    ///
    /// ```python
    /// state = state.rfl()
    /// ```
    pub fn rfl(&self, py: Python) -> PyResult<TacticResultPy> {
        self.run_tactic(py, |ctx, state, _lean| {
            Ok(leo3::meta::tactic::rfl(ctx, state))
        })
    }

    /// Apply a function/lemma to the main goal, creating subgoals for its arguments.
    ///
    /// ```python
    /// lemma = Expr.const("Nat.add_comm")
    /// state = state.apply_expr(lemma)
    /// ```
    fn apply_expr(&self, py: Python, expr: &ExprPy) -> PyResult<TacticResultPy> {
        let expr_unbound = expr.inner.clone();
        self.run_tactic(py, move |ctx, state, lean| {
            let bound_expr = expr_unbound.into_bound(lean).cast();
            Ok(leo3::meta::tactic::apply(ctx, state, &bound_expr))
        })
    }

    // ====================================================================
    // Pure state manipulation
    // ====================================================================

    /// Focus on the first goal, discarding all others.
    ///
    /// ```python
    /// focused = state.focus()
    /// assert focused.num_goals == 1
    /// ```
    fn focus(&self) -> PyResult<Self> {
        if self.goals.is_empty() {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "no goals to focus on",
            ));
        }
        Ok(TacticStatePy {
            goals: vec![self.goals[0].clone()],
            meta_state: self.meta_state.clone(),
        })
    }

    /// Swap the first two goals.
    ///
    /// ```python
    /// swapped = state.swap()
    /// ```
    fn swap(&self) -> PyResult<Self> {
        if self.goals.len() < 2 {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "need at least 2 goals to swap",
            ));
        }
        let mut new_goals = self.goals.clone();
        new_goals.swap(0, 1);
        Ok(TacticStatePy {
            goals: new_goals,
            meta_state: self.meta_state.clone(),
        })
    }

    fn __repr__(&self) -> String {
        if self.is_solved() {
            "TacticState(solved)".into()
        } else {
            format!("TacticState(goals={})", self.num_goals())
        }
    }
}

impl TacticStatePy {
    /// Internal helper: run a tactic closure with proper lifecycle management.
    fn run_tactic<F>(&self, py: Python, tactic_fn: F) -> PyResult<TacticResultPy>
    where
        F: for<'l> FnOnce(
                &mut leo3::meta::MetaMContext<'l>,
                leo3::meta::tactic::TacticState<'l>,
                leo3::Lean<'l>,
            ) -> LeanResult<leo3::meta::tactic::TacticResult<'l>>
            + Send,
    {
        let Some(ref meta) = self.meta_state else {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "TacticState has no MetaM context",
            ));
        };
        if self.goals.is_empty() {
            return Ok(TacticResultPy {
                success: false,
                error: Some("no goals".into()),
                next_state: None,
            });
        }

        let goals: Vec<_> = self.goals.iter().map(|g| g.clone()).collect();
        let meta = meta.clone();
        let meta_for_result = self.meta_state.clone();

        let result = py.allow_threads(move || {
            leo3::with_lean(|lean| {
                let mut ctx = rebuild_ctx(lean, meta);
                let bound_goals: Vec<_> = goals
                    .into_iter()
                    .map(|g| g.into_bound(lean).cast())
                    .collect();
                let state = leo3::meta::tactic::TacticState::new(bound_goals);

                let tactic_result = tactic_fn(&mut ctx, state, lean)?;

                match tactic_result {
                    leo3::meta::tactic::TacticResult::Success(new_state) => {
                        let new_goals: Vec<_> = new_state
                            .into_goals()
                            .into_iter()
                            .map(|g| g.cast::<LeanAny>().unbind_mt())
                            .collect();
                        Ok(Ok(new_goals))
                    }
                    leo3::meta::tactic::TacticResult::Failure(e) => Ok(Err(e)),
                }
            })
        });

        match result {
            Ok(Ok(new_goals)) => Ok(TacticResultPy {
                success: true,
                error: None,
                next_state: Some(TacticStatePy {
                    goals: new_goals,
                    meta_state: meta_for_result,
                }),
            }),
            Ok(Err(e)) | Err(e) => Ok(TacticResultPy {
                success: false,
                error: Some(format!("{e}")),
                next_state: None,
            }),
        }
    }
}

// ============================================================================
// TacticResult
// ============================================================================

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
            "TacticResult(ok)".into()
        } else {
            format!("TacticResult(err={:?})", self.error.as_deref().unwrap_or("?"))
        }
    }

    fn __bool__(&self) -> bool {
        self.success
    }
}

// ============================================================================
// Expr DSL
// ============================================================================

/// A Lean4 expression. Construct with class methods, compose with `.app()`.
///
/// ```python
/// nat = Expr.const("Nat")
/// zero = Expr.const("Nat.zero")
/// one = Expr.const("Nat.succ").app(zero)
/// eq = Expr.const("Eq").app(nat).app(one).app(one)
/// ```
#[pyclass(name = "Expr")]
#[derive(Clone)]
pub struct ExprPy {
    pub(crate) inner: LeanUnbound<LeanAny>,
}

#[pymethods]
impl ExprPy {
    // ---- Constructors (class methods) ----

    /// Named constant: `Expr.const("Nat.add")`
    #[staticmethod]
    fn const_(name: &str) -> PyResult<Self> {
        let result = leo3::with_lean(|lean| {
            let n = leo3::meta::LeanName::from_str(lean, name)?;
            let levels = LeanList::nil(lean)?;
            let expr = leo3::meta::LeanExpr::const_(lean, n, levels)?;
            Ok::<_, leo3::LeanError>(expr.cast::<LeanAny>().unbind_mt())
        });
        match result {
            Ok(inner) => Ok(Self { inner }),
            Err(e) => Err(error_to_py(e)),
        }
    }

    /// Bound variable (de Bruijn index): `Expr.bvar(0)`
    #[staticmethod]
    fn bvar(idx: usize) -> PyResult<Self> {
        let result = leo3::with_lean(|lean| {
            let expr = leo3::meta::LeanExpr::bvar(lean, idx)?;
            Ok::<_, leo3::LeanError>(expr.cast::<LeanAny>().unbind_mt())
        });
        match result {
            Ok(inner) => Ok(Self { inner }),
            Err(e) => Err(error_to_py(e)),
        }
    }

    /// Sort (universe): `Expr.sort(0)` = Prop, `Expr.sort(1)` = Type
    #[staticmethod]
    fn sort(level: usize) -> PyResult<Self> {
        let result = leo3::with_lean(|lean| {
            let lvl = if level == 0 {
                leo3::meta::LeanLevel::zero(lean)?
            } else {
                let mut l = leo3::meta::LeanLevel::zero(lean)?;
                for _ in 0..level {
                    l = leo3::meta::LeanLevel::succ(l)?;
                }
                l
            };
            let expr = leo3::meta::LeanExpr::sort(lean, lvl)?;
            Ok::<_, leo3::LeanError>(expr.cast::<LeanAny>().unbind_mt())
        });
        match result {
            Ok(inner) => Ok(Self { inner }),
            Err(e) => Err(error_to_py(e)),
        }
    }

    /// Function application: `f.app(arg)`
    fn app(&self, arg: &ExprPy) -> PyResult<Self> {
        let f = self.inner.clone();
        let a = arg.inner.clone();
        let result = leo3::with_lean(|lean| {
            let f_bound = f.into_bound(lean).cast();
            let a_bound = a.into_bound(lean).cast();
            let expr = leo3::meta::LeanExpr::app(&f_bound, &a_bound)?;
            Ok::<_, leo3::LeanError>(expr.cast::<LeanAny>().unbind_mt())
        });
        match result {
            Ok(inner) => Ok(Self { inner }),
            Err(e) => Err(error_to_py(e)),
        }
    }

    /// Lambda: `Expr.lam("x", type_expr, body_expr)`
    #[staticmethod]
    fn lam(name: &str, ty: &ExprPy, body: &ExprPy) -> PyResult<Self> {
        let ty_u = ty.inner.clone();
        let body_u = body.inner.clone();
        let result = leo3::with_lean(|lean| {
            let n = leo3::meta::LeanName::from_str(lean, name)?;
            let ty_b = ty_u.into_bound(lean).cast();
            let body_b = body_u.into_bound(lean).cast();
            let expr = leo3::meta::LeanExpr::lambda(
                n,
                ty_b,
                body_b,
                leo3::meta::BinderInfo::Default,
            )?;
            Ok::<_, leo3::LeanError>(expr.cast::<LeanAny>().unbind_mt())
        });
        match result {
            Ok(inner) => Ok(Self { inner }),
            Err(e) => Err(error_to_py(e)),
        }
    }

    /// Forall/Pi type: `Expr.forall_("x", type_expr, body_expr)`
    #[staticmethod]
    #[pyo3(name = "forall_")]
    fn forall_py(name: &str, ty: &ExprPy, body: &ExprPy) -> PyResult<Self> {
        let ty_u = ty.inner.clone();
        let body_u = body.inner.clone();
        let result = leo3::with_lean(|lean| {
            let n = leo3::meta::LeanName::from_str(lean, name)?;
            let ty_b = ty_u.into_bound(lean).cast();
            let body_b = body_u.into_bound(lean).cast();
            let expr = leo3::meta::LeanExpr::forall(
                n,
                ty_b,
                body_b,
                leo3::meta::BinderInfo::Default,
            )?;
            Ok::<_, leo3::LeanError>(expr.cast::<LeanAny>().unbind_mt())
        });
        match result {
            Ok(inner) => Ok(Self { inner }),
            Err(e) => Err(error_to_py(e)),
        }
    }

    /// Arrow type (non-dependent): `Expr.arrow(a, b)` = `a → b`
    #[staticmethod]
    fn arrow(from: &ExprPy, to: &ExprPy) -> PyResult<Self> {
        let from_u = from.inner.clone();
        let to_u = to.inner.clone();
        let result = leo3::with_lean(|lean| {
            let from_b = from_u.into_bound(lean).cast();
            let to_b = to_u.into_bound(lean).cast();
            let expr = leo3::meta::LeanExpr::arrow(from_b, to_b)?;
            Ok::<_, leo3::LeanError>(expr.cast::<LeanAny>().unbind_mt())
        });
        match result {
            Ok(inner) => Ok(Self { inner }),
            Err(e) => Err(error_to_py(e)),
        }
    }

    /// Build `@Eq ty lhs rhs` (equality proposition).
    ///
    /// ```python
    /// prop = Expr.eq(nat, one, one)  # 1 = 1
    /// ```
    #[staticmethod]
    fn eq(ty: &ExprPy, lhs: &ExprPy, rhs: &ExprPy) -> PyResult<Self> {
        let ty_u = ty.inner.clone();
        let lhs_u = lhs.inner.clone();
        let rhs_u = rhs.inner.clone();
        let result = leo3::with_lean(|lean| {
            let ty_b = ty_u.into_bound(lean).cast();
            let lhs_b = lhs_u.into_bound(lean).cast();
            let rhs_b = rhs_u.into_bound(lean).cast();
            let u = leo3::meta::LeanLevel::one(lean)?;
            let levels = LeanList::cons(u.cast::<LeanAny>(), LeanList::nil(lean)?)?;
            let expr = leo3::meta::LeanExpr::mk_eq(lean, levels, &ty_b, &lhs_b, &rhs_b)?;
            Ok::<_, leo3::LeanError>(expr.cast::<LeanAny>().unbind_mt())
        });
        match result {
            Ok(inner) => Ok(Self { inner }),
            Err(e) => Err(error_to_py(e)),
        }
    }

    /// Build `@Eq.refl ty val` (reflexivity proof: val = val).
    ///
    /// ```python
    /// proof = Expr.eq_refl(nat, zero)  # proves 0 = 0
    /// ```
    #[staticmethod]
    fn eq_refl(ty: &ExprPy, val: &ExprPy) -> PyResult<Self> {
        let ty_u = ty.inner.clone();
        let val_u = val.inner.clone();
        let result = leo3::with_lean(|lean| {
            let ty_b = ty_u.into_bound(lean).cast();
            let val_b = val_u.into_bound(lean).cast();
            let u = leo3::meta::LeanLevel::one(lean)?;
            let levels = LeanList::cons(u.cast::<LeanAny>(), LeanList::nil(lean)?)?;
            let expr = leo3::meta::LeanExpr::mk_eq_refl(lean, levels, &ty_b, &val_b)?;
            Ok::<_, leo3::LeanError>(expr.cast::<LeanAny>().unbind_mt())
        });
        match result {
            Ok(inner) => Ok(Self { inner }),
            Err(e) => Err(error_to_py(e)),
        }
    }

    /// Build a Nat literal expression.
    ///
    /// ```python
    /// five = Expr.nat_lit(5)
    /// ```
    #[staticmethod]
    fn nat_lit(n: u64) -> PyResult<Self> {
        let result = leo3::with_lean(|lean| {
            let lit = leo3::meta::LeanLiteral::nat(lean, n)?;
            let expr = leo3::meta::LeanExpr::lit(lean, lit)?;
            Ok::<_, leo3::LeanError>(expr.cast::<LeanAny>().unbind_mt())
        });
        match result {
            Ok(inner) => Ok(Self { inner }),
            Err(e) => Err(error_to_py(e)),
        }
    }

    /// Build a String literal expression.
    ///
    /// ```python
    /// hello = Expr.str_lit("hello")
    /// ```
    #[staticmethod]
    fn str_lit(s: &str) -> PyResult<Self> {
        let result = leo3::with_lean(|lean| {
            let lit = leo3::meta::LeanLiteral::string(lean, s)?;
            let expr = leo3::meta::LeanExpr::lit(lean, lit)?;
            Ok::<_, leo3::LeanError>(expr.cast::<LeanAny>().unbind_mt())
        });
        match result {
            Ok(inner) => Ok(Self { inner }),
            Err(e) => Err(error_to_py(e)),
        }
    }

    fn __repr__(&self) -> String {
        leo3::with_lean(|lean| {
            let bound = self.inner.bind(lean).cast();
            match leo3::meta::LeanExpr::dbg_to_string(&bound) {
                Ok(s) => format!("Expr({s})"),
                Err(_) => "Expr(?)".into(),
            }
        })
    }
}

// ============================================================================
// Helpers
// ============================================================================

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

fn pp_goal<'l>(
    ctx: &mut leo3::meta::MetaMContext<'l>,
    goal: LeanBound<'l, leo3::meta::LeanExpr>,
) -> String {
    match leo3::meta::tactic::goal_type(ctx, &goal) {
        Ok(ty) => match leo3::meta::LeanExpr::dbg_to_string(&ty) {
            Ok(s) => format!("⊢ {s}"),
            Err(_) => "⊢ <pp failed>".into(),
        },
        Err(_) => "⊢ <unknown>".into(),
    }
}

pub(crate) fn error_to_py(e: leo3::LeanError) -> PyErr {
    pyo3::exceptions::PyRuntimeError::new_err(format!("{e}"))
}
