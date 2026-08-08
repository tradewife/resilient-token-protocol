# GMTrade (gmx-solana) on-chain probe

Read-only mainnet instrumentation for GMTrade fee verification (v8e).

## What it reads

Fetches all `Market` accounts from the GMTrade store
(`Gmso1uvJnLbawvw7yezdfCDcPydwW2s2iqG3w6MDucLo`) via the gmsol-sdk
`Client::markets()` API and prints:

- Order fees (positive/negative impact factor, per side of position size)
- Borrowing fee kink model at CURRENT pool usage (base, above-optimal)
- Funding fee parameters + cumulative funding/size
- Liquidation fee, min position/collateral
- Live OI, pool balances, imbalance, collateral sums

## Run

```bash
cd venues/gmx-solana/examples
cargo build --example rtp-probe
CLUSTER=mainnet SOL_PRICE=$(price) cargo run --example rtp-probe --quiet
```

SOL_PRICE feeds the pool-value/utilization math; get it from Pyth or
CoinGecko. No keypair signs anything — a throwaway Keypair is passed
only to satisfy the Client constructor for reads.

## Key facts measured (2026-08-08, SOL=$74.6)

| Field | Value |
|---|---|
| Order fee positive/negative | 0.010% / 0.012% per side |
| Fee receiver factor | 25% of fee |
| Borrow skip-smaller-side | TRUE (minority OI pays 0) |
| Borrow base / above-optimal | 1.43e-8 / 3.17e-8 per second |
| Optimal usage | 0.75 |
| Funding cap | 2.378e-8/s (0.0856%/day) |
| Liquidation fee | 0.05% |
| WSOL-USDC OI long/short | $47,990 / $2,000 |

RB's accumulation market: `SOL/USD[WSOL-USDC]` = `3M4vW1u8RT3HJSWqgEN1WuiUJZuVjJLQYEWvCHCuk56g`.
