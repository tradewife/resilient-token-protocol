# RTP — Agent Onboarding

Read this file first. It is the current starting point for the next build-phase agent.

After this, read:

1. [`CLAUDE.md`](CLAUDE.md)
2. [`BUILD_PLAN_v3.md`](BUILD_PLAN_v3.md)
3. [`soulcontract.md`](soulcontract.md)
4. [`handover.md`](handover.md)
5. [`handover_checklist.md`](handover_checklist.md)

## 0. Where We Actually Are

Do not trust older onboarding assumptions.

Current repo state:

- Python trading research has been restructured under `research/`
- Root `scripts/` and `backtesting/` are gone
- `agents/historical_data_collector.py` was removed because it was not agent code
- Root `data/` remains intentionally outside `research/` as runtime/output storage
- Repo is private
- Black-boxing is deferred while collaborators still need readable source

Status against [`BUILD_PLAN_v3.md`](BUILD_PLAN_v3.md):

- Treasury audit remediation: done
- Core swarm runtime and coordinator path: built
- Bridge + Trading/Security/Knowledge/Futureproof wings: present in code
- Research layer: reorganized and module entry points work in the venv
- Immediate next phase: continue Week 4/5 work as "full-loop integration + demo hardening", not black-boxing

The plan language about black-boxing is now stale as an execution target. Treat the current next phase as:

1. end-to-end integration
2. devnet demo path hardening
3. CI/test hardening
4. documentation accuracy

## 1. Current Goal For The Next Agent

Continue the next build phase from the real state of the repo.

That means:

- do not spend time re-restructuring the research code
- do not reintroduce deleted `scripts/` paths
- do not prioritize PyInstaller black-boxing right now
- prioritize integration and demo readiness

The next agent should treat this as the active milestone:

### Active Milestone

Make the Python research layer, Rust bridge/runtime, and Solana treasury path demonstrably coherent enough for the next demo/hardening phase.

## 2. Source Layout

### Research Layer

All trading research source lives here:

```text
research/
├── data/
├── simulation/
├── optimization/
├── validation/
├── live/
└── orchestration/
```

Important files:

- `research/orchestration/night_shift.py`
- `research/orchestration/night_config.json`
- `research/live/paper_trader.py`
- `research/optimization/per_symbol_optimizer.py`
- `research/optimization/evaluator_calibration.py`
- `research/validation/validate_night_shift.py`
- `research/validation/discrepancy_detector.py`
- `research/simulation/future_blind_simulator.py`
- `research/simulation/run_backtest_r2.py`

### Runtime Data

Runtime artifacts stay at root:

- `data/ohlcv/`
- `data/night_results/`
- `data/calibration/`
- `data/discrepancies/`
- `data/paper_trading/`

Do not move `data/` into `research/` unless you are intentionally redesigning storage paths everywhere.

### Swarm Runtime

Rust swarm source:

- `rtp/swarm/src/bridge.rs`
- `rtp/swarm/src/coordinator/`
- `rtp/swarm/src/wings/`

### Solana Program

Anchor treasury program:

- `rtp/programs/rtp-treasury/`

## 3. What Has Already Been Verified

In the project venv, these module entry points work:

```bash
python -m research.orchestration.night_shift --help
python -m research.validation.validate_night_shift --help
python -m research.optimization.evaluator_calibration --help
```

The GitHub Actions workflow for night shift has already been updated to use the new `research/` path.

Docs already aligned recently:

- `CLAUDE.md`
- `README.md`
- `BUILD_PLAN_v3.md`
- `.github/workflows/night_shift.yml`

## 4. Known Defects / Constraints

### Pre-existing broken module

`research/optimization/save_winning_config.py` — **deprecated but importable**.

The `knowledge_base_schema` dependency does not exist in this repository. The import is now guarded with a try/except. The file imports cleanly and retains historical config data (`WINNING_CONFIGS`, `SIGNAL_WEIGHTS`) as reference. The `save_configs()` function exits gracefully with a deprecation notice.

### Black-boxing is deferred

Do not spend the next phase on:

- obfuscating research code
- encrypted strategy packaging
- packaging-only hardening for secrecy

The repo is private and collaboration-readable source is currently more important.

### `agents/` is not the research folder

If future autonomous agent code is added, it can live in `agents/`.
Do not move trading research helpers back there.

## 5. Environment Setup

From repo root:

```bash
python3 -m venv .venv
. .venv/bin/activate
pip install pandas numpy ccxt pyarrow redis
```

Useful verification:

```bash
python -c "import pandas, numpy, ccxt, pyarrow, redis; print('OK')"
```

## 6. Canonical Commands

Always prefer module execution from repo root:

```bash
. .venv/bin/activate

python -m research.orchestration.night_shift --skip-fetch
python -m research.orchestration.night_shift --skip-fetch --folds 3 --symbols SOL/USDT

python -m research.validation.validate_night_shift --production
python -m research.optimization.evaluator_calibration --samples 20
python -m research.validation.discrepancy_detector

PYTHONUNBUFFERED=1 python -m research.live.paper_trader
python -m research.data.download_ohlcv
```

Rust / Anchor:

```bash
cd rtp/swarm && cargo test
cd rtp/programs/rtp-treasury && anchor build
```

## 7. What The Next Agent Should Focus On

These are the highest-value next-phase tasks.

### Priority 1: End-to-End Path Audit

Confirm the actual demo path works or document the remaining gap:

1. Python research produces usable output
2. Rust bridge can consume/route the expected shape
3. Trading/Audit/Coordinator flow is coherent
4. Treasury devnet path is still aligned with the current swarm/runtime assumptions

This is the most important next task because the codebase is beyond pure scaffolding now.

### Priority 2: Demo-Hardening Gaps

Audit and tighten the gap between "implemented modules exist" and "demoable full loop":

- bridge request/response assumptions
- demo wiring in `rtp/swarm/src/demo.rs`
- places where code claims execution but still uses placeholder behavior
- docs that overstate what is actually live

### Priority 3: CI / Smoke-Test Hardening

Add or improve lightweight checks for:

- `python -m research.orchestration.night_shift --help`
- importability of the main research modules
- `cargo test`
- `anchor build`

The immediate win is catching path drift and integration regressions early.

### Priority 4: Research Reliability Cleanup

Good follow-up work on the research side:

1. decide whether `save_winning_config.py` should be repaired or retired
2. audit validation scripts for hardcoded dates and static candidate blobs
3. add a small smoke-test path for the main research entry points

## 8. Things To Avoid

Do not:

- recreate `scripts/` or `backtesting/`
- move runtime data into `research/`
- assume old `ONBOARDING.md` instructions about `scripts/...` still apply
- assume black-boxing is the active milestone
- assume all roadmap claims are ahead of the code without checking the tree

## 9. Fast Sim / Full Sim Caution

If you touch any of these:

- `research/optimization/per_symbol_optimizer.py`
- `research/simulation/future_blind_simulator.py`
- `research/simulation/run_backtest_r2.py`

then re-check calibration/validation before trusting outputs.

The fast-sim and full-sim relationship is critical to the credibility of the research layer.

## 10. Short Summary For The Next Agent

You are not starting from scratch.

The repo has:

- a restructured research layer under `research/`
- a built Rust swarm path with bridge and wing modules present
- a treasury program that has already gone through audit remediation

Your job is to continue the next phase:

- integration
- demo hardening
- CI/smoke-test hardening
- truthful documentation

Start by validating the real end-to-end path instead of assuming the plan is already reality.
