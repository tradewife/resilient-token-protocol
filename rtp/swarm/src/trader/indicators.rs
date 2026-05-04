//! Technical indicators for the Survivor 2.69 strategy.
//!
//! Ported from Python `research/simulation/run_backtest_r2.py`.
//! Key invariant: ATR = std(returns, 20) × price (NOT True Range).

/// Candle data point used by all indicators.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Candle {
    pub timestamp: i64,    // unix seconds
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// Simple moving average over the last `period` values.
pub fn sma(values: &[f64], period: usize) -> Option<f64> {
    if values.len() < period || period == 0 {
        return None;
    }
    let slice = &values[values.len() - period..];
    Some(slice.iter().sum::<f64>() / period as f64)
}

/// Relative Strength Index (14-period default).
/// Returns RSI in 0..100 range.
pub fn rsi(closes: &[f64], period: usize) -> Option<f64> {
    if closes.len() < period + 1 {
        return None;
    }
    let mut avg_gain = 0.0;
    let mut avg_loss = 0.0;

    // Initial average
    for i in (closes.len() - period)..closes.len() {
        let delta = closes[i] - closes[i - 1];
        if delta > 0.0 {
            avg_gain += delta;
        } else {
            avg_loss += delta.abs();
        }
    }
    avg_gain /= period as f64;
    avg_loss /= period as f64;

    if avg_loss == 0.0 {
        return Some(100.0);
    }
    let rs = avg_gain / avg_loss;
    Some(100.0 - 100.0 / (1.0 + rs))
}

/// ATR proxy: std(returns, period) × current_price.
/// This matches the validated Python formula — NOT True Range.
pub fn atr_proxy(closes: &[f64], period: usize) -> Option<f64> {
    if closes.len() < period + 1 {
        return None;
    }
    let mut returns = Vec::with_capacity(closes.len() - 1);
    for i in 1..closes.len() {
        if closes[i - 1] != 0.0 {
            returns.push((closes[i] - closes[i - 1]) / closes[i - 1]);
        }
    }
    let std = std_dev(&returns[returns.len().saturating_sub(period)..]);
    let price = closes.last()?;
    Some(std * price)
}

/// Standard deviation of a slice.
fn std_dev(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    variance.sqrt()
}

/// Bollinger Band position: (price - lower) / (4 × std).
/// Returns 0.0 at lower band, 0.5 at middle, 1.0 at upper band.
pub fn bollinger_position(closes: &[f64], period: usize) -> Option<f64> {
    if closes.len() < period {
        return None;
    }
    let slice = &closes[closes.len() - period..];
    let mean = slice.iter().sum::<f64>() / period as f64;
    let std = std_dev(slice);
    if std == 0.0 {
        return Some(0.5);
    }
    let price = closes.last()?;
    Some((price - (mean - 2.0 * std)) / (4.0 * std))
}

/// Volume ratio: current volume / SMA(volume, period).
pub fn volume_ratio(volumes: &[f64], period: usize) -> Option<f64> {
    let avg = sma(volumes, period)?;
    if avg == 0.0 {
        return Some(1.0);
    }
    let current = volumes.last()?;
    Some(current / avg)
}

/// Multi-timeframe trend signal for a single lookback window.
pub struct TrendSignal {
    pub trend: String,   // "bullish", "bearish", "neutral"
    pub strength: f64,
    pub rsi: f64,
    pub momentum: f64,
    pub volatility: f64,
}

/// Compute trend signal for a single timeframe (mirrors Python `timeframe_signal`).
pub fn timeframe_signal(closes: &[f64], lookback: usize) -> Option<TrendSignal> {
    if closes.len() < lookback {
        return None;
    }
    let slice = &closes[closes.len() - lookback..];
    let sma_val = slice.iter().sum::<f64>() / lookback as f64;
    let price = *closes.last()?;

    let (trend, strength) = if sma_val == 0.0 {
        ("neutral".to_string(), 0.0)
    } else if price > sma_val {
        ("bullish".to_string(), ((price - sma_val) / sma_val * 100.0).min(2.0))
    } else if price < sma_val {
        ("bearish".to_string(), ((sma_val - price) / sma_val * 100.0).min(2.0))
    } else {
        ("neutral".to_string(), 0.0)
    };

    let rsi_val = rsi(closes, 14).unwrap_or(50.0);

    // Momentum: mean of returns over lookback
    let returns: Vec<f64> = slice.windows(2).map(|w| (w[1] - w[0]) / w[0]).collect();
    let momentum = if returns.len() >= lookback {
        returns[returns.len() - lookback..].iter().sum::<f64>() / lookback as f64
    } else {
        0.0
    };

    // Volatility: std of returns
    let volatility = if returns.len() >= lookback {
        std_dev(&returns[returns.len() - lookback..])
    } else {
        0.0
    };

    Some(TrendSignal {
        trend,
        strength,
        rsi: rsi_val,
        momentum,
        volatility,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sma_basic() {
        let vals = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((sma(&vals, 3).unwrap() - 4.0).abs() < 1e-10);
        assert!((sma(&vals, 5).unwrap() - 3.0).abs() < 1e-10);
        assert!(sma(&vals, 6).is_none());
    }

    #[test]
    fn rsi_all_gains() {
        let closes: Vec<f64> = (1..=20).map(|i| i as f64).collect();
        let r = rsi(&closes, 14).unwrap();
        assert!(r > 90.0, "RSI should be near 100 for all gains, got {}", r);
    }

    #[test]
    fn rsi_all_losses() {
        let closes: Vec<f64> = (1..=20).rev().map(|i| i as f64).collect();
        let r = rsi(&closes, 14).unwrap();
        assert!(r < 10.0, "RSI should be near 0 for all losses, got {}", r);
    }

    #[test]
    fn atr_proxy_positive() {
        let closes: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64 * 0.5)).collect();
        let atr = atr_proxy(&closes, 20).unwrap();
        assert!(atr > 0.0, "ATR should be positive");
    }

    #[test]
    fn bollinger_mid_range() {
        let closes: Vec<f64> = (0..40).map(|i| 100.0 + (i as f64 - 20.0)).collect();
        let pos = bollinger_position(&closes, 20).unwrap();
        assert!(pos > 0.0 && pos < 1.0, "BB position should be in range, got {}", pos);
    }

    #[test]
    fn timeframe_signal_bullish() {
        let closes: Vec<f64> = (1..=25).map(|i| i as f64 * 1.01).collect();
        let sig = timeframe_signal(&closes, 20).unwrap();
        assert_eq!(sig.trend, "bullish");
        assert!(sig.strength > 0.0);
    }
}
