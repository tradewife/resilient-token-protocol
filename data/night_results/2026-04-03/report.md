# Night Shift Report — 2026-04-03

**Runtime:** 4416s | **Folds:** 9 | **Symbols:** BTC/USDT, ETH/USDT, SOL/USDT, BNB/USDT

## Market State

| Symbol | Regime | ADX | ADX Trend | Vol %ile | 30d Return |
|--------|--------|-----|-----------|----------|------------|
| BTC/USDT | RANGE | 18.3 | STABLE | 82% | -1.7% |
| ETH/USDT | TREND | 40.3 | STABLE | 66% | +4.5% |
| SOL/USDT | RANGE | 20.4 | FALLING | 76% | -7.6% |
| BNB/USDT | RANGE | 17.4 | FALLING | 47% | -4.9% |

**Correlations:**
  BNB/USDT_BTC/USDT: 0.90
  BNB/USDT_ETH/USDT: 0.88
  BNB/USDT_SOL/USDT: 0.86
  BTC/USDT_ETH/USDT: 0.91
  BTC/USDT_SOL/USDT: 0.87
  ETH/USDT_SOL/USDT: 0.88

## Production Baseline (Current Config)

| Symbol | OOS Sharpe | OOS PF | OOS WR | Consistency | MaxDD | Survivor |
|--------|-----------|--------|--------|-------------|-------|----------|
| BTC/USDT | -87.12 | 0.3 | 14% | 0% | 4.2% | -0.00 |
| ETH/USDT | -21.37 | 0.5 | 50% | 0% | 5.2% | -0.00 |
| SOL/USDT | -50.15 | 1.4 | 17% | 0% | 4.6% | -0.00 |
| BNB/USDT | -47.66 | 0.3 | 33% | 0% | 0.9% | -0.00 |

## Top 10 Candidates (Ranked by Survivor Score)

*Only candidates validated on 5+ WFA folds are shown.*

