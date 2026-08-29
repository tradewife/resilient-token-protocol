//! GMTrade (gmx-solana) execution adapter — native Rust drop-in for the Flash
//! Trade executor. Validated on mainnet 2026-08-08 via the keeper-fill probe
//! (`research/missions/gmtrade_probe/TRADE-PROBE-RESULTS.md`): $10 LONG + SHORT
//! round trips filled by GMTrade keepers in 24.8–31.0s, fills inside the
//! Chainlink oracle band, measured round-trip order fee 0.022% of notional
//! (matches the v8e model).
//!
//! Model (GMX v2 style):
//! - The user signs a `create_order` instruction (market_increase /
//!   market_decrease); GMTrade's keepers execute it against Chainlink oracles.
//!   The client needs NO oracle access. `execution_fee` (lamports, floor 300k)
//!   pays the keeper.
//! - No LP deposit is needed for trading: `market_increase` deposits collateral
//!   into the position atomically. Production targets SOL/USD[WSOL-WSOL]
//!   (`G96vsSW5…`) with native SOL wrapped to wSOL as collateral (long token).
//!   Override market via `RTP_GM_MARKET` (market account pubkey).
//! - Fill detection: `Client::complete_order(order)` polls CPI events until the
//!   order PDA is removed and returns the `TradeEvent` (execution price, pnl,
//!   fees). Wrapped in a timeout + cancel so an unfilled order cannot hang a
//!   cycle or fill unattended later. IMPORTANT: the SDK watches STORE-WIDE
//!   events and does not verify order attribution (keeper batches mix many
//!   traders' fills), so `wait_for_fill` re-checks `TradeEvent.order` against
//!   our order PDA on every path before trusting price/pnl.
//!
//! Unit conventions (do NOT conflate):
//! - USD values in position state / fees are fixed-point 1e20.
//! - Unit PRICES (execution_price, prices.index.*) are fixed-point at
//!   10^(20 - index_token_decimals) = 1e11 for the 9-decimal SOL index.
//! - Position size in USD passed to orders = dollars × 1e20.
//!
//! The swarm crate uses solana-sdk 2.3 while gmsol-sdk pins >=2.1,<2.2, so the
//! two SDKs stay separate versions; conversion is by pubkey string. The client
//! signer comes from `RTP_TRADER_KEYPAIR` (same file the trader binary loads).

use std::sync::Arc;
use std::time::Duration;

use gmsol_sdk::Client;
use gmsol_sdk::IntoAtomicGroup;
use gmsol_sdk::builders::token::WrapNative;
use gmsol_sdk::client::ops::ExchangeOps;
use gmsol_sdk::solana_utils::cluster::Cluster;
use gmsol_sdk::solana_utils::instruction_group::{ComputeBudgetOptions, GetInstructionsOptions};
use gmsol_sdk::solana_utils::solana_sdk::instruction::Instruction;
use gmsol_sdk::solana_utils::solana_sdk::pubkey::Pubkey as GmPubkey;
use gmsol_sdk::solana_utils::solana_sdk::signature::{Keypair as GmKeypair, read_keypair_file};
use solana_sdk::signer::Signer;

use crate::trader::executor::PositionInfo;

const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";
const SOL_INDEX_MINT: &str = "So1Zu7vPQQxrguzUehKAyVLpjcc769zxgBuDAsxTUMH";
/// Default GMTrade market: SOL/USD[WSOL-WSOL] — pure wSOL pool, SOL collateral.
/// WSOL-USDC (`3M4v…`) hit long OI reserve saturation; do not silently revert.
const DEFAULT_GM_MARKET: &str = "G96vsSW5KXvostjyBT7rZwSVZpbL8r3mdVjAP5zwCRbn";
const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const ATA_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

/// USD fixed-point scale used across the store program.
const USD_SCALE: f64 = 1e20;
/// Keeper execution fee per order (lamports). Floor is 300k; the probe used
/// 500k. Overridable via RTP_GM_EXECUTION_FEE_LAMPORTS.
const DEFAULT_EXECUTION_FEE_LAMPORTS: u64 = 500_000;
/// Max time to wait for a keeper fill before cancelling the order.
const DEFAULT_FILL_TIMEOUT_SECS: u64 = 90;
/// Reserve below which we refuse to open (covers rent + a future close
/// cycle's keeper fee + tx fees). Set to ~0.05 SOL.
const MIN_NATIVE_SOL_RESERVE_LAMPORTS: u64 = 50_000_000;
/// Keep 2% of the available balance back as additional headroom.
const SOL_BALANCE_SAFETY: f64 = 0.98;
/// Fraction of computed headroom we are willing to consume. Near the reserve
/// ceiling other traders + oracle tick can close a thin gap before our keeper
/// runs, so keep half back.
const OI_HEADROOM_SAFETY: f64 = 0.50;
/// Absolute floor (USD) of pessimistic headroom required before we will even
/// size an open. Live keeper reject (2026-08-08) had true headroom ≈ $10 after
/// our order while mid-price math claimed ~$330 — refuse thin capacity rather
/// than burn wrap/create_order fees squeezing into a full book.
const MIN_OI_HEADROOM_USD: f64 = 500.0;
/// Oracle bid/ask haircut applied when valuing pool (min) vs reserved OI
/// (max) for longs. `validate_open_interest_reserve` uses
/// `pool_value(maximize=false)` and `reserved_value(index max price)`.
const OI_PRICE_BAND: f64 = 0.005; // 50 bps each side

type GmClient = Client<Arc<GmKeypair>>;
type GmResult<T> = std::result::Result<T, String>;

/// Immutable venue handles, cheaply cloneable (Arc client + Copy pubkeys).
#[derive(Clone)]
struct Venue {
    client: Arc<GmClient>,
    store: GmPubkey,
    /// Market account address (not the market token mint).
    market: GmPubkey,
    market_token: GmPubkey,
    /// True when long_token_mint == short_token_mint (e.g. WSOL-WSOL pure pool).
    /// Pure pools store total in long_token_amount; model long/short amounts are half.
    is_pure: bool,
    /// Short token is USDC (6dp). False for pure WSOL-WSOL.
    short_is_usdc: bool,
    /// 10^(20 - index_token_decimals); 1e11 for the 9-dp SOL index.
    price_scale: f64,
    /// Owner (trader wallet) the client signs with.
    owner: GmPubkey,
}

/// Soft-skip marker when the venue cannot accept more size on this side.
/// The trader loop treats this as a healthy no-op (not a watchdog error).
pub const CAPACITY_FULL_PREFIX: &str = "GM_CAPACITY_FULL:";

/// Soft-skip marker when the wallet cannot clear the venue's collateral
/// minimum. The trader loop treats this as a healthy no-op WITH an entry
/// cooldown (not a watchdog error) — an undersized wallet will keep failing
/// every poll until funded, so retrying burns gas and the error budget.
pub const INSUFFICIENT_COLLATERAL_PREFIX: &str = "GM_INSUFFICIENT_COLLATERAL:";

/// Soft-skip marker when a SOL position is already open for this owner on
/// the venue. Aug 26-27: duplicate trader processes with fresh internal
/// state stacked 5-6 consecutive orders (up to 3.7× intended size) because
/// each saw its own `open_position = None`. The venue-side check makes
/// stacking structurally impossible: an entry that would create a SECOND
/// venue position is refused before any wrap/order fee is spent.
pub const POSITION_ALREADY_OPEN_PREFIX: &str = "GM_POSITION_ALREADY_OPEN:";

/// GMTrade refuses opens with less than this much collateral (USD).
pub const MIN_OPEN_COLLATERAL_USD: f64 = 1.0;

/// Minimum relative move of the trail floor before we spend a transaction to
/// ratchet the venue stop (fraction of price). The floor only advances when
/// the confirmed-close peak advances, so ratchets are naturally infrequent;
/// the step just filters sub-tick noise. `update_order` is owner-signed
/// (no keeper fee), so each ratchet costs only the ~5000-lamport tx fee.
pub const TRAIL_RATCHET_MIN_STEP: f64 = 0.001;

