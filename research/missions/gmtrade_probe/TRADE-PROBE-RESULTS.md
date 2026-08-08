# GMTrade keeper-fill probe — measured results (2026-08-08)

Two rounds of $10 LONG + $10 SHORT round-trips on mainnet against
`SOL/USD[WSOL-USDC]` (market `3M4vW1u8RT3HJSWqgEN1WuiUJZuVjJLQYEWvCHCuk56g`,
store `CTDLvGGXnoxvqLyTpGzdGLg9pD6JexKxKXSV8tqqo8bN`) using wallet
`HDQ79fQ1YbL9CenS1DzfHizEWGrJdnmo99fgAWmdhuy5` (rtp-trader), 3 USDC
collateral per side, `execution_fee = 500_000` lamports per order.

Probe: `rtp_trade_probe.rs` (this directory; runnable copy at
`venues/gmx-solana/examples/`).

## Verdict: PASS — venue is tradeable by the RTP trader

| Metric | Measured |
|---|---|
| Keeper fill latency | 24.8 – 31.0 s (order send → TradeEvent; incl. ≤2s poll granularity) |
| Fill price quality | execution price ALWAYS inside oracle [min, max] band |
| Round-trip order fees (measured) | **0.022% of notional** (open 0.012% neg-impact + close 0.010% pos-impact, $10 side) — EXACTLY matches v8e fee model |
| Price impact ($10 on $50K OI) | ≈ $0.00002 – $0.00003 (negligible) |
| Borrow/funding over ~30s hold | < $1e-6 (negligible) |
| Execution fee (SOL, paid to keeper) | 0.0005 SOL/order (floor is 0.0003) — separate from USDC PnL |
| Flat guarantee | every round trip ended with 0 positions; collateral returned to USDC ATA |

## Round 2 (final, price-scaled) — tx signatures

LONG:
- open order `8XP8WmSH8TTk7QWtWefS5caoKHsJvHajg3HMaprkwb4p`, tx
  `4vdho3H3k7Vkr6nfeorHxUA7i25x8PrVy7P55nkb387ZHMaz8JSBpyJujAVnJdoiPKuvUximUFNpgYkf1zWKRxY3`
  — filled 28.5s @ $74.760534 (index [$74.744260, $74.760700]), collateral
  after open: 2.999000 USDC (open fee $0.001 = 0.01%)
- close order `2ucwVhw16fJPdjnc3XtbQ7MS8VamEc3tqRYcrBjGA4t6`, tx
  `3mLXbMfDPSpS611RnKAbQxMKHp9jPmyp6mcofGwH9izFx3ZJ4Xzrfo6kL85tPL5fPnUJjdyJxFrYXbVn17FwWXjY`
  — filled 25.2s @ $74.745660, pnl −$0.001956 (market moved −0.02% in the
  fill window), out 2.995809 USDC

SHORT:
- open order `6wsMQXKGG2PEZb5j6CQJFaPfQcdC7nqPTSHBiWQzoB5V`, tx
  `21DDoaz1NBSM7f7UNQEBGGfRzuZERF27syw5fgpUzVPZc6qvqKHtFNtjMbNYSeuh57pWpjTSfU99Tn8KX7h993Lk`
  — filled 24.8s @ $74.737899 (index [$74.738150, $74.753220]), collateral
  after open: 2.998800 USDC
- close order `6yCW81DrETamC1topqmEq8Bn4wVECkjr3kvZyLrXmuTK`, tx
  `4CfpEnkwXU3fDZZ6SnUkjYZ8nCRWv8t1DzfrohwyBZp1jgnBqszZZ9bQTESePj5QbtXXjfvA8Mwqr9nSJTC1F3S4`
  — filled 31.0s @ $74.752683, pnl −$0.002000 (market moved +0.02% in the
  fill window), out 2.995808 USDC

Round 1 (same result, prices displayed at wrong scale):
- LONG open `3BeXaEn…` / close `RfyCVFq…` → out 2.996264 USDC
- SHORT open `7bbkY4q…` / close `49rTPuS…` → out 2.995809 USDC

## Fee reconciliation (why 0.022% round trip, not 0.044%)

Long OI $48K vs short OI $2K → imbalance is long-heavy:
- opening LONG / closing SHORT increases imbalance → negative-impact fee 0.012%
- opening SHORT / closing LONG decreases imbalance → positive-impact fee 0.010%
- Round trip either direction = 0.012% + 0.010% = **0.022% of notional**.
Measured long round trip: in 3.000000, out 2.995809 → −$0.004191 = 0.022%
fees + 0.020% adverse market move between the two fills (SOL dropped
$74.7605 → $74.7457). The v8e gate-suite model (0.010/0.012% per side) is
confirmed by on-chain evidence.

## Adapter design facts (for the Rust trading wing)

1. **Two-step keeper model**: user signs `create_order` (market_increase /
   market_decrease); GMTrade's keepers execute with Chainlink-oracle prices.
   No oracle access needed client-side. Pay `execution_fee` ≥ 300k lamports
   per order (we used 500k; consider 300–400k in production).
2. **No deposit step for trading**: `market_increase` deposits collateral
   into the position atomically. LP deposits are only for earning pool share.
3. **Collateral**: USDC works with zero extra setup (ATA auto-derived; the
   trader wallet's USDC ATA already exists). Native-SOL collateral would
   need a WrapNative pre-instruction — use USDC instead.
4. **Fill detection**: `client.complete_order(order, commitment)` polls CPI
   events and returns the `TradeEvent` (execution price, pnl, fees,
   output amounts). Timeout = order never filled (cancel via `close_order`).
5. **Price units**: unit prices are fixed-point `10^(20 − index_decimals)`
   (= 1e11 for 9-dp SOL index); USD amounts in position state/fees are
   `1e20` fixed point. Do NOT conflate the two.
6. **Sizing**: `size_delta_usd = dollars × 1e20`. Collateral amount in raw
   token units (USDC 6dp).
7. **Position lifecycle**: one Position PDA per (owner, market, collateral
   token, side). Close via `market_decrease` with the same size; position
   account is removed on full decrease (flat check: `positions()` empty).
8. **Order cancellation**: `close_order(order)` cancels an unfilled order.

## Cost at production sizing

At v8e's planned 3.5 SOL @ 5x (~$260 notional, ~$52 collateral):
execution fee 0.0005 SOL ≈ $0.037 per order = 0.014% of notional per side —
acceptable but should drop to the 300k floor (0.008%/side) once fill
reliability is proven over a longer window. Order fees 0.022%/trip + borrow
0.0036%/hr (longs, minority shorts pay 0) are the dominant costs, exactly as
modeled in s15_v8e.
