# RTP — Agent Onboarding

> **Read this file first.** This is your complete orientation + task list. After reading
> this, read [`CLAUDE.md`](CLAUDE.md) for the deep architecture reference.

---

## 0. Current Goal: Get the Python Research Pipeline Fully Operational

The Rust swarm and Solana treasury are **done** (146 tests, 0 clippy warnings, all audit
findings fixed). The next milestone is getting the **Python yield brain** running end-to-end:

1. Night shift optimization (30K configs, 9-fold WFA, Darwinian evolution)
2. Full-sim validation (FutureBlindSimulator with fees + slippage)
3. Self-correction (evaluator calibration + discrepancy detection)
4. Paper trading (live Binance, ADX regime filter)
5. Autoresearch loop (Karpathy-style self-improvement)

**Hackathon deadline: May 11, 2026**

---

## 1. Environment Setup (DO THIS FIRST)

The system was recently wiped. Nothing is installed except Python 3.13.3, Rust/Cargo,
Solana CLI, Anchor, Node.js, and npm. **Docker is not installed.**

### Step 1: Create Python virtual environment

```bash
cd /home/kt/tabs/resilient-token-protocol
python3 -m venv .venv
source .venv/bin/activate
pip install pandas numpy ccxt pyarrow redis
```

**Required packages**: `pandas`, `numpy`, `ccxt`, `pyarrow`, `redis`
**Optional but useful**: `scipy`, `scikit-learn`

Verify:
```bash
python -c "import pandas, numpy, ccxt, pyarrow, redis; print('all OK')"
```

### Step 2: Verify data exists

```bash
ls data/ohlcv/
# Should show: BTC_USDT_{1h,4h,1d}.parquet, ETH_USDT_{1h,4h,1d}.parquet,
#              SOL_USDT_{1h,4h,1d}.parquet, BNB_USDT_{1h,4h,1d}.parquet
```

Data is **3 days old** (last candle: 2026-04-05 23:00 UTC, 9600 rows per 1h file).
To refresh:
```bash
python scripts/download_ohlcv.py
```

### Step 3: Verify Rust swarm still passes

```bash
cd rtp/swarm && cargo test   # expect 146 tests, 0 failures
```

### Step 4: Verify Solana/Anchor

```bash
cd rtp/programs/rtp-treasury && anchor build   # must compile clean
```

---

## 2. Project Architecture

RTP is a Solana-native, self-funding treasury governed by a modular Rust swarm. Three layers:

```
┌─────────────────────────────────────────────────────────────────┐
│                    ON-CHAIN (Solana / Anchor)                    │
│  Treasury PDA: fees → yield → redistribute → self-hydrate       │
│  Phase evolution: Sustenance → Ecosystem → Humanity Fund        │
├─────────────────────────────────────────────────────────────────┤
│                    SWARM RUNTIME (Rust)                          │
│  Coordinator → message bus → 6 wings (trading, security,        │
│  evolve, knowledge, audit, futureproof)                          │
├─────────────────────────────────────────────────────────────────┤
│                    RESEARCH LAYER (Python)                        │
│  Night Shift: 30K configs → WFA → Darwinian → full-sim validate │
│  Paper Trader: live Binance → state persistence → degradation   │
└─────────────────────────────────────────────────────────────────┘
```

**The Python research layer is the yield brain** — it generates strategy parameters that
the Rust swarm's Trading Wing executes via `bridge.rs`. This is the layer we're focused on.

### Data Flow (Yield Brain)

```
Binance → download_ohlcv.py → data/ohlcv/{SYMBOL}_1h.parquet
                                   ↓
              per_symbol_optimizer (compute_indicators, simulate_trades, _compute_score)
              ┌────────────────┴────────────────┐
              │                                 │
         night_shift (grid search)         paper_trader (live)
         fast sim (~30K combos)            real-time Binance
              │                                 │
              ▼                                 ▼
    ┌─────────────────┐                 data/paper_trading/
    │ validate_       │                   state.json
    │ night_shift.py  │
    │ (full sim bridge)│
    └────────┬────────┘
             │
             ▼
    FutureBlindSimulator (fees + slippage)
             │
             ▼
    data/night_results/YYYY-MM-DD/report.md
```

---

## 3. What's Already Built

