# Trading Research Handover

This document is for the agent handling the Python trading research side of RTP.

## Current State

The trading research code has been restructured under `research/`.

Source layout:

- `research/data/`
  - `download_ohlcv.py`
  - `regime_filter.py`
- `research/simulation/`
  - `data_window.py`
  - `future_blind_simulator.py`
  - `run_backtest_r2.py`
- `research/optimization/`
  - `per_symbol_optimizer.py`
  - `wfa_fixed_params.py`
  - `evaluator_calibration.py`
  - `save_winning_config.py`
- `research/validation/`
  - `autoresearch_eval.py`
  - `discrepancy_detector.py`
  - `validate_night_shift.py`
  - `validate_optimized.py`
- `research/live/`
  - `paper_trader.py`
- `research/orchestration/`
  - `night_shift.py`
  - `night_config.json`
  - `autoresearch.py`
  - `night_shift.spec`

Non-source runtime data remains at repo root under `data/`.
This is intentional. It contains parquet files, reports, calibration outputs, paper trading state, and other generated artifacts. It is not part of the Python package layout.

The old locations were removed:

- `scripts/` deleted
- `backtesting/` deleted
- `agents/historical_data_collector.py` removed

`agents/` was previously emptied because `historical_data_collector.py` was not agent code. If real autonomous agent code is added later, it can live there again.

## Canonical Entry Points

Use module execution from repo root with the venv active.

Examples:

```bash
. .venv/bin/activate
python -m research.orchestration.night_shift --skip-fetch
python -m research.live.paper_trader
python -m research.validation.validate_night_shift --production
python -m research.optimization.evaluator_calibration --samples 20
python -m research.validation.discrepancy_detector
python -m research.data.download_ohlcv
```

The GitHub Actions workflow has already been updated to use:

```bash
python3 -m research.orchestration.night_shift --config research/orchestration/night_config.json
```

## What Was Verified

These work in the project venv:

- `python -m research.orchestration.night_shift --help`
- `python -m research.validation.validate_night_shift --help`
- `python -m research.optimization.evaluator_calibration --help`

Core imports also load in the venv for:

- `research.orchestration.night_shift`
- `research.orchestration.autoresearch`
- `research.live.paper_trader`
- `research.validation.validate_night_shift`
- `research.optimization.evaluator_calibration`

## Known Issue

`research/optimization/save_winning_config.py` still depends on a missing module:

```python
from knowledge_base_schema import KnowledgeBase, StrategyGenome, StrategyPerformance
```

`knowledge_base_schema` is not present in this repository. This was already broken before the restructure.

Implication:

- do not treat `save_winning_config.py` as a currently working path
- if needed, either restore that dependency or stub/replace the knowledge base integration deliberately

## Important Path Assumptions

Most research modules still read and write to root-level `data/` via relative paths such as:

- `data/ohlcv/`
- `data/night_results/`
- `data/calibration/`
- `data/discrepancies/`
- `data/paper_trading/`

Do not move `data/` into `research/` unless you are intentionally redesigning runtime storage and updating every path.

## Project-Level Truths

The broader repo is still in active build mode.

Important context from docs and current direction:

- repo is private for now
- black-boxing of the research layer is deferred
- collaborators need readable Python source
- `SOULCONTRACT.md` remains the governance source for immutable constraints
- Rust swarm and Solana treasury are still in development; do not assume the README/plan text is always ahead of the code

## Documentation Alignment Already Done

These were updated to match the new `research/` layout and the deferred black-boxing decision:

- `CLAUDE.md`
- `README.md`
- `BUILD_PLAN_v3.md`
- `.github/workflows/night_shift.yml`

Several research file docstrings/help snippets were also updated to stop pointing at deleted `scripts/` paths.

## Practical Guidance For The Next Agent

If you are changing research code:

1. Run from the venv, not system `python3`.
2. Prefer `python -m research...` entry points.
3. Keep source under `research/`.
4. Treat `data/` as runtime artifacts, not package code.
5. Do not reintroduce data utilities into `agents/`.
6. If touching fast sim logic, re-check calibration assumptions carefully.

## Fast Sim / Full Sim Invariants

These are important enough to restate:

1. ATR formula: `std(returns, 20h) * price`
2. Mean-reversion entry logic must match the production assumptions exactly
3. Sharpe annualization must stay aligned between fast and full sim paths

If modifying:

- `research/optimization/per_symbol_optimizer.py`
- `research/simulation/future_blind_simulator.py`
- `research/simulation/run_backtest_r2.py`

then rerun calibration/validation before treating results as trustworthy.

## Suggested Next Checks

If you are continuing work on trade research, the highest-value follow-ups are:

1. Decide whether `save_winning_config.py` should be repaired or retired.
2. Audit remaining hardcoded report dates and static candidate blobs in validation scripts.
3. Add a lightweight smoke-test path for the `research/` entry points.
4. Keep docs honest when roadmap status changes.

## Short Summary

The restructure is in place and usable.
The main nightly research entry point is now `research.orchestration.night_shift`.
The main unresolved defect is the missing `knowledge_base_schema` dependency in `save_winning_config.py`.
