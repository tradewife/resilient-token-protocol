//! End-to-end demo: runs the full swarm coordination pipeline.
//!
//! Usage: cargo run --example demo
//!
//! Demonstrates:
//!   1. All 6 wings registered with Coordinator
//!   2. Trading Wing proposes strategy
//!   3. Audit Wing tribunal reviews (Byzantine consensus)
//!   4. ExecutePermit routed to Trading Wing
//!   5. Trading Wing executes via bridge → YieldReport
//!   6. Knowledge Wing stores and queries yield data
//!   7. Security Wing monitors for anomalies
//!   8. Futureproof Wing heartbeat

#[tokio::main]
async fn main() {
    let result = rtp_swarm::demo::run_demo_loop().await;
    rtp_swarm::demo::print_demo_result(&result);

    if !result.success {
        std::process::exit(1);
    }
}