### Rust Swarm Runtime — COMPLETE ✅
**Path**: `rtp/swarm/src/`
- 146 tests, 0 failures, 0 clippy warnings
- 6 wings: Trading, Security, Evolve, Knowledge, Audit, Futureproof
- Coordinator with soulguard, router, lifecycle
- `bridge.rs`: typed Python↔Rust interface (`BridgeRequest`/`BridgeResponse`)
- `demo.rs`: end-to-end demo loop (register wings → proposal → audit → execute → yield)
- `config.rs`: AES-256-GCM encrypted configs

### Solana Treasury — COMPLETE ✅
**Path**: `rtp/programs/rtp-treasury/`
- Anchor 1.0, 7 instructions, 15 integration tests
- All 18 security audit findings fixed

### Python Scripts — ALL PRESENT, NEED VENV TO RUN
All scripts are in `scripts/` and import correctly via `sys.path.insert`. They just need
the venv activated. Key scripts:

| File | Purpose |
|------|---------|
| `scripts/night_shift.py` | Main pipeline: grid search → WFA → Darwinian → report → validation |
| `scripts/per_symbol_optimizer.py` | Fast simulator: `compute_indicators()`, `simulate_trades()`, `_compute_score()` |
| `scripts/paper_trader.py` | Live paper trader: polls Binance, ADX filter, per-symbol configs |
| `scripts/validate_night_shift.py` | Bridges fast sim → full sim for candidate validation |
| `scripts/run_backtest_r2.py` | Production `MultiTFStrategy` class + `timeframe_signal()` helper |
| `scripts/evaluator_calibration.py` | Compares fast vs full sim on random configs |
| `scripts/discrepancy_detector.py` | Post-night-shift check, flags fast/full sim divergences |
| `scripts/autoresearch.py` | Karpathy-style self-improvement loop (git commit/revert) |
| `scripts/autoresearch_eval.py` | Evaluation step for autoresearch (outputs JSON metrics) |
| `scripts/download_ohlcv.py` | Downloads OHLCV from Binance (no API key needed) |
| `scripts/night_config.json` | Night shift config (symbols, folds, experiments, thresholds) |
| `backtesting/future_blind_simulator.py` | `FutureBlindSimulator`: 0.1% fees, 10bps slippage, max 20% position |
| `backtesting/fast_simulator.py` | Fast simulator (wrapper around per_symbol_optimizer) |
| `agents/historical_data_collector.py` | `DataWindow` class feeding data to full simulator |

### Night Shift Pipeline Phases

1. **Data** — load cached parquet (Binance geo-blocked on GitHub, data in repo)
2. **WFA Folds** — expanding-window, non-overlapping, 9 folds × 36-day test windows
3. **Production Baseline** — evaluate current config as reference
4. **Coarse Grid** — ~30K parameter combinations per symbol
5. **Fine Refinement** — top 100 per symbol on all folds
6. **Darwinian Evolution** — 5 generations, mutate best candidates
7. **BB Mean Reversion** — separate strategy grid search
8. **Custom Experiments** — configurable param sweeps from `night_config.json`
9. **Regime Analysis** — ADX, volatility percentile, correlations
10. **Morning Report** — markdown + JSON report with top candidates
11. **Auto-Validation** — top 3 through full FutureBlindSimulator
12. **Discrepancy Detection** — compare fast/full sim, flag divergences

### Night Shift Results (last run: 2026-04-08)
- Runtime: 3812s (~63 min) on 3 folds, 3 symbols (BTC/ETH/SOL)
- Top candidate: SOL/USDT with Survivor Score 5.34, OOS Sharpe +5.99, 100% consistency
- 5 actionable HIGH-priority recommendations
- Results in `data/night_results/2026-04-08/`

### Paper Trader State
- Balance: $10,000, 0 completed trades, 0 open positions
- Last activity: 2026-04-06 — all 3 signals FILTERED (ADX below 25 threshold)
- State file: `data/paper_trading/state.json`

---

## 4. Tasks: Get the Python Pipeline Operational

### Task 1: Environment bootstrap + smoke test
**Priority: CRITICAL — everything else depends on this**

