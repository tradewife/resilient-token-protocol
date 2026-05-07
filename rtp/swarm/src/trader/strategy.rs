//! Survivor 2.69 strategy — score computation, entry/exit logic.
//!
//! Ported from Python `research/simulation/run_backtest_r2.py`.
//! Leverage optimization (May 2026): 9x leverage, Calmar=44.89, +554% return, 12.3% DD, 100% consistency.
//! Key: TP=5.0, trail=0.14, SL=2.7 — tight trail captures leveraged gains, wide TP lets winners run.

use super::indicators::{atr_proxy, bollinger_position, rsi, timeframe_signal, volume_ratio};
use serde::{Deserialize, Serialize};

/// Survivor 2.69 strategy parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyParams {
    pub signal_threshold: f64,
    pub tp_atr: f64,
    pub sl_atr: f64,
    pub max_hold_hours: f64,
    pub trailing_stop_atr: f64,
    pub time_decay_hours: f64,
    pub min_alignment: usize,
}

impl Default for StrategyParams {
    fn default() -> Self {
        // SOL/USDT 9x Leverage Optimization (May 2026 night shift)
        // Calmar=44.89, +554% return, 12.3% DD, 100% consistency, 0 liquidations, 419 trades
        // Grid: 16,228 candidates × 9-fold WFA × Flash Trade fee model × compounding
        Self {
            signal_threshold: 0.25,
            tp_atr: 5.0,
            sl_atr: 2.7,
            max_hold_hours: 36.0,
            trailing_stop_atr: 0.14,
            time_decay_hours: 12.0,
            min_alignment: 3,
        }
    }
}

impl StrategyParams {
    /// Load strategy params from the daemon's `config.json` output.
    ///
    /// Resolution order:
    /// 1. `RTP_STRATEGY_CONFIG` env var (explicit override)
    /// 2. `data/devnet-cycles/latest/config.json` (daemon mutation output)
    /// 3. `StrategyParams::default()` (hardcoded baseline)
    ///
    /// Values outside soulcontract bounds are silently clamped — this is a
    /// safety net, not a replacement for the daemon's validation.
    pub fn load_from_daemon_config() -> Self {
        let config_path = std::env::var("RTP_STRATEGY_CONFIG")
            .ok()
            .map(std::path::PathBuf::from)
            .or_else(|| {
                // Resolve repo root relative to CARGO_MANIFEST_DIR (rtp/swarm/)
                let manifest = std::env::var("CARGO_MANIFEST_DIR").ok()?;
                Some(std::path::Path::new(&manifest).join("../../data/devnet-cycles/latest/config.json"))
            });

        let path = match config_path {
            Some(p) => p,
            None => return Self::default(),
        };

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                // The daemon writes a StrategyConfig which has the same fields
                // but is a different type. Parse generically.
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(v) => {
                        let base = Self::default();
                        let params = Self {
                            signal_threshold: clamp_param(
                                v.get("signal_threshold").and_then(|v| v.as_f64()),
                                0.1, 0.5, base.signal_threshold,
                            ),
                            tp_atr: clamp_param(
                                v.get("tp_atr").and_then(|v| v.as_f64()),
                                1.5, 5.0, base.tp_atr,
                            ),
                            sl_atr: clamp_param(
                                v.get("sl_atr").and_then(|v| v.as_f64()),
                                0.5, 3.0, base.sl_atr,
                            ),
                            max_hold_hours: clamp_param(
                                v.get("max_hold_hours").and_then(|v| v.as_f64()),
                                12.0, 72.0, base.max_hold_hours,
                            ),
                            trailing_stop_atr: clamp_param(
                                v.get("trailing_stop_atr").and_then(|v| v.as_f64()),
                                0.2, 1.5, base.trailing_stop_atr,
                            ),
                            time_decay_hours: base.time_decay_hours,
                            min_alignment: base.min_alignment,
                        };
                        tracing::info!(
                            "[STRATEGY] loaded from daemon config: signal={:.2} tp={:.1} sl={:.1} hold={:.0}h trail={:.2}",
                            params.signal_threshold, params.tp_atr, params.sl_atr,
                            params.max_hold_hours, params.trailing_stop_atr,
                        );
                        params
                    }
                    Err(e) => {
                        tracing::warn!("[STRATEGY] parse error on {}: {}. Using defaults.", path.display(), e);
                        Self::default()
                    }
                }
            }
            Err(e) => {
                tracing::info!(
                    "[STRATEGY] no daemon config at {} ({}). Using defaults.",
                    path.display(), e,
                );
                Self::default()
            }
        }
    }
}

/// Clamp a parameter value to soulcontract bounds.
/// Returns the default if the parsed value is None or out of range.
fn clamp_param(parsed: Option<f64>, min: f64, max: f64, default: f64) -> f64 {
    match parsed {
        Some(v) if v >= min && v <= max => v,
        Some(v) => {
            tracing::warn!(
                "[STRATEGY] clamped out-of-bounds param: {} not in [{}, {}]",
                v, min, max,
            );
            default
        }
        None => default,
    }
}

