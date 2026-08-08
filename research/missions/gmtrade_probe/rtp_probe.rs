//! RTP instrumentation probe for GMTrade (gmx-solana / gmsol-store).
//!
//! READ-ONLY: fetches all Market accounts for the default store on
//! mainnet and dumps fee parameters, live open interest, pool balances,
//! and current borrowing factors. No keypair is used for signing.
//!
//! Run: CLUSTER=mainnet cargo run --example rtp-probe --features ...
//! (see examples/Cargo.toml)

use std::env;

use gmsol_sdk::{
    solana_utils::cluster::Cluster,
    solana_utils::solana_sdk::signature::Keypair,
    Client,
};

const WSOL: &str = "So11111111111111111111111111111111111111112";
const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const SOL_INDEX: &str = "So1Zu7vPQQxrguzUehKAyVLpjcc769zxgBuDAsxTUMH";
const MARKET_DEC: f64 = 1e20; // MARKET_DECIMALS = 20 (u128 fixed point)

fn dec(v: u128) -> f64 {
    v as f64 / MARKET_DEC
}

fn short_name(bytes: &[u8; 64]) -> String {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(64);
    String::from_utf8_lossy(&bytes[..end]).to_string()
}

fn mint_label(pk: &str) -> &'static str {
    if pk == WSOL { "WSOL" } else if pk == USDC { "USDC" } else { "?" }
}

/// Kink-model borrowing factor per second at current usage
/// (mirrors BorrowingFeeKinkModelParams::borrowing_factor_per_second).
fn borrow_rate_per_sec(usage: f64, optimal: f64, base: f64, above: f64) -> f64 {
    if optimal <= 0.0 {
        return 0.0;
    }
    let mut rate = usage * base;
    if usage > optimal && 1.0 > optimal {
        let extra = (above - base).max(0.0) * (usage - optimal) / (1.0 - optimal);
        rate += extra;
    }
    rate
}

fn dump_market(pk: &str, market: &gmsol_sdk::programs::gmsol_store::accounts::Market, name: &str, sol_price: f64) {
    let cfg = &market.config;
    let pools = &market.state.pools;

    println!("\n========== {name} ({pk}) ==========");
    println!("  market_token_mint: {}", market.meta.market_token_mint);
    println!("  tokens: long={} short={} index={}",
        market.meta.long_token_mint, market.meta.short_token_mint, market.meta.index_token_mint);
    let flag_val = cfg.flag.value;
    println!("[FLAGS] raw={} | skip_borrow_for_smaller_side={} | ignore_oi_for_usage={} | market_closed_params={}",
        flag_val, flag_val & 1 == 1, flag_val & 2 == 2, flag_val & 4 == 4);

    // --- Order (open/close) fees ---
    println!("[ORDER FEES] (per side, of position size USD)");
    println!("  positive-impact factor: {:.4}%", dec(cfg.order_fee_factor_for_positive_impact) * 100.0);
    println!("  negative-impact factor: {:.4}%", dec(cfg.order_fee_factor_for_negative_impact) * 100.0);
    println!("  fee receiver factor:    {:.2}% of fee", dec(cfg.order_fee_receiver_factor) * 100.0);

    // --- Position impact ---
    println!("[POSITION IMPACT]");
    println!("  exponent: {}  positive factor: {:.3e}  negative factor: {:.3e}",
        cfg.position_impact_exponent,
        dec(cfg.position_impact_positive_factor),
        dec(cfg.position_impact_negative_factor));
    println!("  distribute factor: {:.4}  min impact pool: ${:.2}  impact pool now: ${:.2}",
        dec(cfg.position_impact_distribute_factor),
        dec(cfg.min_position_impact_pool_amount),
        dec(pools.position_impact.pool.long_token_amount));

    // --- Borrowing fee (kink model) at CURRENT usage ---
    let oi_long = dec(pools.open_interest_for_long.pool.long_token_amount);
    let oi_short = dec(pools.open_interest_for_short.pool.long_token_amount);
    let long_tok = market.meta.long_token_mint.to_string();
    let short_tok = market.meta.short_token_mint.to_string();
    let (lprice, sprice) = if long_tok == WSOL { (sol_price, sol_price) } else { (1.0, 1.0) };
    let pool_long_value = pools.primary.pool.long_token_amount as f64 / if long_tok == WSOL { 1e9 } else { 1e6 } * lprice;
    let pool_short_value = pools.primary.pool.short_token_amount as f64 / if short_tok == WSOL { 1e9 } else { 1e6 } * sprice;
    println!("[BORROWING FEE — kink model]");
    let usage_long = if pool_long_value > 0.0 { oi_long / pool_long_value } else { 0.0 };
    let usage_short = if pool_short_value > 0.0 { oi_short / pool_short_value } else { 0.0 };
    for (side, exp, opt, base, above, usage) in [
        ("long", cfg.borrowing_fee_exponent_for_long, cfg.borrowing_fee_optimal_usage_factor_for_long, cfg.borrowing_fee_base_factor_for_long, cfg.borrowing_fee_above_optimal_usage_factor_for_long, usage_long),
        ("short", cfg.borrowing_fee_exponent_for_short, cfg.borrowing_fee_optimal_usage_factor_for_short, cfg.borrowing_fee_base_factor_for_short, cfg.borrowing_fee_above_optimal_usage_factor_for_short, usage_short),
    ] {
        let (optimal, base_f, above_f) = (dec(opt), dec(base), dec(above));
        let rate = borrow_rate_per_sec(usage.min(1.0), optimal, base_f, above_f);
        println!("  {side}: exp={exp} optimal_util={optimal:.2} base={base_f:.2e}/s above={above_f:.2e}/s | NOW usage={usage:.3} rate={rate:.3e}/s = {:.4}%/hr = {:.3}%/day",
            rate * 3600.0 * 100.0, rate * 86400.0 * 100.0);
    }

    // --- Funding fee ---
    println!("[FUNDING FEE — adaptive]");
    println!("  exponent={} factor={:.6} max/s={:.3e} min/s={:.3e} inc/s={:.3e} dec/s={:.3e}",
        cfg.funding_fee_exponent, dec(cfg.funding_fee_factor),
        dec(cfg.funding_fee_max_factor_per_second), dec(cfg.funding_fee_min_factor_per_second),
        dec(cfg.funding_fee_increase_factor_per_second), dec(cfg.funding_fee_decrease_factor_per_second));
    println!("  stable threshold: {:.2}  decrease threshold: {:.4}",
        dec(cfg.funding_fee_threshold_for_stable_funding), dec(cfg.funding_fee_threshold_for_decrease_funding));
    let f_long = dec(pools.funding_amount_per_size_for_long.pool.long_token_amount);
    let f_short = dec(pools.funding_amount_per_size_for_short.pool.long_token_amount);
    println!("  cumulative funding/size: long={f_long:.6} short={f_short:.6}");

    // --- Liquidation ---
    println!("[LIQUIDATION] fee={:.3}%  min_collateral_factor={:.3}%",
        dec(cfg.liquidation_fee_factor) * 100.0,
        dec(cfg.min_collateral_factor_for_liquidation) * 100.0);

    // --- Limits & collateral floors ---
    println!("[LIMITS]");
    println!("  min position size: ${:.2}  min collateral: ${:.2}",
        dec(cfg.min_position_size_usd), dec(cfg.min_collateral_value));
    println!("  max OI long: ${:.0}  max OI short: ${:.0}",
        dec(cfg.max_open_interest_for_long), dec(cfg.max_open_interest_for_short));
    println!("  max pool amount: long_tok={} short_tok={}",
        cfg.max_pool_amount_for_long_token, cfg.max_pool_amount_for_short_token);

    // --- Live state ---
    println!("[LIVE STATE]");
    println!("  OI long: ${oi_long:.0}  OI short: ${oi_short:.0}  imbalance: ${:.0}", oi_long - oi_short);
    println!("  pool primary: long_tok={} ({:.2} tok = ${:.0})  short_tok={} ({:.2} tok = ${:.0})",
        pools.primary.pool.long_token_amount,
        pools.primary.pool.long_token_amount as f64 / if long_tok == WSOL { 1e9 } else { 1e6 },
        pool_long_value,
        pools.primary.pool.short_token_amount,
        pools.primary.pool.short_token_amount as f64 / if short_tok == WSOL { 1e9 } else { 1e6 },
        pool_short_value);
    println!("  collateral sum: long=${:.0} short=${:.0}",
        dec(pools.collateral_sum_for_long.pool.long_token_amount),
        dec(pools.collateral_sum_for_short.pool.long_token_amount));
    println!("  trade_count={} order_count={}", market.indexer.trade_count, market.indexer.order_count);
}

