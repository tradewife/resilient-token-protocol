# Strategy Library

> Seed corpus for the continual evolution infrastructure. Sourced from *151 Trading Strategies* (Ivy League / quant literature compendium) and filtered for crypto perpetuals applicability on Flash Trade (on-chain Solana perps).
>
> **Current best strategy:** SOL/USDT Survivor 2.69 (momentum/trend following, OOS Sharpe 3.96, 9/9 folds). New strategies should complement, not duplicate, this edge.
>
> **Venue constraint:** All strategies must be implementable with OHLCV + funding rate data only. No order book, no social sentiment, no on-chain analytics.

---

## Classification

| ID | Strategy | Type | Regime | Priority |
|----|----------|------|--------|----------|
| S01 | Momentum Persistence | trend | trending | 1 |
| S02 | Breakout-Band Expansion | volatility | trending | 1 |
| S03 | Funding Rate Carry | carry | both | 1 |
| S04 | Mean-Reversion RSI Exhaustion | mean_reversion | ranging | 1 |
| S05 | Bollinger Bounce with Trend Filter | mean_reversion | ranging | 1 |
| S06 | Volatility Breakout (Squeeze) | volatility | both | 1 |
| S07 | Dual Moving Average Cross | trend | trending | 2 |
| S08 | Mean-Reversion Band Walk | mean_reversion | ranging | 2 |
| S09 | Funding Rate Momentum | carry | trending | 2 |
| S10 | Momentum Divergence | trend | both | 2 |
| S11 | ATR Channel Breakout | volatility | trending | 2 |
| S12 | Multi-TF RSI Confluence | mean_reversion | both | 2 |
| S13 | Trend-Following with ADX Filter | trend | trending | 3 |
| S14 | Volatility Regime Switch | volatility | both | 3 |
| S15 | Cumulative Volume Delta Proxy | risk_premium | both | 3 |

---

### S01 — Momentum Persistence
- **Source**: 151 Trading Strategies, Section 2.1 (Momentum)
- **Edge type**: risk_premium
- **Market**: crypto_perps
- **Regime fit**: trending
- **Entry logic**: IF close > SMA(close, 20) AND close[t] > close[t-20] AND volume_ratio > 1.0 THEN buy_long. IF close < SMA(close, 20) AND close[t] < close[t-20] AND volume_ratio > 1.0 THEN sell_short.
- **Exit logic**: Trailing stop at 2x ATR from peak. Max hold 72h. Signal reversal (close crosses below SMA20 for longs, above for shorts).
- **Position sizing**: vol_scaled (ATR-inverse: smaller size when ATR high)
- **Expected behavior**: Captures persistent trends in crypto. Works well when SOL/BTC/ETH trend for 3-10 days consecutively. Fails in choppy range-bound markets where price oscillates around the SMA.
- **Decay risk**: low
- **Priority**: 1

---

### S02 — Breakout-Band Expansion
- **Source**: 151 Trading Strategies, Section 3.4 (Volatility Breakout)
- **Edge type**: risk_premium
- **Market**: crypto_perps
- **Regime fit**: trending
- **Entry logic**: IF Bollinger_Band_Width (upper - lower) / middle < percentile_10(lookback=100) AND close > upper_band THEN buy_long. IF BB_Width < pctl_10 AND close < lower_band THEN sell_short. Requires prior squeeze (narrow bands) before breakout.
- **Exit logic**: Take profit at 3x ATR from entry. Stop loss at 1.5x ATR. Max hold 48h. Exit if BB_Width expands beyond percentile_80 (expansion complete).
- **Position sizing**: vol_scaled (ATR-inverse)
- **Expected behavior**: Catches the start of explosive moves after a low-volatility compression phase. Crypto is prone to these squeezes before large directional moves. Fails when the breakout is a false move and price reverses back into the band.
- **Decay risk**: medium
- **Priority**: 1

---

### S03 — Funding Rate Carry
- **Source**: 151 Trading Strategies, Section 5.2 (Carry Trade)
- **Edge type**: risk_premium
- **Market**: crypto_perps
- **Regime fit**: both
- **Entry logic**: IF funding_rate_8h > 0.05% (annualized ~55%) THEN sell_short (collect funding from longs paying shorts). IF funding_rate_8h < -0.03% THEN buy_long (collect funding from shorts paying longs). Delta-neutral variant: hold equal spot + perp short when funding positive.
- **Exit logic**: Exit when funding rate crosses zero or reverses sign. Stop loss at 3% adverse price movement. Time exit after 7 days (funding rates mean-revert).
- **Position sizing**: fixed_fractional (1-2% of treasury per position)
- **Expected behavior**: Crypto perps consistently exhibit positive funding (long bias), creating a persistent carry premium for short sellers. This is one of the most durable edges in crypto. Fails during rapid directional moves where the price PnL overwhelms the funding collected.
- **Decay risk**: low
- **Priority**: 1

