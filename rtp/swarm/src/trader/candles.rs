//! Candle buffer — Binance warmup + Flash Trade ongoing price feeds.

use super::indicators::Candle;
use serde::{Deserialize, Serialize};

/// Aggregates price ticks into hourly candles and maintains a rolling buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleBuffer {
    candles: Vec<Candle>,
    max_len: usize,
    /// Current hourly candle being built from ticks.
    current_hour: Option<PartialCandle>,
}

/// A partially-built candle for the current hour.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PartialCandle {
    hour_start: i64, // unix seconds, floored to hour
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    tick_count: u32,
}

impl CandleBuffer {
    pub fn new(max_len: usize) -> Self {
        Self {
            candles: Vec::with_capacity(max_len),
            max_len,
            current_hour: None,
        }
    }

    pub fn len(&self) -> usize {
        self.candles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.candles.is_empty()
    }

    pub fn candles(&self) -> &[Candle] {
        &self.candles
    }

    /// Returns closing prices as a flat Vec for indicator computation.
    pub fn closes(&self) -> Vec<f64> {
        self.candles.iter().map(|c| c.close).collect()
    }

    /// Returns volumes as a flat Vec.
    pub fn volumes(&self) -> Vec<f64> {
        self.candles.iter().map(|c| c.volume).collect()
    }

    /// Load pre-built candles (from Binance warmup).
    pub fn load_candles(&mut self, candles: Vec<Candle>) {
        self.candles = candles;
        // Trim to max_len
        if self.candles.len() > self.max_len {
            let start = self.candles.len() - self.max_len;
            self.candles.drain(0..start);
        }
    }

    /// Append a single price tick (from Flash Trade /prices).
    /// Aggregates into the current hour's candle. When the hour flips,
    /// finalizes the previous candle and starts a new one.
    pub fn append_tick(&mut self, price: f64, timestamp_secs: i64) {
        let hour_start = (timestamp_secs / 3600) * 3600;

        match &mut self.current_hour {
            Some(partial) if partial.hour_start == hour_start => {
                // Same hour — update OHLC
                partial.high = partial.high.max(price);
                partial.low = partial.low.min(price);
                partial.close = price;
                partial.tick_count += 1;
            }
            _ => {
                // Hour flipped or first tick — finalize previous candle
                if let Some(prev) = self.current_hour.take() {
                    self.push_candle(Candle {
                        timestamp: prev.hour_start,
                        open: prev.open,
                        high: prev.high,
                        low: prev.low,
                        close: prev.close,
                        volume: prev.tick_count as f64, // proxy: tick count as volume
                    });
                }
                // Start new partial candle
                self.current_hour = Some(PartialCandle {
                    hour_start,
                    open: price,
                    high: price,
                    low: price,
                    close: price,
                    tick_count: 1,
                });
            }
        }
    }

    /// Finalize the current partial candle (if any) into the buffer.
    pub fn finalize_current(&mut self) {
        if let Some(partial) = self.current_hour.take() {
            self.push_candle(Candle {
                timestamp: partial.hour_start,
                open: partial.open,
                high: partial.high,
                low: partial.low,
                close: partial.close,
                volume: partial.tick_count as f64,
            });
        }
    }

    fn push_candle(&mut self, candle: Candle) {
        self.candles.push(candle);
        if self.candles.len() > self.max_len {
            self.candles.remove(0);
        }
    }
}

/// Fetch historical candles from Binance.
/// Returns up to `limit` candles (most recent last) for the given interval.
/// `interval` is one of "1h", "4h", "1d".
pub async fn fetch_binance_ohlcv(
    symbol: &str,
    interval: &str,
    limit: usize,
) -> Result<Vec<Candle>, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.binance.com/api/v3/klines?symbol={}&interval={}&limit={}",
        symbol, interval, limit
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Binance request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Binance returned status {}", resp.status()));
    }

    let data: Vec<Vec<serde_json::Value>> = resp
        .json()
        .await
        .map_err(|e| format!("Binance parse error: {}", e))?;

    let mut candles = Vec::with_capacity(data.len());
    for kline in &data {
        if kline.len() < 6 {
            continue;
        }
        let timestamp = kline[0].as_i64().unwrap_or(0) / 1000; // ms → s
        let open = kline[1]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let high = kline[2]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let low = kline[3]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let close = kline[4]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let volume = kline[5]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        candles.push(Candle {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
        });
    }
    Ok(candles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_rolls_over() {
        let mut buf = CandleBuffer::new(5);
        for i in 0..10 {
            buf.push_candle(Candle {
                timestamp: i * 3600,
                open: i as f64,
                high: i as f64,
                low: i as f64,
                close: i as f64,
                volume: 1.0,
            });
        }
        assert_eq!(buf.len(), 5);
        assert_eq!(buf.candles()[0].timestamp, 5 * 3600); // oldest kept
    }

    #[test]
    fn tick_aggregation() {
        let mut buf = CandleBuffer::new(100);
        let hour = 1000 * 3600;
        buf.append_tick(100.0, hour + 100);
        buf.append_tick(105.0, hour + 200);
        buf.append_tick(98.0, hour + 300);
        buf.append_tick(102.0, hour + 400);
        // Finalize
        buf.append_tick(50.0, hour + 3600); // new hour triggers finalization
        assert_eq!(buf.len(), 1);
        let c = &buf.candles()[0];
        assert_eq!(c.open, 100.0);
        assert_eq!(c.high, 105.0);
        assert_eq!(c.low, 98.0);
        assert_eq!(c.close, 102.0);
    }
}
