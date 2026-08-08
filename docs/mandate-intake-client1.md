# Mandate Intake — Client #1 "RB" (working draft)

> Purpose: capture the engagement parameters for the first bespoke treasury
> engine (close friend, referred to as **RB**). This is the template every
> future engagement will reuse — keep it honest and minimal. Derived from
> `perplexity-strat.md` (the original marubozu idea) and the RTP
> validation gates. Venue decision 2026-08-08: **GMTrade**
> (`research/missions/s15_v8_venue_ranking.md`).

## 1. Client & custodian

| Field | Value |
|---|---|
| Client | RB (close friend; name/entity TBD) |
| Engine operator | RTP swarm (execution via venue adapter; custody stays with RB) |
| Venue | **GMTrade** (Solana-native perps, GMX V2 architecture). Selected 2026-08-08 via venue ranking: 10/10 gates, SOL-collateral longs with profits paid in SOL. HL-direct is the validated fallback if GMTrade live verification disappoints. |
| Custody model | **Self-custody.** RB's wallet retains all funds. The strategy only receives execution permission scoped to perps opens/closes on the venue; it can never withdraw. (Mechanic is venue-specific: Flash used delegate/session keys; GMTrade's equivalent is confirmed in the live verification probe.) |
| Revocation | RB can revoke execution permission at any time from their own wallet — instant kill switch, no operator cooperation needed. |

## 2. Mandate

| Field | Value | Notes |
|---|---|---|
| Instrument | SOL/USDT perpetuals on GMTrade | The strategy family is SOL-native (born as the marubozu retracement idea); other symbols out of scope |
| Objective | Accumulate SOL while trading a defined risk budget | GMTrade SOL-collateral longs pay profits in SOL — accumulation is native, no swap legs |
| Direction | Both long and short | Survivor-style bidirectional; net exposure follows the validated composite |
| Starting capital | (client fills) | Recommend ≥ 2.5 SOL operational floor (venue min-collateral + runway; exact GMTrade floor confirmed in live probe) |
| Position sizing | 20% of capital per trade | Validated fraction; capped at venue min-collateral |
| Leverage | ≤ 5x (validated envelope) | The v5/v6 leverage sweep passes 1–5x with 0 liquidations; GMTrade re-run pending live costs |
| Target cadence | ~0.2–0.5 trades/day | Marubozu setups are rare by design; no overtrading |
| Max drawdown | 25% hard cap, auto-suspend | Same drawdown gate as the on-chain lifecycle: breach → engine halts and requires review, never auto-resumes |
| Horizon | ≥ 6 months | Strategies need fold-length windows to express their edge |

## 3. The engine being delivered

The factory manufactures the strategy to this mandate — the deliverable is
the **S15 marubozu-with-confirmation** family (trigger: full-body candle in
trend; entry: retracement into the candle body; confirmation: close reasserts
before fill). Current state of validation (updates as gates re-run):

- v5 champion (1-year data): 20m composite, OOS +46.9%, Sharpe 2.38, both
  directions net positive
- v7e (2-year data, measured Flash v2 fees): **10/10 DEPLOYABLE** —
  OOS +49.8%, 334 trades, 2.5→3.19 SOL @5x (`s15_final_verdict.md`)
- v8 venue ranking (2026-08-08): **GMTrade 10/10**, HL-direct 10/10 —
  best venues tested (`s15_v8_venue_ranking.md`)
- v8e GMTrade **on-chain measured costs** (2026-08-08, commit
  `103a408`): order fees confirmed 0.010–0.012%/side; borrow
  skip-smaller-side, majority side 0.0036%/hr now. **10/10 in all
  three borrow scenarios** (+52.8% to +58.7%, 3.30–3.57 SOL @5x)

**Cost-validation gate is now CLEARED on GMTrade's live on-chain costs.**
Note: Survivor 2.69 itself skips paper trading — it has years of live
trading history and goes straight to GMTrade after keeper-fill probe
trades + account setup. (RB's manufactured strategy, when built, gets
its own validation path.)

## 4. Reporting & audit trail

- Every open/close is on-chain — verifiable in Solana Explorer, no trust
  required
- Trade ledger with per-trade fee breakdown (entry, exit, borrow, impact)
- Weekly summary: PnL (gross/net), win rate, drawdown, gate status
- The engine is the record: `data/results/s15_v6/` holds the full WFA
  artifacts the client can inspect

## 5. Risk disclosures (non-negotiable, client signs)

1. **Capital at risk.** Leveraged perpetuals can lose value; the 25% DD cap
   limits but does not eliminate loss.
2. **Strategy risk.** A validated backtest is a probability statement, not a
   promise. The 2025–26 SOL regime is not guaranteed to repeat.
3. **Execution venue risk.** GMTrade is RB's venue; pool mechanics, fees,
   and oracle behavior can change (Flash v1→v2 and its wind-down proved
   venues are not permanent) and may require engine updates or venue
   migration. Venue health is monitored continuously.
4. **Operator risk.** The swarm runs autonomously; a bug can cause an
   incorrect fill. On-chain audit trail limits damage visibility to zero —
   every action is inspectable.
5. **Not investment advice.** RTP delivers software + execution rails. The
   client owns the mandate decisions and their capital.

## 6. Commercials (placeholder — decide before first dollar)

- Options: flat setup fee + small AUM%, or performance fee on net profits,
  or flat monthly retainer
- Keep it simple for client #1: the goal is the case study, not revenue
  optimization

## 7. Exit criteria for this engagement

The engagement is "nailed" when: engine live ≥ 3 months, drawdown inside the
agreed envelope, zero liquidations, client can read their own audit trail
without help, and marginal setup cost for client #2 is visibly lower.