---

### S04 — Mean-Reversion RSI Exhaustion
- **Source**: 151 Trading Strategies, Section 4.1 (Mean Reversion)
- **Edge type**: risk_premium
- **Market**: crypto_perps
- **Regime fit**: ranging
- **Entry logic**: IF RSI(14) < 25 AND close < Bollinger_Lower(20, 2.0) AND daily_trend == bullish (SMA200 > price * 0.97) THEN buy_long. IF RSI(14) > 75 AND close > Bollinger_Upper(20, 2.0) AND daily_trend == bearish THEN sell_short. Must have both RSI extreme AND BB extreme for entry.
- **Exit logic**: Take profit when RSI crosses back through 45 (long) or 55 (short). Stop loss at 2x ATR. Max hold 36h. Exit if price reaches BB middle band.
- **Position sizing**: vol_scaled
- **Expected behavior**: Catches exhaustion moves in ranging markets where price has overshot to the downside in an uptrend (or overshoot to upside in downtrend). Works well in crypto because of leverage-induced liquidation cascades that create temporary dislocations. Fails in strong trends where RSI stays extreme for extended periods.
- **Decay risk**: medium
- **Priority**: 1

---

### S05 — Bollinger Bounce with Trend Filter
- **Source**: 151 Trading Strategies, Section 4.3 (Bollinger Band Strategies)
- **Edge type**: risk_premium
- **Market**: crypto_perps
- **Regime fit**: ranging
- **Entry logic**: IF close <= BB_lower(20, 2.0) AND SMA(200) > close * 0.95 (long-term uptrend intact) AND RSI(14) < 35 AND volume_ratio > 0.8 THEN buy_long. Mirror for shorts: close >= BB_upper AND SMA(200) < close * 1.05 AND RSI > 65.
- **Exit logic**: Target at BB_middle for 50% of position, BB_upper for remaining 50%. Stop loss at 1.5x ATR below entry. Max hold 48h.
- **Position sizing**: vol_scaled (half position when volatility above 75th percentile)
- **Expected behavior**: Buys dips in uptrends when price touches the lower band — expects reversion to mean. The trend filter ensures we only buy dips in bull markets. Fails when the "dip" is actually a trend reversal and price continues falling through the lower band.
- **Decay risk**: low
- **Priority**: 1

---

### S06 — Volatility Breakout (Squeeze)
- **Source**: 151 Trading Strategies, Section 3.1 (Volatility Strategies)
- **Edge type**: inefficiency
- **Market**: crypto_perps
- **Regime fit**: both
- **Entry logic**: IF ATR(14) / close < percentile_20(lookback=100) (low vol regime) THEN set breakout levels: highest_high(20) + 0.5*ATR and lowest_low(20) - 0.5*ATR. IF close > upper_breakout_level THEN buy_long. IF close < lower_breakout_level THEN sell_short.
- **Exit logic**: Take profit at 4x ATR from entry. Stop loss at 1.5x ATR. Max hold 60h. Trail stop at 1x ATR after 2x ATR profit reached.
- **Position sizing**: vol_scaled (larger position when vol is low, expecting expansion)
- **Expected behavior**: Low-volatility compression in crypto perps almost always resolves with a sharp move. The strategy positions before the move. Fails when compression continues without breakout (whipsaw on false signals), or when the breakout direction is wrong.
- **Decay risk**: medium
- **Priority**: 1

---

### S07 — Dual Moving Average Cross
- **Source**: 151 Trading Strategies, Section 2.2 (Moving Average Strategies)
- **Edge type**: risk_premium
- **Market**: crypto_perps
- **Regime fit**: trending
- **Entry logic**: IF SMA(close, 10) crosses above SMA(close, 30) AND ADX(14) > 20 AND volume_ratio > 0.9 THEN buy_long. IF SMA(10) crosses below SMA(30) AND ADX > 20 THEN sell_short.
- **Exit logic**: Exit on opposing cross. Stop loss at 2x ATR. No take profit — ride the trend until cross reverses.
- **Position sizing**: fixed_fractional (2% of treasury)
- **Expected behavior**: Simple trend-following that captures medium-term moves. Crypto's trending nature makes MA crosses viable. Fails in ranging markets where SMAs cross repeatedly (whipsaw). The ADX filter reduces false signals.
- **Decay risk**: low
- **Priority**: 2

---

