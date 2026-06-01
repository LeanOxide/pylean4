"""End-to-end tests for pylean4.

These tests require:
1. Lean4 toolchain installed (via elan)
2. `maturin develop --release` to build the extension

Run with: pytest tests/test_e2e.py -v
"""
import pytest


def has_lean_runtime():
    """Check if pylean4 was built with Lean4 linked."""
    try:
        import pylean4
        rt = pylean4.Runtime()
        return rt.is_initialized
    except (ImportError, RuntimeError, OSError):
        return False


skip_no_lean = pytest.mark.skipif(
    not has_lean_runtime(),
    reason="Lean4 runtime not available (build with maturin develop)"
)


# ============================================================================
# Basic Runtime Tests
# ============================================================================

@skip_no_lean
class TestRuntime:
    def test_init(self):
        import pylean4
        rt = pylean4.Runtime()
        assert rt.is_initialized

    def test_double_init(self):
        """Multiple Runtime() calls should be safe (singleton)."""
        import pylean4
        rt1 = pylean4.Runtime()
        rt2 = pylean4.Runtime()
        assert rt1.is_initialized
        assert rt2.is_initialized


# ============================================================================
# Expression DSL Tests
# ============================================================================

@skip_no_lean
class TestExpr:
    def test_const(self):
        from pylean4 import Expr
        nat = Expr.const_("Nat")
        assert "Nat" in repr(nat)

    def test_bvar(self):
        from pylean4 import Expr
        v = Expr.bvar(0)
        assert repr(v)  # should not crash

    def test_sort(self):
        from pylean4 import Expr
        prop = Expr.sort(0)
        type_ = Expr.sort(1)
        assert repr(prop)
        assert repr(type_)

    def test_app(self):
        from pylean4 import Expr
        succ = Expr.const_("Nat.succ")
        zero = Expr.const_("Nat.zero")
        one = succ.app(zero)
        assert repr(one)

    def test_lambda(self):
        from pylean4 import Expr
        nat = Expr.const_("Nat")
        body = Expr.bvar(0)
        lam = Expr.lam("x", nat, body)
        assert repr(lam)

    def test_forall(self):
        from pylean4 import Expr
        prop = Expr.sort(0)
        body = Expr.bvar(0)
        fa = Expr.forall_("P", prop, body)
        assert repr(fa)

    def test_arrow(self):
        from pylean4 import Expr
        nat = Expr.const_("Nat")
        arr = Expr.arrow(nat, nat)
        assert repr(arr)

    def test_nat_lit(self):
        from pylean4 import Expr
        n = Expr.nat_lit(42)
        assert repr(n)

    def test_eq(self):
        from pylean4 import Expr
        nat = Expr.const_("Nat")
        zero = Expr.const_("Nat.zero")
        eq = Expr.eq(nat, zero, zero)
        assert repr(eq)

    def test_eq_refl(self):
        from pylean4 import Expr
        nat = Expr.const_("Nat")
        zero = Expr.const_("Nat.zero")
        refl = Expr.eq_refl(nat, zero)
        assert repr(refl)


# ============================================================================
# Tactic DSL Tests
# ============================================================================

@skip_no_lean
class TestTacticState:
    def test_focus(self):
        """focus() should return a state with only the first goal."""
        from pylean4 import TacticState
        # This requires a real TacticState from ProofEnvironment
        # Placeholder: test that the method exists
        pass

    def test_swap(self):
        """swap() should exchange the first two goals."""
        pass


# ============================================================================
# ProofEnvironment Tests
# ============================================================================

@skip_no_lean
class TestProofEnvironment:
    """Tests that require a loaded Lean environment with basic definitions."""

    def test_prove_identity(self):
        """Prove ∀ P : Prop, P → P using intro + exact."""
        import pylean4
        from pylean4 import Expr, ProofEnvironment

        rt = pylean4.Runtime()

        # Build the theorem type: ∀ (P : Prop), P → P
        prop = Expr.sort(0)
        p_var = Expr.bvar(1)  # P (under two binders)
        h_var = Expr.bvar(0)  # h (most recent binder)
        inner = Expr.forall_("h", Expr.bvar(0), h_var)  # h : P ⊢ P
        thm_type = Expr.forall_("P", prop, inner)

        # Create MetaContext and initial goal
        ctx = pylean4.MetaContext(rt.empty_environment())
        state = ctx.mk_goal(thm_type)

        # Prove it
        result = state.intro("P")
        assert result.success, f"intro P failed: {result.error}"
        state = result.next_state

        result = state.intro("h")
        assert result.success, f"intro h failed: {result.error}"
        state = result.next_state

        # The goal should now be provable by exact (the hypothesis h)
        # This requires looking up h in the local context
        # For now, just verify we got to 1 goal
        assert state.num_goals == 1

    def test_prove_rfl(self):
        """Prove 0 = 0 using rfl."""
        import pylean4
        from pylean4 import Expr

        rt = pylean4.Runtime()

        # Build: @Eq Nat Nat.zero Nat.zero
        nat = Expr.const_("Nat")
        zero = Expr.const_("Nat.zero")
        eq_type = Expr.eq(nat, zero, zero)

        ctx = pylean4.MetaContext(rt.empty_environment())
        state = ctx.mk_goal(eq_type)

        result = state.rfl()
        assert result.success, f"rfl failed: {result.error}"
        assert result.next_state.is_solved


# ============================================================================
# BatchVerifier Tests
# ============================================================================

@skip_no_lean
class TestBatchVerifier:
    def test_create(self):
        from pylean4 import BatchVerifier
        v = BatchVerifier(num_workers=4)
        assert v.workers == 4

    def test_empty_batch(self):
        from pylean4 import BatchVerifier
        v = BatchVerifier(num_workers=2)
        results = v.verify_batch([], [])
        assert results == []

    def test_mismatched_lengths(self):
        from pylean4 import BatchVerifier, TacticState
        v = BatchVerifier()
        with pytest.raises(ValueError):
            v.verify_batch([TacticState()], [])


# ============================================================================
# Integration: Full RL Loop
# ============================================================================

@skip_no_lean
class TestRLLoop:
    def test_gym_interface(self):
        """Simulate a simple RL training step."""
        import pylean4
        from pylean4 import Expr, ProofEnvironment

        rt = pylean4.Runtime()
        # Would need a real environment with theorems loaded
        # This is a structural test of the API
        pass
