# Dead Ends — Failure Memory Log

> This file is the institutional memory of what has been tried and failed. Any new agent or human MUST check this file before proposing a strategy or parameter range. Repeating dead ends wastes compute and time.
>
> **Rule:** Every strategy or parameter set that fails validation gets logged here with the reason. Only remove entries with explicit human approval.

---

## Retirement Criteria (Automated)

A strategy is moved here when `DecayMonitor.get_status()` returns `StrategyStatus.RETIRED`.

Retirement is triggered by either:
- **Hard stop**: any single threshold breach (see `RetirementGate` in `promotion_criteria.py`)
  - 24h drawdown ≥ 10% of allocated capital
  - 5 consecutive losses with no mean-reversion
  - 30-day rolling Sharpe drops below 0.5
- **Soft decay**: 3 cumulative strikes from the soft signal set
  - 30-day Sharpe drops below 50% of promotion Sharpe
  - Win rate drops below 38% over a 50-trade sample
  - Active regime ≠ strategy's regime_fit for > 5 consecutive days
  - Funding rate (carry strategies) average 8h rate < 0.01%
  - Rolling 30-day correlation to portfolio benchmark > 0.6

Each entry must include: strategy_id, retirement_date, trigger_type (hard/soft), specific signal that caused retirement, and final rolling Sharpe at time of retirement.

---

### BTC/USDT Wide TP + Wide SL Overfitting
- **Date logged**: 2026-04-12
- **Hypothesis**: BTC with take_profit_atr=6.0, stop_loss_atr=3.0, max_hold_hours=36 would capture large Bitcoin trend moves with wide stops
- **Test result**: OOS Sharpe +1.35 but overfitting_score=0.57 > threshold 0.5. IS-OOS gap too large — model memorized training data. Flagged in night shift 2026-04-12 report.
- **Root cause**: overfitting
- **Verdict**: retry_with_changes
- **If retry_with_changes**: Tighten SL to 2.0 ATR max. Increase min_alignment to 3. Require ADX > 25 filter. Avoid tp_atr > 5.0 for BTC (lower volatility than SOL/ETH means wider TP relative to typical moves = overfitting bait).

---

### XRP/USDT — Dropped from Active Symbols
- **Date logged**: 2026-04-05
- **Hypothesis**: XRP/USDT was part of the 5-symbol portfolio and would contribute diversification
- **Test result**: Net negative across all tested configs. Portfolio return with XRP: +49.2%. Without XRP: +49.2% (XRP contributed 0.31% but with negative risk-adjusted return). Consistently lowest win rate (28-29%) and weakest Sharpe across all strategies.
- **Root cause**: regime mismatch — XRP price action dominated by regulatory events, not technical signals
- **Verdict**: DO_NOT_RETRY
- **If retry_with_changes**: N/A

---

### ETH/USDT Production Baseline — Marginal
- **Date logged**: 2026-04-12
- **Hypothesis**: Production config (signal_threshold=0.4, tp_atr=6.0, sl_atr=2.5) would work across all 4 active symbols
- **Test result**: ETH/USDT OOS Sharpe +0.97, consistency 56% (5/9 folds), survivor score 0.06. Marginal. Fold 1 and fold 7 had negative OOS (-2.19 and -2.57 Sharpe).
- **Root cause**: regime mismatch — ETH more sensitive to macro events, less driven by pure momentum
- **Verdict**: retry_with_changes
- **If retry_with_changes**: Try lower signal_threshold (0.3-0.35) for ETH. Tighter TP (3.0-4.0 ATR). Consider ETH-specific regime filter (ADX threshold 30+ for trend confirmation).

---

### BNB/USDT Production Baseline — Inconsistent
- **Date logged**: 2026-04-12
- **Hypothesis**: Production config would capture BNB trends
- **Test result**: BNB/USDT OOS Sharpe +1.47, consistency 56% (5/9 folds), survivor 0.11. Fold 1: -8.41 Sharpe, Fold 2: -0.34, Fold 5: -2.54, Fold 7: -1.66. Extremely volatile fold-level performance.
- **Root cause**: regime mismatch — BNB oscillates between Binance ecosystem news-driven moves and quiet ranging
- **Verdict**: retry_with_changes
- **If retry_with_changes**: Try shorter max_hold (24-36h) for BNB. Lower signal_threshold. BNB benefits from faster entries/exits — don't let losers run in BNB.

