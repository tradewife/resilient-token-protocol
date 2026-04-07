# Demo Flow (3 minutes)

Hackathon demo script for RTP (Resilient Token Protocol).

## Setup

- Devnet RPC running
- Phantom wallet connected (CASH enabled)
- Treasury PDA deployed with test USDC
- Mock token with TransferFeeConfig enabled
- Night shift results visible

## Script

### 0:00 — The Hook (15 seconds)

"Every day on Solana, tokens rug. Not because the tech fails, but because 'don't rug' is a social promise — and social promises are cheap. RTP makes it code. Any token that adopts RTP structurally cannot rug. Here's how."

### 0:15 — Adoption (30 seconds)

Show: mock token creation with TransferFeeConfig pointing to RTP Treasury PDA.

"A token project enables RTP. Now every trade on their token auto-routes a fee to this treasury. The fee config is immutable — it can never be revoked. The treasury is PDA-owned — there is no private key. The rug vector is gone."

### 0:45 — The Flywheels (30 seconds)

Show: fee flowing in → treasury balance growing → floor price calculating.

"Those fees don't just sit there. The treasury does three things: it maintains a price floor via autonomous buybacks, it hedges with correlated SOL-shorts on Drift — which are structurally reliable because RTP tokens have no founder noise — and it deploys idle capital to yield protocols. These three flywheels compound. More fees → more buyback pressure → higher floor → stronger correlation → better hedges → more yield."

### 1:15 — Price Defense (30 seconds)

Show: market simulation where price dips → TWAP oracle triggers → buyback fires via Jupiter.

"The price dips. The TWAP oracle catches it. The circuit breaker checks — is this within safe bounds? Yes. The buyback fires through Jupiter. Treasury USDC buys the token. Price recovers. No human in the loop. No governance vote. Code-enforced."

### 1:45 — Verification (30 seconds)

Show: Solana Explorer with the full tx chain — fee routing, buyback CPI, circuit breaker check.

"Every single action is on-chain and provable. The Verifier agent publishes proof of every swap, every hedge, every buyback. You don't have to trust us. You don't have to trust a DAO vote. You verify on-chain."

### 2:15 — Redistribution (15 seconds)

Show: treasury hits threshold → auto-splits to holders, dev, ecosystem.

"When reserves cross the threshold, it auto-redistributes. 70% to holders pro-rata, 20% to dev operations, 10% to ecosystem LP. Atomic CPI — all or nothing. Auditable on Solana Explorer."

### 2:30 — The One-Liner (15 seconds)

- "Token adoption is one config change — TransferFeeConfig"
- "Defense is autonomous — no governance, no trust required"
- "Every action is verifiable — on-chain proof, not promises"

### 2:45 — Q&A Buffer

## Backup Demos

1. **Yield brain results** — "30K strategies tested last night, +118% PnL, 9-fold validation" (shows proven research engine)
2. **Circuit breaker stress test** — simulate rapid withdrawal attempts → breaker blocks
3. **Hedge payoff** — show SOL drawdown → Drift hedge profit → funds buybacks
4. **Token comparison** — side-by-side: non-RTP token vs RTP token during market dump
