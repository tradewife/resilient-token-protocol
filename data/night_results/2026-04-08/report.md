# Night Shift Report — 2026-04-08

**Runtime:** 3812s | **Folds:** 3 | **Symbols:** BTC/USDT, ETH/USDT, SOL/USDT
**Aggregation:** Median OOS Sharpe, per-fold Sharpe winsorized at ±100

## Market State

| Symbol | Regime | ADX | ADX Trend | Vol %ile | 30d Return |
|--------|--------|-----|-----------|----------|------------|
| BTC/USDT | TREND | 34.2 | STABLE | 47% | +1.1% |
| ETH/USDT | TREND | 25.1 | STABLE | 31% | +6.8% |
| SOL/USDT | RANGE | 15.4 | STABLE | 23% | -3.5% |

**Correlations:**
  BTC/USDT_ETH/USDT: 0.91
  BTC/USDT_SOL/USDT: 0.86
  ETH/USDT_SOL/USDT: 0.88

## Production Baseline (Current Config)

| Symbol | OOS Sharpe | OOS PF | OOS WR | Consistency | MaxDD | Survivor |
|--------|-----------|--------|--------|-------------|-------|----------|
| BTC/USDT | +1.31 | 1.2 | 56% | 67% | 5.8% | 0.38 |
| ETH/USDT | +1.38 | 1.1 | 58% | 67% | 12.2% | 0.30 |
| SOL/USDT | +1.68 | 1.6 | 57% | 100% | 5.2% | 1.08 |

## Top 10 Candidates (Ranked by Survivor Score)

*Only candidates validated on 5+ WFA folds are shown.*

**Strategy breakdown:** 1838 MultiTF, 648 BB Mean Reversion

