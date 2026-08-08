# S15 v8 — Jupiter Perps Fee Re-Check: VERDICT

Generated: 2026-08-08 ~12:46 UTC (v8b correction appended)
Cost basis: MEASURED on-chain Jupiter Perps SOL custody (2026-08-08)

## Result: FALSIFIED on Jupiter — 4/10 gates (but see v8b correction below)

| Metric | Flash v2 fees (v7e) | Jupiter fees (v8) |
|---|---|---|
| Gates | **10/10** | **4/10** |
| OOS total PnL | +49.8% | **−28.5%** |
| Consistency | 67% | 48% |
| Long leg | +12.0% | −1.0% |
| Short leg | +37.8% | **−27.5%** |
| Sensitivity | ROBUST (0 flips) | **FRAGILE (17 flips)** |
| Compounded | 2.5→3.19 SOL @5x | 1.64 SOL @1x, −34% |
| Max DD | 20.56% | 34.37% |
| Latency retention | 105% | 100% (PASS) |
| Trades/day | 0.51 | 0.51 (unchanged) |

## Cost basis (measured, not modelled)

Jupiter Perps SOL custody `7xS2gz2bTp3fwCC7knJvUWTEU9Tycczu6VhJYKgi1wdz`, 2026-08-08:
- Base fee: 6 bps open + 6 bps close = **0.12%/trip** (Flash v2: 0.04%)
- Linear impact: scalar 3.75e11 → ~0.0005%/trip at our sizes (negligible)
- **Additive imbalance penalty: ~5.3 bps on longs** — OI imbalance $7.98M
  exceeds the $1.50M threshold (feeFactor=1, exp=1, cap 32 bps/side).
  Applied conservatively to every long trip (imbalance side flips over time).
- **Shorts pay +2×10 bps swap fee** — short SOL is USDC-collateral, so each
  entry/exit crosses SOL↔USDC at the non-stable 10 bps rate = +0.20%/trip.
- Borrow: jump curve at 0.00146%/hr (8.9% util); modelled conservatively at
  0.002%/hr. (Flash v2: 0.0004%/hr.)

Per-trip cost vs Flash v2:
| Side | Jupiter | Flash v2 | Ratio |
|---|---|---|---|
| LONG | 0.18%/trip | 0.06%/trip | 3.0× |
| SHORT | 0.33%/trip | 0.06%/trip | 5.5× |

## Diagnosis

The blind-touch composite's edge is **cost-regime-specific**: it survives
0.06%/trip and dies at 0.18–0.33%/trip. At 0.51 trips/day the fixed per-trip
cost compounds to ~33–60%/yr of notional drag — the entire edge budget.
The short leg is destroyed by the collateral-asset swap fees; the long leg
is destroyed by base fees + imbalance penalty + borrow.

This is exactly the venue-dependency risk `docs/STRATEGIC-DIRECTION.md`
flagged — now demonstrated with measurements, not speculation. The v7e
DEPLOYABLE verdict was always Flash-specific; it does not transfer.

## Implications

1. **Friend's engine is NOT deployable on Jupiter as-configured.**
2. The pipeline did its job: gate suite + measured fees caught it before
   capital. This is the product working.
