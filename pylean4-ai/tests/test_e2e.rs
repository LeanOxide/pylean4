//! End-to-end integration tests for pylean4-ai.
//!
//! These tests exercise the full pipeline: Expr construction → MetaM context
//! → TacticState → tactic application → result extraction.
//!
//! Requires Lean4 runtime linked (run without LEO3_NO_LEAN=1).

use leo3::instance::LeanAny;
use leo3::prelude::*;
use leo3::LeanUnbound;

/// Helper: create a MetaContextState from an empty environment.
fn make_meta_state() -> _ai::tactic_state::MetaContextState {
    leo3::with_lean(|lean| {
        let env = leo3::meta::LeanEnvironment::empty(lean, 0).unwrap();
        let core_ctx = leo3::meta::context::CoreContext::mk_default(lean).unwrap();
        let env_for_state = env.clone();
        let core_state =
            leo3::meta::context::CoreState::mk_core_state(lean, &env_for_state.cast()).unwrap();
        let meta_ctx = leo3::meta::context::MetaContext::mk_default(lean).unwrap();
        let meta_state = leo3::meta::context::MetaState::mk_meta_state(lean).unwrap();

        _ai::tactic_state::MetaContextState {
            env: env.cast::<LeanAny>().unbind_mt(),
            core_ctx: core_ctx.cast::<LeanAny>().unbind_mt(),
            core_state: core_state.cast::<LeanAny>().unbind_mt(),
            meta_ctx: meta_ctx.cast::<LeanAny>().unbind_mt(),
            meta_state: meta_state.cast::<LeanAny>().unbind_mt(),
        }
    })
}

/// Helper: create a goal from a type expression.
fn make_goal(
    meta: &_ai::tactic_state::MetaContextState,
    type_expr: LeanUnbound<LeanAny>,
) -> LeanUnbound<LeanAny> {
    let meta_clone = meta.clone();
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
        let ty = type_expr.into_bound(lean).cast();
        let goal = ctx.mk_goal(&ty).unwrap();
        goal.cast::<LeanAny>().unbind_mt()
    })
}

#[test]
#[ignore] // Requires Lean4 runtime
fn test_expr_construction() {
    leo3::prepare_freethreaded_lean();

    leo3::with_lean(|lean| {
        // Build: Nat
        let nat_name = leo3::meta::LeanName::from_str(lean, "Nat").unwrap();
        let levels = LeanList::nil(lean).unwrap();
        let nat = leo3::meta::LeanExpr::const_(lean, nat_name, levels).unwrap();

        // Build: Nat.zero
        let zero_name = leo3::meta::LeanName::from_str(lean, "Nat.zero").unwrap();
        let levels2 = LeanList::nil(lean).unwrap();
        let zero = leo3::meta::LeanExpr::const_(lean, zero_name, levels2).unwrap();

        // Build: @Eq Nat Nat.zero Nat.zero
        let eq_name = leo3::meta::LeanName::from_str(lean, "Eq").unwrap();
        let u1 = leo3::meta::LeanLevel::succ(leo3::meta::LeanLevel::zero(lean).unwrap()).unwrap();
        let eq_levels = LeanList::cons(u1.cast(), LeanList::nil(lean).unwrap()).unwrap();
        let eq_type = leo3::meta::LeanExpr::mk_eq(lean, eq_levels, &nat, &zero, &zero).unwrap();

        let s = leo3::meta::LeanExpr::dbg_to_string(&eq_type).unwrap();
        assert!(s.contains("Eq"), "Expected Eq in: {s}");
    });
}

