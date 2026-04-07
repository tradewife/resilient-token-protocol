# Demo Flow (3 minutes)

Hackathon demo script for RTP (Resilient Token Protocol).

## Setup

- Devnet RPC running (Triton One free tier)
- Phantom wallet connected (CASH enabled)
- Treasury PDA deployed with test funds
- Night shift results visible (from latest run)

## Script

### 0:00 — Hook (15 seconds)

"This is RTP — a Solana treasury that generates its own yield, funds its own operations, and runs forever. Six autonomous wings govern it. No human needed for day-to-day."

### 0:15 — Adoption (30 seconds)

Show: a token project enabling TransferFeeConfig, pointing fees to the RTP Treasury PDA.

"Any Solana token project can adopt RTP. They enable TransferFeeConfig on their mint — it's immutable once set, it can never be revoked. From that point, every trade on their token auto-routes a fee to this treasury. The swarm goes to work."

### 0:45 — The Research Engine (30 seconds)

Show terminal: `data/night_results/latest/report.md`

"Last night, while we slept, the yield brain tested 30,000 strategy configurations across 9 independent validation windows. The best one: +118% PnL, 78% consistency, 429 validated trades. This is not a backtest — these are out-of-sample walk-forward results with realistic fees and slippage."

### 1:15 — Wing Architecture (30 seconds)

Show: Coordinator routing a proposal through Audit Wing.

"The Trading Wing proposes a deployment. The Audit Wing checks it against the soulcontract — our constitutional governance layer. Every action, every transaction, every proposed change must pass. The swarm is autonomous, but never uncontrolled."

### 1:45 — Fee Flow (30 seconds)

Show: trading fees → Treasury PDA → redistribution on devnet.

"Fees flow in from the adopting project's trades. The swarm converts to USDC and puts it to work. At threshold, it auto-redistributes: 70% to the project's token holders, 20% to the project dev, 10% to ecosystem. The project and its holders benefit — no trust required."

### 1:45 — Self-Hydration (30 seconds)

Show: sustenance PDA balance.

"10% of yield auto-hydrates the swarm's own operations — RPC costs, transaction fees, compute. At $10k reserves generating 20-50% annual yield, ops cost is $100-200/month. The system is economically irrational to shut down."

### 2:15 — Phased Evolution (15 seconds)

Show: phase transition logic (Sustenance → Ecosystem → Humanity).

"As the treasury grows, the protocol evolves. Below $50k: pure sustenance. $50k to $1M: auto-provide liquidity to top RTP-adopting tokens. Above $1M: USDC grants to Solana public-goods projects. Each transition is irreversible, on-chain."

### 2:30 — What Makes This Different (15 seconds)

- "Working yield brain — not vaporware"
- "Constitutional governance — soulcontract enforced on every message"
- "Self-funding — no VC dependency, no token sale"
- "Open-source architecture, protected strategies"

### 2:45 — Q&A Buffer

## Backup Slides

1. Fast sim calibration — how we ensure our 30K-combo grid search matches reality
2. Self-correction architecture — three independent modules, no LLM needed
3. Yield brain results table — all 4 symbols validated profitable
4. Architecture diagram — full three-layer stack