---

### SOL/USDT Production Baseline — Suboptimal
- **Date logged**: 2026-04-12
- **Hypothesis**: Production config (threshold=0.4, tp_atr=6.0, sl_atr=2.5, max_hold=96h) was optimal for SOL
- **Test result**: OOS Sharpe +2.05, consistency 67% (6/9 folds), survivor 0.23. Decent but dominated by the optimized config (Sharpe +3.96, 100% consistency, survivor 2.69). The wide TP and long max_hold were leaving money on the table.
- **Root cause**: overfitting — the production config was overfit to multi-symbol optimization (good average across all symbols, not optimal for SOL specifically)
- **Verdict**: retry_with_changes
- **If retry_with_changes**: ALREADY RESOLVED — the night shift found the SOL-specific optimal config (threshold=0.3, tp_atr=3.0, sl_atr=1.5, max_hold=36, trailing=0.5). This is now the current best strategy. Mark this as resolved.

---

### BB Mean Reversion — Broad Failure
- **Date logged**: 2026-04-12
- **Hypothesis**: Bollinger Band mean reversion strategy (price at lower band + RSI oversold + uptrend filter) would add uncorrelated alpha during range-bound periods
- **Test result**: 864 BB candidates evaluated across all symbols in night shift. None appeared in the top 20 candidates. BB strategies had consistently lower survivor scores than MultiTF strategies for all symbols during the tested period.
- **Root cause**: regime mismatch — April 2026 market is strongly trending (all 4 symbols in TREND regime with ADX 27-50). BB mean reversion requires RANGING regime.
- **Verdict**: retry_with_changes
- **If retry_with_changes**: Only activate BB strategies when ADX < 25 (range confirmed) for at least 3 consecutive days. Consider making the night shift regime-conditional: run BB grid only when market regime is RANGING. Do not abandon BB entirely — it is a key diversifier when trends reverse.

---

### High Signal Threshold (>0.45) for All Symbols
- **Date logged**: 2026-04-12
- **Hypothesis**: Higher signal thresholds would improve trade quality by filtering weak signals
- **Test result**: signal_threshold=0.45 and 0.50 consistently produced fewer trades (10-15/fold) with no improvement in Sharpe. The optimal threshold is 0.30-0.35 for SOL and 0.40 for BTC/BNB. Higher thresholds over-filter and reduce the sample size below statistical reliability.
- **Root cause**: overfitting — fewer trades means noisier metrics, more variance between folds
- **Verdict**: DO_NOT_RETRY
- **If retry_with_changes**: N/A

---

### Long Max Hold (>72h) with Tight SL
- **Date logged**: 2026-04-12
- **Hypothesis**: max_hold_hours=96-120 with tight stop_loss_atr=1.5 would capture extended moves while limiting downside
- **Test result**: Counter-intuitively, longer max hold with tight SL leads to getting stopped out more. The tight SL triggers before the trend develops, and then the max_hold is never reached. Results were uniformly worse than shorter max_hold (36-48h) with matching SL (1.5 ATR).
- **Root cause**: execution cost — tight SL + long hold = high stop-out rate before trend materializes
- **Verdict**: DO_NOT_RETRY
- **If retry_with_changes**: N/A

---

### SOL/USDT Survivor 2.69 Fragility Baseline
- **Date logged**: 2026-04-12
- **Hypothesis**: Production config fragility of 2.89 (high) indicated the strategy was fragile to parameter perturbation
- **Test result**: The optimized config (Survivor 2.69) reduced fragility from 2.89 to 0.29 — a 10x improvement. The production config was fragile because its wide parameters (tp_atr=6.0, sl_atr=2.5) were not specifically tuned for SOL volatility. The new config with tighter parameters (tp_atr=3.0, sl_atr=1.5) is in a flat region of the parameter landscape.
- **Root cause**: overfitting — production config was optimized for average across all symbols, not specifically for SOL
- **Verdict**: DO_NOT_RETRY
- **If retry_with_changes**: N/A (already resolved by Survivor 2.69)

