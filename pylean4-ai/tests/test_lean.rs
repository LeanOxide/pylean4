//! End-to-end integration tests for pylean4-ai (pure Rust, no PyO3).
//!
//! These tests exercise the full pipeline without Python:
//! Expr construction → MetaM context → TacticState → tactic → result.
//!
//! Run with: cargo test -p pylean4-ai --test test_lean -- --ignored

use leo3::instance::LeanAny;
use leo3::prelude::*;
use leo3::LeanUnbound;

fn init() {
    leo3::prepare_freethreaded_lean();
}

#[test]
#[ignore] // Requires Lean4 runtime
fn test_expr_construction() {
    init();

    leo3::with_lean(|lean| {
        let nat_name = leo3::meta::LeanName::from_str(lean, "Nat").unwrap();
        let levels = LeanList::nil(lean).unwrap();
        let nat = leo3::meta::LeanExpr::const_(lean, nat_name, levels).unwrap();

        let zero_name = leo3::meta::LeanName::from_str(lean, "Nat.zero").unwrap();
        let levels2 = LeanList::nil(lean).unwrap();
        let zero = leo3::meta::LeanExpr::const_(lean, zero_name, levels2).unwrap();

        // Build @Eq Nat Nat.zero Nat.zero
        let u1 = leo3::meta::LeanLevel::succ(leo3::meta::LeanLevel::zero(lean).unwrap()).unwrap();
        let eq_levels = LeanList::cons(u1.cast(), LeanList::nil(lean).unwrap()).unwrap();
        let eq_type = leo3::meta::LeanExpr::mk_eq(lean, eq_levels, &nat, &zero, &zero).unwrap();

        let s = leo3::meta::LeanExpr::dbg_to_string(&eq_type).unwrap();
        eprintln!("Eq expr: {s}");
        assert!(s.contains("Eq"), "Expected Eq in: {s}");
    });
}

#[test]
#[ignore] // Requires Lean4 runtime
fn test_prove_zero_eq_zero_rfl() {
    init();

    leo3::with_lean(|lean| {
        // Setup environment and MetaM context
        let env = leo3::meta::LeanEnvironment::empty(lean, 0).unwrap();
        let mut ctx = leo3::meta::MetaMContext::new(lean, env).unwrap();

        // Build goal: @Eq Nat Nat.zero Nat.zero
        let nat_name = leo3::meta::LeanName::from_str(lean, "Nat").unwrap();
        let levels = LeanList::nil(lean).unwrap();
        let nat = leo3::meta::LeanExpr::const_(lean, nat_name, levels).unwrap();

        let zero_name = leo3::meta::LeanName::from_str(lean, "Nat.zero").unwrap();
        let levels2 = LeanList::nil(lean).unwrap();
        let zero = leo3::meta::LeanExpr::const_(lean, zero_name, levels2).unwrap();

        let u1 = leo3::meta::LeanLevel::succ(leo3::meta::LeanLevel::zero(lean).unwrap()).unwrap();
        let eq_levels = LeanList::cons(u1.cast(), LeanList::nil(lean).unwrap()).unwrap();
        let eq_type = leo3::meta::LeanExpr::mk_eq(lean, eq_levels, &nat, &zero, &zero).unwrap();

        // Create goal
        let goal = ctx.mk_goal(&eq_type).unwrap();
        let state = leo3::meta::tactic::TacticState::new(vec![goal]);

        assert_eq!(state.num_goals(), 1);
        assert!(!state.is_solved());

        // Apply rfl
        let result = leo3::meta::tactic::rfl(&mut ctx, state);
        match result {
            leo3::meta::tactic::TacticResult::Success(new_state) => {
                assert!(new_state.is_solved(), "Expected solved after rfl");
                eprintln!("✓ Proved 0 = 0 with rfl");
            }
            leo3::meta::tactic::TacticResult::Failure(e) => {
                panic!("rfl failed: {e}");
            }
        }
    });
}

#[test]
#[ignore] // Requires Lean4 runtime
fn test_prove_p_implies_p() {
    init();

    leo3::with_lean(|lean| {
        let env = leo3::meta::LeanEnvironment::empty(lean, 0).unwrap();
        let mut ctx = leo3::meta::MetaMContext::new(lean, env).unwrap();

        // Build: ∀ (P : Prop), P → P
        let prop = leo3::meta::LeanExpr::sort(lean, leo3::meta::LeanLevel::zero(lean).unwrap()).unwrap();
        let bvar0 = leo3::meta::LeanExpr::bvar(lean, 0).unwrap();
        let bvar1 = leo3::meta::LeanExpr::bvar(lean, 1).unwrap();

        let p_name = leo3::meta::LeanName::from_str(lean, "P").unwrap();
        let h_name = leo3::meta::LeanName::from_str(lean, "h").unwrap();

        let inner = leo3::meta::LeanExpr::forall(
            h_name,
            bvar0,
            bvar1,
            leo3::meta::BinderInfo::Default,
        ).unwrap();

        let thm_type = leo3::meta::LeanExpr::forall(
            p_name,
            prop,
            inner,
            leo3::meta::BinderInfo::Default,
        ).unwrap();

        // Create goal
        let goal = ctx.mk_goal(&thm_type).unwrap();
        let state = leo3::meta::tactic::TacticState::new(vec![goal]);
        assert_eq!(state.num_goals(), 1);

        // intro P
        let p_name2 = leo3::meta::LeanName::from_str(lean, "P").unwrap();
        let result = leo3::meta::tactic::intro(&mut ctx, state, &p_name2);
        let state = match result {
            leo3::meta::tactic::TacticResult::Success(s) => {
                eprintln!("✓ intro P succeeded, goals: {}", s.num_goals());
                s
            }
            leo3::meta::tactic::TacticResult::Failure(e) => panic!("intro P failed: {e}"),
        };

        // intro h
        let h_name2 = leo3::meta::LeanName::from_str(lean, "h").unwrap();
        let result = leo3::meta::tactic::intro(&mut ctx, state, &h_name2);
        let state = match result {
            leo3::meta::tactic::TacticResult::Success(s) => {
                eprintln!("✓ intro h succeeded, goals: {}", s.num_goals());
                s
            }
            leo3::meta::tactic::TacticResult::Failure(e) => panic!("intro h failed: {e}"),
        };

        assert_eq!(state.num_goals(), 1);
        eprintln!("✓ After intro P, intro h: 1 goal remaining (need assumption/exact)");
    });
}

#[test]
#[ignore] // Requires Lean4 runtime
fn test_unbound_roundtrip() {
    init();

    // Test that unbind_mt → bind roundtrip preserves the object
    let unbound: LeanUnbound<LeanAny> = leo3::with_lean(|lean| {
        let nat_name = leo3::meta::LeanName::from_str(lean, "Nat").unwrap();
        let levels = LeanList::nil(lean).unwrap();
        let nat = leo3::meta::LeanExpr::const_(lean, nat_name, levels).unwrap();
        nat.cast::<LeanAny>().unbind_mt()
    });

    // Rebind in a new scope
    leo3::with_lean(|lean| {
        let bound = unbound.bind(lean).cast::<leo3::meta::LeanExpr>();
        let s = leo3::meta::LeanExpr::dbg_to_string(&bound).unwrap();
        assert!(s.contains("Nat"), "Expected Nat in: {s}");
        eprintln!("✓ Unbound roundtrip: {s}");
    });
}
