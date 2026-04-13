# Dead Ends — Failure Memory Log

> This file is the institutional memory of what has been tried and failed. Any new agent or human MUST check this file before proposing a strategy or parameter range. Repeating dead ends wastes compute and time.
>
> **Rule:** Every strategy or parameter set that fails validation gets logged here with the reason. Only remove entries with explicit human approval.

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

*Last updated: 2026-04-13. Populate from future backtest runs and night shift reports.*
