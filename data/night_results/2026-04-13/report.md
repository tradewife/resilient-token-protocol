# Night Shift Report — 2026-04-13

**Runtime:** 9915s | **Folds:** 9 | **Symbols:** BTC/USDT, ETH/USDT, SOL/USDT, BNB/USDT
**Aggregation:** Median OOS Sharpe, per-fold Sharpe winsorized at ±100

## Market State

| Symbol | Regime | ADX | ADX Trend | Vol %ile | 30d Return |
|--------|--------|-----|-----------|----------|------------|
| BTC/USDT | TREND | 43.7 | STABLE | 84% | +5.7% |
| ETH/USDT | TREND | 49.7 | STABLE | 78% | +12.9% |
| SOL/USDT | TREND | 40.6 | FALLING | 71% | +1.4% |
| BNB/USDT | TREND | 26.6 | STABLE | 51% | -2.4% |

**Correlations:**
  BNB/USDT_BTC/USDT: 0.88
  BNB/USDT_ETH/USDT: 0.86
  BNB/USDT_SOL/USDT: 0.84
  BTC/USDT_ETH/USDT: 0.91
  BTC/USDT_SOL/USDT: 0.86
  ETH/USDT_SOL/USDT: 0.88

## Production Baseline (Current Config)

| Symbol | OOS Sharpe | OOS PF | OOS WR | Consistency | MaxDD | Survivor |
|--------|-----------|--------|--------|-------------|-------|----------|
| BTC/USDT | +0.95 | 2.0 | 54% | 67% | 2.7% | 0.12 |
| ETH/USDT | +0.97 | 1.0 | 50% | 56% | 5.5% | 0.06 |
| SOL/USDT | +2.05 | 1.3 | 54% | 67% | 3.0% | 0.23 |
| BNB/USDT | +1.47 | 1.1 | 56% | 56% | 3.6% | 0.11 |

## Top 10 Candidates (Ranked by Survivor Score)

*Only candidates validated on 5+ WFA folds are shown.*

**Strategy breakdown:** 6104 MultiTF, 864 BB Mean Reversion

