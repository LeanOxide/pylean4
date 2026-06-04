"""pylean4: Python FFI bindings for Lean4 via leo3.

Two layers:
- pylean4.core: Low-level bindings (Runtime, LeanObject, Environment, MetaContext)
- pylean4.ai: AI/RL acceleration (TacticState, ProofEnvironment, BatchVerifier)
"""

__version__ = "0.1.0"

from pathlib import Path

from ._runtime import preload_lean_runtime

if preload_lean_runtime(Path(__file__).parent) is None:
    raise ImportError(
        "Could not find a compatible external Lean runtime for pylean4. "
        "Install Lean for runtime use, add lean or lemma to PATH, or set "
        "LEAN_HOME/LEAN_LIB_DIR to a Lean installation that exports lean_mark_mt."
    )

# Re-export native types. The _ai extension registers both the core and AI
# layers so Lean's process-global runtime is loaded only once.
from ._ai import (
    Runtime,
    LeanObject,
    Environment,
    MetaContext,
    TacticState,
    TacticResult,
    ProofEnvironment,
    BatchVerifier,
    Expr,
)

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