### S08 — Mean-Reversion Band Walk
- **Source**: 151 Trading Strategies, Section 4.5 (Band-Based Mean Reversion)
- **Edge type**: risk_premium
- **Market**: crypto_perps
- **Regime fit**: ranging
- **Entry logic**: IF price touches BB_lower(20, 2.0) AND then closes ABOVE BB_lower within next 3 bars AND RSI(14) < 40 AND ADX(14) < 25 (range confirmed) THEN buy_long. Mirror for shorts at BB_upper.
- **Exit logic**: Target at BB_middle. Stop loss at 2x ATR below BB_lower. Max hold 24h. If price walks up the band (consecutive closes along upper band), exit immediately (trend starting).
- **Position sizing**: fixed_fractional (1.5% of treasury)
- **Expected behavior**: Expects the band touch to be a rejection, not a breakout. The "touch and bounce" pattern is common in crypto ranges. Fails when the touch becomes a breakout — the ADX filter helps but is not perfect.
- **Decay risk**: medium
- **Priority**: 2

---

### S09 — Funding Rate Momentum
- **Source**: 151 Trading Strategies, Section 5.3 (Carry with Momentum)
- **Edge type**: risk_premium
- **Market**: crypto_perps
- **Regime fit**: trending
- **Entry logic**: IF funding_rate_8h > 0.03% AND funding_rate_8h_increasing (current > 8h_ago > 16h_ago) AND close > SMA(20) THEN buy_long (crowded long side = momentum). IF funding_rate_8h < -0.02% AND funding_rate_decreasing AND close < SMA(20) THEN sell_short.
- **Exit logic**: Exit when funding_rate changes direction (from increasing to decreasing or vice versa). Stop loss at 2.5x ATR. Max hold 48h.
- **Position sizing**: vol_scaled
- **Expected behavior**: Combines carry (funding) with momentum (trending price). When funding is positive and increasing, it signals strong long conviction — the trend usually continues. Fails at trend exhaustion when funding peaks just before reversal.
- **Decay risk**: medium
- **Priority**: 2

---

### S10 — Momentum Divergence
- **Source**: 151 Trading Strategies, Section 2.5 (Divergence Signals)
- **Edge type**: inefficiency
- **Market**: crypto_perps
- **Regime fit**: both
- **Entry logic**: IF price makes higher high AND RSI(14) makes lower high (bearish divergence) AND RSI > 60 THEN sell_short. IF price makes lower low AND RSI makes higher low (bullish divergence) AND RSI < 40 THEN buy_long. Divergence must span at least 10 bars.
- **Exit logic**: Take profit at 2x ATR. Stop loss at 1.5x ATR. Exit on RSI cross through 50. Max hold 48h.
- **Position sizing**: vol_scaled
- **Expected behavior**: Catches trend reversals by identifying when momentum is fading while price still rising/falling. Crypto has frequent divergence setups due to leverage-driven exhaustion. Fails in strongly trending markets where divergences persist for weeks before any reversal.
- **Decay risk**: high
- **Priority**: 2

---

### S11 — ATR Channel Breakout
- **Source**: 151 Trading Strategies, Section 3.5 (Keltner/ATR Channel)
- **Edge type**: risk_premium
- **Market**: crypto_perps
- **Regime fit**: trending
- **Entry logic**: Compute ATR_Channel = SMA(20) +/- 2*ATR(14). IF close > upper_ATR_channel AND close[1] <= upper_ATR_channel[1] (fresh breakout) AND ATR(14) > ATR(14)[5] (expanding vol) THEN buy_long. Mirror for shorts.
- **Exit logic**: Take profit at 3.5x ATR. Stop loss at 1x ATR (tight for breakout). Max hold 60h. Exit if close returns inside channel.
- **Position sizing**: vol_scaled
- **Expected behavior**: Similar to Bollinger breakout but using ATR channels which adapt faster to crypto volatility. The expanding vol requirement filters false breakouts. Fails in low-vol environments where price chops through channel boundaries without conviction.
- **Decay risk**: medium
- **Priority**: 2

---

### S12 — Multi-TF RSI Confluence
- **Source**: 151 Trading Strategies, Section 4.2 (Multi-Timeframe Mean Reversion)
- **Edge type**: risk_premium
- **Market**: crypto_perps
- **Regime fit**: both
- **Entry logic**: IF RSI(1h, 14) < 30 AND RSI(4h, 14) < 40 AND RSI(1d, 14) < 50 THEN buy_long (oversold across all timeframes). IF RSI(1h) > 70 AND RSI(4h) > 60 AND RSI(1d) > 50 THEN sell_short. All three conditions must be true simultaneously.
- **Exit logic**: Take profit when RSI(1h) crosses above 55 (long) or below 45 (short). Stop loss at 2x ATR. Max hold 36h.
- **Position sizing**: vol_scaled
- **Expected behavior**: Multi-TF oversold conditions in crypto often coincide with liquidation cascades, creating sharp V-shaped reversals. The multi-TF requirement reduces false signals from single-TF noise. Fails when the market is in a persistent downtrend where all TFs stay oversold for extended periods.
- **Decay risk**: medium
- **Priority**: 2

