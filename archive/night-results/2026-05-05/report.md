# Night Shift Report — 2026-05-05

**Runtime:** 11896s | **Folds:** 9 | **Symbols:** SOL/USDT
**Aggregation:** Median OOS Sharpe, per-fold Sharpe winsorized at ±100

## Market State

| Symbol | Regime | ADX | ADX Trend | Vol %ile | 30d Return |
|--------|--------|-----|-----------|----------|------------|
| SOL/USDT | TREND | 40.6 | FALLING | 71% | +1.4% |

**Correlations:**

## Production Baseline (Current Config)

| Symbol | OOS Sharpe | OOS PF | OOS WR | Consistency | MaxDD | Survivor |
|--------|-----------|--------|--------|-------------|-------|----------|
| SOL/USDT | +2.05 | 1.3 | 54% | 67% | 3.0% | 0.23 |

## Top 10 Candidates (Ranked by Survivor Score)

*Only candidates validated on 5+ WFA folds are shown.*

**Strategy breakdown:** 15001 MultiTF, 216 BB Mean Reversion

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
  "score_flip_delay_hrs": 0,
  "leverage": 1.0
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
  "score_flip_delay_hrs": 1,
  "leverage": 1.0
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
  "score_flip_delay_hrs": 2,
  "leverage": 1.0
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
  "score_flip_delay_hrs": 3,
  "leverage": 1.0
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
  "score_flip_delay_hrs": 4,
  "leverage": 1.0
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
  "score_flip_delay_hrs": 0,
  "leverage": 1.0
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
  "score_flip_delay_hrs": 1,
  "leverage": 1.0
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
  "score_flip_delay_hrs": 2,
  "leverage": 1.0
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
  "score_flip_delay_hrs": 3,
  "leverage": 1.0
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
  "score_flip_delay_hrs": 4,
  "leverage": 1.0
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

⚠️ SOL/USDT {'signal_threshold': 0.3, 'take_profit_atr': 3.0, 'stop_loss_atr': 1.25, 'max_hold_hours': 36, 'time_decay_hours': 12, 'min_alignment': 3, 'trailing_stop_atr': 0.5, 'score_flip_delay_hrs': 0, 'leverage': 2.0}: overfitting_score=0.62 > 0.5 (OOS Sharpe: +1.71, IS-OOS gap: 0.62)
⚠️ SOL/USDT {'signal_threshold': 0.3, 'take_profit_atr': 3.0, 'stop_loss_atr': 1.25, 'max_hold_hours': 36, 'time_decay_hours': 12, 'min_alignment': 3, 'trailing_stop_atr': 0.5, 'score_flip_delay_hrs': 1, 'leverage': 2.0}: overfitting_score=0.62 > 0.5 (OOS Sharpe: +1.71, IS-OOS gap: 0.62)

## Per-Symbol WFA Fold Detail

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

## Action Items

1. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3, take_profit_atr: 6.0→3.0, stop_loss_atr: 2.5→1.5, max_hold_hours: 96→36, time_decay_hours: 48→12, trailing_stop_atr: 1.0→0.5, score_flip_delay_hrs: 2→0, leverage: None→1.0
   OOS Sharpe: +3.96 (vs +2.05), consistency: 100%, DD: 4.7%, trades/fold: 47
   ⚠️ Overfitting score: 0.08 — monitor closely

2. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3, take_profit_atr: 6.0→3.0, stop_loss_atr: 2.5→1.5, max_hold_hours: 96→36, time_decay_hours: 48→12, trailing_stop_atr: 1.0→0.5, score_flip_delay_hrs: 2→1, leverage: None→1.0
   OOS Sharpe: +3.96 (vs +2.05), consistency: 100%, DD: 4.7%, trades/fold: 47
   ⚠️ Overfitting score: 0.08 — monitor closely

3. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3, take_profit_atr: 6.0→3.0, stop_loss_atr: 2.5→1.5, max_hold_hours: 96→36, time_decay_hours: 48→12, trailing_stop_atr: 1.0→0.5, leverage: None→1.0
   OOS Sharpe: +3.96 (vs +2.05), consistency: 100%, DD: 4.7%, trades/fold: 47
   ⚠️ Overfitting score: 0.08 — monitor closely

4. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3, take_profit_atr: 6.0→3.0, stop_loss_atr: 2.5→1.5, max_hold_hours: 96→36, time_decay_hours: 48→12, trailing_stop_atr: 1.0→0.5, score_flip_delay_hrs: 2→3, leverage: None→1.0
   OOS Sharpe: +3.96 (vs +2.05), consistency: 100%, DD: 4.7%, trades/fold: 47
   ⚠️ Overfitting score: 0.08 — monitor closely

5. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3, take_profit_atr: 6.0→3.0, stop_loss_atr: 2.5→1.5, max_hold_hours: 96→36, time_decay_hours: 48→12, trailing_stop_atr: 1.0→0.5, score_flip_delay_hrs: 2→4, leverage: None→1.0
   OOS Sharpe: +3.96 (vs +2.05), consistency: 100%, DD: 4.7%, trades/fold: 47
   ⚠️ Overfitting score: 0.08 — monitor closely

Total: 5 actionable recommendations out of 1484 validated candidates.