/// Computed multi-TF confluence score and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalResult {
    pub score: f64,
    pub reasons: Vec<String>,
    pub rsi: f64,
    pub atr: f64,
    pub bullish_count: usize,
    pub bearish_count: usize,
    pub bb_pos: f64,
    pub vol_ratio: f64,
}

/// Compute the multi-TF confluence score.
/// Returns None if not enough data.
pub fn compute_signal(closes: &[f64], volumes: &[f64]) -> Option<SignalResult> {
    if closes.len() < 100 {
        return None;
    }

    // Multi-timeframe trends
    let tf_1h = timeframe_signal(closes, 20);
    let tf_4h = timeframe_signal(closes, 80);
    let tf_1d = timeframe_signal(closes, 200);

    // If we don't have enough for daily, degrade gracefully
    let tf_1h = match tf_1h {
        Some(t) => t,
        None => return None,
    };
    let tf_4h = match tf_4h {
        Some(t) => t,
        None => {
            // Not enough for 4h — can still compute with what we have
            return None;
        }
    };

    let bullish_count = [&tf_1h, &tf_4h]
        .iter()
        .filter(|t| t.trend == "bullish")
        .count()
        + match &tf_1d {
            Some(t) if t.trend == "bullish" => 1,
            _ => 0,
        };

    let bearish_count = [&tf_1h, &tf_4h]
        .iter()
        .filter(|t| t.trend == "bearish")
        .count()
        + match &tf_1d {
            Some(t) if t.trend == "bearish" => 1,
            _ => 0,
        };

    let rsi_val = rsi(closes, 14).unwrap_or(50.0);
    let atr = atr_proxy(closes, 20).unwrap_or_else(|| closes.last().copied().unwrap_or(100.0) * 0.02);
    let bb = bollinger_position(closes, 20).unwrap_or(0.5);
    let vol_r = if !volumes.is_empty() { volume_ratio(volumes, 20).unwrap_or(1.0) } else { 1.0 };

    let mut score = 0.0;
    let mut reasons = Vec::new();

    // 1. Multi-TF trend alignment (weight: 0.4)
    let min_align = 3; // require all 3 TFs aligned for high-conviction entries
    if bullish_count >= min_align {
        score += (bullish_count as f64 / 3.0) * 0.4;
        reasons.push(format!("tf_bull_{}", bullish_count));
        if vol_r > 1.3 {
            score += 0.1;
            reasons.push("vol_confirm".to_string());
        }
    } else if bearish_count >= min_align {
        score -= (bearish_count as f64 / 3.0) * 0.4;
        reasons.push(format!("tf_bear_{}", bearish_count));
        if vol_r > 1.3 {
            score -= 0.1;
            reasons.push("vol_confirm_bear".to_string());
        }
    }

    // 2. Mean reversion (weight: 0.3)
    let mr_signal: f64 = if rsi_val < 30.0 {
        0.3
    } else if rsi_val < 35.0 {
        match &tf_1d {
            Some(t) if t.trend == "bullish" => 0.2,
            _ => 0.0,
        }
    } else if rsi_val > 70.0 {
        -0.3
    } else if rsi_val > 65.0 {
        match &tf_1d {
            Some(t) if t.trend == "bearish" => -0.2,
            _ => 0.0,
        }
    } else {
        0.0
    };

    if mr_signal.abs() > 0.1 {
        score += mr_signal * 0.3;
        if mr_signal > 0.0 {
            reasons.push(if rsi_val < 30.0 { "rsi_oversold" } else { "rsi_near_oversold_daily_bull" }.to_string());
        } else {
            reasons.push(if rsi_val > 70.0 { "rsi_overbought" } else { "rsi_near_overbought_daily_bear" }.to_string());
        }
    }

    // 3. Momentum (weight: 0.15)
    let mom = tf_4h.momentum;
    if mom > 0.003 {
        score += 0.15;
        reasons.push("mom_up".to_string());
    } else if mom < -0.003 {
        score -= 0.15;
        reasons.push("mom_down".to_string());
    }

    // 4. Bollinger Band (weight: 0.15)
    if bb < 0.15 {
        score += 0.15;
        reasons.push("bb_lower".to_string());
    } else if bb > 0.85 {
        score -= 0.15;
        reasons.push("bb_upper".to_string());
    }

    Some(SignalResult {
        score,
        reasons,
        rsi: rsi_val,
        atr,
        bullish_count,
        bearish_count,
        bb_pos: bb,
        vol_ratio: vol_r,
    })
}

/// Exit reason when a position should be closed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExitReason {
    TrailingStop,
    StopLoss,
    TakeProfit,
    MaxHold,
    TimeDecay,
    ScoreFlip,
    MrTarget,
}

