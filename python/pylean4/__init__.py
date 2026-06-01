"""pylean4: Python FFI bindings for Lean4 via leo3.

Two layers:
- pylean4.core: Low-level bindings (Runtime, LeanObject, Environment, MetaContext)
- pylean4.ai: AI/RL acceleration (TacticState, ProofEnvironment, BatchVerifier)
"""

__version__ = "0.1.0"

# Re-export core types
from ._core import Runtime, LeanObject, Environment, MetaContext
from ._ai import TacticState, TacticResult, ProofEnvironment, BatchVerifier

__all__ = [
    # Core
    "Runtime",
    "LeanObject",
    "Environment",
    "MetaContext",
    # AI
    "TacticState",
    "TacticResult",
    "ProofEnvironment",
    "BatchVerifier",
]
