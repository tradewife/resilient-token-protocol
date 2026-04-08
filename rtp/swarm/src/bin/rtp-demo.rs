//! RTP Swarm — End-to-End Demo Binary
//!
//! Run with: cargo run --bin rtp-demo
//!
//! Demonstrates the full swarm coordination pipeline:
//!   1. Trading Wing proposes a strategy
//!   2. Coordinator routes to Audit Wing for tribunal review
//!   3. Audit Wing approves (Byzantine consensus)
//!   4. Coordinator sends ExecutePermit to Trading Wing
//!   5. Trading Wing executes via bridge → YieldReport
//!   6. Knowledge Wing stores yield data
//!   7. Security Wing monitors for anomalies
//!   8. Futureproof Wing checks deprecation status

use rtp_swarm::demo::{run_demo_loop, print_demo_result};

#[tokio::main]
async fn main() {
    println!("RTP Swarm — End-to-End Demo");
    println!("============================\n");

    let result = run_demo_loop().await;
    print_demo_result(&result);

    if result.success {
        println!("\nDemo completed successfully.");
    } else {
        println!("\nDemo completed with failures (see above).");
        std::process::exit(1);
    }
}
