# Trading Research Agent Checklist

## Read First

- Full context: `handover.md`
- Governance: `soulcontract.md`
- Repo guidance: `CLAUDE.md`

## Source Of Truth For Research Code

- All trading research source now lives under `research/`
- Do not recreate `scripts/` or `backtesting/`
- Do not put data utilities back into `agents/`

## Main Entry Points

Run from repo root with the venv active:

```bash
. .venv/bin/activate
python -m research.orchestration.night_shift --skip-fetch
python -m research.live.paper_trader
python -m research.validation.validate_night_shift --production
python -m research.optimization.evaluator_calibration --samples 20
python -m research.validation.discrepancy_detector
```

## Runtime Data

- Runtime data stays in root `data/`
- Do not move `data/` into `research/` unless you are intentionally redesigning paths
- Key folders:
  - `data/ohlcv/`
  - `data/night_results/`
  - `data/calibration/`
  - `data/discrepancies/`
  - `data/paper_trading/`

## Verified Working

- `python -m research.orchestration.night_shift --help`
- `python -m research.validation.validate_night_shift --help`
- `python -m research.optimization.evaluator_calibration --help`

## Known Broken Item

- `research/optimization/save_winning_config.py`
- Reason: missing `knowledge_base_schema` dependency
- Treat as pre-existing broken code, not a regression from the restructure

## Current Project Constraints

- Repo is private
- Black-boxing is deferred
- Research source should stay readable for collaborators
- Rust swarm and Solana treasury are still in progress

## If You Change Research Logic

Be careful with:

- `research/optimization/per_symbol_optimizer.py`
- `research/simulation/future_blind_simulator.py`
- `research/simulation/run_backtest_r2.py`

If touched:

- re-check fast sim vs full sim alignment
- rerun validation/calibration before trusting results

## Fast Sim / Full Sim Alignment

Preserve:

1. ATR formula assumptions
2. entry-condition parity
3. Sharpe annualization parity

## Do Not Assume

- that old docs referring to `scripts/` are still correct
- that roadmap docs are always ahead of the code
- that all “wings” are production-complete just because the plan says so

## Good Next Tasks

1. Decide whether to repair or retire `save_winning_config.py`
2. Audit validation scripts for hardcoded dates and static candidate blobs
3. Add smoke tests for `research/` entry points
4. Keep docs aligned when architecture or status changes