---

### S14 Marubozu Retracement @ 1h — No Gross Edge (v3)
- **Date logged**: 2026-08-06
- **Hypothesis**: Moving S14 from 1m to 1h would make the raw edge survive Flash Trade fees (ATR rises 1m→1h from 0.096% to 1.079% of price, so winner/fee ratio jumps from ~0.8× to ~10×). v2 showed the 1m edge was real raw (OOS Sharpe 1.2–2.2) but died to fees (−86.3% compounded).
- **Test result**: Full-year 1h, real plugin, direction-tracking simulator, relaxed grid (body≥0.5×ATR, wick≤0.30): **204 trades/yr, GROSS Sharpe −0.01, avg gross PnL/trade 0.00%, net Sharpe −1.96, net PnL −71.2%, win rate 26%**. Winner/fee ratio 10.1× — fees are NOT the problem.
- **Root cause**: The 1m edge was a **frequency/microstructure phenomenon**, not a persistent price-structure edge. At 1h, marubozu-retracement signals are rare (relaxing body to 0.5×ATR ≈ any candle) and the pattern loses all predictive power. Gross edge ≈ 0 means no fee regime can rescue it.
- **Verdict**: DO_NOT_RETRY at hourly. The S14 line is exhausted across 1m (fees kill it) and 1h (no raw edge).
- **If retry_with_changes**: N/A. Do not re-propose S14 marubozu-retracement on SOL/ETH/BTC at any timeframe without a demonstrable positive GROSS edge first. The lesson generalizes: any strategy whose gross Sharpe is ~0 cannot be fixed by fee optimization. Screen for gross edge BEFORE tuning fees.

---

### S14 Marubozu Retracement @ Intermediate TFs (5m/15m/30m) — Low Win Rate Kills It (v4)
- **Date logged**: 2026-08-06
- **Hypothesis**: Between 1m (fee-killed) and 1h (signal-extinct), an intermediate TF (5m/15m/30m) would retain the gross edge AND clear the fee floor. Also tested under operational constraints (2.5 SOL starting capital, 0.15 SOL min-collar floor, 0.002 SOL gas/trade, 5-min polling executor).
- **Test result**: Fresh 365-day data at all 3 TFs. **Gross edge SURVIVES intermediates** — 5m +1.50 Sharpe, 15m +1.72, 30m +0.85 (sane configs). Edge-decay is single-peaked in the 5m–30m band, NOT a cliff. **But net-of-fee edge is negative at every TF**: best net Sharpe 5m −9.60, 15m −1.46, 30m −0.39. At the best config (30m, SL=1.5/TP=5.0/hold=48h): avg gross/trade +0.27%, avg fee/trade 0.39%, net expectancy −0.13%/trade = −8.3% over 66 trades. Focused exec sweep (SL/TP/hold/trail) could not reach net ≥ 0 at any TF.
- **Root cause**: **Low win rate (23%) is the binding constraint.** The retracement-into-momentum marubozu entry has an intrinsic ~23% hit rate. To clear the 0.39% fee floor, avg winner must be ~5× avg loser; the achievable R:R (~3.3:1 at TP=5.0/SL=1.5) and low hit rate make net expectancy negative. Winner/fee ratio was 11.5× — fees were NOT the problem; win rate was.
- **Verdict**: DO_NOT_RETRY for S14 (blind-touch) as-is. **SUPERSEDED IN PART by v5 (below)**: the confirmation-entry variant (S15) lifted win rate to 46–53% and cleared the fee floor at 20m. S14 blind-touch remains closed; the win-rate lesson below was CONFIRMED, not refuted.
- **If retry_with_changes**: N/A for S14 blind-touch. GENERALIZABLE LESSON — **require win rate > 40% (ideally >50%) as a pre-filter** before any strategy enters the fee-adjusted pipeline. A sub-40% hit-rate strategy cannot clear the 0.32% fixed fee floor unless its winner is absurdly large. v5 confirmed this empirically: confirmation entry lifting win rate from ~23% → 46–53% was exactly the change that flipped net expectancy positive.

