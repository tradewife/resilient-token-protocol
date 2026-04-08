# Night Shift Report — 2026-04-08

**Runtime:** 1956s | **Folds:** 3 | **Symbols:** SOL/USDT
**Aggregation:** Median OOS Sharpe, per-fold Sharpe winsorized at ±100

## Market State

| Symbol | Regime | ADX | ADX Trend | Vol %ile | 30d Return |
|--------|--------|-----|-----------|----------|------------|
| SOL/USDT | RANGE | 15.4 | STABLE | 23% | -3.5% |

**Correlations:**

## Production Baseline (Current Config)

| Symbol | OOS Sharpe | OOS PF | OOS WR | Consistency | MaxDD | Survivor |
|--------|-----------|--------|--------|-------------|-------|----------|
| SOL/USDT | +1.68 | 1.6 | 57% | 100% | 5.2% | 1.08 |

## Top 10 Candidates (Ranked by Survivor Score)

*Only candidates validated on 5+ WFA folds are shown.*

**Strategy breakdown:** 156 MultiTF, 216 BB Mean Reversion

### #1: SOL/USDT (Survivor: 5.36 +4.28)
```json
{
  "signal_threshold": 0.3174,
  "min_alignment": 3,
  "take_profit_atr": 6.0,
  "stop_loss_atr": 2.5,
  "max_hold_hours": 96,
  "time_decay_hours": 50,
  "trailing_stop_atr": 0.01,
  "score_flip_delay_hrs": 2
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.68 | +5.99 | +4.31 |
| OOS PF | 1.6 | 0.5 | -1.0 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 5.2% | 8.4% | +3.2% |
| Overfitting | 0.05 | 0.00 | -0.05 |
| Fragility | 0.40 | 0.03 | |

✅ **STRONG RECOMMEND** — trades/fold: 171, exits: {'trailing_stop': 377, 'stop_loss': 85, 'score_flip': 47, 'take_profit': 1, 'mr_target': 2}

### #2: SOL/USDT (Survivor: 5.36 +4.28)
```json
{
  "signal_threshold": 0.3174,
  "min_alignment": 3,
  "take_profit_atr": 6.0,
  "stop_loss_atr": 2.5,
  "max_hold_hours": 96,
  "time_decay_hours": 56,
  "trailing_stop_atr": 0.01,
  "score_flip_delay_hrs": 2
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.68 | +5.99 | +4.31 |
| OOS PF | 1.6 | 0.5 | -1.0 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 5.2% | 8.4% | +3.2% |
| Overfitting | 0.05 | 0.00 | -0.05 |
| Fragility | 0.40 | 0.03 | |

✅ **STRONG RECOMMEND** — trades/fold: 171, exits: {'trailing_stop': 377, 'stop_loss': 85, 'score_flip': 47, 'take_profit': 1, 'mr_target': 2}

### #3: SOL/USDT (Survivor: 5.36 +4.28)
```json
{
  "signal_threshold": 0.3174,
  "min_alignment": 3,
  "take_profit_atr": 6.0,
  "stop_loss_atr": 2.5,
  "max_hold_hours": 96,
  "time_decay_hours": 45,
  "trailing_stop_atr": 0.01,
  "score_flip_delay_hrs": 2
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.68 | +5.99 | +4.31 |
| OOS PF | 1.6 | 0.5 | -1.0 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 5.2% | 8.4% | +3.2% |
| Overfitting | 0.05 | 0.00 | -0.05 |
| Fragility | 0.40 | 0.03 | |

✅ **STRONG RECOMMEND** — trades/fold: 171, exits: {'trailing_stop': 377, 'stop_loss': 85, 'score_flip': 47, 'take_profit': 1, 'mr_target': 2}

### #4: SOL/USDT (Survivor: 5.35 +4.27)
```json
{
  "signal_threshold": 0.3174,
  "min_alignment": 3,
  "take_profit_atr": 6.0,
  "stop_loss_atr": 2.5,
  "max_hold_hours": 96,
  "time_decay_hours": 50,
  "trailing_stop_atr": 0.0114,
  "score_flip_delay_hrs": 2
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.68 | +5.98 | +4.30 |
| OOS PF | 1.6 | 0.5 | -1.0 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 5.2% | 8.4% | +3.2% |
| Overfitting | 0.05 | 0.00 | -0.05 |
| Fragility | 0.40 | 0.03 | |

✅ **STRONG RECOMMEND** — trades/fold: 171, exits: {'trailing_stop': 377, 'stop_loss': 85, 'score_flip': 47, 'take_profit': 1, 'mr_target': 2}

### #5: SOL/USDT (Survivor: 5.34 +4.26)
```json
{
  "signal_threshold": 0.3378,
  "min_alignment": 3,
  "take_profit_atr": 6.7348,
  "stop_loss_atr": 2.5,
  "max_hold_hours": 96,
  "time_decay_hours": 48,
  "trailing_stop_atr": 0.01,
  "score_flip_delay_hrs": 2
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.68 | +5.99 | +4.31 |
| OOS PF | 1.6 | 0.5 | -1.0 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 5.2% | 8.4% | +3.2% |
| Overfitting | 0.05 | 0.00 | -0.05 |
| Fragility | 0.40 | 0.03 | |

✅ **STRONG RECOMMEND** — trades/fold: 171, exits: {'trailing_stop': 378, 'stop_loss': 85, 'score_flip': 47, 'mr_target': 2}

### #6: SOL/USDT (Survivor: 5.34 +4.26)
```json
{
  "signal_threshold": 0.3378,
  "min_alignment": 3,
  "take_profit_atr": 6.0,
  "stop_loss_atr": 2.5,
  "max_hold_hours": 96,
  "time_decay_hours": 48,
  "trailing_stop_atr": 0.01,
  "score_flip_delay_hrs": 2
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.68 | +5.99 | +4.31 |
| OOS PF | 1.6 | 0.5 | -1.0 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 5.2% | 8.4% | +3.2% |
| Overfitting | 0.05 | 0.00 | -0.05 |
| Fragility | 0.40 | 0.03 | |

✅ **STRONG RECOMMEND** — trades/fold: 171, exits: {'trailing_stop': 377, 'stop_loss': 85, 'score_flip': 47, 'take_profit': 1, 'mr_target': 2}

### #7: SOL/USDT (Survivor: 5.34 +4.26)
```json
{
  "signal_threshold": 0.3378,
  "min_alignment": 3,
  "take_profit_atr": 6.0,
  "stop_loss_atr": 2.5,
  "max_hold_hours": 81,
  "time_decay_hours": 48,
  "trailing_stop_atr": 0.01,
  "score_flip_delay_hrs": 2
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.68 | +5.99 | +4.31 |
| OOS PF | 1.6 | 0.5 | -1.0 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 5.2% | 8.4% | +3.2% |
| Overfitting | 0.05 | 0.00 | -0.05 |
| Fragility | 0.40 | 0.03 | |

✅ **STRONG RECOMMEND** — trades/fold: 171, exits: {'trailing_stop': 377, 'stop_loss': 85, 'score_flip': 47, 'take_profit': 1, 'mr_target': 2}

### #8: SOL/USDT (Survivor: 5.34 +4.26)
```json
{
  "signal_threshold": 0.3378,
  "min_alignment": 3,
  "take_profit_atr": 6.0,
  "stop_loss_atr": 2.5,
  "max_hold_hours": 83,
  "time_decay_hours": 48,
  "trailing_stop_atr": 0.01,
  "score_flip_delay_hrs": 2
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.68 | +5.99 | +4.31 |
| OOS PF | 1.6 | 0.5 | -1.0 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 5.2% | 8.4% | +3.2% |
| Overfitting | 0.05 | 0.00 | -0.05 |
| Fragility | 0.40 | 0.03 | |

✅ **STRONG RECOMMEND** — trades/fold: 171, exits: {'trailing_stop': 377, 'stop_loss': 85, 'score_flip': 47, 'take_profit': 1, 'mr_target': 2}

### #9: SOL/USDT (Survivor: 5.34 +4.26)
```json
{
  "signal_threshold": 0.343,
  "min_alignment": 3,
  "take_profit_atr": 6.0,
  "stop_loss_atr": 2.5,
  "max_hold_hours": 96,
  "time_decay_hours": 48,
  "trailing_stop_atr": 0.01,
  "score_flip_delay_hrs": 3
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.68 | +5.99 | +4.31 |
| OOS PF | 1.6 | 0.5 | -1.0 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 5.2% | 8.4% | +3.2% |
| Overfitting | 0.05 | 0.00 | -0.05 |
| Fragility | 0.40 | 0.03 | |

✅ **STRONG RECOMMEND** — trades/fold: 171, exits: {'trailing_stop': 377, 'stop_loss': 85, 'score_flip': 47, 'take_profit': 1, 'mr_target': 2}

### #10: SOL/USDT (Survivor: 5.34 +4.25)
```json
{
  "signal_threshold": 0.3356,
  "min_alignment": 3,
  "take_profit_atr": 6.0,
  "stop_loss_atr": 2.5,
  "max_hold_hours": 100,
  "time_decay_hours": 48,
  "trailing_stop_atr": 0.0107,
  "score_flip_delay_hrs": 3
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.68 | +5.98 | +4.30 |
| OOS PF | 1.6 | 0.5 | -1.0 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 5.2% | 8.4% | +3.2% |
| Overfitting | 0.05 | 0.00 | -0.05 |
| Fragility | 0.40 | 0.03 | |

✅ **STRONG RECOMMEND** — trades/fold: 171, exits: {'trailing_stop': 377, 'stop_loss': 85, 'score_flip': 47, 'take_profit': 1, 'mr_target': 2}

## Overfitting Warnings

⚠️ SOL/USDT {'rsi_oversold': 25, 'stop_loss_atr_multiplier': 1.5, 'take_profit_atr_multiplier': 2.0, 'max_hold_hours': 36, 'trend_filter_period': 50, 'min_alignment': 0, 'strategy': 'bb_mean_reversion'}: oos_consistency=0% < 50% (OOS Sharpe: +0.00, IS-OOS gap: 0.00)
⚠️ SOL/USDT {'rsi_oversold': 25, 'stop_loss_atr_multiplier': 1.5, 'take_profit_atr_multiplier': 2.0, 'max_hold_hours': 48, 'trend_filter_period': 50, 'min_alignment': 0, 'strategy': 'bb_mean_reversion'}: oos_consistency=0% < 50% (OOS Sharpe: +0.00, IS-OOS gap: 0.00)
⚠️ SOL/USDT {'rsi_oversold': 25, 'stop_loss_atr_multiplier': 1.5, 'take_profit_atr_multiplier': 2.0, 'max_hold_hours': 36, 'trend_filter_period': 100, 'min_alignment': 0, 'strategy': 'bb_mean_reversion'}: oos_consistency=33% < 50% (OOS Sharpe: +0.00, IS-OOS gap: 0.00)
⚠️ SOL/USDT {'rsi_oversold': 25, 'stop_loss_atr_multiplier': 1.5, 'take_profit_atr_multiplier': 2.0, 'max_hold_hours': 48, 'trend_filter_period': 100, 'min_alignment': 0, 'strategy': 'bb_mean_reversion'}: oos_consistency=33% < 50% (OOS Sharpe: +0.00, IS-OOS gap: 0.00)

## Per-Symbol WFA Fold Detail

### SOL/USDT — Best Validated Candidate (Survivor: 5.36)
| Fold | IS Sharpe | OOS Sharpe | OOS PnL | OOS Trades |
|------|-----------|-----------|---------|------------|
| 0 | +0.00 | +3.44 | +11.77% | 40 ✅ |
| 1 | +3.48 | +8.62 | +33.33% | 47 ✅ |
| 2 | +7.44 | +5.99 | +171.07% | 425 ✅ |

## Action Items

1. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3174, time_decay_hours: 48→50, trailing_stop_atr: 1.0→0.01
   OOS Sharpe: +5.99 (vs +1.68), consistency: 100%, DD: 8.4%, trades/fold: 171

2. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3174, time_decay_hours: 48→56, trailing_stop_atr: 1.0→0.01
   OOS Sharpe: +5.99 (vs +1.68), consistency: 100%, DD: 8.4%, trades/fold: 171

3. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3174, time_decay_hours: 48→45, trailing_stop_atr: 1.0→0.01
   OOS Sharpe: +5.99 (vs +1.68), consistency: 100%, DD: 8.4%, trades/fold: 171

4. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3174, time_decay_hours: 48→50, trailing_stop_atr: 1.0→0.0114
   OOS Sharpe: +5.98 (vs +1.68), consistency: 100%, DD: 8.4%, trades/fold: 171

5. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3378, take_profit_atr: 6.0→6.7348, trailing_stop_atr: 1.0→0.01
   OOS Sharpe: +5.99 (vs +1.68), consistency: 100%, DD: 8.4%, trades/fold: 171

Total: 5 actionable recommendations out of 291 validated candidates.
