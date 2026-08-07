# Mandate Intake — Client #1 (working draft)

> Purpose: capture the engagement parameters for the first bespoke treasury
> engine (close friend). This is the template every future engagement will
> reuse — keep it honest and minimal. Derived from `perplexity-strat.md`
> (the original marubozu idea) and the RTP validation gates.

## 1. Client & custodian

| Field | Value |
|---|---|
| Client | (name / entity) |
| Engine operator | RTP swarm (treasury PDA execution) |
| Custody model | **Self-custody.** Client wallet retains all funds. The strategy only receives session-key execution permission for Flash Trade opens/closes; it can never withdraw. |
| Revocation | Client can revoke the session key at any time from their own wallet — instant kill switch, no operator cooperation needed. |

## 2. Mandate

| Field | Value | Notes |
|---|---|---|
| Instrument | SOL/USDT perpetuals on Flash Trade | The strategy family is SOL-native (born as the marubozu retracement idea); other symbols out of scope |
| Objective | Accumulate SOL while trading a defined risk budget | e.g. "turn idle USDC/SOL into more SOL" |
| Direction | Both long and short | Survivor-style bidirectional; net exposure follows the validated composite |
| Starting capital | (client fills) | Recommend ≥ 2.5 SOL operational floor (Flash min-collateral + runway) |
| Position sizing | 20% of capital per trade | Validated fraction; capped at Flash min-collateral |
| Leverage | ≤ 5x (validated envelope) | The v5/v6 leverage sweep passes 1–5x with 0 liquidations |
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
- v6 re-validation (2-year data): in progress — see
  `data/results/s15_v6/verdict.md`

**Nothing ships until the v6 gate suite passes on 2-year data.** That is the
anti-curve-fit bar the specimen (Survivor 2.69) had to clear.

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
3. **Execution venue risk.** Flash Trade is the sole venue; pool mechanics,
   fees, and API behavior can change (as v2 did) and may require engine
   updates.
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
