# soulcontract.md

Constitutional governance layer for the Resilient Token Protocol.

RTP is post-governance. Commitments are enforced by code, not votes. This contract defines what the protocol promises, what can change, and what is permanently locked.

---

## The Core Promise

**Any token that adopts RTP structurally cannot rug.**

This is not a social commitment. It is a cryptographic and economic property enforced by:
- PDA-owned treasury (no private key can drain it)
- Immutable TransferFeeConfig (fees cannot be revoked post-adoption)
- Circuit breakers (treasury cannot be emptied in any single event)
- On-chain verification (every agent action is provable)
- Price floor enforcement (buybacks fire algorithmically via TWAP oracle)

## Core Values

1. **"Don't rug" is enforced, not stated** — the protocol makes rug-pulls structurally impossible for adopting tokens
2. **No single entity controls the treasury** — PDA ownership is non-negotiable
3. **Human sovereignty over irreversible decisions** — amendments require human signature
4. **Economic rationality** — the protocol is self-funding via fee capture, making it irrational to shut down
5. **Verifiability** — every action is on-chain, timestamped, and reconstructable
6. **Correlated defense** — hedging is structurally reliable because RTP tokens have higher SOL correlation
7. **Circuit breakers are sacred** — no amendment can weaken the drain protection layers

## What Can Evolve

- Strategy parameters (buyback thresholds, hedge weights, floor discount multipliers)
- Risk thresholds (max drawdown, position sizing, leverage limits)
- Redistribution splits (holder/dev/ecosystem percentages)
- Yield deployment targets (Kamino vs Marginfi weights)
- Circuit breaker parameters (cooldown duration, epoch caps, velocity limits) — within bounds
- Agent skill registry (add/retire skills via Verifier certification)
- Phase transition thresholds

## What Cannot Evolve

- **The core promise** — adopting tokens structurally cannot rug
- **PDA ownership** — treasury must always be PDA-owned, never key-owned
- **TransferFeeConfig immutability** — once enabled on a mint, fees are permanent
- **Circuit breaker existence** — drain protection cannot be removed, only parameter-tuned
- **Human sovereign control** — no amendment can remove the human approval requirement
- **On-chain verification** — the Verifier agent cannot be disabled
- **No SOL liquidation** — USDC-only risk management flows
- **Phase reversal** — once a phase transition occurs, it cannot be undone
- **Fee immutability from mint** — adopting projects cannot revoke RTP fees

## Amendment Protocol

1. **Propose** — any agent submits a diff to this document via the Coordinator
2. **Verify** — Verifier agent checks the proposal against core values and invariants
3. **Human sign** — a human operator cryptographically signs the amendment
4. **Deploy** — amendment committed and enforced by the Coordinator
5. **Monitor** — 24-hour monitoring window begins
6. **Auto-rollback** — if system performance degrades > 5% post-amendment, revert automatically

## Circuit Breaker Invariants

The three-layer drain protection is the backbone of the "cannot rug" promise:

| Layer | Purpose | Cannot Be |
|-------|---------|-----------|
| **Cooldown** | Minimum time between treasury operations | Removed |
| **Epoch cap** | Maximum USDC spendable per epoch | Removed or set to unlimited |
| **Velocity limit** | Maximum rate of reserve depletion | Removed or set to unlimited |

Parameters within these layers can be tuned. The layers themselves are permanent.

## Price Floor Invariant

The price floor (`treasury_value_usd / circulating_supply`) is enforced by:
- Pyth TWAP oracle (manipulation-resistant price feed)
- Circuit breaker on buyback execution (prevents self-dealing)
- Minimum reserve protection (buybacks cannot drain treasury below floor reserve)

## Redistribution Invariant

Above threshold, redistribution is:
- **Atomic** — all splits execute in a single CPI or none execute
- **Auditable** — every split is an on-chain instruction
- **Proportional** — holder share is always pro-rata by token balance

## Phase Evolution Constraints

| Phase | Threshold | New Capabilities | Permanent Properties |
|-------|-----------|-----------------|---------------------|
| Sustenance | < $50k | Reinvest all yield | Circuit breakers, verification, floor |
| Ecosystem | $50k–$1M | Auto-provide LP to RTP-adopting tokens | All Sustenance properties |
| Humanity | > $1M | USDC grants to public-goods projects | All previous properties |

Transitions are irreversible. Each phase adds capabilities but never removes constraints.

## Enforcement

The Coordinator's `soulguard.rs` enforces this contract on every message. No agent can execute an action that violates an active constraint. The Verifier agent logs every compliance check and can reconstruct the full causal chain of any decision the swarm ever made.
