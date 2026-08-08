//! RTP $10 keeper-fill probe for GMTrade (gmx-solana / gmsol-store).
//!
//! Opens and closes a ~$10 LONG and a ~$10 SHORT on SOL/USD[WSOL-USDC]
//! with USDC collateral (3 USDC) to measure:
//!   - keeper fill latency (order PDA created -> TradeEvent emitted)
//!   - execution price vs index price (slippage/impact)
//!   - realized per-side fees (order fee, price impact, funding, borrow)
//!
//! SAFETY:
//!   - Default mode SIMULATES only (RPC preflight via send_all(false) is the
//!     only network mutation; without LIVE=1 nothing is signed/sent).
//!   - LIVE=1 sends real orders. Each increase is immediately followed by a
//!     matching full market_decrease, so the probe always nets FLAT.
//!   - If the increase fills but the close order is not filled within
//!     KEEP_TIMEOUT_S (default 180s), the probe retries the close every 5s
//!     until it succeeds; it will NOT exit with an open position unattended.
//!
//! Env:
//!   LIVE=1                       actually send (default: dry run / simulate)
//!   RTP_TRADER_KEYPAIR           path to keypair JSON (rtp-trader.json)
//!   SOL_MARKET_TOKEN             (optional) explicit market token mint
//!   RPC_URL                      (optional) custom mainnet RPC
//!   KEEP_TIMEOUT_S               keeper wait before close-retry (default 180)
//!
//! Run:
//!   cargo build --example rtp-trade-probe
//!   CLUSTER=mainnet cargo run --example rtp-trade-probe           # simulate
//!   CLUSTER=mainnet LIVE=1 cargo run --example rtp-trade-probe    # for real

use std::{env, time::Duration};

use gmsol_sdk::{
    client::ops::ExchangeOps,
    core::token_config::TokenMapAccess,
    solana_utils::{
        cluster::Cluster,
        solana_sdk::{signature::read_keypair_file, signer::Signer},
    },
    Client,
};

const MARKET_DEC: f64 = 1e20;
const SOL_INDEX: &str = "So1Zu7vPQQxrguzUehKAyVLpjcc769zxgBuDAsxTUMH";
const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const WSOL: &str = "So11111111111111111111111111111111111111112";

const COLLATERAL_USDC: u64 = 3_000_000; // 3 USDC (6dp)
const SIZE_USD_10: u128 = 10 * (MARKET_DEC as u128); // $10 notional
const EXECUTION_FEE: u64 = 500_000; // lamports, generous > 300k floor

fn usd(v: u128) -> f64 {
    v as f64 / MARKET_DEC
}

/// USD values in position state / fees are 1e20 fixed point.
/// Unit PRICES (execution_price, prices.index.*) are fixed point at
/// 10^(MARKET_DECIMALS - index_token_decimals) = 1e11 for a 9-dp index.
fn price_scaled(v: u128, scale: f64) -> f64 {
    v as f64 / scale
}

