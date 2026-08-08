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

## Caveats (non-negotiable before client capital)

- **GMTrade costs are docs-based, not yet measured live.** The
  institutional rule stands: no capital until a measured-cost pass
  (on-chain fee parameters + small live probe trades), exactly as was
  done for Flash v2. Docs fees exclude realized price impact, funding,
  and borrowing — all adaptive.
- GMTrade liquidity/depth for SOL markets must be verified (volume
  reported ~$640M/24h but SOL-market OI and oracle quality unconfirmed).
- GMTrade runs keeper-executed orders like Flash; limit/trigger fill
  behavior must be probed live (the Flash lesson: UI and API paths can
  diverge).
- HL path requires bridge ops (Arbitrum EVM) and a conversion leg;
  operational complexity is higher than Solana-native.

## Decision input

- Accumulation thesis + Solana-native + measured gates all point to
  **GMTrade** as primary venue, with **HL direct (maker entries)** as
  the validated fallback if GMTrade's live probe disappoints.
- The client engine config (v5 champion, blind touch, 5x) is
  **DEPLOYABLE on GMTrade pending the live-cost verification step**.

## Scripts & artifacts

- `research/missions/s15_v8_jupiter_fee_check.py` (+ v8b, v8c HL variants)
- `research/missions/s15_v8d_gmtrade_fee_check.py`
- `data/results/s15_v8_jupiter/`, `s15_v8_hl/`, `s15_v8_gmtrade/`
- HL funding query: `curl -X POST https://api.hyperliquid.xyz/info`
  `{"type":"fundingHistory","coin":"SOL"}` (500h: avg +0.000477%/hr)