/// Disable venue-side protective stops entirely (escape hatch; default on).
pub fn venue_stops_enabled() -> bool {
    !matches!(std::env::var("RTP_TRADER_VENUE_STOPS").as_deref(), Ok("0"))
}

/// Venue-side protective stop levels (pure math; no venue I/O).
///
/// Prices are in plain USD (caller converts to unit prices). The plan mirrors
/// the in-process exit levels EXACTLY — same ATR multiples as `check_exit` —
/// so venue execution changes WHEN/IF the stop fills (on-chain, oracle-touched,
/// process-crash-proof), not the strategy's validated exit levels.
#[derive(Debug, Clone, PartialEq)]
pub struct VenueStopPlan {
    /// Hard stop-loss trigger: entry ∓ sl_atr×ATR.
    pub sl_trigger: f64,
    /// Take-profit trigger: entry ± tp_atr×ATR.
    pub tp_trigger: f64,
    /// Trailing floor once in profit (peak ∓ trail_atr×ATR), else None.
    /// Long: floor is a price BELOW the peak; Short: ceiling ABOVE the trough.
    pub trail_floor: Option<f64>,
}

/// Compute venue stop levels for a position.
///
/// - LONG:  SL below entry, TP above, floor = peak − trail×ATR (needs peak > entry).
/// - SHORT: SL above entry, TP below, ceiling = trough + trail×ATR (needs peak < entry).
pub fn venue_stop_plan(
    entry_price: f64,
    atr: f64,
    sl_atr: f64,
    tp_atr: f64,
    trail_atr: f64,
    peak_price: f64,
    side: &str,
) -> VenueStopPlan {
    let is_short = side.eq_ignore_ascii_case("short");
    if is_short {
        VenueStopPlan {
            sl_trigger: entry_price + sl_atr * atr,
            tp_trigger: entry_price - tp_atr * atr,
            trail_floor: if peak_price < entry_price && trail_atr > 0.0 {
                Some(peak_price + trail_atr * atr)
            } else {
                None
            },
        }
    } else {
        VenueStopPlan {
            sl_trigger: entry_price - sl_atr * atr,
            tp_trigger: entry_price + tp_atr * atr,
            trail_floor: if peak_price > entry_price && trail_atr > 0.0 {
                Some(peak_price - trail_atr * atr)
            } else {
                None
            },
        }
    }
}

/// New SL trigger after a ratchet attempt. Returns Some(new_trigger) only when
/// the floor has advanced at least `TRAIL_RATCHET_MIN_STEP` × price beyond the
/// current trigger in the favorable direction AND stays strictly between entry
/// and the TP trigger (never crosses the take-profit, never weakens).
pub fn ratcheted_sl_trigger(
    current_sl_trigger: f64,
    tp_trigger: f64,
    trail_floor: Option<f64>,
    current_price: f64,
    side: &str,
) -> Option<f64> {
    let floor = trail_floor?;
    if current_price <= 0.0 {
        return None;
    }
    let step = current_price * TRAIL_RATCHET_MIN_STEP;
    let is_short = side.eq_ignore_ascii_case("short");
    if is_short {
        // Short: the ceiling is trough + trail×ATR — normally ABOVE the
        // current price (the stop fires on a rise to it). Ratchet the
        // trigger DOWN to the ceiling as the trough falls; never below the
        // ceiling (would exit early), never below TP, never weakening.
        let candidate = floor.min(current_sl_trigger);
        let tightened = current_sl_trigger - candidate >= step;
        let above_tp = candidate > tp_trigger;
        if tightened && above_tp && candidate > 0.0 {
            Some(candidate)
        } else {
            None
        }
    } else {
        // Long: ratchet the floor UP toward profit; must stay above the
        // current trigger (tightening), below TP, and below price.
        let candidate = floor.max(current_sl_trigger).max(0.0);
        let tightened = candidate - current_sl_trigger >= step;
        let below_tp = candidate < tp_trigger;
        let below_price = candidate < current_price - step;
        if tightened && below_tp && below_price {
            Some(candidate)
        } else {
            None
        }
    }
}

/// Restore validated stop levels when the trail armed ILLEGITIMATELY — i.e.
/// no confirmed hourly close has formed since entry yet, but the peak and/or
/// SL trigger moved off entry. That state comes from a pre-entry confirmed
/// close inflating the peak (mid-candle live entry below the previous bar's
/// close — Aug 29: entry $103.45 vs prior close $104.02 → on-chain SL
/// ratcheted from the $101.95 hard stop to $103.42 within 4 minutes).
///
/// Returns Some((restored_peak, restored_trigger)) when anything needs
/// healing: restored_trigger is Some(hard_stop) when the on-chain trigger
/// was tightened beyond the validated hard stop, None when only the peak
/// needs resetting (trigger absent/unknown). Returns None when levels are
/// clean. Healing only applies before any post-entry close exists — the
/// validated model keeps the stop at the hard level until then — so this
/// can never weaken a legitimately ratcheted stop.
pub fn venue_stop_heal_levels(
    entry_price: f64,
    entry_atr: f64,
    sl_atr: f64,
    peak_price: f64,
    sl_trigger: f64,
    side: &str,
) -> Option<(f64, Option<f64>)> {
    if entry_price <= 0.0 || entry_atr <= 0.0 {
        return None;
    }
    let is_short = side.eq_ignore_ascii_case("short");
    let hard_stop = if is_short {
        entry_price + sl_atr * entry_atr
    } else {
        entry_price - sl_atr * entry_atr
    };
    let peak_polluted = if is_short {
        peak_price < entry_price
    } else {
        peak_price > entry_price
    };
    // sl_trigger == 0 means "no on-chain level mirrored yet" — nothing to
    // heal on the venue side; the peak may still need resetting.
    let trigger_polluted = if sl_trigger <= 0.0 {
        false
    } else if is_short {
        sl_trigger < hard_stop - 1e-9 // ceiling tightened below the hard stop
    } else {
        sl_trigger > hard_stop + 1e-9 // floor tightened above the hard stop
    };
    if !peak_polluted && !trigger_polluted {
        return None;
    }
    Some((
        entry_price,
        if trigger_polluted {
            Some(hard_stop)
        } else {
            None
        },
    ))
}

/// A venue-side protective stop order owned by the trader wallet.
#[derive(Debug, Clone)]
pub struct VenueStopOrder {
    pub order: String,
    /// "StopLoss" or "TakeProfit" (classified from order kind).
    pub role: &'static str,
    pub trigger_price: f64,
    pub side: String,
}

fn unit_price(v: &Venue, usd_price: f64) -> u128 {
    (usd_price * v.price_scale) as u128
}

