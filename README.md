# pylean4

Python FFI bindings for the [Lean4](https://lean-lang.org/) theorem prover, built on [leo3](https://github.com/AndPuQing/leo3).

Unlike process-based tools (LeanDojo, LeanInteract), pylean4 links directly to Lean4's C runtime via FFI, enabling **>10,000 tactic verifications per second** — critical for RL-based theorem proving.

## Architecture

```
┌─────────────────────────────────┐
│     Python Application          │
│  (RL training, proof search)    │
└────────────────┬────────────────┘
                 │
    ┌────────────┼────────────┐
    │            │            │
┌───▼────┐  ┌───▼────┐  ┌───▼──────────┐
│  Core  │  │   AI   │  │ BatchVerifier │
│ Layer  │  │ Layer  │  │ (parallel)    │
└───┬────┘  └───┬────┘  └───┬──────────┘
    │            │            │
    └────────────┼────────────┘
                 │  PyO3
         ┌───────▼────────┐
         │  leo3 (Rust)   │
         │  Safe bindings │
         └───────┬────────┘
                 │  FFI
         ┌───────▼────────┐
         │ libleanshared  │
         │  (Lean4 C RT)  │
         └────────────────┘
```

## Quick Start

```python
import pylean4

# Initialize the Lean4 runtime
rt = pylean4.Runtime()

# AI/RL usage
env = pylean4.ProofEnvironment("Mathlib.Tactic.Ring", "one_plus_one")
state = env.reset()

# Apply tactics
result = state.apply("ring")
if result.success:
    print("Proved!")

# Batch verification (parallel, GIL-free)
verifier = pylean4.BatchVerifier(num_workers=8)
results = verifier.verify_batch(states, tactics)
```

## Performance

| Operation | pylean4 (FFI) | LeanDojo (subprocess) | Speedup |
|-----------|---------------|----------------------|---------|
| Single tactic | ~50μs | ~50ms | ~1000x |
| Batch (1000) | ~5ms | ~50s | ~10000x |

## Installation

```bash
pip install pylean4
```

Requires Lean4 toolchain installed via [elan](https://github.com/leanprover/elan).

## Development

```bash
# Install Rust + maturin
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
pip install maturin

# Build and install in development mode
maturin develop --release

# Run tests
pytest tests/
```

## License

MIT OR Apache-2.0
