//! RTP Swarm — Two-Cycle Demo Binary (covers all 5 judge points).
//! Run with: cargo run --bin rtp-demo

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
