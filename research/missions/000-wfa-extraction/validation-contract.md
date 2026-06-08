# Validation Contract — Mission 000

## Statistical Invariants (must be preserved exactly)
- 9-fold expanding-window walk-forward validation
- Each fold uses strictly earlier data for training/optimization and later data for validation
- Consistency metric = percentage of folds that meet a minimum performance threshold (original definition must be matched)
- Darwinian survivor selection: only strategies/configs that pass all gates across folds survive
- Explicit result lineage (timestamp, source commit, parameters, scores, fold outcomes)

## Domain Separation Rules (non-negotiable)
- Zero trading-specific concepts: no signals (RSI, ATR, Bollinger, etc.), no fees, no slippage, no leverage, no Calmar ratio, no PnL simulation, no asset symbols.
- The module must be usable for arbitrary grid search + robust validation problems (tokenomics parameter spaces, security attack surface search, etc.).
- All functions must have clear type contracts and docstrings describing statistical assumptions.

## Output Requirements
- New module must be importable and testable in isolation.
- Migration note must explain how original `night_shift.py` orchestration would call the new module.
- Worker transcripts and validation logs must be retained for audit.

## Failure Conditions (Hermes must reject)
- Any leakage of trading/perps/Flash Trade logic into the new module.
- Weakening or redefinition of the 9-fold expanding window semantics.
- Loss of consistency or survivor selection logic.
- Missing lineage / provenance.

**Validator**: Hermes (final) + human architectural review