### #1: SOL/USDT (Survivor: 16.43 +16.43)
```json
{
  "signal_threshold": 0.3136,
  "take_profit_atr": 4.0,
  "stop_loss_atr": 1.1815,
  "max_hold_hours": 36,
  "time_decay_hours": 24,
  "min_alignment": 3,
  "trailing_stop_atr": 0.6987,
  "score_flip_delay_hrs": 2
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | -50.15 | +17.17 | +67.32 |
| OOS PF | 1.4 | 1.7 | +0.3 |
| Consistency | 0% | 100% | +100% |
| MaxDD | 4.6% | 4.5% | -0.0% |
| Overfitting | 0.00 | 0.00 | +0.00 |
| Fragility | 0.00 | 0.13 | |

✅ **STRONG RECOMMEND** — trades/fold: 31, exits: {'trailing_stop': 173, 'mr_target': 8, 'score_flip': 46, 'stop_loss': 45, 'take_profit': 4}

### #2: SOL/USDT (Survivor: 16.23 +16.23)
```json
{
  "signal_threshold": 0.3626,
  "take_profit_atr": 4.0,
  "stop_loss_atr": 1.3644,
  "max_hold_hours": 72,
  "time_decay_hours": 12,
  "min_alignment": 3,
  "trailing_stop_atr": 0.7281,
  "score_flip_delay_hrs": 2
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | -50.15 | +17.00 | +67.15 |
| OOS PF | 1.4 | 1.7 | +0.4 |
| Consistency | 0% | 100% | +100% |
| MaxDD | 4.6% | 4.7% | +0.2% |
| Overfitting | 0.00 | 0.00 | +0.00 |
| Fragility | 0.00 | 0.31 | |

✅ **STRONG RECOMMEND** — trades/fold: 27, exits: {'trailing_stop': 155, 'mr_target': 8, 'score_flip': 35, 'stop_loss': 36, 'time_decay': 4}

### #3: SOL/USDT (Survivor: 16.23 +16.23)
```json
{
  "signal_threshold": 0.3626,
  "take_profit_atr": 4.0,
  "stop_loss_atr": 1.3644,
  "max_hold_hours": 77,
  "time_decay_hours": 12,
  "min_alignment": 3,
  "trailing_stop_atr": 0.7281,
  "score_flip_delay_hrs": 2
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | -50.15 | +17.00 | +67.15 |
| OOS PF | 1.4 | 1.7 | +0.4 |
| Consistency | 0% | 100% | +100% |
| MaxDD | 4.6% | 4.7% | +0.2% |
| Overfitting | 0.00 | 0.00 | +0.00 |
| Fragility | 0.00 | 0.31 | |

✅ **STRONG RECOMMEND** — trades/fold: 27, exits: {'trailing_stop': 155, 'mr_target': 8, 'score_flip': 35, 'stop_loss': 36, 'time_decay': 4}

### #4: SOL/USDT (Survivor: 16.19 +16.19)
```json
{
  "signal_threshold": 0.3626,
  "take_profit_atr": 3.6494,
  "stop_loss_atr": 1.3644,
  "max_hold_hours": 72,
  "time_decay_hours": 12,
  "min_alignment": 3,
  "trailing_stop_atr": 0.7281,
  "score_flip_delay_hrs": 2
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | -50.15 | +16.96 | +67.11 |
| OOS PF | 1.4 | 1.7 | +0.4 |
| Consistency | 0% | 100% | +100% |
| MaxDD | 4.6% | 4.7% | +0.2% |
| Overfitting | 0.00 | 0.00 | +0.00 |
| Fragility | 0.00 | 0.31 | |

✅ **STRONG RECOMMEND** — trades/fold: 27, exits: {'trailing_stop': 155, 'mr_target': 8, 'score_flip': 35, 'stop_loss': 36, 'time_decay': 4}

### #5: SOL/USDT (Survivor: 16.17 +16.17)
```json
{
  "signal_threshold": 0.3295,
  "take_profit_atr": 4.2062,
  "stop_loss_atr": 1.1628,
  "max_hold_hours": 36,
  "time_decay_hours": 24,
  "min_alignment": 3,
  "trailing_stop_atr": 0.7193,
  "score_flip_delay_hrs": 3
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | -50.15 | +16.95 | +67.10 |
| OOS PF | 1.4 | 1.8 | +0.4 |
| Consistency | 0% | 100% | +100% |
| MaxDD | 4.6% | 4.8% | +0.2% |
| Overfitting | 0.00 | 0.00 | +0.00 |
| Fragility | 0.00 | 0.14 | |

✅ **STRONG RECOMMEND** — trades/fold: 31, exits: {'trailing_stop': 169, 'mr_target': 8, 'score_flip': 48, 'stop_loss': 47, 'take_profit': 4}

### #6: SOL/USDT (Survivor: 16.12 +16.12)
```json
{
  "signal_threshold": 0.3191,
  "take_profit_atr": 4.0,
  "stop_loss_atr": 1.1337,
  "max_hold_hours": 42,
  "time_decay_hours": 36,
  "min_alignment": 3,
  "trailing_stop_atr": 0.7312,
  "score_flip_delay_hrs": 0
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | -50.15 | +16.90 | +67.05 |
| OOS PF | 1.4 | 1.8 | +0.4 |
| Consistency | 0% | 100% | +100% |
| MaxDD | 4.6% | 4.8% | +0.3% |
| Overfitting | 0.00 | 0.00 | +0.00 |
| Fragility | 0.00 | 0.17 | |

✅ **STRONG RECOMMEND** — trades/fold: 31, exits: {'trailing_stop': 168, 'mr_target': 8, 'score_flip': 46, 'stop_loss': 48, 'take_profit': 6}

### #7: SOL/USDT (Survivor: 16.12 +16.12)
```json
{
  "signal_threshold": 0.3191,
  "take_profit_atr": 4.0,
  "stop_loss_atr": 1.1337,
  "max_hold_hours": 42,
  "time_decay_hours": 36,
  "min_alignment": 3,
  "trailing_stop_atr": 0.7312,
  "score_flip_delay_hrs": 1
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | -50.15 | +16.90 | +67.05 |
| OOS PF | 1.4 | 1.8 | +0.4 |
| Consistency | 0% | 100% | +100% |
| MaxDD | 4.6% | 4.8% | +0.3% |
| Overfitting | 0.00 | 0.00 | +0.00 |
| Fragility | 0.00 | 0.17 | |

✅ **STRONG RECOMMEND** — trades/fold: 31, exits: {'trailing_stop': 168, 'mr_target': 8, 'score_flip': 46, 'stop_loss': 48, 'take_profit': 6}

### #8: SOL/USDT (Survivor: 16.10 +16.10)
```json
{
  "signal_threshold": 0.326,
  "take_profit_atr": 3.7112,
  "stop_loss_atr": 1.135,
  "max_hold_hours": 72,
  "time_decay_hours": 12,
  "min_alignment": 3,
  "trailing_stop_atr": 0.7281,
  "score_flip_delay_hrs": 2
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | -50.15 | +16.89 | +67.04 |
| OOS PF | 1.4 | 1.8 | +0.4 |
| Consistency | 0% | 100% | +100% |
| MaxDD | 4.6% | 4.9% | +0.4% |
| Overfitting | 0.00 | 0.00 | +0.00 |
| Fragility | 0.00 | 0.29 | |

✅ **STRONG RECOMMEND** — trades/fold: 31, exits: {'trailing_stop': 168, 'mr_target': 8, 'score_flip': 45, 'stop_loss': 48, 'take_profit': 7}

### #9: SOL/USDT (Survivor: 16.10 +16.10)
```json
{
  "signal_threshold": 0.314,
  "take_profit_atr": 3.7112,
  "stop_loss_atr": 1.135,
  "max_hold_hours": 72,
  "time_decay_hours": 12,
  "min_alignment": 3,
  "trailing_stop_atr": 0.7281,
  "score_flip_delay_hrs": 2
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | -50.15 | +16.89 | +67.04 |
| OOS PF | 1.4 | 1.8 | +0.4 |
| Consistency | 0% | 100% | +100% |
| MaxDD | 4.6% | 4.9% | +0.4% |
| Overfitting | 0.00 | 0.00 | +0.00 |
| Fragility | 0.00 | 0.29 | |

✅ **STRONG RECOMMEND** — trades/fold: 31, exits: {'trailing_stop': 168, 'mr_target': 8, 'score_flip': 45, 'stop_loss': 48, 'take_profit': 7}

### #10: SOL/USDT (Survivor: 16.04 +16.04)
```json
{
  "signal_threshold": 0.35,
  "take_profit_atr": 4.0,
  "stop_loss_atr": 1.3623,
  "max_hold_hours": 48,
  "time_decay_hours": 38,
  "min_alignment": 3,
  "trailing_stop_atr": 0.7312,
  "score_flip_delay_hrs": 0
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | -50.15 | +16.80 | +66.94 |
| OOS PF | 1.4 | 1.7 | +0.3 |
| Consistency | 0% | 100% | +100% |
| MaxDD | 4.6% | 4.7% | +0.1% |
| Overfitting | 0.00 | 0.00 | +0.00 |
| Fragility | 0.00 | 0.15 | |

✅ **STRONG RECOMMEND** — trades/fold: 27, exits: {'trailing_stop': 155, 'mr_target': 8, 'score_flip': 35, 'stop_loss': 38, 'take_profit': 7}

## Overfitting Warnings

⚠️ BTC/USDT {'signal_threshold': 0.45, 'take_profit_atr': 3.5, 'stop_loss_atr': 2.0, 'max_hold_hours': 36, 'time_decay_hours': 12, 'min_alignment': 3, 'trailing_stop_atr': 0.8, 'score_flip_delay_hrs': 0}: fragility=1.03 > 0.4 (OOS Sharpe: +953.95, IS-OOS gap: 0.00)
⚠️ BTC/USDT {'signal_threshold': 0.45, 'take_profit_atr': 3.5, 'stop_loss_atr': 2.0, 'max_hold_hours': 36, 'time_decay_hours': 12, 'min_alignment': 3, 'trailing_stop_atr': 0.8, 'score_flip_delay_hrs': 1}: fragility=1.03 > 0.4 (OOS Sharpe: +953.95, IS-OOS gap: 0.00)

## Per-Symbol WFA Fold Detail

### BTC/USDT — Best Candidate (Survivor: 8.21)
| Fold | IS Sharpe | OOS Sharpe | OOS PnL | OOS Trades |
|------|-----------|-----------|---------|------------|
| 0 | +0.00 | +13.99 | +1.75% | 6 ✅ |

### SOL/USDT — Best Candidate (Survivor: 16.43)
| Fold | IS Sharpe | OOS Sharpe | OOS PnL | OOS Trades |
|------|-----------|-----------|---------|------------|
| 0 | +0.00 | +29.51 | +20.96% | 34 ✅ |
| 1 | +17.31 | +8.51 | +1.79% | 21 ✅ |
| 2 | +17.21 | +29.00 | +23.30% | 45 ✅ |
| 3 | +20.09 | +17.09 | +12.33% | 42 ✅ |
| 4 | +15.43 | +21.69 | +11.32% | 35 ✅ |
| 5 | +16.29 | +13.86 | +3.01% | 18 ✅ |
| 6 | +15.05 | +1.93 | +1.09% | 34 ✅ |
| 7 | +13.64 | +14.66 | +2.41% | 13 ✅ |
| 8 | +13.32 | +18.32 | +10.93% | 34 ✅ |

### BNB/USDT — Best Candidate (Survivor: 25.76)
| Fold | IS Sharpe | OOS Sharpe | OOS PnL | OOS Trades |
|------|-----------|-----------|---------|------------|
| 0 | +0.00 | +26.00 | +2.94% | 14 ✅ |

## Action Items

1. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3136, take_profit_atr: 6.0→4.0, stop_loss_atr: 2.5→1.1815, max_hold_hours: 96→36, time_decay_hours: 48→24, trailing_stop_atr: 1.0→0.6987
   OOS Sharpe: +17.17 (vs -50.15), consistency: 100%, DD: 4.5%, trades/fold: 31

2. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3626, take_profit_atr: 6.0→4.0, stop_loss_atr: 2.5→1.3644, max_hold_hours: 96→72, time_decay_hours: 48→12, trailing_stop_atr: 1.0→0.7281
   OOS Sharpe: +17.00 (vs -50.15), consistency: 100%, DD: 4.7%, trades/fold: 27

3. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3626, take_profit_atr: 6.0→4.0, stop_loss_atr: 2.5→1.3644, max_hold_hours: 96→77, time_decay_hours: 48→12, trailing_stop_atr: 1.0→0.7281
   OOS Sharpe: +17.00 (vs -50.15), consistency: 100%, DD: 4.7%, trades/fold: 27

4. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3626, take_profit_atr: 6.0→3.6494, stop_loss_atr: 2.5→1.3644, max_hold_hours: 96→72, time_decay_hours: 48→12, trailing_stop_atr: 1.0→0.7281
   OOS Sharpe: +16.96 (vs -50.15), consistency: 100%, DD: 4.7%, trades/fold: 27

5. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.3295, take_profit_atr: 6.0→4.2062, stop_loss_atr: 2.5→1.1628, max_hold_hours: 96→36, time_decay_hours: 48→24, trailing_stop_atr: 1.0→0.7193, score_flip_delay_hrs: 2→3
   OOS Sharpe: +16.95 (vs -50.15), consistency: 100%, DD: 4.8%, trades/fold: 31

Total: 5 actionable recommendations out of 365 validated candidates.
