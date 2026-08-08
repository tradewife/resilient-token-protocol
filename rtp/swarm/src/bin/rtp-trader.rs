//! RTP Autonomous Trader — Survivor 2.69 on Flash Trade mainnet.
//!
//! Usage:
//!   RTP_TRADER_KEYPAIR=~/.config/solana/id.json cargo run --bin rtp-trader
//!   RTP_TRADER_KEYPAIR=~/.config/solana/id.json RTP_TRADER_DRY_RUN=1 cargo run --bin rtp-trader

use rtp_swarm::trader::{TraderConfig, run_trader};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    eprintln!("[TRADER] RTP Autonomous Trader — Survivor 2.69");
    eprintln!("[TRADER] Strategy: SOL LONG, OOS Sharpe 3.96, 9/9 folds profitable");
    eprintln!();

    let config = match TraderConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            eprintln!();
            eprintln!("Required env vars:");
            eprintln!("  RTP_TRADER_KEYPAIR       Path to Solana keypair JSON");
            eprintln!();
            eprintln!("Optional env vars:");
            eprintln!("  RTP_TRADER_AMOUNT        SOL per trade (default: 0.20)");
            eprintln!("  RTP_TRADER_LEVERAGE      Leverage multiplier (default: 1.0)");
            eprintln!("  RTP_TRADER_POLL_SECS     Poll interval in seconds (default: 300)");
            eprintln!("  RTP_TRADER_DRY_RUN       Set to enable dry-run mode");
            eprintln!(
                "  RTP_TRADER_STATE_PATH    State file path (default: data/trader-state.json)"
            );
            std::process::exit(1);
        }
    };

    if let Err(e) = run_trader(config).await {
        eprintln!("[TRADER] Fatal: {}", e);
        std::process::exit(1);
    }
}
