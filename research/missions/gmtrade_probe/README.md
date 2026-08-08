# GMTrade (gmx-solana) on-chain probe

Mainnet instrumentation for GMTrade fee verification (v8e) and tradeability
proof (keeper-fill probes). Two probes + measured results.

## Files

| File | Purpose |
|---|---|
| `rtp_probe.rs` | READ-ONLY fee/state dump of all SOL markets |
| `rtp_trade_probe.rs` | $10 LONG + SHORT round-trip trades (needs LIVE=1) |
| `TRADE-PROBE-RESULTS.md` | Measured trade results, fee reconciliation, adapter design facts |

## Addresses (mainnet)

- Store PROGRAM: `Gmso1uvJnLbawvw7yezdfCDcPydwW2s2iqG3w6MDucLo` (gmsol-store)
- Store ACCOUNT (PDA from `find_store_address("")`): `CTDLvGGXnoxvqLyTpGzdGLg9pD6JexKxKXSV8tqqo8bN`
- RB's market: `SOL/USD[WSOL-USDC]` = `3M4vW1u8RT3HJSWqgEN1WuiUJZuVjJLQYEWvCHCuk56g`

## What it reads

Fetches all `Market` accounts from the GMTrade store via the gmsol-sdk
`Client::markets()` API and prints:

- Order fees (positive/negative impact factor, per side of position size)
- Borrowing fee kink model at CURRENT pool usage (base, above-optimal)
- Funding fee parameters + cumulative funding/size
- Liquidation fee, min position/collateral
- Live OI, pool balances, imbalance, collateral sums

## Run

```bash
# read-only fee probe
cd venues/gmx-solana/examples
CLUSTER=mainnet SOL_PRICE=$(price) cargo run --example rtp-probe --quiet

# $10 keeper-fill probe (SIMULATE unless LIVE=1)
CLUSTER=mainnet cargo run --example rtp-trade-probe --quiet          # dry run
CLUSTER=mainnet LIVE=1 cargo run --example rtp-trade-probe --quiet   # trade
```

SOL_PRICE feeds the pool-value/utilization math; get it from CoinGecko.
The read-only probe passes a throwaway Keypair (never signs). The trade
probe loads `RTP_TRADER_KEYPAIR` (default `~/.config/solana/rtp-trader.json`).

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
| Min position / collateral | $1 / $1 |
| WSOL-USDC OI long/short | $47,990 / $2,000 |
| Keeper fill latency (measured) | 24.8 – 31.0 s |
| Round-trip order fees (measured) | 0.022% of notional |

See `TRADE-PROBE-RESULTS.md` for the full trade log, tx signatures, and
adapter design facts.

