# S15 v8 — Jupiter Perps Fee Re-Check: VERDICT

Generated: 2026-08-08 ~12:46 UTC
Cost basis: MEASURED on-chain Jupiter Perps SOL custody (2026-08-08)

## Result: FALSIFIED on Jupiter — 4/10 gates

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
