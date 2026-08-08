# S15 v8 — Venue Ranking Verdict (Jupiter / Hyperliquid / GMTrade)

Generated: 2026-08-08 ~13:45 UTC
Method: identical v7e gate suite (equal 36-day folds, blind-touch composite,
2-year SOL data), with each venue's MEASURED cost basis swapped in.
Config tested: v5 champion, confirm_mode=none (limit-at-zone entries).

## Ranking

| # | Venue / path | Trip cost | Gates | OOS PnL | Long | Short | Final @5x | DD | Sensitivity |
|---|---|---|---|---|---|---|---|---|---|
| 1 | **GMTrade** (0.029%/trip both sides) | 0.029% | **10/10** | **+59.9%** | +15.0 | +44.9 | **3.63 SOL** | 18.5% | ROBUST (0 flips) |
| 2 | **Hyperliquid direct, maker entries** (limit 0.015% + taker 0.045%) | 0.06% | **10/10** | +49.6% | +11.9 | +37.7 | 3.19 SOL | 20.6% | ROBUST (0 flips) |
| 3 | Hyperliquid direct, taker/taker (0.045%×2) | 0.09% | **10/10** | +39.6% | +8.9 | +30.7 | 2.80 SOL | 22.6% | ROBUST |
| 4 | Jupiter hybrid native (long SOL-coll, short USDC) | 0.18/0.13% | 6/10 | +18.3% | −1.0 | +19.3 | 1.86 @1x | 25.8% | MODERATE |
| 5 | Jupiter pure-USDC funding | 0.373/0.13% | 5/10 | −1.0% | −20.3 | +19.3 | 1.76 @1x | 29.7% | MODERATE |
| 6 | Phantom wallet UI (0.095%/side incl. builder markup) | 0.19% | 6/10 | +6.2% | −1.1 | +7.3 | 1.80 @1x | 28.1% | MODERATE |
| ref | Flash v2 (measured, venue dying) | 0.06% | 10/10 | +49.8% | +12.0 | +37.8 | 3.19 SOL | 20.6% | ROBUST |

All variants: borrow/funding modelled at 0.0005%/hr both sides
(conservative; HL actually has no borrow fee — funding is peer-to-peer;
measured SOL funding avg +0.00048%/hr, 76% of hours positive).

## Findings