3. Three honest paths forward:
   a. **Re-forge on Jupiter cost basis** — Night-Shift-style search with
      `net_pnl_jupiter` as the cost model. The family may contain a
      lower-frequency config that amortizes the trip cost. Overnight job.
   b. **Cheaper venue** — GMTrade was reported at 0.4–0.6 bps (Flash-class).
      Verify measured fees; if real, the v7e config may survive there.
      Check SOL-collateral accumulation mechanic (the client's core ask).
   c. **Paper-first sequencing** — client #1 engagement is paper-only
      anyway; capital deployment waits for a cost-validated engine.

## Files

- Script: `research/missions/s15_v8_jupiter_fee_check.py`
- Gates: `data/results/s15_v8_jupiter/jupiter_gate_matrix.csv`
- Folds: `data/results/s15_v8_jupiter/folds_jupiter.csv`
- Instrumentation: `/home/kt/projects/rtp/venues/jupiter-perps-anchor-idl-parsing/src/examples/rtp-instrument.ts`

---

# v8b CORRECTION & USDC-ACCOUNTING TEST (2026-08-08 ~13:20 UTC)

## Modeling error corrected

v8 charged shorts a +0.20%/trip swap fee. That was wrong for Jupiter:
SOL shorts are **natively USDC-collateral** on Jupiter, so no swap leg is
paid by shorts. Additionally, the OI imbalance is **long-heavy**
($26.4M vs $18.4M vs $1.5M threshold), so shorts REDUCE imbalance and pay
**zero** additive penalty until crossover. Corrected short cost:
**0.13%/trip** (base 0.12 + linear ~0.001), not 0.33%.

## Question tested (Grok hypothesis)

"2.69 was not tested on a SOL model. Pure USDC trading would almost
certainly perform better on Jupiter."

## Variants run (exact v7e gate suite, corrected Jupiter costs)

| Variant | Long trip | Short trip | Gates | OOS | Long leg | Short leg |
|---|---|---|---|---|---|---|
| v7e Flash reference | 0.06% | 0.06% | **10/10** | **+49.8%** | +12.0 | +37.8 |
| v8 (SOL acct, shorts overcharged) | 0.18% | 0.33% | 4/10 | −28.5% | −1.0 | −27.5 |
| **A hybrid native** (Jupiter-native accounting) | 0.18% | **0.13%** | **6/10** | **+18.3%** | −1.0 | **+19.3** |
| **B pure USDC** (Grok proposal: USDC funds both sides) | **0.373%** | 0.13% | 5/10 | −1.0% | **−20.3** | +19.3 |
| C short-only (consistency check) | — | 0.13% | n/a | +19.3% | — | +19.3 but cons **33%**, medsh −0.66, folds_pos 7/21 |

## Verdicts on the hypothesis

1. **"Pure USDC performs better" is FALSE.** Variant B (0.373%/trip on
   longs = base + 2×10bps SOL↔USDC swaps + imbalance penalty) is WORSE
   than hybrid native: 5/10 vs 6/10, −1.0% vs +18.3%. Funding SOL longs
   from USDC adds two swap legs that native SOL collateral avoids.
2. **The shorts correction IS real and important.** With correct native
   accounting the short leg goes from −27.5% (v8) to **+19.3%** — the
   short leg SURVIVES Jupiter. The v8 falsification overstated the
   damage by ~10 gates' worth of short-leg drag.
3. **But the engine still does not deploy.** Even corrected, 6/10:
   longs die (−1.0%) under base + imbalance + borrow; dd 25.76% at 1x;
   sensitivity MODERATE (0 flips but spread 29.5). No leverage level
   passed the full gate battery.
4. **The surviving short leg cannot stand alone**: +19.3% total comes
   from cons 33% (14/21 folds non-positive, median Sharpe −0.66). It is
   a downtrend-regime artifact, not a strategy — the
   bidirectional-attribution gate was built precisely to reject it.

## What actually survives where

| Leg | Flash v2 (0.06%) | Jupiter native | Jupiter USDC-funded |
|---|---|---|---|
| LONG | +12.0 ✓ | −1.0 ✗ | −20.3 ✗ |
| SHORT | +37.8 ✓ | +19.3 ✓ (but regime-bound) | +19.3 ✓ |

The long leg requires Flash-class costs (~0.06%/trip) to survive. The
short leg survives Jupiter native costs. No current venue measured gives
both legs Flash-class pricing.

## Implications (updated)

1. Friend's engine remains NOT deployable on Jupiter (6/10, best case).
2. The USDC pivot does not help — it hurts the long leg.
3. Honest remaining paths:
   a. **Verify GMTrade measured fees** — if 0.4–0.6 bps is real AND SOL
      collateral/PnL mechanics support accumulation, v7e config may pass
      there unchanged. Highest expected value, cheapest test.
   b. **Re-forge a lower-frequency family** on Jupiter's native cost
      basis (short-heavy or short-only families need regime filters).
   c. **Phantom→Hyperliquid USDC path**: NOT yet cost-measured for our
      sizes; HL fees must be instrumented before it can be ranked. The
      accumulation outcome requires an explicit USDC→SOL conversion leg
      (extra swap cost per harvest).
