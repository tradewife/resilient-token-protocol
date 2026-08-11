# Night Shift Report — 2026-06-24

**Runtime:** 25225s | **Folds:** 9 | **Symbols:** BTC/USDT, ETH/USDT, SOL/USDT, BNB/USDT
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

**Strategy breakdown:** 48224 MultiTF, 864 BB Mean Reversion

### #1: SOL/USDT (Survivor: 2.24 +2.02)
```json
{
  "signal_threshold": 0.3,
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
| OOS Sharpe | +2.05 | +2.66 | +0.61 |
| OOS PF | 1.3 | 1.4 | +0.1 |
| Consistency | 67% | 89% | +22% |
| MaxDD | 3.0% | 5.3% | +2.2% |
| Overfitting | 0.33 | 0.00 | -0.33 |
| Fragility | 2.89 | 0.00 | |

✅ **STRONG RECOMMEND** — trades/fold: 36, exits: {'trailing_stop': 195, 'score_flip': 60, 'stop_loss': 49, 'mr_target': 11, 'take_profit': 5}

### #2: BTC/USDT (Survivor: 1.63 +1.51)
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
| OOS Sharpe | +0.95 | +1.89 | +0.95 |
| OOS PF | 2.0 | 1.3 | -0.7 |
| Consistency | 67% | 89% | +22% |
| MaxDD | 2.7% | 3.5% | +0.8% |
| Overfitting | 0.59 | 0.00 | -0.59 |
| Fragility | 1.13 | 0.00 | |

✅ **STRONG RECOMMEND** — trades/fold: 33, exits: {'trailing_stop': 181, 'score_flip': 57, 'stop_loss': 48, 'mr_target': 2, 'take_profit': 6}

### #3: SOL/USDT (Survivor: 1.62 +1.40)
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
| OOS Sharpe | +2.05 | +2.19 | +0.14 |
| OOS PF | 1.3 | 1.3 | -0.0 |
| Consistency | 67% | 78% | +11% |
| MaxDD | 3.0% | 4.9% | +1.9% |
| Overfitting | 0.33 | 0.00 | -0.33 |
| Fragility | 2.89 | 0.00 | |

✅ **STRONG RECOMMEND** — trades/fold: 30, exits: {'trailing_stop': 169, 'score_flip': 43, 'stop_loss': 44, 'take_profit': 6, 'mr_target': 6}

### #4: BNB/USDT (Survivor: 1.61 +1.50)
```json
{
  "signal_threshold": 0.4,
  "min_alignment": 3,
  "take_profit_atr": 3.5,
  "stop_loss_atr": 1.25,
  "max_hold_hours": 96,
  "time_decay_hours": 48,
  "trailing_stop_atr": 1.0,
  "score_flip_delay_hrs": 2,
  "experiment": "tighter_stops_wider_tp"
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.47 | +2.98 | +1.51 |
| OOS PF | 1.1 | 1.3 | +0.1 |
| Consistency | 56% | 56% | +0% |
| MaxDD | 3.6% | 2.4% | -1.2% |
| Overfitting | 0.15 | 0.00 | -0.15 |
| Fragility | 4.98 | 0.00 | |

✅ **STRONG RECOMMEND** — trades/fold: 16, exits: {'stop_loss': 36, 'score_flip': 12, 'take_profit': 18, 'trailing_stop': 78, 'mr_target': 1}

### #5: BTC/USDT (Survivor: 1.54 +1.42)
```json
{
  "signal_threshold": 0.3,
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
| OOS Sharpe | +0.95 | +2.06 | +1.11 |
| OOS PF | 2.0 | 1.2 | -0.8 |
| Consistency | 67% | 78% | +11% |
| MaxDD | 2.7% | 4.2% | +1.5% |
| Overfitting | 0.59 | 0.00 | -0.59 |
| Fragility | 1.13 | 0.00 | |

✅ **STRONG RECOMMEND** — trades/fold: 37, exits: {'trailing_stop': 206, 'score_flip': 62, 'stop_loss': 55, 'mr_target': 5, 'take_profit': 7}

### #6: BNB/USDT (Survivor: 1.53 +1.42)
```json
{
  "signal_threshold": 0.4,
  "min_alignment": 3,
  "take_profit_atr": 4.0,
  "stop_loss_atr": 1.25,
  "max_hold_hours": 96,
  "time_decay_hours": 48,
  "trailing_stop_atr": 1.0,
  "score_flip_delay_hrs": 2,
  "experiment": "tighter_stops_wider_tp"
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +1.47 | +2.83 | +1.36 |
| OOS PF | 1.1 | 1.3 | +0.2 |
| Consistency | 56% | 56% | +0% |
| MaxDD | 3.6% | 2.5% | -1.1% |
| Overfitting | 0.15 | 0.00 | -0.15 |
| Fragility | 4.98 | 0.00 | |

✅ **STRONG RECOMMEND** — trades/fold: 16, exits: {'stop_loss': 37, 'score_flip': 14, 'take_profit': 13, 'trailing_stop': 80, 'mr_target': 1}

### #7: BNB/USDT (Survivor: 1.43 +1.32)
```json
{
  "signal_threshold": 0.3,
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
| OOS Sharpe | +1.47 | +2.24 | +0.77 |
| OOS PF | 1.1 | 1.3 | +0.2 |
| Consistency | 56% | 67% | +11% |
| MaxDD | 3.6% | 4.3% | +0.7% |
| Overfitting | 0.15 | 0.00 | -0.15 |
| Fragility | 4.98 | 0.00 | |

✅ **STRONG RECOMMEND** — trades/fold: 37, exits: {'stop_loss': 58, 'score_flip': 59, 'trailing_stop': 203, 'take_profit': 5, 'mr_target': 10}

### #8: BNB/USDT (Survivor: 1.35 +1.23)
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
| OOS Sharpe | +1.47 | +2.13 | +0.66 |
| OOS PF | 1.1 | 1.2 | +0.1 |
| Consistency | 56% | 67% | +11% |
| MaxDD | 3.6% | 5.3% | +1.7% |
| Overfitting | 0.15 | 0.00 | -0.15 |
| Fragility | 4.98 | 0.00 | |

✅ **STRONG RECOMMEND** — trades/fold: 33, exits: {'stop_loss': 52, 'trailing_stop': 192, 'score_flip': 46, 'take_profit': 5, 'mr_target': 4}

### #9: SOL/USDT (Survivor: 1.33 +1.10)
```json
{
  "signal_threshold": 0.4,
  "min_alignment": 3,
  "take_profit_atr": 6.0,
  "stop_loss_atr": 2.5,
  "max_hold_hours": 24,
  "time_decay_hours": 24,
  "trailing_stop_atr": 1.0,
  "score_flip_delay_hrs": 2,
  "experiment": "shorter_holds"
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +2.05 | +2.05 | +0.00 |
| OOS PF | 1.3 | 1.3 | +0.0 |
| Consistency | 67% | 67% | +0% |
| MaxDD | 3.0% | 3.0% | +0.0% |
| Overfitting | 0.33 | 0.00 | -0.33 |
| Fragility | 2.89 | 0.00 | |

✅ **STRONG RECOMMEND** — trades/fold: 14, exits: {'trailing_stop': 81, 'take_profit': 4, 'mr_target': 7, 'score_flip': 19, 'stop_loss': 19}

### #10: SOL/USDT (Survivor: 1.33 +1.10)
```json
{
  "signal_threshold": 0.4,
  "min_alignment": 3,
  "take_profit_atr": 6.0,
  "stop_loss_atr": 2.5,
  "max_hold_hours": 24,
  "time_decay_hours": 36,
  "trailing_stop_atr": 1.0,
  "score_flip_delay_hrs": 2,
  "experiment": "shorter_holds"
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +2.05 | +2.05 | +0.00 |
| OOS PF | 1.3 | 1.3 | +0.0 |
| Consistency | 67% | 67% | +0% |
| MaxDD | 3.0% | 3.0% | +0.0% |
| Overfitting | 0.33 | 0.00 | -0.33 |
| Fragility | 2.89 | 0.00 | |

✅ **STRONG RECOMMEND** — trades/fold: 14, exits: {'trailing_stop': 81, 'take_profit': 4, 'mr_target': 7, 'score_flip': 19, 'stop_loss': 19}

## Overfitting Warnings

⚠️ SOL/USDT {'signal_threshold': 0.3, 'take_profit_atr': 3.0, 'stop_loss_atr': 1.25, 'max_hold_hours': 36, 'time_decay_hours': 12, 'min_alignment': 3, 'trailing_stop_atr': 0.5, 'score_flip_delay_hrs': 0, 'leverage': 3.0}: overfitting_score=0.62 > 0.5 (OOS Sharpe: +1.71, IS-OOS gap: 0.62)
⚠️ SOL/USDT {'signal_threshold': 0.3, 'take_profit_atr': 3.0, 'stop_loss_atr': 1.25, 'max_hold_hours': 36, 'time_decay_hours': 12, 'min_alignment': 3, 'trailing_stop_atr': 0.5, 'score_flip_delay_hrs': 1, 'leverage': 3.0}: overfitting_score=0.62 > 0.5 (OOS Sharpe: +1.71, IS-OOS gap: 0.62)

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
| 0 | +0.00 | +3.47 | +10.10% | 14 ✅ |
| 1 | +0.00 | +3.30 | +4.34% | 6 ✅ |
| 2 | +0.00 | +1.98 | +5.60% | 20 ✅ |
| 3 | +0.00 | -6.00 | -5.97% | 8 ❌ |
| 4 | +0.00 | -4.65 | -8.02% | 7 ❌ |
| 5 | +0.00 | +0.43 | +0.63% | 8 ✅ |
| 6 | +0.00 | +1.64 | +1.68% | 15 ✅ |
| 7 | +0.00 | -2.48 | -1.04% | 2 ❌ |
| 8 | +0.00 | +3.62 | +8.17% | 10 ✅ |

### SOL/USDT — Best Validated Candidate (Survivor: 2.24)
| Fold | IS Sharpe | OOS Sharpe | OOS PnL | OOS Trades |
|------|-----------|-----------|---------|------------|
| 0 | +0.00 | +6.12 | +24.98% | 45 ✅ |
| 1 | +0.00 | +1.12 | +2.76% | 31 ✅ |
| 2 | +0.00 | +2.66 | +12.48% | 46 ✅ |
| 3 | +0.00 | +7.31 | +22.79% | 47 ✅ |
| 4 | +0.00 | -1.30 | -3.29% | 34 ❌ |
| 5 | +0.00 | +0.53 | +1.53% | 24 ✅ |
| 6 | +0.00 | +1.19 | +3.49% | 49 ✅ |
| 7 | +0.00 | +3.28 | +6.64% | 15 ✅ |
| 8 | +0.00 | +4.40 | +10.96% | 29 ✅ |

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

1. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3, experiment: None→lower_threshold
   OOS Sharpe: +2.66 (vs +2.05), consistency: 89%, DD: 5.3%, trades/fold: 36

2. **[HIGH]** BTC/USDT: signal_threshold: 0.4→0.35, experiment: None→lower_threshold
   OOS Sharpe: +1.89 (vs +0.95), consistency: 89%, DD: 3.5%, trades/fold: 33

3. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.35, experiment: None→lower_threshold
   OOS Sharpe: +2.19 (vs +2.05), consistency: 78%, DD: 4.9%, trades/fold: 30

4. **[HIGH]** BNB/USDT: take_profit_atr: 6.0→3.5, stop_loss_atr: 2.5→1.25, experiment: None→tighter_stops_wider_tp
   OOS Sharpe: +2.98 (vs +1.47), consistency: 56%, DD: 2.4%, trades/fold: 16

5. **[HIGH]** BTC/USDT: signal_threshold: 0.4→0.3, experiment: None→lower_threshold
   OOS Sharpe: +2.06 (vs +0.95), consistency: 78%, DD: 4.2%, trades/fold: 37

Total: 5 actionable recommendations out of 169 validated candidates.
