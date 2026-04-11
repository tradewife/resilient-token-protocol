//! RTP Swarm — Two-Cycle Demo Binary
//!
//! Run with: cargo run --bin rtp-demo
//!
//! Demonstrates all 5 judge points:
//!   1. On-chain constraint rejection (visible log line)
//!   2. Autonomous operation (8-step pipeline)
//!   3. Memory persistence (cycle 1 → cycle 2 reference)
//!   4. Heartbeat redirect (visible log line)
//!   5. Treasury state (explorer URLs in output)

use rtp_swarm::demo::{print_two_cycle_demo, run_two_cycle_demo};

#[tokio::main]
async fn main() {
    let result = run_two_cycle_demo().await;
    print_two_cycle_demo(&result);

    if result.success {
        println!();
        println!("All 5 judge points covered ✅");
        std::process::exit(0);
    } else {
        println!();
        println!("Demo completed with failures ❌");
        std::process::exit(1);
    }
}