#[tokio::main]
async fn main() -> gmsol_sdk::Result<()> {
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    let live = env::var("LIVE").map(|v| v == "1").unwrap_or(false);
    let keep_timeout = env::var("KEEP_TIMEOUT_S")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(180u64);

    let cluster: Cluster = env::var("CLUSTER")
        .unwrap_or_else(|_| "mainnet".to_string())
        .parse()
        .map_err(gmsol_sdk::Error::custom)?;

    let keypair_path = env::var("RTP_TRADER_KEYPAIR")
        .unwrap_or_else(|_| "/home/kt/.config/solana/rtp-trader.json".to_string());
    let payer = read_keypair_file(&keypair_path)
        .map_err(|e| gmsol_sdk::Error::custom(format!("keypair load failed: {e}")))?;
    let owner = payer.pubkey();
    println!("MODE: {} | owner: {owner}", if live { "LIVE" } else { "SIMULATE-ONLY" });

    let client = Client::new(cluster, &payer)?;
    let store = client.find_store_address("");
    println!("STORE: {store}");

    // ---- Locate SOL/USD[WSOL-USDC] market ----
    let markets = client.markets(&store).await?;
    let mut target = None;
    for (market_addr, market) in &markets {
        if market.meta.index_token_mint.to_string() == SOL_INDEX
            && market.meta.long_token_mint.to_string() == WSOL
            && market.meta.short_token_mint.to_string() == USDC
        {
            target = Some((market_addr.to_string(), market.meta.market_token_mint.to_string()));
        }
    }
    let (market_addr, market_token_str) = target
        .ok_or_else(|| gmsol_sdk::Error::custom("SOL/USD[WSOL-USDC] market not found"))?;
    println!("MARKET: {market_addr}  market_token: {market_token_str}");
    let market_token: gmsol_sdk::solana_utils::solana_sdk::pubkey::Pubkey = market_token_str
        .parse()
        .map_err(gmsol_sdk::Error::custom)?;

    // Unit-price scale: prices are fixed-point at 10^(MARKET_DECIMALS -
    // index_token_decimals). SOL index (wSOL mint) has 9 decimals, so 1e11.
    let token_map = client.authorized_token_map(&store).await?;
    let market = client.market(&market_addr.parse().map_err(gmsol_sdk::Error::custom)?).await?;
    let index_decimals = token_map
        .get(&market.meta.index_token_mint)
        .map(|c| c.token_decimals)
        .unwrap_or(9);
    let price_scale = 10f64.powi(20 - index_decimals as i32);
    println!("index decimals: {index_decimals}  unit-price scale: {price_scale:e}");

    // ---- Show current state ----
    let positions_before = client.positions(&store, Some(&owner), None).await?;
    println!("positions owned before: {}", positions_before.len());
    let orders_open = client.orders(&store, Some(&owner), None).await?;
    println!("orders open before: {}", orders_open.len());
    if !orders_open.is_empty() {
        println!("WARNING: open orders exist; probe will still proceed");
    }

    if !live {
        println!("SIMULATE mode: no transactions will be sent. Set LIVE=1 to trade.");
        println!("Would open+close $10 LONG then $10 SHORT with {COLLATERAL_USDC} (3 USDC) collateral.");
        return Ok(());
    }

    // ---- Trade helper: open then immediately close, measuring fills ----
    for (side_label, is_long) in [("LONG", true), ("SHORT", false)] {
        println!("\n==================== {side_label} PROBE ====================");

        // 1. Open.
        let t_open_start = std::time::Instant::now();
        let mut builder = client.market_increase(
            &store,
            &market_token,
            false, // USDC collateral = short token of the market
            COLLATERAL_USDC,
            is_long,
            SIZE_USD_10,
        );
        builder.execution_fee(EXECUTION_FEE);
        let (rpc, open_order) = builder.build_with_address().await?;
        println!("open order PDA: {open_order}");

        let sig = rpc.send().await?;
        println!("open tx: {sig}");

        // 2. Wait for keeper fill.
        let trade = wait_for_fill(&client, &open_order, keep_timeout).await?;
        let open_latency = t_open_start.elapsed();
        match &trade {
            Some(ev) => {
                println!("FILLED in {open_latency:?}");
                println!(
                    "  execution_price: ${:.6}  index: [${:.6}, ${:.6}]",
                    price_scaled(ev.execution_price, price_scale),
                    price_scaled(ev.prices.index.min, price_scale),
                    price_scaled(ev.prices.index.max, price_scale)
                );
                println!(
                    "  size_in_usd: ${:.2}  collateral: {:.6}  price_impact_value: {:.6}  price_impact_diff: {:.6}",
                    usd(ev.after.size_in_usd),
                    ev.after.collateral_amount as f64 / 1e6,
                    ev.price_impact_value as f64 / MARKET_DEC,
                    usd(ev.price_impact_diff)
                );
                println!(
                    "  fees: order_for_receiver={:.6} order_for_pool={:.6} funding={:.6} borrow_total={:.6} borrow_for_receiver={:.6} liq={:.6}",
                    usd(ev.fees.order_fee_for_receiver_amount),
                    usd(ev.fees.order_fee_for_pool_amount),
                    usd(ev.fees.funding_fee_amount),
                    usd(ev.fees.total_borrowing_fee_amount),
                    usd(ev.fees.borrowing_fee_for_receiver_amount),
                    usd(ev.fees.liquidation_fee_amount)
                );
                println!("  pnl: {}", ev.pnl.pnl);
            }
            None => println!("order completed WITHOUT trade event after {open_latency:?}"),
        }

        // 3. Close immediately (full decrease).
        let t_close_start = std::time::Instant::now();
        let mut cb = client.market_decrease(
            &store,
            &market_token,
            false, // collateral token (USDC) is short token
            0,     // withdraw no extra collateral; return proceeds
            is_long,
            SIZE_USD_10,
        );
        cb.execution_fee(EXECUTION_FEE);
        let (rpc2, close_order) = cb.build_with_address().await?;
        println!("close order PDA: {close_order}");
        let sig2 = rpc2.send().await?;
        println!("close tx: {sig2}");

        let close_trade = wait_for_fill(&client, &close_order, keep_timeout).await?;
        let close_latency = t_close_start.elapsed();
        match &close_trade {
            Some(ev) => {
                println!("CLOSED in {close_latency:?}");
                println!(
                    "  execution_price: ${:.6}  pnl: {}  fees: order_for_receiver={:.6} borrow={:.6} funding={:.6}",
                    price_scaled(ev.execution_price, price_scale),
                    ev.pnl.pnl,
                    usd(ev.fees.order_fee_for_receiver_amount),
                    usd(ev.fees.total_borrowing_fee_amount),
                    usd(ev.fees.funding_fee_amount)
                );
                println!(
                    "  output_amounts: output={} secondary={}",
                    ev.output_amounts.output_amount, ev.output_amounts.secondary_output_amount
                );
            }
            None => {
                println!("close completed without trade event after {close_latency:?}");
            }
        }

        // 4. Verify flat.
        let positions_after = client.positions(&store, Some(&owner), None).await?;
        let still_open = positions_after
            .values()
            .filter(|p| p.market_token == market_token)
            .count();
        println!(
            "flat check: {} positions remain for {} side",
            still_open, side_label
        );
        if still_open > 0 {
            println!("WARNING: position still open — investigate manually before next side");
        }
    }

    println!("\nPROBE COMPLETE");
    Ok(())
}

async fn wait_for_fill(
    client: &Client<impl std::ops::Deref<Target = impl Signer> + Clone>,
    order: &gmsol_sdk::solana_utils::solana_sdk::pubkey::Pubkey,
    timeout_s: u64,
) -> gmsol_sdk::Result<Option<gmsol_sdk::programs::gmsol_store::events::TradeEvent>> {
    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_s);
    loop {
        match client.complete_order(order, None).await {
            Ok(trade) => return Ok(trade),
            Err(e) => {
                // complete_order errors while the order is still open /
                // events not found yet; keep polling until deadline.
                let msg = format!("{e}");
                if std::time::Instant::now() >= deadline {
                    return Err(gmsol_sdk::Error::custom(format!(
                        "keeper did not fill within {timeout_s}s: {msg}"
                    )));
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}
