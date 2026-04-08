# Night Shift Report — 2026-04-08

**Runtime:** 2118s | **Folds:** 3 | **Symbols:** SOL/USDT
**Aggregation:** Median OOS Sharpe, per-fold Sharpe winsorized at ±100

## Market State

| Symbol | Regime | ADX | ADX Trend | Vol %ile | 30d Return |
|--------|--------|-----|-----------|----------|------------|
| SOL/USDT | TREND | 40.6 | FALLING | 71% | +1.4% |

**Correlations:**

## Production Baseline (Current Config)

| Symbol | OOS Sharpe | OOS PF | OOS WR | Consistency | MaxDD | Survivor |
|--------|-----------|--------|--------|-------------|-------|----------|
| SOL/USDT | +1.85 | 1.3 | 61% | 100% | 4.3% | 0.56 |

## Top 10 Candidates (Ranked by Survivor Score)

*Only candidates validated on 5+ WFA folds are shown.*

**Strategy breakdown:** 1626 MultiTF, 216 BB Mean Reversion

### #1: SOL/USDT (Survivor: 5.11 +4.54)
```json
{
  "signal_threshold": 0.35,
  "min_alignment": 3,
  "take_profit_atr": 6.0,
  "stop_loss_atr": 2.5,
  "max_hold_hours": 96,
  "time_decay_hours": 48,
  "trailing_stop_atr": 1.0,
  "score_flip_delay_hrs": 2,
  "experiment": "lower_threshold"
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.85 | +5.43 | +3.59 |
| OOS PF | 1.3 | 1.2 | -0.2 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 4.3% | 6.4% | +2.1% |
| Overfitting | 0.54 | 0.00 | -0.54 |
| Fragility | 0.44 | 0.00 | |

✅ **STRONG RECOMMEND** — trades/fold: 103, exits: {'trailing_stop': 199, 'score_flip': 46, 'stop_loss': 52, 'take_profit': 6, 'mr_target': 6}

### #2: SOL/USDT (Survivor: 4.34 +3.78)
```json
{
  "signal_threshold": 0.3,
  "take_profit_atr": 2.1698,
  "stop_loss_atr": 1.25,
  "max_hold_hours": 72,
  "time_decay_hours": 12,
  "min_alignment": 3,
  "trailing_stop_atr": 0.5,
  "score_flip_delay_hrs": 1
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.85 | +7.03 | +5.19 |
| OOS PF | 1.3 | 1.3 | -0.1 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 4.3% | 7.0% | +2.7% |
| Overfitting | 0.54 | 0.00 | -0.54 |
| Fragility | 0.44 | 0.51 | |

✅ **STRONG RECOMMEND** — trades/fold: 171, exits: {'trailing_stop': 235, 'stop_loss': 135, 'take_profit': 112, 'mr_target': 2, 'score_flip': 28}

### #3: SOL/USDT (Survivor: 4.28 +3.72)
```json
{
  "signal_threshold": 0.3,
  "take_profit_atr": 2.3276,
  "stop_loss_atr": 1.25,
  "max_hold_hours": 72,
  "time_decay_hours": 12,
  "min_alignment": 3,
  "trailing_stop_atr": 0.5,
  "score_flip_delay_hrs": 0
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.85 | +7.28 | +5.43 |
| OOS PF | 1.3 | 1.2 | -0.1 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 4.3% | 7.0% | +2.7% |
| Overfitting | 0.54 | 0.00 | -0.54 |
| Fragility | 0.44 | 0.59 | |

✅ **STRONG RECOMMEND** — trades/fold: 170, exits: {'trailing_stop': 240, 'stop_loss': 136, 'take_profit': 102, 'mr_target': 2, 'score_flip': 29}

### #4: SOL/USDT (Survivor: 4.28 +3.72)
```json
{
  "signal_threshold": 0.3,
  "take_profit_atr": 2.3276,
  "stop_loss_atr": 1.25,
  "max_hold_hours": 72,
  "time_decay_hours": 10,
  "min_alignment": 3,
  "trailing_stop_atr": 0.5,
  "score_flip_delay_hrs": 0
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.85 | +7.28 | +5.43 |
| OOS PF | 1.3 | 1.2 | -0.1 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 4.3% | 7.0% | +2.7% |
| Overfitting | 0.54 | 0.00 | -0.54 |
| Fragility | 0.44 | 0.59 | |

✅ **STRONG RECOMMEND** — trades/fold: 170, exits: {'trailing_stop': 240, 'stop_loss': 136, 'take_profit': 102, 'mr_target': 2, 'score_flip': 29}

### #5: SOL/USDT (Survivor: 4.28 +3.72)
```json
{
  "signal_threshold": 0.3,
  "take_profit_atr": 2.3276,
  "stop_loss_atr": 1.25,
  "max_hold_hours": 67,
  "time_decay_hours": 12,
  "min_alignment": 3,
  "trailing_stop_atr": 0.5,
  "score_flip_delay_hrs": 0
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.85 | +7.28 | +5.43 |
| OOS PF | 1.3 | 1.2 | -0.1 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 4.3% | 7.0% | +2.7% |
| Overfitting | 0.54 | 0.00 | -0.54 |
| Fragility | 0.44 | 0.59 | |

✅ **STRONG RECOMMEND** — trades/fold: 170, exits: {'trailing_stop': 240, 'stop_loss': 136, 'take_profit': 102, 'mr_target': 2, 'score_flip': 29}

### #6: SOL/USDT (Survivor: 4.28 +3.72)
```json
{
  "signal_threshold": 0.3,
  "take_profit_atr": 2.3276,
  "stop_loss_atr": 1.25,
  "max_hold_hours": 72,
  "time_decay_hours": 12,
  "min_alignment": 3,
  "trailing_stop_atr": 0.5,
  "score_flip_delay_hrs": 1
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.85 | +7.28 | +5.43 |
| OOS PF | 1.3 | 1.2 | -0.1 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 4.3% | 7.0% | +2.7% |
| Overfitting | 0.54 | 0.00 | -0.54 |
| Fragility | 0.44 | 0.59 | |

✅ **STRONG RECOMMEND** — trades/fold: 170, exits: {'trailing_stop': 240, 'stop_loss': 136, 'take_profit': 102, 'mr_target': 2, 'score_flip': 29}

### #7: SOL/USDT (Survivor: 4.28 +3.72)
```json
{
  "signal_threshold": 0.3,
  "take_profit_atr": 2.3663,
  "stop_loss_atr": 1.25,
  "max_hold_hours": 36,
  "time_decay_hours": 22,
  "min_alignment": 3,
  "trailing_stop_atr": 0.5,
  "score_flip_delay_hrs": 1
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.85 | +7.28 | +5.43 |
| OOS PF | 1.3 | 1.2 | -0.1 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 4.3% | 7.0% | +2.7% |
| Overfitting | 0.54 | 0.00 | -0.54 |
| Fragility | 0.44 | 0.59 | |

✅ **STRONG RECOMMEND** — trades/fold: 170, exits: {'trailing_stop': 243, 'stop_loss': 136, 'take_profit': 97, 'mr_target': 2, 'score_flip': 31}

### #8: SOL/USDT (Survivor: 4.28 +3.72)
```json
{
  "signal_threshold": 0.3,
  "take_profit_atr": 2.3729,
  "stop_loss_atr": 1.25,
  "max_hold_hours": 72,
  "time_decay_hours": 12,
  "min_alignment": 3,
  "trailing_stop_atr": 0.5,
  "score_flip_delay_hrs": 1
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.85 | +7.28 | +5.43 |
| OOS PF | 1.3 | 1.2 | -0.1 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 4.3% | 7.0% | +2.7% |
| Overfitting | 0.54 | 0.00 | -0.54 |
| Fragility | 0.44 | 0.59 | |

✅ **STRONG RECOMMEND** — trades/fold: 170, exits: {'trailing_stop': 243, 'stop_loss': 136, 'take_profit': 97, 'mr_target': 2, 'score_flip': 31}

### #9: SOL/USDT (Survivor: 4.28 +3.72)
```json
{
  "signal_threshold": 0.3249,
  "take_profit_atr": 2.3663,
  "stop_loss_atr": 1.25,
  "max_hold_hours": 36,
  "time_decay_hours": 22,
  "min_alignment": 3,
  "trailing_stop_atr": 0.5,
  "score_flip_delay_hrs": 1
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.85 | +7.28 | +5.43 |
| OOS PF | 1.3 | 1.2 | -0.1 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 4.3% | 7.0% | +2.7% |
| Overfitting | 0.54 | 0.00 | -0.54 |
| Fragility | 0.44 | 0.59 | |

✅ **STRONG RECOMMEND** — trades/fold: 170, exits: {'trailing_stop': 243, 'stop_loss': 136, 'take_profit': 97, 'mr_target': 2, 'score_flip': 31}

### #10: SOL/USDT (Survivor: 4.28 +3.72)
```json
{
  "signal_threshold": 0.3422,
  "take_profit_atr": 2.3663,
  "stop_loss_atr": 1.25,
  "max_hold_hours": 36,
  "time_decay_hours": 22,
  "min_alignment": 3,
  "trailing_stop_atr": 0.5,
  "score_flip_delay_hrs": 1
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.85 | +7.28 | +5.43 |
| OOS PF | 1.3 | 1.2 | -0.1 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 4.3% | 7.0% | +2.7% |
| Overfitting | 0.54 | 0.00 | -0.54 |
| Fragility | 0.44 | 0.59 | |

✅ **STRONG RECOMMEND** — trades/fold: 170, exits: {'trailing_stop': 243, 'stop_loss': 136, 'take_profit': 97, 'mr_target': 2, 'score_flip': 31}

## Overfitting Warnings

⚠️ SOL/USDT {'signal_threshold': 0.3, 'take_profit_atr': 3.0, 'stop_loss_atr': 1.25, 'max_hold_hours': 36, 'time_decay_hours': 24, 'min_alignment': 3, 'trailing_stop_atr': 1.5, 'score_flip_delay_hrs': 0}: overfitting_score=0.52 > 0.5 (OOS Sharpe: +1.67, IS-OOS gap: 0.52)
⚠️ SOL/USDT {'signal_threshold': 0.3, 'take_profit_atr': 3.0, 'stop_loss_atr': 1.25, 'max_hold_hours': 36, 'time_decay_hours': 24, 'min_alignment': 3, 'trailing_stop_atr': 1.5, 'score_flip_delay_hrs': 1}: overfitting_score=0.52 > 0.5 (OOS Sharpe: +1.67, IS-OOS gap: 0.52)

## Per-Symbol WFA Fold Detail

### SOL/USDT — Best Validated Candidate (Survivor: 5.11)
| Fold | IS Sharpe | OOS Sharpe | OOS PnL | OOS Trades |
|------|-----------|-----------|---------|------------|
| 0 | +0.00 | +6.42 | +24.93% | 32 ✅ |
| 1 | +0.00 | +5.43 | +5.42% | 17 ✅ |
| 2 | +0.00 | +1.85 | +43.12% | 260 ✅ |

## Action Items

1. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.35, experiment: None→lower_threshold
   OOS Sharpe: +5.43 (vs +1.85), consistency: 100%, DD: 6.4%, trades/fold: 103

2. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3, take_profit_atr: 6.0→2.1698, stop_loss_atr: 2.5→1.25, max_hold_hours: 96→72, time_decay_hours: 48→12, trailing_stop_atr: 1.0→0.5, score_flip_delay_hrs: 2→1
   OOS Sharpe: +7.03 (vs +1.85), consistency: 100%, DD: 7.0%, trades/fold: 171

3. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3, take_profit_atr: 6.0→2.3276, stop_loss_atr: 2.5→1.25, max_hold_hours: 96→72, time_decay_hours: 48→12, trailing_stop_atr: 1.0→0.5, score_flip_delay_hrs: 2→0
   OOS Sharpe: +7.28 (vs +1.85), consistency: 100%, DD: 7.0%, trades/fold: 170

4. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3, take_profit_atr: 6.0→2.3276, stop_loss_atr: 2.5→1.25, max_hold_hours: 96→72, time_decay_hours: 48→10, trailing_stop_atr: 1.0→0.5, score_flip_delay_hrs: 2→0
   OOS Sharpe: +7.28 (vs +1.85), consistency: 100%, DD: 7.0%, trades/fold: 170

5. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3, take_profit_atr: 6.0→2.3276, stop_loss_atr: 2.5→1.25, max_hold_hours: 96→67, time_decay_hours: 48→12, trailing_stop_atr: 1.0→0.5, score_flip_delay_hrs: 2→0
   OOS Sharpe: +7.28 (vs +1.85), consistency: 100%, DD: 7.0%, trades/fold: 170

Total: 5 actionable recommendations out of 1386 validated candidates.
