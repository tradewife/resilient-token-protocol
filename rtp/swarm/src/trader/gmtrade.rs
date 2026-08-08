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
//!   into the position atomically. We use USDC collateral (6dp); the trader
//!   wallet's USDC ATA needs no extra setup.
//! - Fill detection: `Client::complete_order(order)` polls CPI events until the
//!   order PDA is removed and returns the `TradeEvent` (execution price, pnl,
//!   fees). Wrapped in a timeout + cancel so an unfilled order cannot hang a
//!   cycle or fill unattended later.
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
use gmsol_sdk::builders::token::WrapNative;
use gmsol_sdk::client::ops::ExchangeOps;
use gmsol_sdk::solana_utils::cluster::Cluster;
use gmsol_sdk::solana_utils::instruction_group::{ComputeBudgetOptions, GetInstructionsOptions};
use gmsol_sdk::solana_utils::solana_sdk::instruction::Instruction;
use gmsol_sdk::solana_utils::solana_sdk::pubkey::Pubkey as GmPubkey;
use gmsol_sdk::solana_utils::solana_sdk::signature::{Keypair as GmKeypair, read_keypair_file};
use gmsol_sdk::IntoAtomicGroup;
use solana_sdk::signer::Signer;

use crate::trader::executor::PositionInfo;

const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";
const SOL_INDEX_MINT: &str = "So1Zu7vPQQxrguzUehKAyVLpjcc769zxgBuDAsxTUMH";
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

type GmClient = Client<Arc<GmKeypair>>;
type GmResult<T> = std::result::Result<T, String>;

/// Immutable venue handles, cheaply cloneable (Arc client + Copy pubkeys).
#[derive(Clone)]
struct Venue {
    client: Arc<GmClient>,
    store: GmPubkey,
    market_token: GmPubkey,
    /// 10^(20 - index_token_decimals); 1e11 for the 9-dp SOL index.
    price_scale: f64,
    /// Owner (trader wallet) the client signs with.
    owner: GmPubkey,
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

fn rpc_url() -> String {
    std::env::var("RTP_GM_RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string())
}

fn gm_pubkey(s: &str) -> GmResult<GmPubkey> {
    s.parse().map_err(|e| format!("bad pubkey {s}: {e}"))
}

/// Connect once: load the signer keypair, locate the default store and the
/// SOL/USD[WSOL-USDC] market, and compute the unit-price scale.
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

    // Locate SOL/USD[WSOL-USDC].
    let markets = client
        .markets(&store)
        .await
        .map_err(|e| format!("GM markets fetch failed: {e}"))?;
    let wsol = gm_pubkey(WSOL_MINT)?;
    let usdc = gm_pubkey(USDC_MINT)?;
    let sol_index = gm_pubkey(SOL_INDEX_MINT)?;
    let mut found = None;
    for (addr, market) in &markets {
        if market.meta.index_token_mint == sol_index
            && market.meta.long_token_mint == wsol
            && market.meta.short_token_mint == usdc
        {
            found = Some((addr.to_string(), market.meta.market_token_mint.to_string()));
        }
    }
    let (market_str, market_token_str) =
        found.ok_or("SOL/USD[WSOL-USDC] market not found in GMTrade store")?;
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
    let index_decimals = token_map
        .get(&market_account.meta.index_token_mint)
        .map(|c| c.token_decimals)
        .unwrap_or(9);
    let price_scale = 10f64.powi(20 - index_decimals as i32);

    tracing::info!(
        "[GM] venue ready — market: {market} market_token: {market_token} \
         index_decimals: {index_decimals} price_scale: {price_scale:e} owner: {owner}"
    );

    Ok(Venue {
        client,
        store,
        market_token,
        price_scale,
        owner,
    })
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

    let sol_price = get_sol_price().await?;
    let native_sol = native_sol_balance(&v).await?;

    // Available native SOL after safety reserves.
    let reserved = MIN_NATIVE_SOL_RESERVE_LAMPORTS as f64 / 1e9;
    let available_sol = ((native_sol - reserved).max(0.0)) * SOL_BALANCE_SAFETY;
    let collateral_sol = amount_sol.min(available_sol);

    if collateral_sol < 0.001 {
        return Err(format!(
            "GM open refused: collateral budget {amount_sol:.4} SOL vs available \
             {available_sol:.4} SOL (native wallet {native_sol:.4} minus reserve {reserved:.4})"
        ));
    }

    let collateral_usd = collateral_sol * sol_price;
    if collateral_usd < 1.0 {
        return Err(format!(
            "GM open refused: ${collateral_usd:.2} collateral below $1 floor"
        ));
    }
    let size_usd = collateral_usd * leverage;
    let collateral_lamports = (collateral_sol * 1e9) as u64;
    let size_delta = (size_usd * USD_SCALE) as u128;
    let is_long = trade_type.eq_ignore_ascii_case("long");

    tracing::info!(
        "[GM-OPEN] {} {:.4} SOL collateral (${collateral_usd:.2}) @ {leverage}x → \
         ${size_usd:.2} notional (SOL ${sol_price:.2}, native balance {native_sol:.4})",
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
    match tokio::time::timeout(timeout, client.complete_order(&order_addr, None)).await {
        Ok(Ok(Some(t))) => Ok(Some(t)),
        Ok(Ok(None)) => {
            // Fill+cleanup was faster than the WS attachment. Scan historical
            // signatures on the order PDA for a TradeEvent.
            tracing::warn!(
                "[GM] complete_order returned no TradeEvent for {order}; \
                 trying historical event scan"
            );
            match client
                .last_order_events(&order_addr, u64::MAX, CommitmentConfig::confirmed())
                .await
            {
                Ok(events) => {
                    let trade = events.into_iter().find_map(|ev| match ev {
                        GMSOLCPIEvent::TradeEvent(t) => Some(t),
                        _ => None,
                    });
                    if trade.is_some() {
                        tracing::info!(
                            "[GM] recovered TradeEvent from historical scan for {order}"
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
        Ok(Err(e)) => Err(format!("GM fill watch error: {e}")),
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
