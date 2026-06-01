"""pylean4: Python FFI bindings for Lean4 via leo3.

Two layers:
- pylean4.core: Low-level bindings (Runtime, LeanObject, Environment, MetaContext)
- pylean4.ai: AI/RL acceleration (TacticState, ProofEnvironment, BatchVerifier)
"""

__version__ = "0.1.0"

# Re-export core types
from ._core import Runtime, LeanObject, Environment, MetaContext
from ._ai import TacticState, TacticResult, ProofEnvironment, BatchVerifier, Expr

__all__ = [
    # Core
    "Runtime",
    "LeanObject",
    "Environment",
    "MetaContext",
    # AI
    "Expr",
    "TacticState",
    "TacticResult",
    "ProofEnvironment",
    "BatchVerifier",
]

# Auto-register Jupyter formatters if in a notebook
try:
    from ._jupyter import register_jupyter_formatters
    register_jupyter_formatters()
except Exception:
    pass