1. Create venv and install deps (see Section 1)
2. Refresh OHLCV data (it's 3 days old):
   ```bash
   python scripts/download_ohlcv.py
   ```
3. Smoke test each script imports cleanly:
   ```bash
   python -c "import scripts.per_symbol_optimizer; import scripts.night_shift"
   python -c "import backtesting.future_blind_simulator; import agents.historical_data_collector"
   ```
4. Run a quick night shift (3 folds, 1 symbol, ~10 min):
   ```bash
   python scripts/night_shift.py --skip-fetch --folds 3 --symbols SOL/USDT
   ```
5. Verify report generated at `data/night_results/YYYY-MM-DD/`

### Task 2: Full night shift run
**Priority: HIGH — validates the complete pipeline**

1. Run with production config (4 symbols, 9 folds):
   ```bash
   python scripts/night_shift.py --skip-fetch
   ```
   Expected runtime: ~4-8 hours. Use `nohup` or `tmux`:
   ```bash
   nohup python scripts/night_shift.py --skip-fetch > night_shift.log 2>&1 &
   ```
2. After completion, review:
   - `data/night_results/YYYY-MM-DD/report.md` — top candidates
   - `data/night_results/YYYY-MM-DD/full_sim_validation.json` — full sim results
   - `data/night_results/YYYY-MM-DD/summary.json` — structured summary

### Task 3: Full-sim validation
**Priority: HIGH — confirms fast sim isn't lying**

1. Run validation on the night shift's top candidates:
   ```bash
   python scripts/validate_night_shift.py --production
   ```
2. Run evaluator calibration (fast vs full sim agreement):
   ```bash
   python scripts/evaluator_calibration.py --samples 20
   ```
   Target: >80% sign agreement (PnL direction).
3. Run discrepancy detector:
   ```bash
   python scripts/discrepancy_detector.py
   ```

### Task 4: Paper trading
**Priority: HIGH — live market validation**

1. Start paper trader (runs indefinitely, polls Binance):
   ```bash
   PYTHONUNBUFFERED=1 python scripts/paper_trader.py
   ```
2. Paper trader needs **live Binance data** — it uses `ccxt` to fetch OHLCV.
   No API key needed for public endpoints.
3. It uses ADX regime filter (threshold=25): only trades when ADX > 25 (trending).
4. State persisted in `data/paper_trading/state.json`.
5. Run in background with `nohup` or `tmux`.

**Important**: The paper trader currently uses production baseline configs. After the night
shift identifies better candidates (Task 2), update the paper trader to use them. The
top SOL candidate from 2026-04-08 (Survivor 5.34) is a strong candidate to try.

### Task 5: Autoresearch loop
**Priority: MEDIUM — self-improvement cycle**

The autoresearch script iteratively improves strategy parameters:
```bash
python scripts/autoresearch.py          # run the loop
python scripts/autoresearch_eval.py     # evaluate current configs (JSON output)
```

This is a Karpathy-style self-improvement loop: identify worst symbol, mutate params,
run WFA, keep if better or revert. Designed to run overnight.

### Task 6: (Optional) PyInstaller binary + bridge integration
**Priority: LOW — nice to have for demo**

A `night_shift.spec` PyInstaller spec exists at the repo root. A `--bridge-mode`
argument was already added to `night_shift.py`. To build the binary:

```bash
pip install pyinstaller
pyinstaller night_shift.spec
# Output: dist/night_shift.bin
```

The Rust swarm's `bridge.rs` calls this binary via subprocess. Test:
```bash
echo '{"symbol":"SOL/USDT","config":{}}' | ./night_shift.bin --bridge-mode
```

---

## 5. Critical Calibration Invariants

The fast simulator (`per_symbol_optimizer`) MUST match the full simulator. Three invariants
discovered the hard way — **do not change these without running `evaluator_calibration.py`**:

1. **ATR formula**: `std(returns, 20h) × price` — NOT True Range
2. **MR entry condition**: `rsi < 35 and daily_trend == bullish` — NOT `bull_count >= min_alignment`
3. **Sharpe annualization**: `sqrt(n_trades / total_hours × 8760)` — NOT `sqrt(24 × 365)`

---

## 6. Key Files Reference

### Python (Yield Brain) — THIS IS THE FOCUS

| File | Purpose |
|------|---------|
| `scripts/night_shift.py` | Main pipeline: grid search → WFA → Darwinian → report |
| `scripts/per_symbol_optimizer.py` | Fast simulator: compute_indicators, simulate_trades, _compute_score |
| `scripts/paper_trader.py` | Live paper trader: Binance streaming, ADX filter, state persistence |
| `scripts/validate_night_shift.py` | Full sim validation of night shift candidates |
| `scripts/evaluator_calibration.py` | Fast vs full sim agreement check |
| `scripts/discrepancy_detector.py` | Post-night-shift divergence detection |
| `scripts/autoresearch.py` | Karpathy-style self-improvement loop |
| `scripts/autoresearch_eval.py` | Evaluation step for autoresearch |
| `scripts/download_ohlcv.py` | Binance OHLCV downloader |
| `scripts/night_config.json` | Night shift configuration |
| `backtesting/future_blind_simulator.py` | Ground truth simulator (fees + slippage) |
| `agents/historical_data_collector.py` | DataWindow class for full simulator |

### Rust (Swarm Runtime) — DONE

| File | Purpose |
|------|---------|
| `rtp/swarm/src/lib.rs` | Module declarations + re-exports |
| `rtp/swarm/src/types.rs` | Message, Payload, WingId, Priority |
| `rtp/swarm/src/coordinator/mod.rs` | Coordinator (soulguard → router → lifecycle) |
| `rtp/swarm/src/bridge.rs` | Python↔Rust typed interface |
| `rtp/swarm/src/demo.rs` | End-to-end demo loop |
| `rtp/swarm/src/config.rs` | AES-256-GCM encrypted configs |

### Solana (Treasury) — DONE

| File | Purpose |
|------|---------|
| `rtp/programs/rtp-treasury/programs/rtp-treasury/src/lib.rs` | Treasury program |
| `rtp/programs/rtp-treasury/tests/treasury.ts` | 15 integration tests |

### Governance

| File | Purpose |
|------|---------|
| [`CLAUDE.md`](CLAUDE.md) | Architecture, commands, design decisions, invariant list |
| [`BUILD_PLAN_v3.md`](BUILD_PLAN_v3.md) | Post-audit remediation, weekly schedule |
| `soulcontract.md` | Constitutional constraints (**DO NOT MODIFY**) |
| `CODEREVIEW.md` | Code review instructions for AI agents |

---

## 7. Docker (NOT installed — may be needed)

Docker is not currently installed. If you need it for CI or isolated builds:
```bash
sudo apt update && sudo apt install -y docker.io docker-compose-v2
sudo usermod -aG docker $USER
# Log out and back in for group change to take effect
```

For PyInstaller builds on a clean environment, Docker is useful but not required —
the spec file works natively on Ubuntu 25.04.

---

## 8. .gitignore Rules (IMPORTANT)

The `.gitignore` header takes precedence: *"Source code (scripts, backtesting, agents,
strategies) is committed — collaborator needs it to build and run locally. Only data,
artifacts, and secrets are excluded."*