---

### S15 Marubozu-with-Confirmation @ 20m — Bidirectional OOS Survivor (v5) — PARTIAL PASS, NOT A DEAD END
- **Date logged**: 2026-08-06
- **Hypothesis**: The confirmation entry step from the original marubozu idea ("enter only if price holds and resumes", not blind touch) would lift win rate from ~23% to ≥35% and flip net expectancy above the fee floor on the 5m–30m ladder.
- **Test result**: A/B ladder (blind vs 6 confirmation variants × 216 base configs × 5 TFs): **NET GATE PASS** — +0.152%/trade @ 20m close_reassert_b1. Direction-aware finisher (exec sweep + 9-fold net WFA + composite long∪short): **20m_COMPOSITE champion — survivor 0.767, median OOS Sharpe 2.38, consistency 78%, OOS PnL +46.9% with BOTH directions net positive (long +16.9% / short +30.1%)**. Sensitivity ROBUST (±20%, 0 sign flips). 2.5 SOL compounds to 3.79 SOL @5x (dd 8.5%, 0 liq, 0 halts, 0.21 trades/day).
- **Open gaps (why PARTIAL, not deploy)**: (1) latency retention 48% < 80% gate — the 1-poll-delayed variant keeps only half the edge; needs limit-order entry or a 20m-executor with <5-min poll; (2) SOL-specific — BTC −28.2% and ETH −26.5% OOS; (3) thin folds — avg 7.7 trades/fold, min 3 (floor is 10).
- **Root cause (of the rescue)**: confirmation entry is a genuine win-rate lever. close_reassert_b1 at 20m: blind 24–27% WR → 40–53% WR depending on config, which alone was enough to cross the fee floor.
- **Verdict**: retry_with_changes. S15 at 20m is the first bidirectional, OOS-positive, fee-cleared candidate in the marubozu family. Config + provenance in `research/data/results/s15_v5/winning_config.json`.
- **If retry_with_changes**: (a) re-test latency with confirm_bars=2 or limit-at-zone entries; (b) accumulate a second year of data to thicken folds (need ≥90 trades/yr at 20m); (c) do NOT propose for BTC/ETH without symbol-specific re-optimization.

---