### #1: SOL/USDT (Survivor: 2.69 +2.46)
```json
{
  "signal_threshold": 0.3,
  "take_profit_atr": 3.0,
  "stop_loss_atr": 1.5,
  "max_hold_hours": 36,
  "time_decay_hours": 12,
  "min_alignment": 3,
  "trailing_stop_atr": 0.5,
  "score_flip_delay_hrs": 0
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +2.05 | +3.96 | +1.91 |
| OOS PF | 1.3 | 1.2 | -0.2 |
| Consistency | 67% | 100% | +33% |
| MaxDD | 3.0% | 4.7% | +1.6% |
| Overfitting | 0.33 | 0.08 | -0.25 |
| Fragility | 2.89 | 0.29 | |

✅ **STRONG RECOMMEND** — trades/fold: 47, exits: {'trailing_stop': 239, 'stop_loss': 98, 'take_profit': 48, 'mr_target': 3, 'score_flip': 31}

### #2: SOL/USDT (Survivor: 2.69 +2.46)
```json
{
  "signal_threshold": 0.3,
  "take_profit_atr": 3.0,
  "stop_loss_atr": 1.5,
  "max_hold_hours": 36,
  "time_decay_hours": 12,
  "min_alignment": 3,
  "trailing_stop_atr": 0.5,
  "score_flip_delay_hrs": 1
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +2.05 | +3.96 | +1.91 |
| OOS PF | 1.3 | 1.2 | -0.2 |
| Consistency | 67% | 100% | +33% |
| MaxDD | 3.0% | 4.7% | +1.6% |
| Overfitting | 0.33 | 0.08 | -0.25 |
| Fragility | 2.89 | 0.29 | |

✅ **STRONG RECOMMEND** — trades/fold: 47, exits: {'trailing_stop': 239, 'stop_loss': 98, 'take_profit': 48, 'mr_target': 3, 'score_flip': 31}

### #3: SOL/USDT (Survivor: 2.69 +2.46)
```json
{
  "signal_threshold": 0.3,
  "take_profit_atr": 3.0,
  "stop_loss_atr": 1.5,
  "max_hold_hours": 36,
  "time_decay_hours": 12,
  "min_alignment": 3,
  "trailing_stop_atr": 0.5,
  "score_flip_delay_hrs": 2
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +2.05 | +3.96 | +1.91 |
| OOS PF | 1.3 | 1.2 | -0.2 |
| Consistency | 67% | 100% | +33% |
| MaxDD | 3.0% | 4.7% | +1.6% |
| Overfitting | 0.33 | 0.08 | -0.25 |
| Fragility | 2.89 | 0.29 | |

✅ **STRONG RECOMMEND** — trades/fold: 47, exits: {'trailing_stop': 239, 'stop_loss': 98, 'take_profit': 48, 'mr_target': 3, 'score_flip': 31}

### #4: SOL/USDT (Survivor: 2.69 +2.46)
```json
{
  "signal_threshold": 0.3,
  "take_profit_atr": 3.0,
  "stop_loss_atr": 1.5,
  "max_hold_hours": 36,
  "time_decay_hours": 12,
  "min_alignment": 3,
  "trailing_stop_atr": 0.5,
  "score_flip_delay_hrs": 3
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +2.05 | +3.96 | +1.91 |
| OOS PF | 1.3 | 1.2 | -0.2 |
| Consistency | 67% | 100% | +33% |
| MaxDD | 3.0% | 4.7% | +1.6% |
| Overfitting | 0.33 | 0.08 | -0.25 |
| Fragility | 2.89 | 0.29 | |

✅ **STRONG RECOMMEND** — trades/fold: 47, exits: {'trailing_stop': 239, 'stop_loss': 98, 'take_profit': 48, 'mr_target': 3, 'score_flip': 31}

### #5: SOL/USDT (Survivor: 2.69 +2.46)
```json
{
  "signal_threshold": 0.3,
  "take_profit_atr": 3.0,
  "stop_loss_atr": 1.5,
  "max_hold_hours": 36,
  "time_decay_hours": 12,
  "min_alignment": 3,
  "trailing_stop_atr": 0.5,
  "score_flip_delay_hrs": 4
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +2.05 | +3.96 | +1.91 |
| OOS PF | 1.3 | 1.2 | -0.2 |
| Consistency | 67% | 100% | +33% |
| MaxDD | 3.0% | 4.7% | +1.6% |
| Overfitting | 0.33 | 0.08 | -0.25 |
| Fragility | 2.89 | 0.29 | |

✅ **STRONG RECOMMEND** — trades/fold: 47, exits: {'trailing_stop': 239, 'stop_loss': 98, 'take_profit': 48, 'mr_target': 3, 'score_flip': 31}

### #6: SOL/USDT (Survivor: 2.69 +2.46)
```json
{
  "signal_threshold": 0.3,
  "take_profit_atr": 3.0,
  "stop_loss_atr": 1.5,
  "max_hold_hours": 48,
  "time_decay_hours": 12,
  "min_alignment": 3,
  "trailing_stop_atr": 0.5,
  "score_flip_delay_hrs": 0
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +2.05 | +3.96 | +1.91 |
| OOS PF | 1.3 | 1.2 | -0.2 |
| Consistency | 67% | 100% | +33% |
| MaxDD | 3.0% | 4.7% | +1.6% |
| Overfitting | 0.33 | 0.08 | -0.25 |
| Fragility | 2.89 | 0.29 | |

✅ **STRONG RECOMMEND** — trades/fold: 47, exits: {'trailing_stop': 239, 'stop_loss': 98, 'take_profit': 48, 'mr_target': 3, 'score_flip': 31}

### #7: SOL/USDT (Survivor: 2.69 +2.46)
```json
{
  "signal_threshold": 0.3,
  "take_profit_atr": 3.0,
  "stop_loss_atr": 1.5,
  "max_hold_hours": 48,
  "time_decay_hours": 12,
  "min_alignment": 3,
  "trailing_stop_atr": 0.5,
  "score_flip_delay_hrs": 1
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +2.05 | +3.96 | +1.91 |
| OOS PF | 1.3 | 1.2 | -0.2 |
| Consistency | 67% | 100% | +33% |
| MaxDD | 3.0% | 4.7% | +1.6% |
| Overfitting | 0.33 | 0.08 | -0.25 |
| Fragility | 2.89 | 0.29 | |

✅ **STRONG RECOMMEND** — trades/fold: 47, exits: {'trailing_stop': 239, 'stop_loss': 98, 'take_profit': 48, 'mr_target': 3, 'score_flip': 31}

### #8: SOL/USDT (Survivor: 2.69 +2.46)
```json
{
  "signal_threshold": 0.3,
  "take_profit_atr": 3.0,
  "stop_loss_atr": 1.5,
  "max_hold_hours": 48,
  "time_decay_hours": 12,
  "min_alignment": 3,
  "trailing_stop_atr": 0.5,
  "score_flip_delay_hrs": 2
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +2.05 | +3.96 | +1.91 |
| OOS PF | 1.3 | 1.2 | -0.2 |
| Consistency | 67% | 100% | +33% |
| MaxDD | 3.0% | 4.7% | +1.6% |
| Overfitting | 0.33 | 0.08 | -0.25 |
| Fragility | 2.89 | 0.29 | |

✅ **STRONG RECOMMEND** — trades/fold: 47, exits: {'trailing_stop': 239, 'stop_loss': 98, 'take_profit': 48, 'mr_target': 3, 'score_flip': 31}

### #9: SOL/USDT (Survivor: 2.69 +2.46)
```json
{
  "signal_threshold": 0.3,
  "take_profit_atr": 3.0,
  "stop_loss_atr": 1.5,
  "max_hold_hours": 48,
  "time_decay_hours": 12,
  "min_alignment": 3,
  "trailing_stop_atr": 0.5,
  "score_flip_delay_hrs": 3
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +2.05 | +3.96 | +1.91 |
| OOS PF | 1.3 | 1.2 | -0.2 |
| Consistency | 67% | 100% | +33% |
| MaxDD | 3.0% | 4.7% | +1.6% |
| Overfitting | 0.33 | 0.08 | -0.25 |
| Fragility | 2.89 | 0.29 | |

✅ **STRONG RECOMMEND** — trades/fold: 47, exits: {'trailing_stop': 239, 'stop_loss': 98, 'take_profit': 48, 'mr_target': 3, 'score_flip': 31}

### #10: SOL/USDT (Survivor: 2.69 +2.46)
```json
{
  "signal_threshold": 0.3,
  "take_profit_atr": 3.0,
  "stop_loss_atr": 1.5,
  "max_hold_hours": 48,
  "time_decay_hours": 12,
  "min_alignment": 3,
  "trailing_stop_atr": 0.5,
  "score_flip_delay_hrs": 4
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +2.05 | +3.96 | +1.91 |
| OOS PF | 1.3 | 1.2 | -0.2 |
| Consistency | 67% | 100% | +33% |
| MaxDD | 3.0% | 4.7% | +1.6% |
| Overfitting | 0.33 | 0.08 | -0.25 |
| Fragility | 2.89 | 0.29 | |

✅ **STRONG RECOMMEND** — trades/fold: 47, exits: {'trailing_stop': 239, 'stop_loss': 98, 'take_profit': 48, 'mr_target': 3, 'score_flip': 31}

## Overfitting Warnings

⚠️ BTC/USDT {'signal_threshold': 0.4, 'take_profit_atr': 6.0, 'stop_loss_atr': 3.0, 'max_hold_hours': 36, 'time_decay_hours': 24, 'min_alignment': 3, 'trailing_stop_atr': 0.5, 'score_flip_delay_hrs': 0}: overfitting_score=0.57 > 0.5 (OOS Sharpe: +1.35, IS-OOS gap: 0.57)
⚠️ BTC/USDT {'signal_threshold': 0.4, 'take_profit_atr': 6.0, 'stop_loss_atr': 3.0, 'max_hold_hours': 36, 'time_decay_hours': 24, 'min_alignment': 3, 'trailing_stop_atr': 0.5, 'score_flip_delay_hrs': 1}: overfitting_score=0.57 > 0.5 (OOS Sharpe: +1.35, IS-OOS gap: 0.57)

## Per-Symbol WFA Fold Detail

### BTC/USDT — Best Validated Candidate (Survivor: 1.63)
| Fold | IS Sharpe | OOS Sharpe | OOS PnL | OOS Trades |
|------|-----------|-----------|---------|------------|
| 0 | +0.00 | +2.44 | +5.12% | 55 ✅ |
| 1 | +0.00 | +1.81 | +2.38% | 40 ✅ |
| 2 | +0.00 | +0.87 | +1.21% | 36 ✅ |
| 3 | +0.00 | +5.65 | +6.48% | 37 ✅ |
| 4 | +0.00 | -0.69 | -0.99% | 33 ❌ |
| 5 | +0.00 | +2.16 | +4.05% | 21 ✅ |
| 6 | +0.00 | +5.18 | +7.25% | 30 ✅ |
| 7 | +0.00 | +0.93 | +0.72% | 11 ✅ |
| 8 | +0.00 | +1.89 | +4.34% | 31 ✅ |

### ETH/USDT — Best Validated Candidate (Survivor: 1.05)
| Fold | IS Sharpe | OOS Sharpe | OOS PnL | OOS Trades |
|------|-----------|-----------|---------|------------|
| 0 | +0.00 | +3.14 | +13.99% | 53 ✅ |
| 1 | +3.26 | -2.19 | -4.83% | 35 ❌ |
| 2 | +1.57 | +7.63 | +30.17% | 68 ✅ |
| 3 | +5.10 | +0.34 | +0.74% | 25 ✅ |
| 4 | +3.82 | +1.17 | +2.66% | 32 ✅ |
| 5 | +3.80 | +3.22 | +5.53% | 33 ✅ |
| 6 | +3.50 | +7.41 | +13.17% | 44 ✅ |
| 7 | +3.59 | -2.57 | -4.94% | 11 ❌ |
| 8 | +3.09 | +6.45 | +20.00% | 35 ✅ |

### SOL/USDT — Best Validated Candidate (Survivor: 2.69)
| Fold | IS Sharpe | OOS Sharpe | OOS PnL | OOS Trades |
|------|-----------|-----------|---------|------------|
| 0 | +0.00 | +4.80 | +14.85% | 63 ✅ |
| 1 | +6.04 | +3.96 | +9.88% | 41 ✅ |
| 2 | +5.17 | +4.58 | +18.38% | 61 ✅ |
| 3 | +5.52 | +5.78 | +17.78% | 63 ✅ |
| 4 | +5.07 | +1.40 | +3.29% | 40 ✅ |
| 5 | +4.64 | +4.37 | +11.02% | 32 ✅ |
| 6 | +4.66 | +2.62 | +6.99% | 65 ✅ |
| 7 | +4.02 | +0.56 | +1.02% | 19 ✅ |
| 8 | +3.69 | +3.18 | +9.85% | 36 ✅ |

### BNB/USDT — Best Validated Candidate (Survivor: 1.61)
| Fold | IS Sharpe | OOS Sharpe | OOS PnL | OOS Trades |
|------|-----------|-----------|---------|------------|
| 0 | +0.00 | +7.93 | +11.88% | 26 ✅ |
| 1 | +0.00 | -8.41 | -2.80% | 16 ❌ |
| 2 | +0.00 | -0.34 | -0.68% | 24 ❌ |
| 3 | +0.00 | +3.36 | +5.18% | 19 ✅ |
| 4 | +0.00 | +3.11 | +4.98% | 14 ✅ |
| 5 | +0.00 | -2.54 | -1.68% | 9 ❌ |
| 6 | +0.00 | +2.98 | +2.28% | 17 ✅ |
| 7 | +0.00 | -1.66 | -1.14% | 5 ❌ |
| 8 | +0.00 | +4.68 | +4.64% | 15 ✅ |

## Action Items

1. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3, take_profit_atr: 6.0→3.0, stop_loss_atr: 2.5→1.5, max_hold_hours: 96→36, time_decay_hours: 48→12, trailing_stop_atr: 1.0→0.5, score_flip_delay_hrs: 2→0
   OOS Sharpe: +3.96 (vs +2.05), consistency: 100%, DD: 4.7%, trades/fold: 47
   ⚠️ Overfitting score: 0.08 — monitor closely

2. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3, take_profit_atr: 6.0→3.0, stop_loss_atr: 2.5→1.5, max_hold_hours: 96→36, time_decay_hours: 48→12, trailing_stop_atr: 1.0→0.5, score_flip_delay_hrs: 2→1
   OOS Sharpe: +3.96 (vs +2.05), consistency: 100%, DD: 4.7%, trades/fold: 47
   ⚠️ Overfitting score: 0.08 — monitor closely

3. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3, take_profit_atr: 6.0→3.0, stop_loss_atr: 2.5→1.5, max_hold_hours: 96→36, time_decay_hours: 48→12, trailing_stop_atr: 1.0→0.5
   OOS Sharpe: +3.96 (vs +2.05), consistency: 100%, DD: 4.7%, trades/fold: 47
   ⚠️ Overfitting score: 0.08 — monitor closely

4. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3, take_profit_atr: 6.0→3.0, stop_loss_atr: 2.5→1.5, max_hold_hours: 96→36, time_decay_hours: 48→12, trailing_stop_atr: 1.0→0.5, score_flip_delay_hrs: 2→3
   OOS Sharpe: +3.96 (vs +2.05), consistency: 100%, DD: 4.7%, trades/fold: 47
   ⚠️ Overfitting score: 0.08 — monitor closely

5. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3, take_profit_atr: 6.0→3.0, stop_loss_atr: 2.5→1.5, max_hold_hours: 96→36, time_decay_hours: 48→12, trailing_stop_atr: 1.0→0.5, score_flip_delay_hrs: 2→4
   OOS Sharpe: +3.96 (vs +2.05), consistency: 100%, DD: 4.7%, trades/fold: 47
   ⚠️ Overfitting score: 0.08 — monitor closely

Total: 5 actionable recommendations out of 4180 validated candidates.