### #1: SOL/USDT (Survivor: 5.34 +4.26)
```json
{
  "signal_threshold": 0.3282,
  "min_alignment": 3,
  "take_profit_atr": 6.0,
  "stop_loss_atr": 2.5,
  "max_hold_hours": 96,
  "time_decay_hours": 48,
  "trailing_stop_atr": 0.01,
  "score_flip_delay_hrs": 1
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

### #2: SOL/USDT (Survivor: 5.33 +4.25)
```json
{
  "signal_threshold": 0.3343,
  "min_alignment": 3,
  "take_profit_atr": 6.0,
  "stop_loss_atr": 2.5,
  "max_hold_hours": 96,
  "time_decay_hours": 48,
  "trailing_stop_atr": 0.0108,
  "score_flip_delay_hrs": 1
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

### #3: SOL/USDT (Survivor: 5.12 +4.04)
```json
{
  "signal_threshold": 0.3578,
  "min_alignment": 3,
  "take_profit_atr": 5.1314,
  "stop_loss_atr": 2.3275,
  "max_hold_hours": 96,
  "time_decay_hours": 48,
  "trailing_stop_atr": 0.01,
  "score_flip_delay_hrs": 3
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.68 | +5.69 | +4.01 |
| OOS PF | 1.6 | 0.5 | -1.0 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 5.2% | 6.4% | +1.2% |
| Overfitting | 0.05 | 0.00 | -0.05 |
| Fragility | 0.40 | 0.04 | |

✅ **STRONG RECOMMEND** — trades/fold: 149, exits: {'trailing_stop': 329, 'stop_loss': 77, 'score_flip': 33, 'take_profit': 6, 'mr_target': 2}

### #4: SOL/USDT (Survivor: 5.01 +3.93)
```json
{
  "signal_threshold": 0.3578,
  "min_alignment": 3,
  "take_profit_atr": 5.1314,
  "stop_loss_atr": 2.5,
  "max_hold_hours": 96,
  "time_decay_hours": 48,
  "trailing_stop_atr": 0.01,
  "score_flip_delay_hrs": 3
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.68 | +5.78 | +4.10 |
| OOS PF | 1.6 | 0.5 | -1.1 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 5.2% | 6.4% | +1.2% |
| Overfitting | 0.05 | 0.00 | -0.05 |
| Fragility | 0.40 | 0.08 | |

✅ **STRONG RECOMMEND** — trades/fold: 148, exits: {'trailing_stop': 330, 'stop_loss': 73, 'score_flip': 33, 'take_profit': 6, 'mr_target': 2}

### #5: SOL/USDT (Survivor: 5.01 +3.93)
```json
{
  "signal_threshold": 0.3578,
  "min_alignment": 3,
  "take_profit_atr": 5.4069,
  "stop_loss_atr": 2.5,
  "max_hold_hours": 96,
  "time_decay_hours": 48,
  "trailing_stop_atr": 0.01,
  "score_flip_delay_hrs": 3
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.68 | +5.78 | +4.10 |
| OOS PF | 1.6 | 0.5 | -1.1 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 5.2% | 6.4% | +1.2% |
| Overfitting | 0.05 | 0.00 | -0.05 |
| Fragility | 0.40 | 0.08 | |

✅ **STRONG RECOMMEND** — trades/fold: 148, exits: {'trailing_stop': 331, 'stop_loss': 73, 'score_flip': 33, 'take_profit': 5, 'mr_target': 2}

### #6: SOL/USDT (Survivor: 5.01 +3.92)
```json
{
  "signal_threshold": 0.3578,
  "min_alignment": 3,
  "take_profit_atr": 5.1314,
  "stop_loss_atr": 2.5,
  "max_hold_hours": 96,
  "time_decay_hours": 48,
  "trailing_stop_atr": 0.0111,
  "score_flip_delay_hrs": 3
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.68 | +5.78 | +4.09 |
| OOS PF | 1.6 | 0.5 | -1.1 |
| Consistency | 100% | 100% | +0% |
| MaxDD | 5.2% | 6.4% | +1.2% |
| Overfitting | 0.05 | 0.00 | -0.05 |
| Fragility | 0.40 | 0.08 | |

✅ **STRONG RECOMMEND** — trades/fold: 148, exits: {'trailing_stop': 330, 'stop_loss': 73, 'score_flip': 33, 'take_profit': 6, 'mr_target': 2}

### #7: BTC/USDT (Survivor: 4.30 +3.93)
```json
{
  "signal_threshold": 0.3614,
  "min_alignment": 3,
  "take_profit_atr": 6.0,
  "stop_loss_atr": 2.3425,
  "max_hold_hours": 96,
  "time_decay_hours": 48,
  "trailing_stop_atr": 0.01,
  "score_flip_delay_hrs": 2
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.31 | +4.75 | +3.43 |
| OOS PF | 1.2 | 0.6 | -0.6 |
| Consistency | 67% | 100% | +33% |
| MaxDD | 5.8% | 3.9% | -2.0% |
| Overfitting | 0.00 | 0.00 | +0.00 |
| Fragility | 1.21 | 0.06 | |

✅ **STRONG RECOMMEND** — trades/fold: 165, exits: {'trailing_stop': 352, 'stop_loss': 84, 'score_flip': 51, 'take_profit': 6, 'mr_target': 1}

### #8: BTC/USDT (Survivor: 4.29 +3.91)
```json
{
  "signal_threshold": 0.3614,
  "min_alignment": 3,
  "take_profit_atr": 6.0,
  "stop_loss_atr": 2.3466,
  "max_hold_hours": 109,
  "time_decay_hours": 48,
  "trailing_stop_atr": 0.01,
  "score_flip_delay_hrs": 2
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.31 | +4.75 | +3.43 |
| OOS PF | 1.2 | 0.6 | -0.6 |
| Consistency | 67% | 100% | +33% |
| MaxDD | 5.8% | 3.9% | -2.0% |
| Overfitting | 0.00 | 0.00 | +0.00 |
| Fragility | 1.21 | 0.07 | |

✅ **STRONG RECOMMEND** — trades/fold: 165, exits: {'trailing_stop': 352, 'stop_loss': 84, 'score_flip': 51, 'take_profit': 6, 'mr_target': 1}

### #9: BTC/USDT (Survivor: 4.16 +3.79)
```json
{
  "signal_threshold": 0.3614,
  "min_alignment": 3,
  "take_profit_atr": 6.8345,
  "stop_loss_atr": 2.5,
  "max_hold_hours": 96,
  "time_decay_hours": 48,
  "trailing_stop_atr": 0.01,
  "score_flip_delay_hrs": 2
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.31 | +4.59 | +3.28 |
| OOS PF | 1.2 | 0.5 | -0.7 |
| Consistency | 67% | 100% | +33% |
| MaxDD | 5.8% | 4.0% | -1.9% |
| Overfitting | 0.00 | 0.00 | +0.00 |
| Fragility | 1.21 | 0.06 | |

✅ **STRONG RECOMMEND** — trades/fold: 163, exits: {'trailing_stop': 357, 'stop_loss': 76, 'score_flip': 53, 'take_profit': 2, 'mr_target': 1}

### #10: BTC/USDT (Survivor: 4.16 +3.79)
```json
{
  "signal_threshold": 0.3614,
  "min_alignment": 3,
  "take_profit_atr": 6.7231,
  "stop_loss_atr": 2.5,
  "max_hold_hours": 96,
  "time_decay_hours": 48,
  "trailing_stop_atr": 0.01,
  "score_flip_delay_hrs": 2
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.31 | +4.59 | +3.28 |
| OOS PF | 1.2 | 0.5 | -0.7 |
| Consistency | 67% | 100% | +33% |
| MaxDD | 5.8% | 4.0% | -1.9% |
| Overfitting | 0.00 | 0.00 | +0.00 |
| Fragility | 1.21 | 0.06 | |

✅ **STRONG RECOMMEND** — trades/fold: 163, exits: {'trailing_stop': 357, 'stop_loss': 76, 'score_flip': 53, 'take_profit': 2, 'mr_target': 1}

## Overfitting Warnings

⚠️ BTC/USDT {'rsi_oversold': 25, 'stop_loss_atr_multiplier': 1.5, 'take_profit_atr_multiplier': 2.0, 'max_hold_hours': 36, 'trend_filter_period': 50, 'min_alignment': 0, 'strategy': 'bb_mean_reversion'}: oos_consistency=0% < 50% (OOS Sharpe: +0.00, IS-OOS gap: 0.00)
⚠️ BTC/USDT {'rsi_oversold': 25, 'stop_loss_atr_multiplier': 1.5, 'take_profit_atr_multiplier': 2.0, 'max_hold_hours': 48, 'trend_filter_period': 50, 'min_alignment': 0, 'strategy': 'bb_mean_reversion'}: oos_consistency=0% < 50% (OOS Sharpe: +0.00, IS-OOS gap: 0.00)
⚠️ BTC/USDT {'rsi_oversold': 25, 'stop_loss_atr_multiplier': 1.5, 'take_profit_atr_multiplier': 2.0, 'max_hold_hours': 36, 'trend_filter_period': 100, 'min_alignment': 0, 'strategy': 'bb_mean_reversion'}: oos_consistency=33% < 50% (OOS Sharpe: +0.00, IS-OOS gap: 0.00)
⚠️ BTC/USDT {'rsi_oversold': 25, 'stop_loss_atr_multiplier': 1.5, 'take_profit_atr_multiplier': 2.0, 'max_hold_hours': 48, 'trend_filter_period': 100, 'min_alignment': 0, 'strategy': 'bb_mean_reversion'}: oos_consistency=33% < 50% (OOS Sharpe: +0.00, IS-OOS gap: 0.00)

## Per-Symbol WFA Fold Detail

### BTC/USDT — Best Validated Candidate (Survivor: 4.30)
| Fold | IS Sharpe | OOS Sharpe | OOS PnL | OOS Trades |
|------|-----------|-----------|---------|------------|
| 0 | +0.00 | +3.58 | +7.36% | 40 ✅ |
| 1 | +2.29 | +9.93 | +16.53% | 66 ✅ |
| 2 | +5.78 | +4.75 | +68.16% | 388 ✅ |

### ETH/USDT — Best Validated Candidate (Survivor: 1.82)
| Fold | IS Sharpe | OOS Sharpe | OOS PnL | OOS Trades |
|------|-----------|-----------|---------|------------|
| 0 | +0.00 | +1.99 | +3.96% | 6 ✅ |
| 1 | +0.00 | +3.00 | +8.13% | 14 ✅ |
| 2 | +0.00 | +0.26 | +3.95% | 102 ✅ |

### SOL/USDT — Best Validated Candidate (Survivor: 5.34)
| Fold | IS Sharpe | OOS Sharpe | OOS PnL | OOS Trades |
|------|-----------|-----------|---------|------------|
| 0 | +0.00 | +3.44 | +11.77% | 40 ✅ |
| 1 | +3.48 | +8.62 | +33.33% | 47 ✅ |
| 2 | +7.44 | +5.99 | +171.07% | 425 ✅ |

## Action Items

1. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3282, trailing_stop_atr: 1.0→0.01, score_flip_delay_hrs: 2→1
   OOS Sharpe: +5.99 (vs +1.68), consistency: 100%, DD: 8.4%, trades/fold: 171

2. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3343, trailing_stop_atr: 1.0→0.0108, score_flip_delay_hrs: 2→1
   OOS Sharpe: +5.98 (vs +1.68), consistency: 100%, DD: 8.4%, trades/fold: 171

3. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3578, take_profit_atr: 6.0→5.1314, stop_loss_atr: 2.5→2.3275, trailing_stop_atr: 1.0→0.01, score_flip_delay_hrs: 2→3
   OOS Sharpe: +5.69 (vs +1.68), consistency: 100%, DD: 6.4%, trades/fold: 149

4. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3578, take_profit_atr: 6.0→5.1314, trailing_stop_atr: 1.0→0.01, score_flip_delay_hrs: 2→3
   OOS Sharpe: +5.78 (vs +1.68), consistency: 100%, DD: 6.4%, trades/fold: 148

5. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3578, take_profit_atr: 6.0→5.4069, trailing_stop_atr: 1.0→0.01, score_flip_delay_hrs: 2→3
   OOS Sharpe: +5.78 (vs +1.68), consistency: 100%, DD: 6.4%, trades/fold: 148

Total: 5 actionable recommendations out of 1763 validated candidates.