/// Place one venue stop order (StopLossDecrease or LimitDecrease) covering the
/// FULL position size. The keeper executes it against the oracle whenever the
/// trigger is touched — independent of our process being alive.
///
/// `kind`: "StopLoss" (StopLossDecrease) or "TakeProfit" (LimitDecrease).
/// GMX-style trigger semantics (confirmed in the SDK keeper simulation):
/// - StopLossDecrease LONG fills when price <= trigger (downside protection)
/// - StopLossDecrease SHORT fills when price >= trigger
/// - LimitDecrease LONG fills when price >= trigger (upside harvest)
/// - LimitDecrease SHORT fills when price <= trigger
async fn place_stop_order(
    v: &Venue,
    kind: &str,
    trigger_usd: f64,
    size_usd: f64,
    is_long: bool,
) -> GmResult<GmPubkey> {
    use gmsol_sdk::programs::gmsol_store::types::DecreasePositionSwapType;

    if trigger_usd <= 0.0 || size_usd <= 0.0 {
        return Err(format!(
            "venue stop: invalid trigger ${trigger_usd} or size ${size_usd}"
        ));
    }
    let size_units = (size_usd * USD_SCALE) as u128;
    let trigger = unit_price(v, trigger_usd);

    let mut builder = if kind == "StopLoss" {
        v.client.stop_loss(
            &v.store,
            &v.market_token,
            is_long,
            size_units,
            trigger,
            true,
            0,
        )
    } else {
        v.client.limit_decrease(
            &v.store,
            &v.market_token,
            is_long,
            size_units,
            trigger,
            true,
            0,
        )
    };
    // Proceeds return as native SOL (same as the manual close path), and PnL
    // swaps to the collateral token (no-op on the pure WSOL pool, but matches
    // the SDK CLI reference).
    builder
        .execution_fee(execution_fee())
        .decrease_position_swap_type(Some(DecreasePositionSwapType::PnlTokenToCollateralToken))
        .should_unwrap_native_token(true);

    let (rpc, order) = builder
        .build_with_address()
        .await
        .map_err(|e| format!("GM {kind} stop build failed: {e}"))?;
    let sig = rpc
        .send()
        .await
        .map_err(|e| format!("GM {kind} stop tx failed: {e}"))?;
    tracing::info!("[GM-STOP] {kind} order {order} @ ${trigger_usd:.2} placed (tx {sig})");
    Ok(order)
}

/// Place a single venue stop order covering the FULL position size.
/// `kind`: "StopLoss" (StopLossDecrease) or "TakeProfit" (LimitDecrease).
/// Returns the order pubkey string.
pub async fn place_venue_stop(
    keypair: &solana_sdk::signature::Keypair,
    kind: &str,
    trigger_usd: f64,
    size_usd: f64,
    side: &str,
) -> GmResult<String> {
    let v = venue().await?;
    assert_owner(&v, keypair)?;
    let is_long = !side.eq_ignore_ascii_case("short");
    let order = place_stop_order(&v, kind, trigger_usd, size_usd, is_long).await?;
    Ok(order.to_string())
}

/// Ratchet an existing stop order's trigger price via `update_order`
/// (owner-signed; no keeper fee). `new_trigger_usd` replaces the trigger.
pub async fn update_stop_trigger(
    keypair: &solana_sdk::signature::Keypair,
    order: &str,
    new_trigger_usd: f64,
) -> GmResult<()> {
    use gmsol_sdk::programs::gmsol_store::types::UpdateOrderParams;

    let v = venue().await?;
    assert_owner(&v, keypair)?;
    let order_addr = gm_pubkey(order)?;
    let params = UpdateOrderParams {
        size_delta_value: None,
        acceptable_price: None,
        trigger_price: Some(unit_price(&v, new_trigger_usd)),
        min_output: None,
        valid_from_ts: None,
    };
    let rpc = v
        .client
        .update_order(&v.store, &v.market_token, &order_addr, params, None)
        .await
        .map_err(|e| format!("GM update_order build failed: {e}"))?;
    let sig = rpc
        .send()
        .await
        .map_err(|e| format!("GM update_order tx failed: {e}"))?;
    tracing::info!("[GM-STOP] ratcheted {order_addr} trigger → ${new_trigger_usd:.2} (tx {sig})");
    Ok(())
}

/// Cancel (close) a venue order account. Best-effort by design: callers treat
/// failure as a warning (a stranded stop with no position fails keeper
/// validation and self-cancels on next execution attempt).
pub async fn cancel_order(
    keypair: &solana_sdk::signature::Keypair,
    order: &str,
) -> GmResult<String> {
    let v = venue().await?;
    assert_owner(&v, keypair)?;
    let order_addr = gm_pubkey(order)?;
    let mut builder = v
        .client
        .close_order(&order_addr)
        .map_err(|e| format!("GM cancel builder failed: {e}"))?;
    let rpc = builder
        .build()
        .await
        .map_err(|e| format!("GM cancel build failed: {e}"))?;
    let sig = rpc
        .send()
        .await
        .map_err(|e| format!("GM cancel tx failed: {e}"))?;
    Ok(sig.to_string())
}

/// List this owner's live stop orders on the SOL market, classified into
/// StopLoss / TakeProfit roles with their current trigger prices.
pub async fn list_venue_stops() -> GmResult<Vec<VenueStopOrder>> {
    use gmsol_sdk::core::order::OrderKind;

    let v = venue().await?;
    let orders = v
        .client
        .orders(&v.store, Some(&v.owner), Some(&v.market_token))
        .await
        .map_err(|e| format!("GM orders fetch failed: {e}"))?;

    let mut out = Vec::new();
    for (addr, order) in orders {
        let kind = match order.params.kind() {
            Ok(k) => k,
            Err(_) => continue,
        };
        let role = match kind {
            OrderKind::StopLossDecrease => "StopLoss",
            OrderKind::LimitDecrease => "TakeProfit",
            _ => continue,
        };
        let side = match order.params.side() {
            Ok(s) if s.is_long() => "Long".to_string(),
            Ok(_) => "Short".to_string(),
            Err(_) => continue,
        };
        out.push(VenueStopOrder {
            order: addr.to_string(),
            role,
            trigger_price: order.params.trigger_price as f64 / v.price_scale,
            side,
        });
    }
    Ok(out)
}

/// Cancel every stop order this owner holds on the SOL market (used while
/// flat to sweep stragglers left behind by a venue-executed or failed close).
/// Returns the number cancelled.
pub async fn cancel_all_venue_stops(keypair: &solana_sdk::signature::Keypair) -> GmResult<u32> {
    let stops = list_venue_stops().await?;
    let mut n = 0;
    for stop in stops {
        match cancel_order(keypair, &stop.order).await {
            Ok(sig) => {
                tracing::info!(
                    "[GM-STOP] cancelled stray {} order {} (tx {sig})",
                    stop.role,
                    stop.order
                );
                n += 1;
            }
            Err(e) => {
                tracing::warn!(
                    "[GM-STOP] failed to cancel stray {} order {}: {e}",
                    stop.role,
                    stop.order
                );
            }
        }
    }
    Ok(n)
}

/// Fill report of a CONSUMED venue stop order.
#[derive(Debug, Clone)]
pub struct VenueStopFill {
    pub execution_price: f64,
    pub pnl_usd: f64,
    pub order_fee_usd: f64,
    pub borrow_fee_usd: f64,
}