#[tokio::main]
async fn main() -> gmsol_sdk::Result<()> {
    let cluster: Cluster = env::var("CLUSTER")
        .unwrap_or_else(|_| "mainnet".to_string())
        .parse()
        .map_err(gmsol_sdk::Error::custom)?;
    let payer = Keypair::new(); // throwaway; reads only, never signs
    let client = Client::new(cluster, &payer)?;
    let store = client.find_store_address("");
    println!("STORE: {store}");

    let markets = client.markets(&store).await?;
    println!("MARKETS: {}", markets.len());

    // Crude SOL price from the deep USDC market's OI/pool ratio is unreliable;
    // fetch from the SOL/USD[USDC-USDC] market's index via a simple heuristic:
    // use pool token amounts of the WSOL-WSOL market (both WSOL) -> price
    // cancels. Fall back to a recent-ish constant if unknown.
    let sol_price: f64 = env::var("SOL_PRICE").ok().and_then(|s| s.parse().ok()).unwrap_or(170.0);
    println!("SOL price assumption: ${sol_price} (override with SOL_PRICE env)");

    let mut sol_markets = vec![];
    for (pk, market) in &markets {
        let name = short_name(&market.name);
        if market.meta.index_token_mint.to_string() == SOL_INDEX || name.starts_with("SOL") {
            sol_markets.push((pk.to_string(), market.clone(), name));
        }
    }
    println!("SOL-indexed markets found: {}", sol_markets.len());

    for (pk, market, name) in &sol_markets {
        dump_market(pk, market, name, sol_price);
    }

    // Also dump the deep USDC-collateral SOL market if matched by name prefix only
    for (pk, market) in &markets {
        let name = short_name(&market.name);
        if name.starts_with("SOL/USD") && !sol_markets.iter().any(|(p, _, _)| p == &pk.to_string()) {
            dump_market(&pk.to_string(), market, &name, sol_price);
        }
    }

    Ok(())
}
