"""Jupyter display integration for pylean4.

Provides rich rendering of proof states and expressions in Jupyter notebooks.
Automatically registers display formatters when imported.
"""

# CSS for proof state rendering
PROOF_STATE_CSS = """
<style>
.lean4-proof-state {
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    border: 1px solid #e0e0e0;
    border-radius: 6px;
    padding: 12px;
    margin: 8px 0;
    background: #fafafa;
    max-width: 600px;
}
.lean4-proof-state .goal-separator {
    border: none;
    border-top: 1px dashed #bdbdbd;
    margin: 8px 0;
}
.lean4-proof-state .goal-target {
    color: #1565c0;
    font-weight: 500;
}
.lean4-proof-state .goal-hyp {
    color: #424242;
    padding-left: 8px;
}
.lean4-proof-state .goal-label {
    color: #757575;
    font-size: 0.85em;
}
.lean4-proof-state .solved {
    color: #2e7d32;
    font-weight: bold;
}
.lean4-expr {
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    color: #1a237e;
}
</style>
"""

_css_injected = False


def _ensure_css():
    """Inject CSS once per notebook session."""
    global _css_injected
    if not _css_injected:
        try:
            from IPython.display import display, HTML
            display(HTML(PROOF_STATE_CSS))
            _css_injected = True
        except ImportError:
            pass


def format_proof_state_html(goals_pp: list[str]) -> str:
    """Format a list of goal strings as HTML."""
    _ensure_css()
    if not goals_pp:
        return '<div class="lean4-proof-state"><span class="solved">No goals. ∎</span></div>'

    parts = ['<div class="lean4-proof-state">']
    for i, goal in enumerate(goals_pp):
        if i > 0:
            parts.append('<hr class="goal-separator"/>')
        parts.append(f'<div class="goal-label">goal {i + 1}</div>')
        parts.append(f'<div class="goal-target">{_html_escape(goal)}</div>')
    parts.append('</div>')
    return '\n'.join(parts)


def format_expr_html(expr_str: str) -> str:
    """Format an expression string as HTML."""
    return f'<span class="lean4-expr">{_html_escape(expr_str)}</span>'


def format_expr_latex(expr_str: str) -> str:
    """Convert expression string to LaTeX."""
    s = expr_str
    replacements = {
        '∀': r'\forall\,',
        '∃': r'\exists\,',
        '→': r'\to ',
        '←': r'\leftarrow ',
        '↔': r'\leftrightarrow ',
        '¬': r'\neg ',
        '∧': r'\land ',
        '∨': r'\lor ',
        '⊢': r'\vdash ',
        '⊥': r'\bot ',
        '⊤': r'\top ',
        'λ': r'\lambda\,',
        'ℕ': r'\mathbb{N}',
        'ℤ': r'\mathbb{Z}',
        'ℝ': r'\mathbb{R}',
        'Prop': r'\mathrm{Prop}',
        'Type': r'\mathrm{Type}',
        'Nat': r'\mathbb{N}',
    }
    for k, v in replacements.items():
        s = s.replace(k, v)
    return f'${s}$'


def _html_escape(s: str) -> str:
    return s.replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;')


def register_jupyter_formatters():
    """Register rich display formatters for Jupyter.

    Call this once to enable automatic rich rendering of pylean4 objects
    in Jupyter notebooks.
    """
    try:
        from IPython import get_ipython
        ip = get_ipython()
        if ip is None:
            return

        formatter = ip.display_formatter.formatters['text/html']

        # Import pylean4 types
        from ._ai import TacticState, TacticResult, Expr

        # TacticState → HTML proof state box
        def _tactic_state_html(state):
            return format_proof_state_html(state.goals_pp)

        formatter.for_type(TacticState, _tactic_state_html)

        # TacticResult → colored success/failure
        def _tactic_result_html(result):
            if result.success:
                ns = result.next_state
                if ns and ns.is_solved:
                    return '<div style="color:#2e7d32;font-weight:bold">✓ Proved!</div>'
                return format_proof_state_html(ns.goals_pp if ns else [])
            else:
                return f'<div style="color:#c62828">✗ {_html_escape(result.error or "failed")}</div>'

        formatter.for_type(TacticResult, _tactic_result_html)

        # Expr → LaTeX
        latex_formatter = ip.display_formatter.formatters['text/latex']

        def _expr_latex(expr):
            return format_expr_latex(str(expr))

        latex_formatter.for_type(Expr, _expr_latex)

    except (ImportError, AttributeError):
        pass  # Not in Jupyter, skip