/// Recover the fill report of a CONSUMED stop order (venue stop fired):
/// execution price, PnL and close-leg fees. Scans historical CPI events
/// scoped to the order PDA with the same attribution filter as
/// `wait_for_fill` — keeper batches mix many traders' events, so only our
/// order's TradeEvent counts. Returns Ok(None) when no attributable fill
/// exists (order still live, or events aged out).
pub async fn venue_stop_fill_report(order: &str) -> GmResult<Option<VenueStopFill>> {
    use gmsol_sdk::decode::gmsol::programs::GMSOLCPIEvent;
    use solana_sdk::commitment_config::CommitmentConfig;

    let v = venue().await?;
    let order_addr = gm_pubkey(order)?;
    let events = match v
        .client
        .last_order_events(&order_addr, u64::MAX, CommitmentConfig::confirmed())
        .await
    {
        Ok(events) => events,
        Err(e) => {
            tracing::warn!("[GM-STOP] fill-report scan failed for {order_addr}: {e}");
            return Ok(None);
        }
    };
    let trade = events.into_iter().find_map(|ev| match ev {
        GMSOLCPIEvent::TradeEvent(t) if t.order == order_addr => Some(t),
        _ => None,
    });
    Ok(trade.map(|t| VenueStopFill {
        execution_price: t.execution_price as f64 / v.price_scale,
        pnl_usd: t.pnl.pnl as f64 / USD_SCALE,
        order_fee_usd: t.fees.order_fee_for_receiver_amount as f64 / USD_SCALE,
        borrow_fee_usd: t.fees.total_borrowing_fee_amount as f64 / USD_SCALE,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_plan_long_levels() {
        let plan = venue_stop_plan(100.0, 2.0, 2.5, 6.0, 1.0, 100.0, "Long");
        assert_eq!(plan.sl_trigger, 95.0); // 100 - 2.5*2
        assert_eq!(plan.tp_trigger, 112.0); // 100 + 6*2
        assert_eq!(plan.trail_floor, None); // peak == entry: not in profit
    }

    #[test]
    fn stop_plan_long_trail_floor() {
        let plan = venue_stop_plan(100.0, 2.0, 2.5, 6.0, 1.0, 105.0, "Long");
        assert_eq!(plan.trail_floor, Some(103.0)); // 105 - 1*2
    }

    #[test]
    fn stop_plan_short_levels_inverted() {
        let plan = venue_stop_plan(100.0, 2.0, 2.5, 6.0, 1.0, 95.0, "Short");
        assert_eq!(plan.sl_trigger, 105.0); // 100 + 2.5*2
        assert_eq!(plan.tp_trigger, 88.0); // 100 - 6*2
        assert_eq!(plan.trail_floor, Some(97.0)); // trough 95 + 1*2
    }

    #[test]
    fn ratchet_long_advances_toward_floor() {
        // SL at 95, TP at 112, floor 103, price 109 → ratchet to 103.
        let t = ratcheted_sl_trigger(95.0, 112.0, Some(103.0), 109.0, "Long");
        assert_eq!(t, Some(103.0));
    }

    #[test]
    fn ratchet_long_refuses_sub_step_moves() {
        // Floor only 0.05 above current trigger (< 0.1% step of price 109).
        let t = ratcheted_sl_trigger(103.0, 112.0, Some(103.05), 109.0, "Long");
        assert_eq!(t, None);
    }

    #[test]
    fn ratchet_long_never_crosses_tp_or_price() {
        // Floor 113 > TP 112 → refuse (would kill the take-profit).
        assert_eq!(
            ratcheted_sl_trigger(103.0, 112.0, Some(113.0), 115.0, "Long"),
            None
        );
        // Floor 108.96 ≈ price 109 (not a full step below) → refuse.
        assert_eq!(
            ratcheted_sl_trigger(103.0, 120.0, Some(108.96), 109.0, "Long"),
            None
        );
    }

    #[test]
    fn ratchet_short_tightens_downward() {
        // Short: SL ceiling at 105, TP 88, ceiling-floor 97, price 91.
        let t = ratcheted_sl_trigger(105.0, 88.0, Some(97.0), 91.0, "Short");
        assert_eq!(t, Some(97.0));
        // Sub-step refinement refused.
        assert_eq!(
            ratcheted_sl_trigger(97.0, 88.0, Some(96.98), 91.0, "Short"),
            None
        );
        // Never crosses TP (candidate 87 < TP 88 → refuse).
        assert_eq!(
            ratcheted_sl_trigger(97.0, 88.0, Some(87.0), 90.0, "Short"),
            None
        );
    }

    #[test]
    fn ratchet_requires_floor() {
        assert_eq!(ratcheted_sl_trigger(95.0, 112.0, None, 109.0, "Long"), None);
    }

    #[test]
    fn heal_long_polluted_peak_and_trigger() {
        // Long entered 103.45 (ATR 0.60, sl 2.5 → hard stop 101.95). A
        // pre-entry close of 104.02 inflated the peak, and the on-chain
        // trigger got ratcheted to 103.42 (the Aug 29 incident). Both must
        // heal: peak → entry, trigger → hard stop.
        let heal = venue_stop_heal_levels(103.45, 0.60, 2.5, 104.02, 103.42, "Long");
        assert_eq!(heal, Some((103.45, Some(101.95))));
    }

    #[test]
    fn heal_long_clean_levels_noop() {
        // Peak at entry and trigger at the hard stop: nothing polluted.
        assert_eq!(
            venue_stop_heal_levels(103.45, 0.60, 2.5, 103.45, 101.95, "Long"),
            None
        );
    }

    #[test]
    fn heal_long_trigger_unknown_peak_polluted() {
        // Trigger not mirrored yet (0) but the peak is polluted: heal the
        // peak only (no on-chain trigger to move).
        assert_eq!(
            venue_stop_heal_levels(103.45, 0.60, 2.5, 104.02, 0.0, "Long"),
            Some((103.45, None))
        );
    }

    #[test]
    fn heal_short_polluted_levels() {
        // Short entered 100 (ATR 2, sl 2.5 → hard ceiling 105). A pre-entry
        // trough of 98 polluted the peak; ceiling tightened to 99.
        assert_eq!(
            venue_stop_heal_levels(100.0, 2.0, 2.5, 98.0, 99.0, "Short"),
            Some((100.0, Some(105.0)))
        );
    }

    #[test]
    fn heal_rejects_invalid_inputs() {
        assert_eq!(
            venue_stop_heal_levels(0.0, 1.0, 2.5, 100.0, 95.0, "Long"),
            None
        );
        assert_eq!(
            venue_stop_heal_levels(100.0, 0.0, 2.5, 100.0, 95.0, "Long"),
            None
        );
    }

    #[test]
    fn heal_never_weakens_legitimate_ratchet() {
        // After a real post-entry advance, trigger ABOVE the hard stop is
        // legitimate; but heal is only called BEFORE any post-entry close,
        // so here a trigger above hard-stop with peak above entry is treated
        // as pollution and restored. This is correct: before a post-entry
        // close, the validated model keeps the stop at the hard level.
        let heal = venue_stop_heal_levels(103.45, 0.60, 2.5, 104.02, 102.5, "Long");
        assert_eq!(heal, Some((103.45, Some(101.95))));
    }
}

fn venue_slot() -> &'static tokio::sync::Mutex<Option<Venue>> {
    static SLOT: std::sync::OnceLock<tokio::sync::Mutex<Option<Venue>>> =
        std::sync::OnceLock::new();
    SLOT.get_or_init(|| tokio::sync::Mutex::new(None))
}

fn execution_fee() -> u64 {
    std::env::var("RTP_GM_EXECUTION_FEE_LAMPORTS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_EXECUTION_FEE_LAMPORTS)
}

fn fill_timeout() -> Duration {
    Duration::from_secs(
        std::env::var("RTP_GM_FILL_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_FILL_TIMEOUT_SECS),
    )
}

/// Minimum collateral (lamports) for an open to proceed.
///
/// The venue floor is only $1 (`MIN_OPEN_COLLATERAL_USD`), which merely
/// clears the keeper's validation — it does NOT clear the fixed per-order
/// costs (execution fee + wrap/rent ≈ 0.0012 SOL per round trip). At
/// $1 collateral those fixed costs are ~4% per leg, so tiny positions bleed
/// fees regardless of edge (Aug 2026: a drained wallet churned ~$1
/// collateral positions at ~0.012 SOL net loss per cycle). The floor keeps
/// fixed costs a small fraction of the position. Overridable via
/// `RTP_TRADER_MIN_OPEN_COLLATERAL_LAMPORTS`.
pub const DEFAULT_MIN_OPEN_COLLATERAL_LAMPORTS: u64 = 500_000_000; // 0.5 SOL

pub fn min_open_collateral_lamports() -> u64 {
    std::env::var("RTP_TRADER_MIN_OPEN_COLLATERAL_LAMPORTS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_MIN_OPEN_COLLATERAL_LAMPORTS)
}

fn rpc_url() -> String {
    std::env::var("RTP_GM_RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string())
}

fn gm_pubkey(s: &str) -> GmResult<GmPubkey> {
    s.parse().map_err(|e| format!("bad pubkey {s}: {e}"))
}

/// Connect once: load the signer keypair, locate the GMTrade store and SOL
/// market (default SOL/USD[WSOL-WSOL]), and compute the unit-price scale.
async fn init_venue() -> GmResult<Venue> {
    let path = std::env::var("RTP_TRADER_KEYPAIR")
        .map_err(|_| "RTP_TRADER_KEYPAIR not set (GM venue needs the signer)".to_string())?;
    let gm_keypair =
        read_keypair_file(&path).map_err(|e| format!("GM keypair load failed: {e}"))?;
    let owner = gm_keypair.pubkey();

    let cluster: Cluster = rpc_url()
        .parse()
        .map_err(|e: gmsol_sdk::SolanaUtilsError| format!("GM cluster parse failed: {e}"))?;
    let client = Client::new(cluster, Arc::new(gm_keypair))
        .map_err(|e| format!("GM client init failed: {e}"))?;
    let client = Arc::new(client);

    let store = client.find_store_address("");
    tracing::info!("[GM] store: {store}");

    let wsol = gm_pubkey(WSOL_MINT)?;
    let usdc = gm_pubkey(USDC_MINT)?;
    let sol_index = gm_pubkey(SOL_INDEX_MINT)?;

    // Market selection:
    //   1. RTP_GM_MARKET = market account pubkey (explicit override)
    //   2. else default G96 SOL/USD[WSOL-WSOL]
    //   3. else discover SOL index + WSOL + WSOL from store
    let override_market = std::env::var("RTP_GM_MARKET")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let target = override_market.unwrap_or_else(|| DEFAULT_GM_MARKET.to_string());

    let markets = client
        .markets(&store)
        .await
        .map_err(|e| format!("GM markets fetch failed: {e}"))?;

    let mut found: Option<(String, String, bool, bool)> = None; // addr, token, is_pure, short_is_usdc
    // Prefer exact pubkey match first.
    for (addr, market) in &markets {
        if addr.to_string() == target {
            let is_pure = market.meta.long_token_mint == market.meta.short_token_mint;
            let short_is_usdc = market.meta.short_token_mint == usdc;
            found = Some((
                addr.to_string(),
                market.meta.market_token_mint.to_string(),
                is_pure,
                short_is_usdc,
            ));
            break;
        }
    }
    // Discover WSOL-WSOL if target was default but pubkey listing used different key form.
    if found.is_none() {
        for (addr, market) in &markets {
            if market.meta.index_token_mint == sol_index
                && market.meta.long_token_mint == wsol
                && market.meta.short_token_mint == wsol
            {
                found = Some((
                    addr.to_string(),
                    market.meta.market_token_mint.to_string(),
                    true,
                    false,
                ));
                break;
            }
        }
    }
    let (market_str, market_token_str, is_pure, short_is_usdc) = found.ok_or_else(|| {
        format!(
            "GM market not found (wanted {target}; expected SOL/USD[WSOL-WSOL] or RTP_GM_MARKET)"
        )
    })?;
    let market = gm_pubkey(&market_str)?;
    let market_token = gm_pubkey(&market_token_str)?;

    // Unit-price scale: 10^(20 - index_token_decimals).
    use gmsol_sdk::core::token_config::TokenMapAccess;
    let token_map = client
        .authorized_token_map(&store)
        .await
        .map_err(|e| format!("GM token map fetch failed: {e}"))?;
    let market_account = client
        .market(&market)
        .await
        .map_err(|e| format!("GM market fetch failed: {e}"))?;
    // Sanity: index must be SOL; collateral path expects wSOL long token.
    if market_account.meta.index_token_mint != sol_index {
        return Err(format!(
            "GM market {market} index is {}, expected SOL index {SOL_INDEX_MINT}",
            market_account.meta.index_token_mint
        ));
    }
    if market_account.meta.long_token_mint != wsol {
        return Err(format!(
            "GM market {market} long token is {}, expected wSOL (adapter wraps native SOL)",
            market_account.meta.long_token_mint
        ));
    }
    let index_decimals = token_map
        .get(&market_account.meta.index_token_mint)
        .map(|c| c.token_decimals)
        .unwrap_or(9);
    let price_scale = 10f64.powi(20 - index_decimals as i32);

    let label = if is_pure {
        "SOL/USD[WSOL-WSOL]"
    } else if short_is_usdc {
        "SOL/USD[WSOL-USDC]"
    } else {
        "SOL/USD[?]"
    };
    tracing::info!(
        "[GM] venue ready — {label} market: {market} market_token: {market_token} \
         pure={is_pure} index_decimals: {index_decimals} price_scale: {price_scale:e} owner: {owner}"
    );

    Ok(Venue {
        client,
        store,
        market,
        market_token,
        is_pure,
        short_is_usdc,
        price_scale,
        owner,
    })
}

/// Live open-interest reserve headroom (USD) for one side of the SOL market.
///
/// Mirrors GMTrade's `validate_open_interest_reserve`:
///   max_reserved = pool_value_without_pnl(min prices) * open_interest_reserve_factor
///   reserved     = OI_in_tokens * index_max_price   (long)
///                = OI_usd                           (short)
///   headroom     = max_reserved - reserved
///
/// Keeper validation runs *after* the new size is added to OI, so callers must
/// still reserve room for the proposed notional on top of this figure.
///
/// Note: live `open_interest_reserve_factor` can be > 1.0 (observed 3.75).
/// Do not clamp it to [0,1]. Pure pools (WSOL-WSOL) use half of primary
/// `long_token_amount` per side (`div_ceil` long / `div` short).
///
/// Returns Ok(headroom_usd). Negative means the side is already over capacity.
async fn oi_headroom_usd(v: &Venue, is_long: bool, sol_price: f64) -> GmResult<f64> {
    let market = v
        .client
        .market(&v.market)
        .await
        .map_err(|e| format!("GM market fetch for capacity check failed: {e}"))?;
    let cfg = &market.config;
    let pools = &market.state.pools;

    let reserve_factor = cfg.open_interest_reserve_factor as f64 / USD_SCALE;
    if reserve_factor <= 0.0 || !reserve_factor.is_finite() {
        return Err(format!(
            "GM open_interest_reserve_factor out of range: {reserve_factor}"
        ));
    }

    // OI-in-tokens / OI pools: gmsol Merged/Balance long_amount sums both legs.
    let pool_amount =
        |long_raw: u128, short_raw: u128| -> u128 { long_raw.saturating_add(short_raw) };

    // Pessimistic oracle band: pool at min, long reserved at index max.
    let pool_px = sol_price * (1.0 - OI_PRICE_BAND);
    let index_max_px = sol_price * (1.0 + OI_PRICE_BAND);

    let primary_long_raw = pools.primary.pool.long_token_amount;
    let primary_short_raw = pools.primary.pool.short_token_amount;

    // Pure pool model: long_amount = div_ceil(total/2), short_amount = total/2.
    let (side_pool_raw, side_decimals_is_sol) = if is_long {
        let raw = if v.is_pure {
            primary_long_raw.saturating_add(1) / 2
        } else {
            primary_long_raw
        };
        (raw, true) // long token is wSOL
    } else if v.is_pure {
        (primary_long_raw / 2, true) // short side also wSOL
    } else if v.short_is_usdc {
        (primary_short_raw, false) // USDC 6dp
    } else {
        (primary_short_raw, true)
    };

    let (pool_value_usd, reserved_usd, oi_usd) = if is_long {
        let pool_tokens = side_pool_raw as f64 / 1e9;
        let pool_value = pool_tokens * pool_px;
        let oi_tok_raw = pool_amount(
            pools
                .open_interest_in_tokens_for_long
                .pool
                .long_token_amount,
            pools
                .open_interest_in_tokens_for_long
                .pool
                .short_token_amount,
        );
        let reserved = (oi_tok_raw as f64 / 1e9) * index_max_px;
        let oi = pool_amount(
            pools.open_interest_for_long.pool.long_token_amount,
            pools.open_interest_for_long.pool.short_token_amount,
        ) as f64
            / USD_SCALE;
        (pool_value, reserved, oi)
    } else {
        let oi = pool_amount(
            pools.open_interest_for_short.pool.long_token_amount,
            pools.open_interest_for_short.pool.short_token_amount,
        ) as f64
            / USD_SCALE;
        // Short reserved = OI USD. Pool value: USDC face value or wSOL*min_px.
        let pool_value = if side_decimals_is_sol {
            (side_pool_raw as f64 / 1e9) * pool_px
        } else {
            side_pool_raw as f64 / 1e6
        };
        (pool_value, oi, oi)
    };

    let max_reserved = pool_value_usd * reserve_factor;
    let max_oi = if is_long {
        cfg.max_open_interest_for_long as f64 / USD_SCALE
    } else {
        cfg.max_open_interest_for_short as f64 / USD_SCALE
    };
    let headroom_reserve = max_reserved - reserved_usd;
    let headroom_oi_cap = max_oi - oi_usd;
    let headroom = headroom_reserve.min(headroom_oi_cap);

    tracing::info!(
        "[GM] capacity {} headroom=${:.2} (reserve_max=${:.2} reserved=${:.2} \
         oi=${:.2}/{:.0} factor={:.4} band={:.2}% pure={} market={})",
        if is_long { "LONG" } else { "SHORT" },
        headroom,
        max_reserved,
        reserved_usd,
        oi_usd,
        max_oi,
        reserve_factor,
        OI_PRICE_BAND * 100.0,
        v.is_pure,
        v.market
    );
    Ok(headroom)
}

/// Get initialized venue handles (initializes on first call).
async fn venue() -> GmResult<Venue> {
    let slot = venue_slot();
    {
        let guard = slot.lock().await;
        if let Some(v) = guard.as_ref() {
            return Ok(v.clone());
        }
    }
    let v = init_venue().await?;
    let mut guard = slot.lock().await;
    if guard.is_none() {
        *guard = Some(v.clone());
    }
    Ok(guard.as_ref().expect("just set").clone())
}

/// Assert the caller's keypair is the venue owner (guards against signing with
/// the wrong wallet after a keypair swap).
fn assert_owner(v: &Venue, keypair: &solana_sdk::signature::Keypair) -> GmResult<()> {
    if keypair.pubkey().to_string() != v.owner.to_string() {
        return Err(format!(
            "GM venue owner {} != caller keypair {}",
            v.owner,
            keypair.pubkey()
        ));
    }
    Ok(())
}

/// GMTrade needs no one-time setup (no basket, no deposit ledger, no delegate):
/// market_increase deposits collateral atomically with the order.
pub async fn v2_one_time_setup(_keypair: &solana_sdk::signature::Keypair) -> GmResult<Vec<String>> {
    Ok(vec!["gmtrade: no setup required".to_string()])
}

/// SOL price from CoinGecko (the Flash /prices endpoint is winding down).
pub async fn get_sol_price() -> GmResult<f64> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client build failed: {e}"))?;
    let val: serde_json::Value = client
        .get("https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd")
        .header("User-Agent", "rtp-trader/1.0 (venue-price)")
        .send()
        .await
        .map_err(|e| format!("CoinGecko SOL price fetch failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("CoinGecko SOL price parse failed: {e}"))?;
    let price = val
        .get("solana")
        .and_then(|v| v.get("usd"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    if price <= 0.0 {
        return Err(format!("Invalid SOL price from CoinGecko: {val}"));
    }
    Ok(price)
}

/// Derive the associated token address (owner, mint) on the token program.
fn spl_ata(owner: &GmPubkey, mint: &GmPubkey) -> GmResult<GmPubkey> {
    let token_program = gm_pubkey(TOKEN_PROGRAM)?;
    let ata_program = gm_pubkey(ATA_PROGRAM)?;
    let (addr, _bump) = GmPubkey::find_program_address(
        &[owner.as_ref(), token_program.as_ref(), mint.as_ref()],
        &ata_program,
    );
    Ok(addr)
}

/// Native SOL balance of the venue owner (in SOL units).
async fn native_sol_balance(v: &Venue) -> GmResult<f64> {
    let lamports = v
        .client
        .rpc()
        .get_balance(&v.owner)
        .await
        .map_err(|e| format!("native SOL balance fetch failed: {e}"))?;
    Ok(lamports as f64 / 1e9)
}

/// Build a WrapNative pre-instruction that transfers `lamports` of native SOL
/// from the owner into the wSOL ATA and calls sync_native. Returns the raw
/// system + token instructions ready to be passed to
/// `TransactionBuilder::pre_instructions(.., false)`.
fn wrap_native_ixs(owner: &GmPubkey, lamports: u64) -> GmResult<Vec<Instruction>> {
    if lamports == 0 {
        return Err("wrap_native_ixs: zero lamports".to_string());
    }
    Ok(WrapNative::builder()
        .owner(*owner)
        .lamports(lamports)
        .build()
        .into_atomic_group(&false)
        .map_err(|e| format!("WrapNative build failed: {e}"))?
        .instructions_with_options(GetInstructionsOptions {
            compute_budget: ComputeBudgetOptions {
                without_compute_budget: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .map(|ix| (*ix).clone())
        .collect())
}

/// Open positions on GMTrade for a wallet, shaped as Flash-style
/// `PositionInfo` so the trader loop is unchanged.
///
/// `entry_price_ui` = average execution price derived from size_in_usd /
/// size_in_tokens (both fixed-point; the ratio is at unit-price scale).
///
/// Fee fields are ESTIMATES for the prospective close leg: worst-case order
/// fee (0.012% negative-impact) + borrow accrued at the kink-model base rate.
/// Actual close-leg fees are logged from the TradeEvent at close time.
pub async fn get_positions(wallet: &str) -> GmResult<Vec<PositionInfo>> {
    let v = venue().await?;
    if wallet != v.owner.to_string() {
        // The trader only ever queries its own wallet; anything else is a bug.
        return Err(format!(
            "GM get_positions called for {wallet} but venue owner is {}",
            v.owner
        ));
    }

    let positions = v
        .client
        .positions(&v.store, Some(&v.owner), Some(&v.market_token))
        .await
        .map_err(|e| format!("GM positions fetch failed: {e}"))?;

    let now = chrono::Utc::now().timestamp();
    let mut out = Vec::new();
    for (addr, pos) in positions {
        let is_long = pos.try_is_long().unwrap_or(true);
        let size_usd = pos.state.size_in_usd as f64 / USD_SCALE;
        if size_usd <= 0.0 {
            continue;
        }
        // Average execution price = size_usd / size_tokens, at unit-price scale.
        let entry_price = if pos.state.size_in_tokens > 0 {
            (pos.state.size_in_usd as f64 / pos.state.size_in_tokens as f64) / v.price_scale
        } else {
            0.0
        };
        // Estimated close-leg fees (see module docs).
        let exit_fee = size_usd * 0.00012;
        let held_secs = (now - pos.state.increased_at).max(0) as f64;
        let borrow_fee = size_usd * 1.0e-8 * held_secs;
        let total_fee = exit_fee + borrow_fee;

        out.push(PositionInfo {
            key: addr.to_string(),
            side_ui: if is_long {
                "Long".to_string()
            } else {
                "Short".to_string()
            },
            market_symbol: "SOL".to_string(),
            collateral_symbol: "SOL".to_string(),
            size_usd_ui: format!("{size_usd:.6}"),
            entry_price_ui: format!("{entry_price:.6}"),
            pnl_with_fee_usd_ui: "0".to_string(),
            leverage_ui: "0".to_string(),
            exit_fee_usd: format!("{}", (exit_fee * 1e6) as u64),
            borrow_fee_usd: format!("{}", (borrow_fee * 1e6) as u64),
            price_impact_usd: "0".to_string(),
            total_fee_usd: format!("{}", (total_fee * 1e6) as u64),
            opened_at_secs: pos.state.increased_at,
        });
    }
    Ok(out)
}

/// Open a position on GMTrade with wSOL collateral.
///
/// `amount_sol` is the native SOL collateral budget (wallet balance ×
/// position_fraction). The order transfers native SOL → wSOL ATA via a
/// `WrapNative` pre-instruction and uses wSOL as collateral for the increase
/// (`is_collateral_token_long = true`, since wSOL is the market's long token).
///
/// Safety floors:
///   - reserve `MIN_NATIVE_SOL_RESERVE_LAMPORTS` for rent + future close fees
///   - cap collateral at `SOL_BALANCE_SAFETY` × (balance - reserve)
///   - refuse if available collateral < $1
///
/// Returns (signature, size_usd, entry_price).
pub async fn open_position(
    keypair: &solana_sdk::signature::Keypair,
    amount_sol: f64,
    leverage: f64,
    trade_type: &str,
) -> GmResult<(String, f64, f64)> {
    let v = venue().await?;
    assert_owner(&v, keypair)?;

    // Stacking guard: refuse if this owner already holds a SOL position on
    // the venue. The trader runs one position at a time; a second open can
    // only come from state desync (fresh-state duplicate process, missed
    // reconciliation) — exactly the Aug 26-27 incident where 5-6 stacked
    // orders ballooned a position to 3.7× intended size. Checked HERE (not
    // just upstream) because every open path must be protected.
    match get_positions(&v.owner.to_string()).await {
        Ok(positions) => {
            if let Some(existing) = positions.iter().find(|p| p.market_symbol == "SOL") {
                return Err(format!(
                    "{POSITION_ALREADY_OPEN_PREFIX} owner already holds SOL {} \
                     (${:.2} notional, key={}...) — refusing to stack another open",
                    existing.side_ui,
                    existing.size_usd_ui,
                    &existing.key[..existing.key.len().min(8)]
                ));
            }
        }
        Err(e) => {
            // Fail closed: if we can't verify the book is empty, don't open.
            return Err(format!(
                "{POSITION_ALREADY_OPEN_PREFIX} cannot verify venue book before open ({e}) \
                 — refusing to open on unknown state"
            ));
        }
    }

    let sol_price = get_sol_price().await?;
    let native_sol = native_sol_balance(&v).await?;

    // Available native SOL after safety reserves.
    let reserved = MIN_NATIVE_SOL_RESERVE_LAMPORTS as f64 / 1e9;
    let available_sol = ((native_sol - reserved).max(0.0)) * SOL_BALANCE_SAFETY;
    let collateral_sol = amount_sol.min(available_sol);

    if collateral_sol < 0.001 {
        return Err(format!(
            "{INSUFFICIENT_COLLATERAL_PREFIX} collateral budget {amount_sol:.4} SOL vs \
             available {available_sol:.4} SOL (native wallet {native_sol:.4} minus \
             reserve {reserved:.4})"
        ));
    }

    let is_long = trade_type.eq_ignore_ascii_case("long");

    // Pre-flight: refuse before spending wrap/create_order fees if the side
    // cannot accept more open interest. Soft-skip (CAPACITY_FULL_PREFIX) so
    // the trader loop stays healthy and retries later when capacity frees.
    //
    // Keeper validate_open_interest_reserve runs AFTER the new size is added
    // to reserved OI, so required headroom is the full proposed notional
    // (plus band/safety). Do not squeeze into a near-full book — that is how
    // we burned fees on Model 6006 with "headroom $330" that was really ~$10.
    let headroom = oi_headroom_usd(&v, is_long, sol_price).await?;
    let usable_headroom = (headroom * OI_HEADROOM_SAFETY).max(0.0);
    if usable_headroom < MIN_OI_HEADROOM_USD {
        return Err(format!(
            "{CAPACITY_FULL_PREFIX} {} headroom ${headroom:.2} (usable ${usable_headroom:.2}) \
             below ${MIN_OI_HEADROOM_USD:.0} — not opening",
            if is_long { "LONG" } else { "SHORT" }
        ));
    }

    let collateral_usd = collateral_sol * sol_price;
    let size_usd = collateral_usd * leverage;
    // Long reserved grows by ~size_in_tokens * index_max ≈ size_usd * (1+band).
    let size_cost = if is_long {
        size_usd * (1.0 + OI_PRICE_BAND)
    } else {
        size_usd
    };
    if size_cost > usable_headroom {
        return Err(format!(
            "{CAPACITY_FULL_PREFIX} {} need ${size_cost:.2} notional room but usable \
             headroom is only ${usable_headroom:.2} (raw ${headroom:.2}) — not opening",
            if is_long { "LONG" } else { "SHORT" }
        ));
    }
    let collateral_lamports = (collateral_sol * 1e9) as u64;
    // Fee-sane collateral floor (see `min_open_collateral_lamports`): the
    // venue's $1 minimum only clears keeper validation, not the fixed
    // per-order costs. Below the floor the trader stays flat via the
    // INSUFFICIENT_COLLATERAL soft-skip (entry cooldown arms upstream).
    let min_collateral = min_open_collateral_lamports();
    if collateral_lamports < min_collateral {
        return Err(format!(
            "{INSUFFICIENT_COLLATERAL_PREFIX} {collateral_sol:.4} SOL collateral below the \
             {} lamport fee-sane floor (${collateral_usd:.2} @ SOL ${sol_price:.2})",
            min_collateral
        ));
    }
    if collateral_usd < MIN_OPEN_COLLATERAL_USD {
        return Err(format!(
            "{INSUFFICIENT_COLLATERAL_PREFIX} ${collateral_usd:.2} collateral below \
             ${MIN_OPEN_COLLATERAL_USD:.0} floor"
        ));
    }
    let size_delta = (size_usd * USD_SCALE) as u128;

    tracing::info!(
        "[GM-OPEN] {} {:.4} SOL collateral (${collateral_usd:.2}) @ {leverage}x → \
         ${size_usd:.2} notional (SOL ${sol_price:.2}, native balance {native_sol:.4}, \
         headroom ${headroom:.2} usable ${usable_headroom:.2})",
        if is_long { "LONG" } else { "SHORT" },
        collateral_sol,
    );

    // Pre-instruction: wrap native SOL → wSOL in the owner's wSOL ATA so the
    // market_increase instruction can pull from it.
    let wsol_ata = spl_ata(&v.owner, &gm_pubkey(WSOL_MINT)?)?;
    let wrap_ixs = wrap_native_ixs(&v.owner, collateral_lamports)?;

    let mut builder = v.client.market_increase(
        &v.store,
        &v.market_token,
        true, // wSOL is the market's long token → collateral side
        collateral_lamports,
        is_long,
        size_delta,
    );
    builder
        .execution_fee(execution_fee())
        .initial_collateral_token(&gm_pubkey(WSOL_MINT)?, Some(&wsol_ata));
    let (rpc, order) = builder
        .build_with_address()
        .await
        .map_err(|e| format!("GM order build failed: {e}"))?;
    let rpc = rpc.pre_instructions(wrap_ixs, false);
    tracing::info!("[GM-OPEN] order PDA: {order}");

    let sig = rpc
        .send()
        .await
        .map_err(|e| format!("GM open tx failed: {e}"))?;
    tracing::info!("[GM-OPEN] tx: {sig}");

    // Prefer TradeEvent execution_price. If the keeper fills faster than the
    // WS subscription can attach (common on mainnet — order PDA already gone
    // within a few seconds), recover entry price from the on-chain Position.
    let entry_price = match wait_for_fill(&v, &order).await? {
        Some(ev) => {
            let p = ev.execution_price as f64 / v.price_scale;
            tracing::info!("[GM-OPEN] FILLED @ ${p:.4} (TradeEvent)");
            p
        }
        None => match entry_price_from_open_position(&v, is_long).await? {
            Some(p) => {
                tracing::warn!(
                    "[GM-OPEN] TradeEvent missed for {order}; recovered entry \
                     ${p:.4} from on-chain Position (order filled)"
                );
                p
            }
            None => {
                return Err(format!(
                    "GM open {sig} confirmed but no TradeEvent was recovered \
                     and no on-chain Position exists for this side"
                ));
            }
        },
    };
    Ok((sig.to_string(), size_usd, entry_price))
}

/// Wait for a keeper fill with timeout. On timeout, cancel the order (best
/// effort) and error so the cycle backoff kicks in — an unfilled order must
/// never linger to fill unattended.
///
/// Fallback chain (when complete_order returns Ok(None)):
///   1. SDK historical scan via last_order_events (order PDA already closed)
///   2. Caller uses entry_price_from_open_position as the final recovery
async fn wait_for_fill(
    v: &Venue,
    order: &GmPubkey,
) -> GmResult<Option<gmsol_sdk::programs::gmsol_store::events::TradeEvent>> {
    use gmsol_sdk::decode::gmsol::programs::GMSOLCPIEvent;
    use solana_sdk::commitment_config::CommitmentConfig;

    let timeout = fill_timeout();
    let client = v.client.clone();
    let order_addr = *order;

    // Step 1: live fill watch via the SDK.
    let ws_trade = match tokio::time::timeout(timeout, client.complete_order(&order_addr, None))
        .await
    {
        Ok(Ok(t)) => t,
        Ok(Err(e)) => return Err(format!("GM fill watch error: {e}")),
        Err(_) => {
            tracing::warn!("[GM] fill timeout after {timeout:?}; cancelling order {order}");
            match client.close_order(&order_addr) {
                Ok(mut builder) => match builder.build().await {
                    Ok(rpc) => match rpc.send().await {
                        Ok(sig) => tracing::info!("[GM] order cancelled: {sig}"),
                        Err(e) => {
                            tracing::warn!("[GM] cancel send failed ({e}) — reconcile will recover")
                        }
                    },
                    Err(e) => tracing::warn!("[GM] cancel build failed: {e}"),
                },
                Err(e) => tracing::warn!("[GM] cancel builder failed: {e}"),
            }
            // Even after cancel attempt: if the order already filled, a
            // Position may exist. Caller will recover via on-chain query.
            None
        }
    };

    // Order-attribution check. The SDK fill watch subscribes to STORE-WIDE
    // CPI events and returns the last TradeEvent seen before any
    // OrderRemoved — it never verifies the event belongs to our order PDA.
    // Keepers batch fills from many traders in one tx, so a foreign fill can
    // be handed back (Aug 9: a close logged "FILLED @ $3.12 pnl $391.12" —
    // another trader's event on our $320 SOL long). Reject mismatches and
    // fall through to the order-PDA-scoped historical scan.
    let ws_trade = match ws_trade {
        Some(t) if t.order == order_addr => Some(t),
        Some(t) => {
            tracing::warn!(
                "[GM] fill watch returned a TradeEvent for foreign order {} (ours: {order}) — \
                 keeper batch mixed traders; scanning our order PDA instead",
                t.order
            );
            None
        }
        None => None,
    };
    if ws_trade.is_some() {
        return Ok(ws_trade);
    }

    // Step 2: historical scan scoped to our order PDA's signatures. The txs
    // can still be keeper batches carrying other traders' events, so apply
    // the same order-attribution filter here.
    tracing::warn!(
        "[GM] no attributable TradeEvent from fill watch for {order}; \
         trying historical event scan"
    );
    match client
        .last_order_events(&order_addr, u64::MAX, CommitmentConfig::confirmed())
        .await
    {
        Ok(events) => {
            let trade = events.into_iter().find_map(|ev| match ev {
                GMSOLCPIEvent::TradeEvent(t) if t.order == order_addr => Some(t),
                _ => None,
            });
            if trade.is_some() {
                tracing::info!("[GM] recovered TradeEvent from historical scan for {order}");
            } else {
                tracing::warn!(
                    "[GM] historical scan found no TradeEvent for our order {order} \
                     (only foreign events in shared keeper batches)"
                );
            }
            Ok(trade)
        }
        Err(e) => {
            tracing::warn!("[GM] historical event scan failed: {e}");
            Ok(None)
        }
    }
}

/// Recover average entry price from an on-chain GMTrade Position for our
/// wallet + market + side. Used when TradeEvent recovery fails but the
/// keeper already filled the increase order.
async fn entry_price_from_open_position(v: &Venue, is_long: bool) -> GmResult<Option<f64>> {
    let positions = v
        .client
        .positions(&v.store, Some(&v.owner), Some(&v.market_token))
        .await
        .map_err(|e| format!("on-chain position fallback failed: {e}"))?;

    for (_addr, pos) in positions {
        if pos.try_is_long().unwrap_or(true) != is_long {
            continue;
        }
        if pos.state.size_in_usd == 0 || pos.state.size_in_tokens == 0 {
            continue;
        }
        // Average execution price = size_usd / size_tokens, at unit-price scale.
        let entry =
            (pos.state.size_in_usd as f64 / pos.state.size_in_tokens as f64) / v.price_scale;
        if entry > 0.0 {
            return Ok(Some(entry));
        }
    }
    Ok(None)
}

/// Close a position on GMTrade. Reads the live position first (fresh size)
/// and market-decreases the FULL size; all proceeds (collateral ± pnl) are
/// unwrapped from wSOL back to native SOL and returned to the owner.
/// Returns (signature, pnl_usd) where pnl is the raw price PnL from the
/// TradeEvent (fees are logged separately).
pub async fn close_position(
    keypair: &solana_sdk::signature::Keypair,
    _market_symbol: &str,
    side: &str,
    _size_usd: &str,
    _withdraw_token: &str,
) -> GmResult<(String, f64)> {
    let v = venue().await?;
    assert_owner(&v, keypair)?;
    let is_long = side.eq_ignore_ascii_case("long");

    let size_units = {
        let positions = v
            .client
            .positions(&v.store, Some(&v.owner), Some(&v.market_token))
            .await
            .map_err(|e| format!("GM positions fetch failed: {e}"))?;
        let pos = positions
            .values()
            .find(|p| p.try_is_long().unwrap_or(true) == is_long)
            .ok_or_else(|| format!("no GM {side} position to close"))?;
        pos.state.size_in_usd
    };

    tracing::info!(
        "[GM-CLOSE] {} size ${:.2} (fresh from chain)",
        side,
        size_units as f64 / USD_SCALE
    );

    let mut builder = v.client.market_decrease(
        &v.store,
        &v.market_token,
        true, // wSOL is the market's long token → collateral side
        0,    // withdraw no extra collateral; all proceeds return
        is_long,
        size_units,
    );
    builder
        .execution_fee(execution_fee())
        .should_unwrap_native_token(true);
    let (rpc, order) = builder
        .build_with_address()
        .await
        .map_err(|e| format!("GM close build failed: {e}"))?;
    tracing::info!("[GM-CLOSE] order PDA: {order}");

    let sig = rpc
        .send()
        .await
        .map_err(|e| format!("GM close tx failed: {e}"))?;
    tracing::info!("[GM-CLOSE] tx: {sig}");

    let event = wait_for_fill(&v, &order).await?;
    let Some(ev) = event else {
        return Err(format!(
            "GM close {sig} confirmed but no TradeEvent was recovered"
        ));
    };
    let pnl_usd = ev.pnl.pnl as f64 / USD_SCALE;
    tracing::info!(
        "[GM-CLOSE] FILLED @ ${:.4} pnl ${pnl_usd:.4} | actual close-leg fees: \
         order=${:.6} borrow=${:.6} funding=${:.6} impact={:.6}",
        ev.execution_price as f64 / v.price_scale,
        ev.fees.order_fee_for_receiver_amount as f64 / USD_SCALE,
        ev.fees.total_borrowing_fee_amount as f64 / USD_SCALE,
        ev.fees.funding_fee_amount as f64 / USD_SCALE,
        ev.price_impact_value as f64 / USD_SCALE
    );
    Ok((sig.to_string(), pnl_usd))
}
