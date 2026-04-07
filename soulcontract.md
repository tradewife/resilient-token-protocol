# soulcontract.md

Constitutional governance layer for the Resilient Token Protocol.

## Core Values

1. The protocol exists to generate sustainable yield for adopting projects and their token holders
2. No single entity controls the treasury — PDA ownership is non-negotiable
3. Human sovereignty over irreversible decisions is absolute
4. The protocol must be economically rational to keep running (self-hydration)
5. Yield generation must never risk the principal beyond defined risk budgets

## What Can Evolve

- Strategy parameters (entry/exit thresholds, hold times, stop losses)
- Risk thresholds (max drawdown, position sizing, leverage)
- Execution venue weights (Hyperliquid vs Jupiter vs Solana lending)
- Redistribution splits (holder/dev/ecosystem percentages)
- Strategy portfolio composition (which symbols, which strategies)
- Wing performance benchmarks and health-check intervals
- Knowledge Wing ingestion sources and retention policies
- Security Wing scanning frequency and threat model

## What Cannot Evolve

- **Core values** — the five statements above are immutable
- **Human-sovereign control** — no amendment can remove the human approval requirement for irreversible actions
- **Self-modification of this contract** — amendments require human signature + 24h monitoring
- **Risk budget expansion** — increasing max risk without explicit human consent
- **PDA ownership** — treasury must always be PDA-owned, never key-owned
- **Fee immutability** — SPL TransferFeeConfig must remain immutable from mint.
  Once a token enables RTP fees, they cannot be revoked. The withdraw authority
  is locked into the mint account permanently — no "unmint" button exists.
- **Phase reversal** — once a phase transition occurs, it cannot be undone

## Amendment Protocol

1. **Propose** — any wing submits a diff to this document via the Coordinator
2. **Audit** — Audit Wing checks the proposal against core values and invariants
3. **Human sign** — a human operator cryptographically signs the amendment
4. **Deploy** — amendment is committed to the repository and enforced by the Coordinator
5. **Monitor** — 24-hour monitoring window begins
6. **Auto-rollback** — if system performance degrades > 5% post-amendment, revert automatically

## Enforcement

The Coordinator's `soulguard.rs` enforces this contract on every message. No wing can execute an action that violates an active constraint. The Audit Wing logs every compliance check and can reconstruct the full causal chain of any decision.

## Fee Routing Mechanism

Token projects adopt RTP by enabling `TransferFeeConfig` on their SPL Token-2022 mint,
setting the RTP Treasury PDA as the withdraw authority. Once set, this is immutable.

1. **Every trade** on the adopting token auto-deducts the configured fee
2. **Fees accumulate** in the mint's withheld token account
3. **Treasury program withdraws** via `token::withdraw_withheld_tokens_from_mint` CPI
4. **Fees land** in the Treasury PDA — owned by the program, no private key

This is not a custom integration or middleman. It's a standard SPL extension —
verifiable on Solana Explorer, used by thousands of tokens, and cryptographically
permanent once enabled.

## Phase Evolution Constraints

| Phase | Threshold | New Capabilities |
|-------|-----------|-----------------|
| Sustenance | < $50k | Reinvest all yield, no distribution |
| Ecosystem | $50k–$1M | Auto-provide LP to top RTP-adopting tokens |
| Humanity | > $1M | USDC grants to Solana public-goods projects |

Transitions are irreversible. Each phase adds capabilities but never removes constraints.