### S16 Real multi-TF re-validation — parity gap CONFIRMED, no candidate clears gates
- **Date logged**: 2026-08-23
- **Hypothesis**: The live trader runs REAL multi-TF (independent 1h/4h/1d Binance series, each with its own 20-period SMA), while every prior validation artifact (Calmar 44.89, OOS Sharpe 3.96, sensitivity CSV, night-shift candidates) was computed on FAKE multi-TF (lookback 20/80/200 on a single 1h series — `per_symbol_optimizer.py:37`). Re-validating the REAL model should find a promotable config for the engine as deployed.
- **Test result**: `research/missions/s16_real_tf_revalidation.py`, 1y SOL data (2025-08-08 → 2026-08-23), 9 anchored 36-day folds, GMTrade measured fees (0.022%/trip + long borrow 0.0036%/hr), 9× leverage. Raw edge (1×, zero fees): +0.035%/trade @ threshold 0.24 (align 2), ≈0 at threshold ≥ 0.28. At 9× with measured fees: deployed baseline (0.30/align2/trail1.0) net −197.7%, live override (0.24) net +32.7%, night-shift candidate (0.30/align3) net +80.0% — best median OOS Sharpe across 17 candidates was 0.04. **No candidate cleared the promotion gates** (min Sharpe 2.5, consistency 70%). Calibration (s16b) showed the same simulator is also negative on the FAKE model over this window — the validated-era numbers were regime-dependent (2023-2025 windows), not reproducible on the last 12 months under either model.
- **Root cause**: (1) the validated model and the shipped model are genuinely different signal generators (today's data: real score +0.417 vs fake +0.267 on identical bars); (2) at 9× leverage the fixed cost basis (~0.29%/trip of collateral) is ~90% of the gross edge — the strategy is fee-marginal at current trade frequency; (3) trailing stop is the dominant exit and net-negative in both models on this window, matching the live tape (TrailingStop −17.3% cumulative vs TakeProfit +27.1%).
- **Verdict**: retry_with_changes. NO config promotion is justified; the live config stays unchanged. Do NOT tighten `trailing_stop_atr` on the real model (0.5 ATR: −453% vs 1.0 baseline) — the fake-TF sensitivity sweep that favored 0.4 does not transfer.
- **If retry_with_changes**: (a) rebuild the night-shift validation on the REAL multi-TF model (4h/1d parquet fetch + per_symbol_optimizer parity) so the pipeline validates what the trader runs; (b) re-optimize entry frequency/quality there — gross edge per trade is the binding constraint, not exits; (c) consider the venue-side stop-loss order (StopLossDecrease) as tail-risk insurance only; (d) artifacts: `data/results/s16_real_tf/revalidation.json` (gitignored), scripts `research/missions/s16_real_tf_revalidation.py` + `s16b_calibration.py`.

---

### S17 Trailing-stop arm variants — armed/breakeven trails do NOT clear gates
- **Date logged**: 2026-08-26
- **Hypothesis**: Live-forensics (161-trade audit) showed the trailing-stop family is the biggest live drag (95 exits, −26.4 net pts; 60/87 trail exits led ≥1 ATR then gave it back; 37 saw >2% favorable runs within 24h after exit). S16's zone counterfactual proved naive "hold to TP" is flat, so test the mechanism-based alternative: delay the trailing stop until the trade leads ≥N×ATR (arm_1.0/1.5/2.0), optionally ratchet to breakeven first (be_N, widths 1.0–2.0×ATR).
- **Test result**: `research/missions/s17_trail_arm_variants.py` (S16 real multi-TF harness imported verbatim — 1h/4h/1d Binance feeds, compute_signal score, ATR=std(20)×price, check_exit priority), 2025-08-11 → 2026-08-26 (9,119 bars), 9 anchored 36-day folds, GMTrade measured fees, 9×. 22 configs (11 trail variants × thresholds 0.24/0.30). **No config cleared the statistical gates** (Sharpe≥1.5, folds≥3, WR≥45%, PF≥1.3, DD≤20%); best median Sharpe 0.03 (trail_off @ 0.24). At thr 0.24: live_trail1.0 net +117.9% vs arm_2.0 −122.5% and wider be variants −376% to −454%. At thr 0.30: trail_off +93.9% was best but live stayed negative; every arm/be variant at thr 0.24 LOST money. The arm variants trade MORE (SL exits rise 152→247 as trail defers), confirming the deferred trail lets losers ride to SL. Fresh-data note: full-year 0.24 baseline net rose from S16's +32.7% (window ending Aug 23) to +117.9% (window ending Aug 26) — engine parity verified (S16 evaluator reproduces S17's live spec exactly), the delta is market data, not simulation change.
- **Root cause**: (1) the trail is not the leak — it is the only exit family that converts chop into small controlled losses; deferring it converts them into full SL hits; (2) TP harvest is 12% of reachable zones because chop touches SL and TP alike and SL usually lands first — patience does not recover it; (3) consistent with S16: tighter trails −453%, now ALSO confirmed wider/armed trails are negative; the 1.0×ATR trail sits at the local optimum of a genuinely rough payoff surface.
- **Verdict**: dead_end. Do NOT change trailing_stop_atr or add arm/breakeven logic. The live trail (1.0×ATR, arm on favorable lead) stays. Combined with S16, the exit side of this strategy is saturated: further exit tuning on the real model is not a profit source.
- **Artifacts**: `data/results/s17_trail_variants/variants.json` (gitignored), `research/missions/s17_trail_arm_variants.py`.

---

*Last updated: 2026-08-26. Populate from future backtest runs and night shift reports.*