---

### S13 — Trend-Following with ADX Filter
- **Source**: 151 Trading Strategies, Section 2.3 (Directional Movement)
- **Edge type**: risk_premium
- **Market**: crypto_perps
- **Regime fit**: trending
- **Entry logic**: IF ADX(14) > 30 AND Plus_DI(14) > Minus_DI(14) AND close > SMA(close, 50) THEN buy_long. IF ADX > 30 AND Minus_DI > Plus_DI AND close < SMA(50) THEN sell_short. ADX must be rising (current ADX > ADX[5]).
- **Exit logic**: Exit when ADX drops below 20 or DI cross reverses. Stop loss at 2x ATR. Trail stop at 1.5x ATR from peak after 2x ATR profit.
- **Position sizing**: fixed_fractional (2% of treasury)
- **Expected behavior**: Classic DMI trend-following. ADX > 30 confirms a strong trend exists, DI determines direction. Works well for medium-to-long crypto trends (3-15 days). Fails when ADX gives late signals or when trends reverse sharply without DI crossing first.
- **Decay risk**: low
- **Priority**: 3

---

### S14 — Volatility Regime Switch
- **Source**: 151 Trading Strategies, Section 3.2 (Regime-Based Strategies)
- **Edge type**: risk_premium
- **Market**: crypto_perps
- **Regime fit**: both
- **Entry logic**: Compute vol_regime = ATR(14) / close relative to percentile bands. IF vol_regime < 25th_percentile THEN use S07 (Dual MA Cross) parameters for trend entry. IF vol_regime > 75th_percentile THEN use S04 (RSI Exhaustion) parameters for mean-reversion entry. IF vol_regime between 25th-75th THEN no trade.
- **Exit logic**: Exit by the active sub-strategy's rules. Switch strategy when vol regime changes. Force exit on regime switch.
- **Position sizing**: vol_scaled (larger in low-vol for trend, smaller in high-vol for MR)
- **Expected behavior**: Adapts strategy type to market volatility regime. In low-vol crypto, trends tend to develop; in high-vol, mean-reversion works better after liquidation cascades. Fails during regime transitions when the switch is too slow or the classification is wrong.
- **Decay risk**: low
- **Priority**: 3

---

### S15 — Cumulative Volume Delta Proxy
- **Source**: 151 Trading Strategies, Section 6.1 (Volume-Based Strategies)
- **Edge type**: risk_premium
- **Market**: crypto_perps
- **Regime fit**: both
- **Entry logic**: Compute CVD_proxy = cumsum(volume * sign(close - close[1])). IF CVD_proxy makes new 20-bar high AND close has NOT made new 20-bar high (bullish CVD divergence) AND SMA(50) > SMA(200) THEN buy_long. IF CVD_proxy makes new 20-bar low AND close has NOT made new 20-bar low (bearish CVD divergence) AND SMA(50) < SMA(200) THEN sell_short.
- **Exit logic**: Take profit at 3x ATR. Stop loss at 2x ATR. Max hold 72h. Exit on CVD divergence exhaustion (CVD reverses direction for 5+ bars).
- **Position sizing**: fixed_fractional (1.5% of treasury)
- **Expected behavior**: CVD proxy detects when "smart money" is accumulating or distributing before price reflects it. The divergence between volume-weighted buying pressure and price signals an imminent move. Fails in crypto because retail-driven volume can create false signals, and the CVD proxy is less accurate than true order flow CVD.
- **Decay risk**: high
- **Priority**: 3

---

## Portfolio Balance Check

| Category | Count | Strategy IDs |
|----------|-------|-------------|
| Trend/Momentum | 5 | S01, S02, S07, S10, S13 |
| Mean Reversion | 4 | S04, S05, S08, S12 |
| Carry/Funding | 2 | S03, S09 |
| Volatility | 3 | S06, S11, S14 |
| Volume-based | 1 | S15 |
| **Priority 1** | **6** | S01, S02, S03, S04, S05, S06 |
| **Priority 2** | **6** | S07, S08, S09, S10, S11, S12 |
| **Priority 3** | **3** | S13, S14, S15 |

**Complementarity to SOL Survivor 2.69** (momentum/trend-following):
- S03 (Funding Carry) adds uncorrelated carry income
- S04, S05 (Mean Reversion) add range-bound performance
- S06 (Volatility Breakout) adds compression/expansion alpha
- These four are the highest-priority diversifiers

---

*Maintained by the continual evolution infrastructure. Update when new strategies are validated or existing ones are moved to dead_ends.md.*