**Tracked**: `scripts/`, `backtesting/`, `agents/`, `rtp/`, `.github/`, governance docs
**Gitignored**: `data/`, `configs/*.json`, `night_shift.bin`, `target/`, `.env`, `.anchor/`

---

## 9. Verify After Every Change

```bash
# Python — smoke test imports
python -c "from scripts.per_symbol_optimizer import compute_indicators, simulate_trades; print('OK')"

# Rust swarm — must stay 146/146
cd rtp/swarm && cargo test

# Anchor treasury — must compile
cd rtp/programs/rtp-treasury && anchor build
```

---

## 10. Rules

- **Read the file before changing it.**
- **Don't modify `soulcontract.md`.**
- **Don't use Anchor 0.31** — this is Anchor 1.0.0 with Solana 3.x (Agave 3.1.12).
- **Don't load skills/plugins** — most are stubs or mocks (see `~/tabs/SKILL_AUDIT_2026-04-07.md`).
- **Don't break the Rust swarm** — 146 tests must stay green.
- **Wings NEVER modify each other directly** — all cross-wing communication via Coordinator.
- **Every message passes through soulguard** — the Coordinator enforces this.
- **Wings must never silently drop messages** — unhandled payloads return `Payload::Error`.
- **Token-2022 gotchas** (if you touch treasury tests):
  - `transferCheckedWithFee` stores withheld fees in the **DESTINATION** token account
  - Use `sendAndConfirmTransaction` directly — NOT the `@solana/spl-token` wrapper
  - Add 200-300ms sleeps after `.rpc()` calls that perform CPI before reading state
  - CPI from Anchor to Token-2022 requires `mut` on accounts the CPI marks as writable
