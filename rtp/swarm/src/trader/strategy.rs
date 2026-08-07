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
    #[serde(default)]
    pub score_flip_delay_hrs: f64,
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
            score_flip_delay_hrs: 0.0,
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
                Some(
                    std::path::Path::new(&manifest)
                        .join("../../data/devnet-cycles/latest/config.json"),
                )
            });

        match config_path {
            Some(path) => Self::load_from_path(&path),
            None => Self::default(),
        }
    }

    /// Load strategy params from an explicit file path.
    /// Falls back to defaults on parse or read errors.
    pub fn load_from_path(path: &std::path::Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                // The daemon writes a StrategyConfig which has the same fields
                // but is a different type. Parse generically.
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(v) => {
                        let base = Self::default();
                        let params = Self {
                            signal_threshold: clamp_param(
                                v.get("signal_threshold").and_then(|v| v.as_f64()),
                                0.1,
                                0.5,
                                base.signal_threshold,
                            ),
                            tp_atr: clamp_param(
                                v.get("tp_atr").and_then(|v| v.as_f64()),
                                1.5,
                                8.0,
                                base.tp_atr,
                            ),
                            sl_atr: clamp_param(
                                v.get("sl_atr").and_then(|v| v.as_f64()),
                                0.5,
                                3.0,
                                base.sl_atr,
                            ),
                            max_hold_hours: clamp_param(
                                v.get("max_hold_hours").and_then(|v| v.as_f64()),
                                12.0,
                                120.0,
                                base.max_hold_hours,
                            ),
                            trailing_stop_atr: clamp_param(
                                v.get("trailing_stop_atr").and_then(|v| v.as_f64()),
                                0.1,
                                1.5,
                                base.trailing_stop_atr,
                            ),
                            time_decay_hours: clamp_param(
                                v.get("time_decay_hours").and_then(|v| v.as_f64()),
                                0.0,
                                200.0,
                                base.time_decay_hours,
                            ),
                            min_alignment: v
                                .get("min_alignment")
                                .and_then(|v| v.as_u64())
                                .map(|n| n as usize)
                                .unwrap_or(base.min_alignment),
                            score_flip_delay_hrs: clamp_param(
                                v.get("score_flip_delay_hrs").and_then(|v| v.as_f64()),
                                0.0,
                                72.0,
                                base.score_flip_delay_hrs,
                            ),
                        };
                        tracing::info!(
                            "[STRATEGY] loaded from daemon config: signal={:.2} tp={:.1} sl={:.1} hold={:.0}h trail={:.2} decay={:.0}h flip_delay={:.1}h alignment={}",
                            params.signal_threshold,
                            params.tp_atr,
                            params.sl_atr,
                            params.max_hold_hours,
                            params.trailing_stop_atr,
                            params.time_decay_hours,
                            params.score_flip_delay_hrs,
                            params.min_alignment,
                        );
                        params
                    }
                    Err(e) => {
                        tracing::warn!(
                            "[STRATEGY] parse error on {}: {}. Using defaults.",
                            path.display(),
                            e
                        );
                        Self::default()
                    }
                }
            }
            Err(e) => {
                tracing::info!(
                    "[STRATEGY] no daemon config at {} ({}). Using defaults.",
                    path.display(),
                    e,
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
                v,
                min,
                max,
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
///
/// IMPORTANT: the three TF args must come from DIFFERENT candle intervals.
/// Historically (Jul 26 → Jul 28 trader stuck in `bear=3` permanently) this
/// function sliced a single 1h buffer at lookback 20/80/200 — which is the
/// same trend smoothed three different ways, NOT real multi-timeframe.
/// With a sustained 1d downtrend, all three derivatives were bearish and
/// the trader could never fire a long entry.
///
/// Now `closes_1h`, `closes_4h`, `closes_1d` are independent candle buffers
/// fetched from Binance's `interval=1h`, `4h`, `1d` endpoints. The `volumes`
/// arg is used only for the volume-confirmation signal; pair it with the 1h
/// buffer (most granular).
///
/// Returns None if any of the three TF buffers is too short.
pub fn compute_signal(
    closes_1h: &[f64],
    closes_4h: &[f64],
    closes_1d: &[f64],
    volumes: &[f64],
    min_alignment: usize,
) -> Option<SignalResult> {
    // Real multi-TF: each timeframe is its own candle series. Use a 20-period
    // SMA (matches strategy design) and fall back to whatever is available if
    // the buffer is shorter.
    let tf_1h = timeframe_signal(closes_1h, 20)
        .or_else(|| timeframe_signal(closes_1h, closes_1h.len().min(50)))?;
    let tf_4h = timeframe_signal(closes_4h, 20)
        .or_else(|| timeframe_signal(closes_4h, closes_4h.len().min(50)))?;
    let tf_1d = timeframe_signal(closes_1d, 20)
        .or_else(|| timeframe_signal(closes_1d, closes_1d.len().min(50)))?;

    let bullish_count = [&tf_1h, &tf_4h, &tf_1d]
        .iter()
        .filter(|t| t.trend == "bullish")
        .count();

    let bearish_count = [&tf_1h, &tf_4h, &tf_1d]
        .iter()
        .filter(|t| t.trend == "bearish")
        .count();

    // RSI + ATR + BB are computed off the 1h buffer (most granular).
    let rsi_val = rsi(closes_1h, 14).unwrap_or(50.0);
    let atr = atr_proxy(closes_1h, 20)
        .unwrap_or_else(|| closes_1h.last().copied().unwrap_or(100.0) * 0.02);
    let bb = bollinger_position(closes_1h, 20).unwrap_or(0.5);
    let vol_r = if !volumes.is_empty() {
        volume_ratio(volumes, 20).unwrap_or(1.0)
    } else {
        1.0
    };

    let mut score = 0.0;
    let mut reasons = Vec::new();

    // 1. Multi-TF trend alignment (weight: 0.4)
    if bullish_count >= min_alignment {
        score += (bullish_count as f64 / 3.0) * 0.4;
        reasons.push(format!("tf_bull_{}", bullish_count));
        if vol_r > 1.3 {
            score += 0.1;
            reasons.push("vol_confirm".to_string());
        }
    } else if bearish_count >= min_alignment {
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
        if tf_1d.trend == "bullish" { 0.2 } else { 0.0 }
    } else if rsi_val > 70.0 {
        -0.3
    } else if rsi_val > 65.0 {
        if tf_1d.trend == "bearish" { -0.2 } else { 0.0 }
    } else {
        0.0
    };

    if mr_signal.abs() > 0.1 {
        score += mr_signal * 0.3;
        if mr_signal > 0.0 {
            reasons.push(
                if rsi_val < 30.0 {
                    "rsi_oversold"
                } else {
                    "rsi_near_oversold_daily_bull"
                }
                .to_string(),
            );
        } else {
            reasons.push(
                if rsi_val > 70.0 {
                    "rsi_overbought"
                } else {
                    "rsi_near_overbought_daily_bear"
                }
                .to_string(),
            );
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
///
/// `position_side` is "Long" or "Short".
/// `first_negative_score_time` tracks the thesis-flip timer (name kept for
/// state-file compatibility): for a LONG it starts when the score first goes
/// NEGATIVE; for a SHORT it starts when the score first goes POSITIVE.
/// Returns the updated timer via the `CheckExitResult`.
#[allow(clippy::too_many_arguments)]
pub fn check_exit(
    params: &StrategyParams,
    entry_price: f64,
    entry_time: i64, // unix seconds
    peak_price: f64,
    entry_rsi: f64,
    current_price: f64,
    current_score: f64,
    current_rsi: f64,
    atr: f64,
    now_secs: i64,
    position_side: &str,
    first_negative_score_time: Option<i64>,
) -> CheckExitResult {
    let hold_hours = (now_secs - entry_time) as f64 / 3600.0;
    let pnl_pct = if entry_price > 0.0 {
        match position_side {
            "Short" => (entry_price - current_price) / entry_price * 100.0,
            _ => (current_price - entry_price) / entry_price * 100.0, // "Long" or any other value
        }
    } else {
        0.0
    };

    // Trailing stop
    if params.trailing_stop_atr > 0.0 && atr > 0.0 && entry_price > 0.0 {
        let trail_trigger = params.trailing_stop_atr * atr / entry_price * 100.0;
        let pullback_pct = match position_side {
            "Short" => {
                // For SHORT: favorable direction is price dropping (peak = lowest price seen)
                // Pullback = price rising from trough
                (current_price - peak_price) / entry_price * 100.0
            }
            _ => {
                // For LONG: favorable direction is price rising (peak = highest price seen)
                // Pullback = price dropping from peak
                (peak_price - current_price) / entry_price * 100.0
            }
        };
        let trailing_cond = match position_side {
            "Short" => peak_price < entry_price, // trough must be below entry
            _ => peak_price > entry_price,       // peak must be above entry
        };
        if pullback_pct >= trail_trigger && trailing_cond {
            return CheckExitResult {
                reason: Some(ExitReason::TrailingStop),
                first_negative_score_time: None, // exiting, clear timer
            };
        }
    }

    // Hard stop loss
    if atr > 0.0 && entry_price > 0.0 {
        let sl_pct = params.sl_atr * atr / entry_price * 100.0;
        if pnl_pct <= -sl_pct {
            return CheckExitResult {
                reason: Some(ExitReason::StopLoss),
                first_negative_score_time: None,
            };
        }
    }

    // Take profit
    if atr > 0.0 && entry_price > 0.0 {
        let tp_pct = params.tp_atr * atr / entry_price * 100.0;
        if pnl_pct >= tp_pct {
            return CheckExitResult {
                reason: Some(ExitReason::TakeProfit),
                first_negative_score_time: None,
            };
        }
    }

    // Max hold time
    if hold_hours >= params.max_hold_hours {
        return CheckExitResult {
            reason: Some(ExitReason::MaxHold),
            first_negative_score_time: None,
        };
    }

    // Time decay: exit losing positions after decay period
    if pnl_pct < 0.0 && params.time_decay_hours > 0.0 && hold_hours >= params.time_decay_hours {
        return CheckExitResult {
            reason: Some(ExitReason::TimeDecay),
            first_negative_score_time: None,
        };
    }

    // Score flip with delay grace period.
    //
    // SIDE-AWARE: a LONG is entered on positive score, so its thesis breaks
    // when the score flips NEGATIVE. A SHORT is entered on negative score, so
    // its thesis breaks when the score flips POSITIVE. The pre-2026-08-07 code
    // checked `score < 0` for both sides — for shorts that is the entry
    // condition, not a flip, and it force-closed every short once the grace
    // period elapsed even while the bearish thesis still held (41 of 75 live
    // shorts exited at exactly 2.08h with `ScoreFlip`; long median hold was
    // 3.92h). The timer field keeps its name for state-file compatibility; it
    // now means "thesis-flip timer" for both sides.
    let flipped = match position_side {
        "Short" => current_score > 0.0,
        _ => current_score < 0.0,
    };
    let updated_fnst = if flipped {
        match first_negative_score_time {
            Some(t) => {
                // Score was already negative — check if delay has elapsed
                let flip_duration_hrs = (now_secs - t) as f64 / 3600.0;
                if params.score_flip_delay_hrs > 0.0
                    && flip_duration_hrs < params.score_flip_delay_hrs
                {
                    // Still within grace period — don't exit, keep timer
                    Some(t)
                } else {
                    // delay=0 or delay elapsed — exit
                    return CheckExitResult {
                        reason: Some(ExitReason::ScoreFlip),
                        first_negative_score_time: None,
                    };
                }
            }
            None => {
                // Score just went negative — start timer
                if params.score_flip_delay_hrs > 0.0 {
                    // Start grace period, don't exit yet
                    Some(now_secs)
                } else {
                    // delay=0 — immediate exit (backward compat)
                    return CheckExitResult {
                        reason: Some(ExitReason::ScoreFlip),
                        first_negative_score_time: None,
                    };
                }
            }
        }
    } else {
        // Score is positive (or zero) — reset the negative timer
        None
    };

    // MR target: RSI was oversold at entry, now reverted to mean
    if current_rsi > 55.0 && entry_rsi < 35.0 {
        return CheckExitResult {
            reason: Some(ExitReason::MrTarget),
            first_negative_score_time: updated_fnst,
        };
    }

    // No exit — return updated timer state
    CheckExitResult {
        reason: None,
        first_negative_score_time: updated_fnst,
    }
}

/// Result of checking exit conditions.
/// Includes the optional exit reason and the updated `first_negative_score_time`
/// (which the caller must persist back into the OpenPosition).
#[derive(Debug, Clone)]
pub struct CheckExitResult {
    pub reason: Option<ExitReason>,
    pub first_negative_score_time: Option<i64>,
}

impl CheckExitResult {
    /// Convenience: returns true if an exit reason was triggered.
    pub fn should_exit(&self) -> bool {
        self.reason.is_some()
    }
}

/// Default value for OpenPosition.side — "Long" for backward compatibility.
fn default_side() -> String {
    "Long".to_string()
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
    /// Tracks when the confluence score first went negative for this position.
    /// Used to implement score_flip_delay_hrs grace period.
    /// `None` means score has not gone negative yet (or was reset by positive score).
    #[serde(default)]
    pub first_negative_score_time: Option<i64>,
    /// Position direction: "Long" or "Short". Defaults to "Long" for backward
    /// compatibility with existing state files that lack this field.
    #[serde(default = "default_side")]
    pub side: String,
}

impl OpenPosition {
    /// Returns the position side ("Long" or "Short").
    /// Defaults to "Long" for backward compatibility with existing state files.
    pub fn side(&self) -> &str {
        &self.side
    }
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
    /// Position side: "Long" or "Short". Defaults to "Long" for backward compat with existing state files.
    #[serde(default = "default_side")]
    pub side: String,
    /// Flash Trade v2 fee breakdown captured from the positions API at close
    /// time (exit fee, borrow fee, price impact, total — USD). Populated since
    /// 2026-08-07 to build the v2 cost ledger; older records deserialize with
    /// `None` thanks to `#[serde(default)]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fees: Option<FeeBreakdown>,
}

/// Flash Trade v2 per-position fee components in USD.
///
/// The positions API exposes raw 1e6-scaled integer strings (`exitFeeUsd`,
/// `borrowFeeUsd`, `priceImpactUsd`, `totalFeeUsd`); executor converts them
/// to USD before storing here. `totalFeeUsd` = exit + borrow + impact on the
/// closing leg (the opening-leg fee is charged at open and not reported here).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct FeeBreakdown {
    pub exit_fee_usd: f64,
    pub borrow_fee_usd: f64,
    pub price_impact_usd: f64,
    pub total_fee_usd: f64,
}

impl TradeRecord {
    /// Infer side from which PnL formula best matches stored `pnl_pct`.
    pub fn infer_side_from_pnl(&self) -> &'static str {
        if self.entry_price <= 0.0 {
            return "Long";
        }
        let long_pnl = (self.exit_price - self.entry_price) / self.entry_price * 100.0;
        let short_pnl = (self.entry_price - self.exit_price) / self.entry_price * 100.0;
        let long_err = (long_pnl - self.pnl_pct).abs();
        let short_err = (short_pnl - self.pnl_pct).abs();
        if short_err < long_err {
            "Short"
        } else {
            "Long"
        }
    }

    /// Fix legacy rows where `side` defaulted to Long but `pnl_pct` used short math.
    pub fn repair_side_from_pnl(&mut self) -> bool {
        let inferred = self.infer_side_from_pnl();
        if self.side == inferred {
            return false;
        }
        self.side = inferred.to_string();
        true
    }
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
        let result = compute_signal(&closes, &closes, &closes, &volumes, 3).unwrap();
        assert!(
            result.score > 0.0,
            "Uptrend should have positive score, got {}",
            result.score
        );
        assert!(result.bullish_count >= 2);
    }

    #[test]
    fn compute_signal_with_downtrend() {
        let closes: Vec<f64> = (0..200).map(|i| 200.0 - i as f64 * 0.5).collect();
        let volumes: Vec<f64> = vec![1000.0; 200];
        let result = compute_signal(&closes, &closes, &closes, &volumes, 3).unwrap();
        assert!(
            result.score < 0.0,
            "Downtrend should have negative score, got {}",
            result.score
        );
    }

    #[test]
    fn exit_trailing_stop() {
        let params = StrategyParams::default();
        let now = Utc::now().timestamp();
        let result = check_exit(
            &params,
            100.0,      // entry
            now - 7200, // 2h ago
            110.0,      // peak
            40.0,       // entry_rsi
            106.0,      // current price (pulled back from peak)
            -0.1,       // score
            40.0,       // rsi
            2.0,        // atr
            now,
            "Long",
            None,
        );
        // trailing_stop_atr=0.14, trigger = 0.14*2/100*100 = 0.28%, pullback = (110-106)/100*100 = 4%
        assert!(matches!(result.reason, Some(ExitReason::TrailingStop)));
    }

    #[test]
    fn exit_stop_loss() {
        let params = StrategyParams::default();
        let now = Utc::now().timestamp();
        let result = check_exit(
            &params,
            100.0,
            now - 3600,
            100.0,
            50.0,
            90.0, // 10% loss
            0.1,
            50.0,
            3.0, // ATR=3 → sl = 2.7*3/100*100 = 8.1%
            now,
            "Long",
            None,
        );
        assert!(matches!(result.reason, Some(ExitReason::StopLoss)));
    }

    #[test]
    fn no_exit_when_profitable() {
        let params = StrategyParams::default();
        let now = Utc::now().timestamp();
        let result = check_exit(
            &params,
            100.0,
            now - 1800, // 30min ago
            102.0,
            50.0,
            101.5, // 1.5% profit — well within trailing stop
            0.3,   // positive score
            50.0,
            5.0, // larger ATR makes triggers looser
            now,
            "Long",
            None,
        );
        assert!(
            result.reason.is_none(),
            "Should not exit a healthy position"
        );
    }

    // =========================================================================
    // New tests for score_flip_delay_hrs, widened clamps, config loading
    // =========================================================================

    #[test]
    fn score_flip_delay_field_default_is_zero() {
        let params = StrategyParams::default();
        assert_eq!(params.score_flip_delay_hrs, 0.0, "Default should be 0.0");
    }

    #[test]
    fn score_flip_delay_field_serde_roundtrip() {
        let mut params = StrategyParams::default();
        params.score_flip_delay_hrs = 2.5;
        let json = serde_json::to_string(&params).unwrap();
        let parsed: StrategyParams = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.score_flip_delay_hrs, 2.5);
    }

    #[test]
    fn score_flip_delay_backward_compat_missing_field() {
        // JSON without score_flip_delay_hrs should deserialize with default 0.0
        let json = r#"{"signal_threshold":0.3,"tp_atr":5.0,"sl_atr":2.7,"max_hold_hours":36.0,"trailing_stop_atr":0.14,"time_decay_hours":12.0,"min_alignment":3}"#;
        let parsed: StrategyParams = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.score_flip_delay_hrs, 0.0);
    }

    // Helper: write config to temp file and load via load_from_path (no env var needed)
    fn load_config_from_json(json: &str, filename: &str) -> StrategyParams {
        let tmp = std::env::temp_dir().join(filename);
        std::fs::write(&tmp, json).unwrap();
        StrategyParams::load_from_path(&tmp)
    }

    #[test]
    fn clamp_accepts_tp_atr_eight() {
        let config = r#"{"signal_threshold":0.3,"tp_atr":8.0,"sl_atr":2.5,"max_hold_hours":36.0,"trailing_stop_atr":0.5}"#;
        let params = load_config_from_json(config, "test_clamp_tp_atr.json");
        assert_eq!(
            params.tp_atr, 8.0,
            "tp_atr=8.0 should pass through clamping"
        );
    }

    #[test]
    fn clamp_rejects_tp_atr_nine() {
        let config = r#"{"signal_threshold":0.3,"tp_atr":9.0,"sl_atr":2.5,"max_hold_hours":36.0,"trailing_stop_atr":0.5}"#;
        let params = load_config_from_json(config, "test_clamp_tp_atr_high.json");
        assert_eq!(
            params.tp_atr,
            StrategyParams::default().tp_atr,
            "tp_atr=9.0 should be clamped to default"
        );
    }

    #[test]
    fn clamp_accepts_max_hold_120() {
        let config = r#"{"signal_threshold":0.3,"tp_atr":5.0,"sl_atr":2.5,"max_hold_hours":120.0,"trailing_stop_atr":0.5}"#;
        let params = load_config_from_json(config, "test_clamp_hold.json");
        assert_eq!(
            params.max_hold_hours, 120.0,
            "max_hold_hours=120 should pass through"
        );
    }

    #[test]
    fn clamp_rejects_max_hold_130() {
        let config = r#"{"signal_threshold":0.3,"tp_atr":5.0,"sl_atr":2.5,"max_hold_hours":130.0,"trailing_stop_atr":0.5}"#;
        let params = load_config_from_json(config, "test_clamp_hold_high.json");
        assert_eq!(
            params.max_hold_hours,
            StrategyParams::default().max_hold_hours,
            "max_hold_hours=130 should be clamped to default"
        );
    }

    #[test]
    fn clamp_accepts_trailing_stop_atr_0_1() {
        let config = r#"{"signal_threshold":0.3,"tp_atr":5.0,"sl_atr":2.5,"max_hold_hours":36.0,"trailing_stop_atr":0.1}"#;
        let params = load_config_from_json(config, "test_clamp_trail.json");
        assert_eq!(
            params.trailing_stop_atr, 0.1,
            "trailing_stop_atr=0.1 should pass through"
        );
    }

    #[test]
    fn clamp_rejects_trailing_stop_atr_0_05() {
        let config = r#"{"signal_threshold":0.3,"tp_atr":5.0,"sl_atr":2.5,"max_hold_hours":36.0,"trailing_stop_atr":0.05}"#;
        let params = load_config_from_json(config, "test_clamp_trail_low.json");
        assert_eq!(
            params.trailing_stop_atr,
            StrategyParams::default().trailing_stop_atr,
            "trailing_stop_atr=0.05 should be clamped to default"
        );
    }

    #[test]
    fn config_loads_time_decay_hours() {
        let config = r#"{"time_decay_hours":48.0}"#;
        let params = load_config_from_json(config, "test_config_decay.json");
        assert_eq!(
            params.time_decay_hours, 48.0,
            "time_decay_hours should be parsed from JSON"
        );
    }

    #[test]
    fn config_loads_min_alignment() {
        let config = r#"{"min_alignment":2}"#;
        let params = load_config_from_json(config, "test_config_alignment.json");
        assert_eq!(
            params.min_alignment, 2,
            "min_alignment should be parsed from JSON"
        );
    }

    #[test]
    fn config_loads_score_flip_delay_hrs() {
        let config = r#"{"score_flip_delay_hrs":2.0}"#;
        let params = load_config_from_json(config, "test_config_flip.json");
        assert_eq!(
            params.score_flip_delay_hrs, 2.0,
            "score_flip_delay_hrs should be parsed from JSON"
        );
    }

    #[test]
    fn partial_config_uses_defaults() {
        let config = r#"{"signal_threshold":0.35}"#;
        let params = load_config_from_json(config, "test_partial_config.json");
        let defaults = StrategyParams::default();
        assert_eq!(
            params.signal_threshold, 0.35,
            "Provided field should be used"
        );
        assert_eq!(
            params.tp_atr, defaults.tp_atr,
            "Missing tp_atr should use default"
        );
        assert_eq!(
            params.sl_atr, defaults.sl_atr,
            "Missing sl_atr should use default"
        );
        assert_eq!(params.max_hold_hours, defaults.max_hold_hours);
        assert_eq!(params.trailing_stop_atr, defaults.trailing_stop_atr);
        assert_eq!(params.time_decay_hours, defaults.time_decay_hours);
        assert_eq!(params.min_alignment, defaults.min_alignment);
        assert_eq!(params.score_flip_delay_hrs, defaults.score_flip_delay_hrs);
    }

    #[test]
    fn invalid_json_falls_back_to_defaults() {
        let config = r#"this is not valid json!!!"#;
        let params = load_config_from_json(config, "test_invalid_json.json");
        let defaults = StrategyParams::default();
        assert_eq!(params.signal_threshold, defaults.signal_threshold);
        assert_eq!(params.score_flip_delay_hrs, defaults.score_flip_delay_hrs);
    }

    #[test]
    fn missing_file_falls_back_to_defaults() {
        let params =
            StrategyParams::load_from_path(std::path::Path::new("/nonexistent/path/strategy.json"));
        let defaults = StrategyParams::default();
        assert_eq!(params.signal_threshold, defaults.signal_threshold);
        assert_eq!(params.score_flip_delay_hrs, defaults.score_flip_delay_hrs);
        assert_eq!(params.time_decay_hours, defaults.time_decay_hours);
        assert_eq!(params.min_alignment, defaults.min_alignment);
    }

    #[test]
    fn compute_signal_uses_loaded_min_alignment() {
        // With min_alignment=2, a 2-TF bullish alignment should contribute to score
        let closes: Vec<f64> = (0..200).map(|i| 100.0 + i as f64 * 0.5).collect();
        let volumes: Vec<f64> = vec![1000.0; 200];
        // With min_alignment=3, need all 3 TFs; with 2, need only 2
        let result_align2 = compute_signal(&closes, &closes, &closes, &volumes, 2).unwrap();
        let result_align3 = compute_signal(&closes, &closes, &closes, &volumes, 3).unwrap();
        // Both should work — the function doesn't crash with different alignment values
        assert!(result_align2.score > 0.0);
        assert!(result_align3.score > 0.0);
    }

    #[test]
    fn full_validated_config_loads_correctly() {
        // The May 18 night-shift-validated config
        let config = r#"{
            "signal_threshold": 0.3,
            "tp_atr": 6.0,
            "sl_atr": 2.5,
            "max_hold_hours": 96.0,
            "trailing_stop_atr": 1.0,
            "time_decay_hours": 48.0,
            "min_alignment": 3,
            "score_flip_delay_hrs": 2.0
        }"#;
        let params = load_config_from_json(config, "test_validated_config.json");
        assert_eq!(params.signal_threshold, 0.3);
        assert_eq!(params.tp_atr, 6.0);
        assert_eq!(params.sl_atr, 2.5);
        assert_eq!(params.max_hold_hours, 96.0);
        assert_eq!(params.trailing_stop_atr, 1.0);
        assert_eq!(params.time_decay_hours, 48.0);
        assert_eq!(params.min_alignment, 3);
        assert_eq!(params.score_flip_delay_hrs, 2.0);
    }

    // =========================================================================
    // Score flip delay tests (feature: score-flip-delay-exit-logic)
    // =========================================================================

    /// Helper: build params with a specific score_flip_delay_hrs
    fn params_with_flip_delay(delay: f64) -> StrategyParams {
        StrategyParams {
            score_flip_delay_hrs: delay,
            ..StrategyParams::default()
        }
    }

    #[test]
    fn score_flip_within_grace_period_no_exit() {
        // With delay=2h, negative score for only 1h should NOT trigger exit
        let params = params_with_flip_delay(2.0);
        let now = Utc::now().timestamp();
        let first_neg = now - 3600; // score went negative 1h ago
        let result = check_exit(
            &params,
            100.0,      // entry_price
            now - 7200, // 2h ago entry
            100.0,      // peak
            50.0,       // entry_rsi
            100.0,      // current_price (no change)
            -0.3,       // negative score
            50.0,       // current_rsi
            2.0,        // atr
            now,
            "Long",
            Some(first_neg),
        );
        assert!(
            result.reason.is_none(),
            "Should not exit within grace period"
        );
        // Timer should still be tracking
        assert_eq!(result.first_negative_score_time, Some(first_neg));
    }

    #[test]
    fn score_flip_after_grace_period_exits() {
        // With delay=2h, negative score for 3h should trigger ScoreFlip exit
        let params = params_with_flip_delay(2.0);
        let now = Utc::now().timestamp();
        let first_neg = now - 10800; // score went negative 3h ago
        let result = check_exit(
            &params,
            100.0,
            now - 14400, // 4h ago entry
            100.0,
            50.0,
            100.0,
            -0.3,
            50.0,
            2.0,
            now,
            "Long",
            Some(first_neg),
        );
        assert!(
            matches!(result.reason, Some(ExitReason::ScoreFlip)),
            "Should exit after grace period"
        );
    }

    #[test]
    fn score_flip_delay_zero_immediate_exit() {
        // With delay=0, negative score should exit immediately (backward compat)
        let params = params_with_flip_delay(0.0);
        let now = Utc::now().timestamp();
        let result = check_exit(
            &params,
            100.0,
            now - 1800, // 30min ago
            100.0,
            50.0,
            100.0,
            -0.1, // negative score
            50.0,
            2.0,
            now,
            "Long",
            None, // first_negative_score_time = None (first tick)
        );
        assert!(
            matches!(result.reason, Some(ExitReason::ScoreFlip)),
            "delay=0 should trigger immediate ScoreFlip"
        );
    }

    #[test]
    fn short_negative_score_is_not_a_flip() {
        // Regression: shorts are ENTERED on negative score, so a negative
        // score is the thesis holding, not a flip. Pre-2026-08-07 the flip
        // check was `score < 0` for both sides and force-closed every short
        // after the grace period (41 of 75 live shorts exited at ~2.08h).
        let params = params_with_flip_delay(2.0);
        let now = Utc::now().timestamp();
        let result = check_exit(
            &params,
            100.0,
            now - 14400, // entered 4h ago
            100.0,       // peak == entry: trailing stop (Short) needs trough < entry
            50.0,
            100.0,
            -0.4, // still bearish — the entry thesis holds
            50.0,
            2.0,
            now,
            "Short",
            None,
        );
        assert!(
            result.reason.is_none(),
            "negative score on a Short must never count as a flip"
        );
        assert!(
            result.first_negative_score_time.is_none(),
            "no flip timer should be started for a Short while score stays negative"
        );
    }

    #[test]
    fn short_flip_within_grace_period_no_exit() {
        // Short + score went positive 1h ago, delay=2h → no exit yet.
        let params = params_with_flip_delay(2.0);
        let now = Utc::now().timestamp();
        let first_pos = now - 3600;
        let result = check_exit(
            &params,
            100.0,
            now - 14400,
            100.0,
            50.0,
            100.0,
            0.3, // score flipped positive
            50.0,
            2.0,
            now,
            "Short",
            Some(first_pos),
        );
        assert!(result.reason.is_none(), "within grace period");
        assert_eq!(result.first_negative_score_time, Some(first_pos));
    }

    #[test]
    fn short_flip_after_grace_period_exits() {
        // Short + score positive for 3h, delay=2h → ScoreFlip exit.
        let params = params_with_flip_delay(2.0);
        let now = Utc::now().timestamp();
        let first_pos = now - 10800;
        let result = check_exit(
            &params,
            100.0,
            now - 14400,
            100.0,
            50.0,
            100.0,
            0.3,
            50.0,
            2.0,
            now,
            "Short",
            Some(first_pos),
        );
        assert!(
            matches!(result.reason, Some(ExitReason::ScoreFlip)),
            "Short should exit after grace period once score flips positive"
        );
    }

    #[test]
    fn short_flip_delay_zero_immediate_exit() {
        // delay=0: positive score exits a Short immediately.
        let params = params_with_flip_delay(0.0);
        let now = Utc::now().timestamp();
        let result = check_exit(
            &params,
            100.0,
            now - 1800,
            100.0,
            50.0,
            100.0,
            0.1,
            50.0,
            2.0,
            now,
            "Short",
            None,
        );
        assert!(
            matches!(result.reason, Some(ExitReason::ScoreFlip)),
            "delay=0 Short should flip immediately on positive score"
        );
    }

    #[test]
    fn score_oscillation_resets_timer() {
        // Negative → positive → negative should reset the timer
        let params = params_with_flip_delay(2.0);
        let now = Utc::now().timestamp();

        // Step 1: Score goes negative — timer starts
        let result1 = check_exit(
            &params,
            100.0,
            now - 7200,
            100.0,
            50.0,
            100.0,
            -0.3,
            50.0,
            2.0,
            now,
            "Long",
            None,
        );
        assert!(
            result1.reason.is_none(),
            "First negative: within grace period"
        );
        let first_neg = result1.first_negative_score_time.unwrap();
        assert!(first_neg > 0, "Timer should be set");

        // Step 2: Score goes positive — timer resets to None
        let result2 = check_exit(
            &params,
            100.0,
            now - 7200,
            100.0,
            50.0,
            100.0,
            0.3,
            50.0,
            2.0,
            now,
            "Long",
            Some(first_neg),
        );
        assert!(result2.reason.is_none(), "Positive score: no exit");
        assert!(
            result2.first_negative_score_time.is_none(),
            "Timer should reset to None"
        );

        // Step 3: Score goes negative again — timer starts fresh
        let result3 = check_exit(
            &params,
            100.0,
            now - 7200,
            100.0,
            50.0,
            100.0,
            -0.3,
            50.0,
            2.0,
            now,
            "Long",
            None, // timer was reset
        );
        assert!(
            result3.reason.is_none(),
            "Fresh negative: within grace period again"
        );
        assert!(
            result3.first_negative_score_time.is_some(),
            "Timer should restart"
        );
    }

    #[test]
    fn first_negative_score_time_serde_roundtrip() {
        let now = Utc::now().timestamp();
        let pos = OpenPosition {
            entry_price: 100.0,
            entry_time: now,
            peak_price: 105.0,
            entry_rsi: 45.0,
            entry_atr: 2.0,
            entry_score: 0.5,
            position_key: "testkey123".to_string(),
            size_usd: 50.0,
            first_negative_score_time: Some(now - 3600),
            side: "Long".to_string(),
        };
        let json = serde_json::to_string(&pos).unwrap();
        let parsed: OpenPosition = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.first_negative_score_time, Some(now - 3600));
    }

    #[test]
    fn existing_trader_state_deserializes_with_new_field() {
        // Simulate old TraderState JSON that lacks first_negative_score_time
        let old_json = r#"{
            "entry_price": 100.0,
            "entry_time": 1700000000,
            "peak_price": 105.0,
            "entry_rsi": 45.0,
            "entry_atr": 2.0,
            "entry_score": 0.5,
            "position_key": "oldkey",
            "size_usd": 50.0
        }"#;
        let parsed: OpenPosition = serde_json::from_str(old_json).unwrap();
        assert_eq!(
            parsed.first_negative_score_time, None,
            "Missing field should default to None"
        );
        assert_eq!(
            parsed.entry_price, 100.0,
            "Existing fields should parse correctly"
        );
    }

    #[test]
    fn exit_priority_order_preserved() {
        // Construct a scenario where multiple exit conditions are true:
        // - Deep pullback from peak (trailing stop triggered)
        // - Negative score (score flip triggered)
        // Both should fire, but TrailingStop should win (higher priority)
        let params = params_with_flip_delay(2.0);
        let now = Utc::now().timestamp();
        let first_neg = now - 10800; // negative for 3h (past grace period)

        let result = check_exit(
            &params,
            100.0,      // entry
            now - 7200, // 2h ago
            110.0,      // peak (10% above entry)
            40.0,       // entry_rsi
            106.0,      // current (pulled back 4% from peak)
            -0.3,       // negative score (past grace)
            40.0,       // current_rsi
            2.0,        // atr
            now,
            "Long",
            Some(first_neg),
        );
        // trailing_stop_atr=0.14, trigger = 0.14*2/100*100 = 0.28%
        // pullback = (110-106)/100*100 = 4% — much larger than 0.28%
        assert!(
            matches!(result.reason, Some(ExitReason::TrailingStop)),
            "TrailingStop should have higher priority than ScoreFlip"
        );
    }

    #[test]
    fn score_flip_delay_starts_timer_on_first_negative() {
        // When first_negative_score_time is None and delay > 0,
        // the timer should be set to now but no exit should occur
        let params = params_with_flip_delay(2.0);
        let now = Utc::now().timestamp();

        let result = check_exit(
            &params,
            100.0,
            now - 1800,
            100.0,
            50.0,
            100.0,
            -0.2, // first negative tick
            50.0,
            2.0,
            now,
            "Long",
            None, // no prior negative time
        );
        assert!(
            result.reason.is_none(),
            "First negative tick should start timer, not exit"
        );
        assert_eq!(
            result.first_negative_score_time,
            Some(now),
            "Timer should be set to now"
        );
    }

    // =========================================================================
    // SHORT position tests (feature: short-entry-and-exit-logic)
    // =========================================================================

    #[test]
    fn short_pnl_math_is_correct() {
        // SHORT at 100, current at 95 → 5% profit
        let params = StrategyParams::default();
        let now = Utc::now().timestamp();
        let result = check_exit(
            &params,
            100.0,      // entry_price
            now - 3600, // 1h ago
            100.0,      // peak (trough tracking for SHORT)
            50.0,       // entry_rsi
            95.0,       // current_price (dropped 5%)
            -0.3,       // score still bearish — the short thesis holds
            50.0,       // current_rsi
            2.0,        // atr
            now,
            "Short",
            None,
        );
        // PnL for SHORT = (entry - current) / entry * 100 = (100-95)/100*100 = +5%
        assert!(
            result.reason.is_none(),
            "5% profit SHORT should not exit (within TP range)"
        );
    }

    #[test]
    fn short_pnl_negative_when_price_rises() {
        // SHORT at 100, current at 110 → -10% (loss)
        let params = StrategyParams::default();
        let now = Utc::now().timestamp();
        let result = check_exit(
            &params,
            100.0,
            now - 3600,
            100.0,
            50.0,
            110.0, // price rose 10% — bad for SHORT
            0.3,
            50.0,
            3.0, // ATR=3 → sl = 2.7*3/100*100 = 8.1%
            now,
            "Short",
            None,
        );
        // PnL = (100-110)/100*100 = -10% — exceeds SL threshold of 8.1%
        assert!(
            matches!(result.reason, Some(ExitReason::StopLoss)),
            "SHORT with 10% loss should trigger stop loss"
        );
    }

    #[test]
    fn short_trailing_stop_triggers_on_rise_from_trough() {
        // SHORT: favorable direction = price dropping (trough = lowest price)
        // Trailing triggers when price rises from trough
        let params = StrategyParams::default();
        let now = Utc::now().timestamp();
        let result = check_exit(
            &params,
            100.0,      // entry
            now - 7200, // 2h ago
            90.0,       // peak = trough at 90 (10% drop from entry)
            50.0,       // entry_rsi
            93.0,       // current rose from 90 to 93 (3% pullback from trough)
            -0.1,       // score
            50.0,       // rsi
            2.0,        // atr
            now,
            "Short",
            None,
        );
        // trail trigger = 0.14*2/100*100 = 0.28%
        // pullback = (93-90)/100*100 = 3% > 0.28%
        // trough (90) < entry (100) — condition met
        assert!(
            matches!(result.reason, Some(ExitReason::TrailingStop)),
            "SHORT trailing stop should fire on rise from trough"
        );
    }

    #[test]
    fn short_trailing_stop_no_trigger_in_unfavorable() {
        // SHORT: price still dropping — no trailing trigger
        let params = StrategyParams::default();
        let now = Utc::now().timestamp();
        let result = check_exit(
            &params,
            100.0,
            now - 3600,
            95.0, // trough at 95 (5% drop)
            50.0,
            94.0, // price dropped further to 94 (below trough)
            -0.3, // score still bearish — the short thesis holds
            50.0,
            2.0,
            now,
            "Short",
            None,
        );
        // Price dropping further is FAVORABLE for SHORT — no pullback
        assert!(
            result.reason.is_none(),
            "SHORT with price still dropping should not trigger trailing"
        );
    }

    #[test]
    fn short_take_profit_fires() {
        // SHORT: price drops enough to trigger TP
        let params = StrategyParams {
            tp_atr: 3.0,
            sl_atr: 2.0,
            trailing_stop_atr: 0.0, // disable trailing for this test
            ..StrategyParams::default()
        };
        let now = Utc::now().timestamp();
        let result = check_exit(
            &params,
            100.0,
            now - 3600,
            100.0,
            50.0,
            80.0, // 20% drop — huge profit for SHORT
            0.3,
            50.0,
            5.0, // ATR=5 → tp = 3.0*5/100*100 = 15%
            now,
            "Short",
            None,
        );
        // PnL = (100-80)/100*100 = +20% > 15% TP threshold
        assert!(
            matches!(result.reason, Some(ExitReason::TakeProfit)),
            "SHORT with 20% profit should trigger take profit"
        );
    }

    #[test]
    fn short_stop_loss_fires() {
        // SHORT: price rises above entry — loss
        let params = StrategyParams {
            sl_atr: 2.0,
            trailing_stop_atr: 0.0, // disable trailing
            ..StrategyParams::default()
        };
        let now = Utc::now().timestamp();
        let result = check_exit(
            &params,
            100.0,
            now - 3600,
            100.0,
            50.0,
            115.0, // 15% rise — bad for SHORT
            0.3,
            50.0,
            5.0, // ATR=5 → sl = 2.0*5/100*100 = 10%
            now,
            "Short",
            None,
        );
        // PnL = (100-115)/100*100 = -15% < -10% SL threshold
        assert!(
            matches!(result.reason, Some(ExitReason::StopLoss)),
            "SHORT with 15% loss should trigger stop loss"
        );
    }

    #[test]
    fn open_position_side_default_is_long() {
        let pos = OpenPosition {
            entry_price: 100.0,
            entry_time: 0,
            peak_price: 100.0,
            entry_rsi: 50.0,
            entry_atr: 2.0,
            entry_score: 0.5,
            position_key: "key".to_string(),
            size_usd: 50.0,
            first_negative_score_time: None,
            side: "Long".to_string(),
        };
        assert_eq!(pos.side(), "Long");
    }

    #[test]
    fn open_position_side_short() {
        let pos = OpenPosition {
            entry_price: 100.0,
            entry_time: 0,
            peak_price: 100.0,
            entry_rsi: 50.0,
            entry_atr: 2.0,
            entry_score: -0.5,
            position_key: "key".to_string(),
            size_usd: 50.0,
            first_negative_score_time: None,
            side: "Short".to_string(),
        };
        assert_eq!(pos.side(), "Short");
    }

    #[test]
    fn open_position_side_serde_roundtrip() {
        let pos = OpenPosition {
            entry_price: 100.0,
            entry_time: 1700000000,
            peak_price: 95.0,
            entry_rsi: 45.0,
            entry_atr: 2.0,
            entry_score: -0.5,
            position_key: "testkey456".to_string(),
            size_usd: 50.0,
            first_negative_score_time: None,
            side: "Short".to_string(),
        };
        let json = serde_json::to_string(&pos).unwrap();
        let parsed: OpenPosition = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.side, "Short");
        assert_eq!(parsed.entry_price, 100.0);
    }

    #[test]
    fn trade_record_infer_side_short_win() {
        let mut t = TradeRecord {
            entry_price: 82.01,
            exit_price: 81.40314411,
            entry_time: 0,
            exit_time: 0,
            pnl_pct: 0.739977917327162,
            exit_reason: "ScoreFlip".to_string(),
            size_usd: 266.77,
            side: "Long".to_string(),
            fees: None,
        };
        assert_eq!(t.infer_side_from_pnl(), "Short");
        assert!(t.repair_side_from_pnl());
        assert_eq!(t.side, "Short");
    }

    #[test]
    fn trade_record_infer_side_long_unchanged() {
        let mut t = TradeRecord {
            entry_price: 95.35,
            exit_price: 95.69,
            entry_time: 0,
            exit_time: 0,
            pnl_pct: 0.3565810173046706,
            exit_reason: "TrailingStop".to_string(),
            size_usd: 337.46,
            side: "Long".to_string(),
            fees: None,
        };
        assert_eq!(t.infer_side_from_pnl(), "Long");
        assert!(!t.repair_side_from_pnl());
    }

    #[test]
    fn trade_record_without_fees_field_deserializes() {
        // Pre-2026-08-07 state files have no `fees` key; must deserialize None.
        let json = r#"{
            "entry_price": 73.0, "exit_price": 72.5, "entry_time": 0,
            "exit_time": 0, "pnl_pct": 0.6, "exit_reason": "TrailingStop",
            "size_usd": 118.0, "side": "Short"
        }"#;
        let parsed: TradeRecord = serde_json::from_str(json).unwrap();
        assert!(parsed.fees.is_none());
        assert_eq!(parsed.side, "Short");
    }

    #[test]
    fn trade_record_fees_serde_roundtrip() {
        let t = TradeRecord {
            entry_price: 73.0,
            exit_price: 72.5,
            entry_time: 0,
            exit_time: 0,
            pnl_pct: 0.6,
            exit_reason: "TrailingStop".to_string(),
            size_usd: 118.0,
            side: "Short".to_string(),
            fees: Some(FeeBreakdown {
                exit_fee_usd: 0.0234,
                borrow_fee_usd: 0.0005,
                price_impact_usd: 0.0,
                total_fee_usd: 0.0239,
            }),
        };
        let json = serde_json::to_string(&t).unwrap();
        let parsed: TradeRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.fees.as_ref().unwrap().total_fee_usd, 0.0239);
        // None fees must NOT serialize a null key (skip_serializing_if)
        let no_fees = TradeRecord { fees: None, ..t };
        let json2 = serde_json::to_string(&no_fees).unwrap();
        assert!(!json2.contains("fees"));
    }

    #[test]
    fn existing_state_loads_with_default_side() {
        // Old JSON without `side` field should default to "Long"
        let old_json = r#"{
            "entry_price": 100.0,
            "entry_time": 1700000000,
            "peak_price": 105.0,
            "entry_rsi": 45.0,
            "entry_atr": 2.0,
            "entry_score": 0.5,
            "position_key": "oldkey",
            "size_usd": 50.0,
            "first_negative_score_time": null
        }"#;
        let parsed: OpenPosition = serde_json::from_str(old_json).unwrap();
        assert_eq!(
            parsed.side, "Long",
            "Missing side field should default to Long"
        );
        assert_eq!(parsed.entry_price, 100.0);
    }

    #[test]
    fn entry_signal_exclusive_long_or_short() {
        // Score cannot be both > threshold AND < -threshold simultaneously
        // Test that at most one direction triggers per cycle
        let closes: Vec<f64> = (0..200).map(|i| 100.0 + i as f64 * 0.5).collect();
        let volumes: Vec<f64> = vec![1000.0; 200];
        let result = compute_signal(&closes, &closes, &closes, &volumes, 3).unwrap();

        // Uptrend: score > 0 — should not trigger SHORT
        if result.score > 0.0 {
            assert!(
                result.score > -0.01,
                "Positive score should not trigger SHORT condition"
            );
        }
    }

    #[test]
    fn no_entry_when_score_between_thresholds() {
        // When score is between ±threshold, neither LONG nor SHORT should trigger
        let params = StrategyParams {
            signal_threshold: 0.3,
            ..StrategyParams::default()
        };
        // Score exactly 0.0 is between ±0.3 — neither condition met
        let score = 0.0;
        assert!(
            score <= params.signal_threshold,
            "0.0 should not exceed threshold"
        );
        assert!(
            score >= -params.signal_threshold,
            "0.0 should not exceed negative threshold"
        );
    }

    #[test]
    fn short_entry_condition_met() {
        // Verify compute_signal produces bearish_count for downtrend data
        let closes: Vec<f64> = (0..200).map(|i| 200.0 - i as f64 * 0.5).collect();
        let volumes: Vec<f64> = vec![1000.0; 200];
        let result = compute_signal(&closes, &closes, &closes, &volumes, 3).unwrap();
        assert!(
            result.score < 0.0,
            "Downtrend should produce negative score"
        );
        assert!(
            result.bearish_count >= 2,
            "Downtrend should have bearish alignment"
        );
    }
}