/// Check if an open position should be exited.
/// Returns Some(reason) if an exit is triggered.
pub fn check_exit(
    params: &StrategyParams,
    entry_price: f64,
    entry_time: i64,       // unix seconds
    peak_price: f64,
    entry_rsi: f64,
    current_price: f64,
    current_score: f64,
    current_rsi: f64,
    atr: f64,
    now_secs: i64,
) -> Option<ExitReason> {
    let hold_hours = (now_secs - entry_time) as f64 / 3600.0;
    let pnl_pct = if entry_price > 0.0 {
        (current_price - entry_price) / entry_price * 100.0
    } else {
        0.0
    };

    // Trailing stop
    if params.trailing_stop_atr > 0.0 && atr > 0.0 && entry_price > 0.0 {
        let trail_trigger = params.trailing_stop_atr * atr / entry_price * 100.0;
        let pullback_pct = (peak_price - current_price) / entry_price * 100.0;
        if pullback_pct >= trail_trigger && peak_price > entry_price {
            return Some(ExitReason::TrailingStop);
        }
    }

    // Hard stop loss
    if atr > 0.0 && entry_price > 0.0 {
        let sl_pct = params.sl_atr * atr / entry_price * 100.0;
        if pnl_pct <= -sl_pct {
            return Some(ExitReason::StopLoss);
        }
    }

    // Take profit
    if atr > 0.0 && entry_price > 0.0 {
        let tp_pct = params.tp_atr * atr / entry_price * 100.0;
        if pnl_pct >= tp_pct {
            return Some(ExitReason::TakeProfit);
        }
    }

    // Max hold time
    if hold_hours >= params.max_hold_hours {
        return Some(ExitReason::MaxHold);
    }

    // Time decay: exit losing positions after decay period
    if pnl_pct < 0.0 && hold_hours >= params.time_decay_hours {
        return Some(ExitReason::TimeDecay);
    }

    // Score flip (no delay — score_flip_delay_hrs = 0)
    if current_score < 0.0 {
        return Some(ExitReason::ScoreFlip);
    }

    // MR target: RSI was oversold at entry, now reverted to mean
    if current_rsi > 55.0 && entry_rsi < 35.0 {
        return Some(ExitReason::MrTarget);
    }

    None
}

/// Persistent state for the trader's open position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenPosition {
    pub entry_price: f64,
    pub entry_time: i64,
    pub peak_price: f64,
    pub entry_rsi: f64,
    pub entry_atr: f64,
    pub entry_score: f64,
    pub position_key: String, // Flash Trade position account pubkey
    pub size_usd: f64,
}

/// Completed trade record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub entry_price: f64,
    pub exit_price: f64,
    pub entry_time: i64,
    pub exit_time: i64,
    pub pnl_pct: f64,
    pub exit_reason: String,
    pub size_usd: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn compute_signal_with_uptrend() {
        // 200 candles of steady uptrend
        let closes: Vec<f64> = (0..200).map(|i| 100.0 + i as f64 * 0.5).collect();
        let volumes: Vec<f64> = vec![1000.0; 200];
        let result = compute_signal(&closes, &volumes).unwrap();
        assert!(result.score > 0.0, "Uptrend should have positive score, got {}", result.score);
        assert!(result.bullish_count >= 2);
    }

    #[test]
    fn compute_signal_with_downtrend() {
        let closes: Vec<f64> = (0..200).map(|i| 200.0 - i as f64 * 0.5).collect();
        let volumes: Vec<f64> = vec![1000.0; 200];
        let result = compute_signal(&closes, &volumes).unwrap();
        assert!(result.score < 0.0, "Downtrend should have negative score, got {}", result.score);
    }

    #[test]
    fn exit_trailing_stop() {
        let params = StrategyParams::default();
        let now = Utc::now().timestamp();
        let exit = check_exit(
            &params,
            100.0,       // entry
            now - 7200,  // 2h ago
            110.0,       // peak
            40.0,        // entry_rsi
            106.0,       // current price (pulled back from peak)
            -0.1,        // score
            40.0,        // rsi
            2.0,         // atr
            now,
        );
        // trailing_stop_atr=0.14, trigger = 0.14*2/100*100 = 0.28%, pullback = (110-106)/100*100 = 4%
        assert!(matches!(exit, Some(ExitReason::TrailingStop)));
    }

    #[test]
    fn exit_stop_loss() {
        let params = StrategyParams::default();
        let now = Utc::now().timestamp();
        let exit = check_exit(
            &params,
            100.0,
            now - 3600,
            100.0,
            50.0,
            90.0,  // 10% loss
            0.1,
            50.0,
            3.0,   // ATR=3 → sl = 2.7*3/100*100 = 8.1%
            now,
        );
        assert!(matches!(exit, Some(ExitReason::StopLoss)));
    }

    #[test]
    fn no_exit_when_profitable() {
        let params = StrategyParams::default();
        let now = Utc::now().timestamp();
        let exit = check_exit(
            &params,
            100.0,
            now - 1800,  // 30min ago
            102.0,
            50.0,
            101.5,  // 1.5% profit — well within trailing stop
            0.3,    // positive score
            50.0,
            5.0,    // larger ATR makes triggers looser
            now,
        );
        assert!(exit.is_none(), "Should not exit a healthy position");
    }
}
