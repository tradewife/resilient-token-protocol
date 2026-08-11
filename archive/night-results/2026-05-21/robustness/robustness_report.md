# Robustness Report — BNB/USDT @ 1x

**Params:** thresh=0.35, sl=1.0, tp=3.0, trail=1.0, align=3
**Runtime:** 66s

## Verdict: FAIL — Multiple robustness concerns

- PBO=67% indicates likely overfitting
- PBO=67% is elevated (>15%)

## Monte Carlo Drawdown (10,000 simulations)

| Metric | Value |
|--------|-------|
| Trades | 361 |
| Observed DD | 5.6% |
| DD p50 | 4.5% |
| DD p75 | 5.2% |
| DD p90 | 5.9% |
| DD p95 | 6.3% |
| DD p99 | 7.1% |
| DD worst | 8.4% |
| P(DD > 20%) | 0.0% |
| P(DD > 30%) | 0.0% |
| P(DD > 50%) | 0.0% |
| P(liquidation) | 0.0% |
| Return p10 | -2.2% |
| Return p50 | -2.2% |
| Return p90 | -2.2% |

## CPCV + Probability of Backtest Overfitting

| Metric | Value |
|--------|-------|
| Folds | 9 |
| Test folds/path | 3 |
| Total paths | 84 |
| **PBO** | **66.67%** |
| Logit mean | -0.468 |
| Logit median | -0.387 |

*PBO < 15%: SAFE | 15-30%: ELEVATED | > 30%: OVERFITTING*