1. **GMTrade is the strongest candidate.** 10/10 gates with the BEST OOS
   of any venue tested (+59.9%, better than Flash's +49.8%) and the
   lowest drawdown (18.5%). Official docs confirm the accumulation
   mechanic the client thesis requires: "Long SOL with SOL as collateral"
   with "profits paid in SOL" (docs.gmtrade.xyz/about/trading). Crypto
   open/close fees 0.010–0.012% per side; modelled conservatively at
   0.012% + 0.005% impact = 0.029%/trip.
2. **Hyperliquid direct is a clean #2** — passes 10/10 even at
   taker/taker worst case; the blind-touch limit-at-zone entries earn
   maker rate (0.015%), reproducing Flash-equivalent economics (+49.6%).
   No borrow fee exists on HL. BUT: USDC margin only — accumulation
   requires an explicit USDC→SOL harvest swap (amortized, but real);
   and execution is off-Solana (bridged), breaking the pure
   Solana-native self-custody story.
3. **Trading through Phantom's perps UI fails the gates** (6/10, +6.2%)
   because of the 0.05% builder markup (0.095%/side). If Phantom is used
   at all, it is a funding/UX layer for humans — the engine must trade
   the venue API directly.
4. **Jupiter cannot host this config** in any accounting tested.
5. Grok's "pure USDC performs better" hypothesis is falsified by v8b
   (USDC funding costs LONGS +2×10bps swap legs; 5/10 vs hybrid 6/10).

## Caveats (status after v8e measured-cost pass)

- ~~**GMTrade costs are docs-based, not yet measured live.**~~
  **RESOLVED by v8e** — fees, borrow model, and funding read on-chain
  from the live store; all three borrow scenarios pass 10/10.
- **WSOL-USDC market liquidity is thin** ($48K long OI, 139K historical
  orders). Our position sizes are small, but impact-fill behavior at
  our notional must be confirmed with probe trades before RB capital.
- GMTrade runs keeper-executed orders like Flash; limit/trigger fill
  behavior must be probed live (the Flash lesson: UI and API paths can
  diverge).
- HL path requires bridge ops (Arbitrum EVM) and a conversion leg;
  operational complexity is higher than Solana-native.

## Decision input

- Accumulation thesis + Solana-native + measured gates all point to
  **GMTrade** as primary venue, with **HL direct (maker entries)** as
  the validated fallback.
- The client engine config (v5 champion, blind touch, 5x) is
  **MEASURED-COST VALIDATED on GMTrade (v8e, 10/10 in all three borrow
  scenarios)**. Remaining steps before RB capital: keeper-fill probe
  trades, account setup, then paper trading on the live venue.

## Scripts & artifacts

- `research/missions/s15_v8_jupiter_fee_check.py` (+ v8b, v8c HL variants)
- `research/missions/s15_v8d_gmtrade_fee_check.py`
- `research/missions/s15_v8e_gmtrade_measured.py` (measured-cost basis)
- `data/results/s15_v8_jupiter/`, `s15_v8_hl/`, `s15_v8_gmtrade/`
- HL funding query: `curl -X POST https://api.hyperliquid.xyz/info`
  `{"type":"fundingHistory","coin":"SOL"}` (500h: avg +0.000477%/hr)
- GMTrade probe: `venues/gmx-solana/examples/rtp_probe.rs`

---

## ADDENDUM 2026-08-08 ~14:40 UTC — LIVE-COST VERIFICATION PASSED (v8e)

The docs-based cost basis above has been superseded by **on-chain
measured costs**. Probe: `venues/gmx-solana/examples/rtp_probe.rs`
(gmsol-sdk, Rust) against mainnet store
`Gmso1uvJnLbawvw7yezdfCDcPydwW2s2iqG3w6MDucLo` (open-source gmx-solana,
Zenith-audited 2026-01-20).

**Measured SOL markets (SOL = $74.6 at probe time):**

| Market | Collateral | OI long / short | Borrow (majority side) |
|---|---|---|---|
| SOL/USD[USDC-USDC] `CJg17D…` | USDC | $4.83M / $4.87M | 0.0114%/hr (kink-max) |
| **SOL/USD[WSOL-USDC] `3M4vW1…`** | **WSOL** | **$47,990 / $2,000** | **0.0036%/hr** |
| SOL/USD[WSOL-WSOL] `G96vsS…` | WSOL | $747 / $2,578 | ~0.0001%/hr |

**Measured fee mechanics (all SOL markets identical):**
- Order fees: 0.010% (balance-improving) / 0.012% per side of position
  size USD. Fee receiver 25%. Docs CONFIRMED.
- `skip_borrow_for_smaller_side = TRUE` — the minority OI side pays ZERO
  borrow. Currently longs are the majority on the accumulation market, so
  longs pay, shorts pay 0.
- Borrow = usage × base_factor (kink model, optimal usage 0.75, base
  1.43e-8/s, above-optimal 3.17e-8/s). Charged on POSITION SIZE
  (leverage-inclusive).
- Funding: adaptive, cap 2.378e-8/s = 0.0856%/day. Modelled 0.0005%/hr
  both sides (conservative floor).
- Liquidation fee 0.05%; min position $1; min collateral $1.

**RB's accumulation market is SOL/USD[WSOL-USDC]** — the only SOL-market
where longs post SOL collateral and profits pay in SOL. Its borrow is
higher than the docs-basis assumed (0.0036%/hr vs 0.0005%/hr, ~7x),
because longs are currently the majority OI side.

**v8e gate results on the MEASURED cost basis (trip 0.029% + measured borrow):**

| Variant | Long borrow | Gates | OOS PnL | Long | Short | Final @5x | DD |
|---|---|---|---|---|---|---|---|
| E1_measured_now | 0.0036%/hr | **10/10** | +57.7% | +12.8 | +44.9 | 3.53 SOL | 18.96% |
| E2_stress_kink | 0.0114%/hr | **10/10** | +52.8% | +7.9 | +44.9 | 3.30 SOL | 19.96% |
| E3_low_usage | 0.0020%/hr | **10/10** | +58.7% | +13.8 | +44.9 | 3.57 SOL | 18.75% |

**All three pass 10/10**, including the stress variant where long borrow
is pinned at the kink-max. The edge absorbs the real borrow cost
comfortably; avg long hold 6.3h keeps borrow per trip small (~0.11% of
collateral at 5x). Sensitivity ROBUST, 0 flips, all variants.

**Institutional gate CLEARED**: the config now passes on the chosen
venue's MEASURED live costs, not docs. Remaining live checks before RB
capital are keeper-fill behavior and account setup, not strategy
economics.
