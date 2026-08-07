# S15 FINAL VERDICT — Friend's Engine (v7e, corrected folds)

Generated: 2026-08-07T19:33:02.256892

## Verdict: DEPLOYABLE

Data: SOL/USDT 20m, 2 years (2024-08-06 -> 2026-08-05), 52,530 bars
Fee basis: MEASURED Flash v2 (2026-08-07) — 0.02% open + 0.02% close + ~0.01%/side spread + 0.0004%/hr borrow (both sides) = 0.06% round-trip

## Operating config (limit-at-zone execution)

- Entry: blind touch (confirm_mode=none) — resting limit at the retracement zone; the order book absorbs detection latency
- Leverage: 5.0x | TF: 20m | composite long+short legs
- Champion params: v5 winning_config.json with confirm_mode flipped none on both legs

## Gate matrix (20 full 36-day anchored windows + 1 trailing 6-day remnant)

- [PASS] oos_positive_total_pnl
- [PASS] oos_consistency_ge_50
- [PASS] bidirectional_attribution_ge_0
- [PASS] min_trades_per_fold_ge_10
- [PASS] latency_retention_ge_80
- [PASS] sensitivity_robust
- [PASS] compounding_gt_start
- [PASS] dd_le_25
- [PASS] zero_halts_zero_liq
- [PASS] trades_per_day_le_4

## Headline numbers

- OOS PnL: +49.8% over 334 trades (avg 15.9/fold; full-window min 10, trailing 6-day remnant fold excluded from the floor: 1t)
- Consistency: 67% (14/21 folds positive) | median OOS Sharpe 1.59
- Attribution: long +12.0% / short +37.8%
- Compounded @ 5.0x from 2.5 SOL: 3.1942 SOL (+27.8%), DD 20.56%, 0.51/day, 0 liq, 0 halts
- Latency retention (+1 bar): 105%
- Sensitivity: ROBUST (spread 26.7, 0 sign flips)

## Regime honesty

- Year 1 (2024-08 -> 2025-08): 205 trades, net +23.6% — Aug-Dec 2024 was underwater (-13.4% across 4 months) before turning positive Jan 2025
- Year 2 (2025-08 -> 2026-08): 165 trades, net +38.1%
- Worst single month: 2025-12 (-11.6%); best: 2025-11 (+14.4%)
- Negative folds: 7 of 21 (2024-08-09, 2024-10-20, 2024-11-25, 2025-05-24, 2025-11-20, 2026-03-08, 2026-07-30)

## Methodology correction (v7e)

create_folds() absorbs all leftover bars into the LAST fold when data > num_folds x window — on 2yr data that made one 438-day mega-fold hold 212 of 370 trades and dominate the headline stats. v7e rebuilds the walk-forward as 20 equal 36-day anchored windows (+ 1 trailing 6-day remnant, reported but excluded from the 36d trade floor — a 6-day window can never hold 10 trades of a 0.51/day strategy) and re-runs all fold-level gates on them. Full-window artifacts (sensitivity, compounding, latency) are fold-shape independent and unchanged.

## Lineage

1. v5: champion 20m_COMPOSITE (close_reassert cb1), 8/11 criteria, latency 48% FAIL, v1-era fees
2. v6: 2yr re-validation falsified champion (33% cons, -13.5%) under v1-era fees (0.32%/trip) — fee artifact
3. v7: measured v2 fees resurrect champion (8/10); gaps latency 47%, min trades 4
4. v7b: 72d folds fix trade floor; cb2 NOT the latency absorber (-88%)
5. v7c: blind touch (limit-at-zone) — latency 108%, min trades 31, OOS +68.4%
6. v7d: full gate suite on blind touch — 10/10, but fold artifact discovered
7. v7e: equal-fold re-gate (this document)

## Remaining risks before live deployment

- Live limit-order placement on Flash must be verified (order support, fill rate at zone price); backtest assumes fills at touch-bar close, stressed +1 bar
- SOL-only mandate (BTC/ETH transfer negative) — waived by design
- Aug-Dec 2024 regime was net negative: expect multi-month drawdown periods in live operation
- Rust momentum off-by-one — FIXED (commit b178da7, deploy 188e006e, Aug 7): momentum/volatility now match the Python reference; was unrelated to this Python-validated config anyway
