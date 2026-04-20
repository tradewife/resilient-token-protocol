# Agentic Engineering Grant Application — Resilient Token Protocol (RTP)

**Grant Amount:** 200 USDG
**Submitted by:** @trade_wife | x.com/trade_wife | github.com/Tradewife

---

## Step 1: Basics

**Project Title:** Resilient Token Protocol (RTP)

**One Line Description:** Solana-native, self-funding treasury governed by a modular Rust swarm — any token project adopts RTP, their trading fees route to the swarm, which autonomously researches, validates, and executes yield strategies on Hyperliquid perps, returning yield back to the project and its holders.

**TG username:** t.me/trade_wife

**Wallet Address:** [WILL PROVIDE BEFORE SUBMISSION]

---

## Step 2: Details

### Project Details

**Problem:** Token projects on Solana have no sustainable funding mechanism. Teams raise, launch, and slowly bleed treasury. There is no autonomous yield layer that can turn idle fee revenue into compounding returns — every project has to manually manage treasury, or just hold static SOL/USDC.

**Solution:** Resilient Token Protocol (RTP) is a Solana-native, self-funding treasury governed by a modular Rust swarm. Any token project integrates RTP via a single SDK call — their trading fees (SOL creator fees from platforms like Pump.fun, Bags.fm, Raydium) route to a per-mint PDA-owned treasury. TransferFeeConfig fee percentages are immutable once set on the mint. A 6-wing swarm (trading, security, evolve, knowledge, audit, futureproof) autonomously researches, validates, and executes yield strategies. The Trading Wing executes USDC-margined perpetuals trades on Hyperliquid, signed via Phantom Connect (agentic wallet). Yield flows back to the treasury PDA on Solana, redistributed via a deterministic 70/20/10 split (project/community/humanity fund). The system is designed to be self-funding from day one — funded by its own yield, forever.

**Architecture:** Three layers — on-chain Solana treasury (Anchor, PDA-gated), Rust swarm runtime (6 wings, coordinator, soulguard), and Python research layer (Night Shift: 30K configs/night, 9-fold walk-forward validation, Darwinian selection). 167 commits, 306 Rust tests passing, full BUY→fill→SELL→fill→PnL round-trip verified on Hyperliquid testnet with yield deposits confirmed on Solana devnet. Multi-platform launcher supports Metaplex, Pump.fun, Bags.fm, and RTP Direct.

**What the grant funds:** Final mainnet hardening — security audit remediation, mainnet deployment of treasury program, Phantom Connect production integration, and hackathon submission (SWARMs/Canteen x Colosseum, deadline May 11, 2026).

### Deadline

May 11, 2026 (Australia/Sydney)

### Proof of Work

- **Live site**: www.resilientprotocol.xyz (dashboard with wallet connect, token launch, SDK docs)
- **GitHub**: github.com/Tradewife/resilient-token-protocol (167 commits, 306 passing Rust tests)
- **On-chain**: Treasury program deployed to Solana devnet (program ID: `8rt6yiBn...`), PDA-gated fee collection, redistribution, and phase evolution all verified
- **Hyperliquid integration**: Full EIP-712 signed order flow from Rust, BUY→SELL→PnL round-trip confirmed on HL testnet, yield deposited to treasury PDA
- **Night Shift pipeline**: 30K strategy configs/night, 9-fold walk-forward validation, Darwinian survivor selection — top live candidate: SOL/USDT Survivor 2.69 with +118.3% optimized PnL
- **Paper trading**: Live Binance paper trader with ADX filter and per-symbol configs running continuously
- **Multi-platform launcher**: Metaplex, Pump.fun, Bags.fm, RTP Direct — token launch flow operational on devnet
- **Devnet loop daemon**: 6h cron, LLM-driven strategy evolution, auditable trail committed to repo
- **AI-assisted development**: Entire project built agentic from day one — **30 Droid (Factory) sessions** (20MB of transcripts) plus Codex session transcript, all using solana.new. Every major feature (Rust swarm, Anchor program, Hyperliquid integration, dashboard, SDK) was developed with AI pair programming via Droid.
- **Security audit completed**: 18 findings documented, remediation in progress

### Personal X Profile

x.com/trade_wife

### Personal GitHub Profile

github.com/Tradewife

### Colosseum Crowdedness Score

[TO BE ADDED — screenshot uploaded to Google Drive]

### AI Session Transcript

- **30 Droid (Factory) session transcripts** in `droid-sessions/` directory (20MB) — primary AI development environment
- **Codex session transcript**: `codex-session.jsonl` (908KB)
- Both demonstrate extensive agentic engineering on solana.new
- All files uploaded to Google Drive: [DRIVE LINK TO BE ADDED]

---

## Step 3: Milestones

### Goals and Milestones

| # | Milestone | Target Date | Deliverable |
|---|-----------|------------|-------------|
| 1 | Security audit remediation | Apr 25, 2026 | All 18 security findings resolved, clippy clean, `cargo test --lib` green |
| 2 | Mainnet treasury deployment | May 2, 2026 | Anchor program deployed to mainnet, Phantom Connect production wallet wired, SDK published |
| 3 | Hyperliquid mainnet execution | May 6, 2026 | First real USDC-margined trade executed from Rust swarm, yield confirmed on mainnet treasury PDA |
| 4 | Hackathon submission | May 11, 2026 | 3-minute demo video, Colosseum project page, all sponsor integrations (Phantom, CASH, Squads) documented |

### Primary KPI

First real mainnet yield deposit (USDC) flowing from Hyperliquid trade execution back to the Solana treasury PDA — a single self-funding cycle completed end-to-end.

### Final Tranche

To receive the final tranche: Colosseum project link, GitHub repo, and AI subscription receipt will be submitted.

---

## Files Included for Review

| File | Size | Description |
|------|------|-------------|
| `AGENTIC-ENGINEERING-GRANT-APPLICATION.md` | — | This file — complete grant application |
| `droid-sessions/*.jsonl` | 20MB | 30 Droid (Factory) AI session transcripts |
| `codex-session.jsonl` | 908KB | Codex session transcript |

---

**Submit at:** https://superteam.fun/earn/grants/agentic-engineering
