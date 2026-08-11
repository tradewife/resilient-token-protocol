# Night Shift Report — 2026-04-04

**Runtime:** 4321s | **Folds:** 9 | **Symbols:** BTC/USDT, ETH/USDT, SOL/USDT, BNB/USDT
**Aggregation:** Median OOS Sharpe, per-fold Sharpe winsorized at ±100

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
| BTC/USDT | -13.69 | 3.2 | 37% | 44% | 3.6% | -0.00 |
| ETH/USDT | +3.72 | 1.2 | 46% | 67% | 5.6% | 0.12 |
| SOL/USDT | +10.04 | 1.2 | 51% | 56% | 4.8% | 0.64 |
| BNB/USDT | -4.97 | 1.8 | 43% | 44% | 4.1% | -0.00 |

## Top 10 Candidates (Ranked by Survivor Score)

*Only candidates validated on 5+ WFA folds are shown.*

### #1: SOL/USDT (Survivor: 16.27 +15.63)
```json
{
  "signal_threshold": 0.35,
  "take_profit_atr": 2.9537,
  "stop_loss_atr": 1.25,
  "max_hold_hours": 48,
  "time_decay_hours": 28,
  "min_alignment": 3,
  "trailing_stop_atr": 0.7012,
  "score_flip_delay_hrs": 4
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +10.04 | +18.50 | +8.46 |
| OOS PF | 1.2 | 1.5 | +0.3 |
| Consistency | 56% | 100% | +44% |
| MaxDD | 4.8% | 4.8% | +0.0% |
| Overfitting | 0.29 | 0.00 | -0.29 |
| Fragility | 4.91 | 0.09 | |

✅ **STRONG RECOMMEND** — trades/fold: 28, exits: {'trailing_stop': 152, 'mr_target': 6, 'score_flip': 31, 'stop_loss': 47, 'take_profit': 15}

### #2: SOL/USDT (Survivor: 15.80 +15.16)
```json
{
  "signal_threshold": 0.2984,
  "take_profit_atr": 4.0,
  "stop_loss_atr": 1.25,
  "max_hold_hours": 48,
  "time_decay_hours": 33,
  "min_alignment": 3,
  "trailing_stop_atr": 0.7012,
  "score_flip_delay_hrs": 4
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +10.04 | +19.01 | +8.97 |
| OOS PF | 1.2 | 1.6 | +0.3 |
| Consistency | 56% | 100% | +44% |
| MaxDD | 4.8% | 4.5% | -0.2% |
| Overfitting | 0.29 | 0.00 | -0.29 |
| Fragility | 4.91 | 0.15 | |

✅ **STRONG RECOMMEND** — trades/fold: 32, exits: {'trailing_stop': 179, 'mr_target': 11, 'score_flip': 48, 'stop_loss': 46, 'take_profit': 5}

### #3: SOL/USDT (Survivor: 15.80 +15.16)
```json
{
  "signal_threshold": 0.2984,
  "take_profit_atr": 3.5615,
  "stop_loss_atr": 1.25,
  "max_hold_hours": 48,
  "time_decay_hours": 33,
  "min_alignment": 3,
  "trailing_stop_atr": 0.7012,
  "score_flip_delay_hrs": 4
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +10.04 | +19.01 | +8.97 |
| OOS PF | 1.2 | 1.6 | +0.3 |
| Consistency | 56% | 100% | +44% |
| MaxDD | 4.8% | 4.5% | -0.2% |
| Overfitting | 0.29 | 0.00 | -0.29 |
| Fragility | 4.91 | 0.15 | |

✅ **STRONG RECOMMEND** — trades/fold: 32, exits: {'trailing_stop': 179, 'mr_target': 11, 'score_flip': 47, 'stop_loss': 46, 'take_profit': 6}

### #4: SOL/USDT (Survivor: 15.80 +15.16)
```json
{
  "signal_threshold": 0.2984,
  "take_profit_atr": 4.0,
  "stop_loss_atr": 1.25,
  "max_hold_hours": 51,
  "time_decay_hours": 33,
  "min_alignment": 3,
  "trailing_stop_atr": 0.7012,
  "score_flip_delay_hrs": 4
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +10.04 | +19.01 | +8.97 |
| OOS PF | 1.2 | 1.6 | +0.3 |
| Consistency | 56% | 100% | +44% |
| MaxDD | 4.8% | 4.5% | -0.2% |
| Overfitting | 0.29 | 0.00 | -0.29 |
| Fragility | 4.91 | 0.15 | |

✅ **STRONG RECOMMEND** — trades/fold: 32, exits: {'trailing_stop': 179, 'mr_target': 11, 'score_flip': 48, 'stop_loss': 46, 'take_profit': 5}

### #5: SOL/USDT (Survivor: 15.80 +15.16)
```json
{
  "signal_threshold": 0.2701,
  "take_profit_atr": 4.0,
  "stop_loss_atr": 1.25,
  "max_hold_hours": 48,
  "time_decay_hours": 33,
  "min_alignment": 3,
  "trailing_stop_atr": 0.7012,
  "score_flip_delay_hrs": 4
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +10.04 | +19.01 | +8.97 |
| OOS PF | 1.2 | 1.6 | +0.3 |
| Consistency | 56% | 100% | +44% |
| MaxDD | 4.8% | 4.5% | -0.2% |
| Overfitting | 0.29 | 0.00 | -0.29 |
| Fragility | 4.91 | 0.15 | |

✅ **STRONG RECOMMEND** — trades/fold: 32, exits: {'trailing_stop': 179, 'mr_target': 11, 'score_flip': 48, 'stop_loss': 46, 'take_profit': 5}

### #6: SOL/USDT (Survivor: 15.80 +15.16)
```json
{
  "signal_threshold": 0.2984,
  "take_profit_atr": 4.0,
  "stop_loss_atr": 1.25,
  "max_hold_hours": 48,
  "time_decay_hours": 30,
  "min_alignment": 3,
  "trailing_stop_atr": 0.7012,
  "score_flip_delay_hrs": 4
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +10.04 | +19.01 | +8.97 |
| OOS PF | 1.2 | 1.6 | +0.3 |
| Consistency | 56% | 100% | +44% |
| MaxDD | 4.8% | 4.5% | -0.2% |
| Overfitting | 0.29 | 0.00 | -0.29 |
| Fragility | 4.91 | 0.15 | |

✅ **STRONG RECOMMEND** — trades/fold: 32, exits: {'trailing_stop': 179, 'mr_target': 11, 'score_flip': 48, 'stop_loss': 46, 'take_profit': 5}

### #7: SOL/USDT (Survivor: 15.80 +15.16)
```json
{
  "signal_threshold": 0.2984,
  "take_profit_atr": 4.0,
  "stop_loss_atr": 1.25,
  "max_hold_hours": 52,
  "time_decay_hours": 33,
  "min_alignment": 3,
  "trailing_stop_atr": 0.7012,
  "score_flip_delay_hrs": 4
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +10.04 | +19.01 | +8.97 |
| OOS PF | 1.2 | 1.6 | +0.3 |
| Consistency | 56% | 100% | +44% |
| MaxDD | 4.8% | 4.5% | -0.2% |
| Overfitting | 0.29 | 0.00 | -0.29 |
| Fragility | 4.91 | 0.15 | |

✅ **STRONG RECOMMEND** — trades/fold: 32, exits: {'trailing_stop': 179, 'mr_target': 11, 'score_flip': 48, 'stop_loss': 46, 'take_profit': 5}

### #8: SOL/USDT (Survivor: 15.80 +15.16)
```json
{
  "signal_threshold": 0.2984,
  "take_profit_atr": 3.5615,
  "stop_loss_atr": 1.25,
  "max_hold_hours": 41,
  "time_decay_hours": 33,
  "min_alignment": 3,
  "trailing_stop_atr": 0.7012,
  "score_flip_delay_hrs": 4
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +10.04 | +19.01 | +8.97 |
| OOS PF | 1.2 | 1.6 | +0.3 |
| Consistency | 56% | 100% | +44% |
| MaxDD | 4.8% | 4.5% | -0.2% |
| Overfitting | 0.29 | 0.00 | -0.29 |
| Fragility | 4.91 | 0.15 | |

✅ **STRONG RECOMMEND** — trades/fold: 32, exits: {'trailing_stop': 179, 'mr_target': 11, 'score_flip': 47, 'stop_loss': 46, 'take_profit': 6}

### #9: SOL/USDT (Survivor: 15.80 +15.16)
```json
{
  "signal_threshold": 0.2984,
  "take_profit_atr": 3.5615,
  "stop_loss_atr": 1.25,
  "max_hold_hours": 42,
  "time_decay_hours": 33,
  "min_alignment": 3,
  "trailing_stop_atr": 0.7012,
  "score_flip_delay_hrs": 4
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +10.04 | +19.01 | +8.97 |
| OOS PF | 1.2 | 1.6 | +0.3 |
| Consistency | 56% | 100% | +44% |
| MaxDD | 4.8% | 4.5% | -0.2% |
| Overfitting | 0.29 | 0.00 | -0.29 |
| Fragility | 4.91 | 0.15 | |

✅ **STRONG RECOMMEND** — trades/fold: 32, exits: {'trailing_stop': 179, 'mr_target': 11, 'score_flip': 47, 'stop_loss': 46, 'take_profit': 6}

### #10: SOL/USDT (Survivor: 15.80 +15.16)
```json
{
  "signal_threshold": 0.2984,
  "take_profit_atr": 3.5615,
  "stop_loss_atr": 1.25,
  "max_hold_hours": 48,
  "time_decay_hours": 30,
  "min_alignment": 3,
  "trailing_stop_atr": 0.7012,
  "score_flip_delay_hrs": 4
}
```
| Metric | Baseline | Candidate | Delta |
|--------|----------|-----------|-------|
| OOS Sharpe | +10.04 | +19.01 | +8.97 |
| OOS PF | 1.2 | 1.6 | +0.3 |
| Consistency | 56% | 100% | +44% |
| MaxDD | 4.8% | 4.5% | -0.2% |
| Overfitting | 0.29 | 0.00 | -0.29 |
| Fragility | 4.91 | 0.15 | |

✅ **STRONG RECOMMEND** — trades/fold: 32, exits: {'trailing_stop': 179, 'mr_target': 11, 'score_flip': 47, 'stop_loss': 46, 'take_profit': 6}

## Overfitting Warnings

⚠️ BTC/USDT {'signal_threshold': 0.4, 'min_alignment': 3, 'take_profit_atr': 6.0, 'stop_loss_atr': 2.5, 'max_hold_hours': 96, 'time_decay_hours': 48, 'trailing_stop_atr': 1.0, 'score_flip_delay_hrs': 2}: oos_consistency=44% < 50% (OOS Sharpe: -13.69, IS-OOS gap: 22.02)
⚠️ BTC/USDT {'signal_threshold': 0.45, 'take_profit_atr': 3.5, 'stop_loss_atr': 1.5, 'max_hold_hours': 36, 'time_decay_hours': 12, 'min_alignment': 3, 'trailing_stop_atr': 1.0, 'score_flip_delay_hrs': 0}: oos_consistency=44% < 50% (OOS Sharpe: -14.80, IS-OOS gap: 6.24)
⚠️ ETH/USDT {'signal_threshold': 0.4, 'min_alignment': 3, 'take_profit_atr': 6.0, 'stop_loss_atr': 2.5, 'max_hold_hours': 96, 'time_decay_hours': 48, 'trailing_stop_atr': 1.0, 'score_flip_delay_hrs': 2}: overfitting_score=0.69 > 0.5 (OOS Sharpe: +3.72, IS-OOS gap: 0.69)

## Per-Symbol WFA Fold Detail

### BTC/USDT — Best Validated Candidate (Survivor: 2.55)
| Fold | IS Sharpe | OOS Sharpe | OOS PnL | OOS Trades |
|------|-----------|-----------|---------|------------|
| 0 | +0.00 | +29.81 | +4.55% | 17 ✅ |
| 1 | +10.86 | +15.02 | +1.65% | 11 ✅ |
| 2 | +8.59 | -38.36 | -3.12% | 12 ❌ |
| 3 | +1.22 | +7.16 | +0.74% | 11 ✅ |
| 4 | +2.36 | +22.73 | +2.53% | 11 ✅ |
| 5 | +5.24 | +67.55 | +2.97% | 3 ✅ |
| 6 | +9.12 | -24.80 | -1.72% | 8 ❌ |
| 7 | +5.09 | +100.00 (raw: +637) | +0.48% | 2 ✅ |
| 8 | +6.24 | -5.00 | -0.73% | 11 ❌ |

### ETH/USDT — No validated candidates
  1 rejected: overfitting_score=0.69 > 0.5

### SOL/USDT — Best Validated Candidate (Survivor: 16.27)
| Fold | IS Sharpe | OOS Sharpe | OOS PnL | OOS Trades |
|------|-----------|-----------|---------|------------|
| 0 | +0.00 | +28.72 | +20.42% | 32 ✅ |
| 1 | +17.16 | +9.65 | +1.97% | 20 ✅ |
| 2 | +17.51 | +23.32 | +18.87% | 44 ✅ |
| 3 | +19.26 | +19.37 | +14.23% | 40 ✅ |
| 4 | +15.54 | +18.50 | +7.59% | 30 ✅ |
| 5 | +15.49 | +3.77 | +0.67% | 14 ✅ |
| 6 | +13.92 | +3.85 | +1.80% | 30 ✅ |
| 7 | +12.65 | +5.36 | +0.70% | 10 ✅ |
| 8 | +12.30 | +19.45 | +11.49% | 31 ✅ |

### BNB/USDT — Best Validated Candidate (Survivor: 9.17)
| Fold | IS Sharpe | OOS Sharpe | OOS PnL | OOS Trades |
|------|-----------|-----------|---------|------------|
| 0 | +0.00 | +31.85 | +14.26% | 33 ✅ |
| 1 | +28.16 | -15.12 | -1.64% | 28 ❌ |
| 2 | +18.96 | +18.98 | +11.34% | 45 ✅ |
| 3 | +16.43 | +8.48 | +4.15% | 46 ✅ |
| 4 | +14.53 | +30.01 | +17.43% | 35 ✅ |
| 5 | +16.03 | -35.37 | -5.76% | 16 ❌ |
| 6 | +13.98 | +50.10 | +10.53% | 31 ✅ |
| 7 | +14.36 | -9.30 | -0.96% | 10 ❌ |
| 8 | +13.11 | +17.70 | +5.35% | 30 ✅ |

## Action Items

1. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.35, take_profit_atr: 6.0→2.9537, stop_loss_atr: 2.5→1.25, max_hold_hours: 96→48, time_decay_hours: 48→28, trailing_stop_atr: 1.0→0.7012, score_flip_delay_hrs: 2→4
   OOS Sharpe: +18.50 (vs +10.04), consistency: 100%, DD: 4.8%, trades/fold: 28

2. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.2984, take_profit_atr: 6.0→4.0, stop_loss_atr: 2.5→1.25, max_hold_hours: 96→48, time_decay_hours: 48→33, trailing_stop_atr: 1.0→0.7012, score_flip_delay_hrs: 2→4
   OOS Sharpe: +19.01 (vs +10.04), consistency: 100%, DD: 4.5%, trades/fold: 32

3. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.2984, take_profit_atr: 6.0→3.5615, stop_loss_atr: 2.5→1.25, max_hold_hours: 96→48, time_decay_hours: 48→33, trailing_stop_atr: 1.0→0.7012, score_flip_delay_hrs: 2→4
   OOS Sharpe: +19.01 (vs +10.04), consistency: 100%, DD: 4.5%, trades/fold: 32

4. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.2984, take_profit_atr: 6.0→4.0, stop_loss_atr: 2.5→1.25, max_hold_hours: 96→51, time_decay_hours: 48→33, trailing_stop_atr: 1.0→0.7012, score_flip_delay_hrs: 2→4
   OOS Sharpe: +19.01 (vs +10.04), consistency: 100%, DD: 4.5%, trades/fold: 32

5. **[HIGH]** SOL/USDT: signal_threshold: 0.4→0.2701, take_profit_atr: 6.0→4.0, stop_loss_atr: 2.5→1.25, max_hold_hours: 96→48, time_decay_hours: 48→33, trailing_stop_atr: 1.0→0.7012, score_flip_delay_hrs: 2→4
   OOS Sharpe: +19.01 (vs +10.04), consistency: 100%, DD: 4.5%, trades/fold: 32

Total: 5 actionable recommendations out of 3651 validated candidates.