#[test]
#[ignore] // Requires Lean4 runtime
fn test_prove_rfl() {
    leo3::prepare_freethreaded_lean();

    let meta = make_meta_state();

    // Build goal type: @Eq Nat Nat.zero Nat.zero
    let eq_type = leo3::with_lean(|lean| {
        let nat_name = leo3::meta::LeanName::from_str(lean, "Nat").unwrap();
        let levels = LeanList::nil(lean).unwrap();
        let nat = leo3::meta::LeanExpr::const_(lean, nat_name, levels).unwrap();

        let zero_name = leo3::meta::LeanName::from_str(lean, "Nat.zero").unwrap();
        let levels2 = LeanList::nil(lean).unwrap();
        let zero = leo3::meta::LeanExpr::const_(lean, zero_name, levels2).unwrap();

        let u1 = leo3::meta::LeanLevel::succ(leo3::meta::LeanLevel::zero(lean).unwrap()).unwrap();
        let eq_levels = LeanList::cons(u1.cast(), LeanList::nil(lean).unwrap()).unwrap();
        let eq_expr = leo3::meta::LeanExpr::mk_eq(lean, eq_levels, &nat, &zero, &zero).unwrap();
        eq_expr.cast::<LeanAny>().unbind_mt()
    });

    // Create goal
    let goal = make_goal(&meta, eq_type);

    // Create TacticState and apply rfl
    let state = _ai::tactic_state::TacticStatePy {
        goals: vec![goal],
        meta_state: Some(meta),
    };

    assert_eq!(state.num_goals(), 1);
    assert!(!state.is_solved());

    // Apply rfl — this should close the goal
    pyo3::prepare_freethreaded_python();
    pyo3::Python::with_gil(|py| {
        let result = state.rfl(py).unwrap();
        assert!(result.success, "rfl failed: {:?}", result.error);
        let next = result.next_state.unwrap();
        assert!(next.is_solved(), "Expected solved state after rfl");
    });
}

#[test]
#[ignore] // Requires Lean4 runtime
fn test_prove_intro_then_exact() {
    leo3::prepare_freethreaded_lean();

    let meta = make_meta_state();

    // Build goal type: ∀ (P : Prop), P → P
    let thm_type = leo3::with_lean(|lean| {
        let prop = leo3::meta::LeanExpr::sort(lean, leo3::meta::LeanLevel::zero(lean).unwrap()).unwrap();
        let bvar0 = leo3::meta::LeanExpr::bvar(lean, 0).unwrap();
        let bvar1 = leo3::meta::LeanExpr::bvar(lean, 1).unwrap();

        let p_name = leo3::meta::LeanName::from_str(lean, "P").unwrap();
        let h_name = leo3::meta::LeanName::from_str(lean, "h").unwrap();

        // inner: ∀ (h : P), P  (where P = bvar(0) under one binder, body = bvar(0) = h)
        let inner = leo3::meta::LeanExpr::forall(
            h_name,
            bvar0,
            bvar1,
            leo3::meta::BinderInfo::Default,
        ).unwrap();

        // outer: ∀ (P : Prop), (∀ (h : P), P)
        let outer = leo3::meta::LeanExpr::forall(
            p_name,
            prop,
            inner,
            leo3::meta::BinderInfo::Default,
        ).unwrap();

        outer.cast::<LeanAny>().unbind_mt()
    });

    let goal = make_goal(&meta, thm_type);

    let state = _ai::tactic_state::TacticStatePy {
        goals: vec![goal],
        meta_state: Some(meta),
    };

    pyo3::prepare_freethreaded_python();
    pyo3::Python::with_gil(|py| {
        // intro P
        let result = state.intro(py, "P").unwrap();
        assert!(result.success, "intro P failed: {:?}", result.error);
        let state2 = result.next_state.unwrap();
        assert_eq!(state2.num_goals(), 1);

        // intro h
        let result = state2.intro(py, "h").unwrap();
        assert!(result.success, "intro h failed: {:?}", result.error);
        let state3 = result.next_state.unwrap();
        assert_eq!(state3.num_goals(), 1);

        // At this point the goal should be `P` with `h : P` in context
        // We'd need `assumption` or `exact h` to close it
    });
}
