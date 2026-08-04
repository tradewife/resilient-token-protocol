# Mission: S14 Marubozu Retracement — SOL/USDT 1m Overnight Optimization

## Objective
Run a staged parameter optimization of S14_Marubozu_Retracement on SOL/USDT 1-minute data overnight. Deliverable on wake: a ranked results table + one winning config saved via save_winning_config.py, or a clear "no edge found" verdict with diagnostics.

## Data
- Symbol: SOL/USDT
- Timeframe: 1m
- Source: check research/data/ and data/ for existing SOL 1m OHLCV; if absent, fetch from Binance klines (spot, SOLUSDT, 1m). Minimum 90 days; prefer 6–12 months.
- Required columns: open, high, low, close, volume (+ timestamp index)

## CRITICAL: Plugin instantiation
MarubozuRetracementPlugin is STATEFUL — it carries `self._pending` between check_entry calls. You MUST instantiate a fresh `MarubozuRetracementPlugin()` for every single (params combo × data fold) backtest run. Never reuse an instance across combos. If night_shift.py reuses one instance per strategy across the combo loop, patch it to re-instantiate inside the loop before running. Do not parallelize across combos with shared instances; if using ProcessPoolExecutor, each worker must construct its own plugin inside the worker function.

## Phase 0 — Smoke test (5 min)
- Run default_params() on a 7-day slice.
- Sanity checks: trades > 0, no exceptions, entries actually occur after retracement (not on trigger bar), exits attributed to expected reasons.
- If zero trades: log candle-level diagnostics (how many trigger candles found, how many retracements filled) before proceeding.

## Phase 1 — Coarse grid (~1–2 hrs)
Sweep the strategy-defining params only; hold execution params at defaults:
- retracement_pct: [0.25, 0.38, 0.50, 0.62, 0.75]
- wick_tolerance_pct: [0.05, 0.10, 0.15, 0.20]
- body_atr_multiplier: [1.0, 1.5, 2.0, 2.5]
- expiry_bars: [5, 10, 15, 20]
- trend_fast_period / trend_slow_period: [(9, 20), (9, 50), (20, 50)]
- direction_filter: ["both", "long", "short"]
- volume_multiplier: [0.0, 1.5]
Fixed: stop_loss_atr=1.5, take_profit_atr=3.0, max_hold_hours=4, time_decay_hours=2, trailing_stop_atr=0.0, leverage=1.0
Total: 5×4×4×4×3×3×2 = 5,760 combos.
Rank by Sharpe (primary), then profit factor, then trade count. Discard combos with < 30 trades (statistically meaningless on 1m data at this sample size).

## Phase 2 — Fine grid on top-20 (~1–2 hrs)
Take top 20 coarse configs. Sweep execution params around them:
- stop_loss_atr: [1.0, 1.5, 2.0, 2.5]
- take_profit_atr: [2.0, 3.0, 4.0, 5.0]
- trailing_stop_atr: [0.0, 0.5, 1.0]
- max_hold_hours: [1, 2, 4, 8]
Keep leverage=1.0 for ranking (leverage amplifies but doesn't change edge).

## Phase 3 — Walk-forward validation (~2–4 hrs)
Top 5 configs from Phase 2 → walk-forward analysis per research/optimization/wfa_fixed_params.py conventions:
- 3:1 train/test ratio, anchored walk-forward, minimum 4 folds.
- A config PASSES only if: positive OOS expectancy on ≥ 75% of folds, OOS Sharpe degradation < 40% vs IS, no fold with catastrophic drawdown (> 30% at 1x).
- If all 5 fail: report the best in-sample config with explicit "FAILED WFA — curve-fit risk high" label and the per-fold diagnostics. Do NOT save it as winning config.

## Success criteria (for save_winning_config.py)
- Sharpe ≥ 1.0 OOS
- Profit factor ≥ 1.3 OOS
- ≥ 30 trades per OOS fold (else widen data window, don't lower the bar)
- Max drawdown ≤ 20% at leverage 1.0

## Outputs (write to research/data/results/s14_overnight/)
- phase1_coarse_results.csv — all combos + metrics
- phase2_fine_results.csv
- phase3_wfa_report.md — per-fold IS/OOS table for top 5
- verdict.md — one-page summary: winning config (or no-edge verdict), key param sensitivities (what moved Sharpe most), recommended next experiment
- Call save_winning_config.py only if success criteria met.

## Failure handling
- Log every exception per combo to errors.log; never abort the whole run for a single combo failure.
- If the run is killed mid-way, resume from the last completed phase (checkpoint results CSVs after each phase).
- If compute budget runs out before Phase 3: save Phase 2 results and mark verdict "INCOMPLETE — WFA pending".

## Explicit non-goals
- Do not modify S14 plugin logic mid-run. If you find a bug, log it in research/dead_ends.md and continue with the spec'd version.
- Do not tune leverage during ranking. Report top config at leverage 1.0, 3.0, 5.0 for reference only.
