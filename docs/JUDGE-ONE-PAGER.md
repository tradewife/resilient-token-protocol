# RTP — Judge One-Pager

**Resilient Token Protocol**: A Solana-native, self-funding treasury governed by a modular Rust swarm.

## Five Demo Points

| # | Point | Proof | Location |
|---|-------|-------|----------|
| 1 | **On-chain constraint rejection** | Anchor program rejects `evolvePhase` with `BelowThreshold` when vault < $50k. 10+ rejection tests in Anchor test suite. | `rtp/programs/rtp-treasury/tests/treasury.ts` lines 515, 777 |
| 2 | **Autonomous operation** | 6h cron GitHub Action runs `rtp-daemon`, commits cycle output. Multiple completed cycles with LLM-driven mutations. | `.github/workflows/devnet-loop.yml`, `data/devnet-cycles/` |
| 3 | **Persistent memory** | Memory promotion ladder (working → project → overview). Files persist in `data/swarm-memory/` across CI cycles. | `rtp/swarm/src/memory_promotion.rs`, `data/swarm-memory/` |
| 4 | **Visible adaptation** | Strategy params mutate between cycles (e.g. `signal_threshold: 0.28 → 0.25`). LLM generates rationale. Soulcontract bounds enforced. | `data/devnet-cycles/latest/cycle.json` |
| 5 | **Observable treasury state** | Live SOL balance from devnet RPC (10s refresh). Explorer link to Treasury PDA. Redistribution TX on-chain. | Dashboard: `dashboard/`, Explorer link below |

## Live on Devnet

| Item | Value |
|------|-------|
| Program ID | `8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB` |
| Treasury PDA | Per-mint — demo: `FNQbK1Vw77aT7qM1EMSmeEPDGizSNhX4rkkYBKQNFotF` |
| Explorer | [View on Solana Explorer](https://explorer.solana.com/address/FNQbK1Vw77aT7qM1EMSmeEPDGizSNhX4rkkYBKQNFotF?cluster=devnet) |
| Redistribution TX | [View transaction](https://explorer.solana.com/tx/9HzWgBfwYxs5ModdjF5mT6gdTfayQq8mMYipopyHfGPmYqk6KESHFqgDrc9Mcie573ttcdPqMHSyJP5nNBKK3bR?cluster=devnet) |
| Dashboard | https://resilientprotocol.xyz |
| SDK | `@resilient-protocol/sdk` — one function call to register any token |

## Quick Demo

```bash
./demo.sh    # Three layers: Python WFA → Rust swarm → on-chain treasury
```

## Key Metrics

- **307** Rust tests passing
- **6** swarm wings (Trading, Security, Evolve, Knowledge, Audit, Futureproof)
- **30K** strategy configs evaluated per symbol per night
- **9-fold** walk-forward validation
- **8/8** on-chain steps completed including live redistribution

## Sponsor Integrations

| Sponsor | Integration |
|---------|------------|
| Phantom Connect | Browser wallet connection (dashboard), Solana CPI signing path |
| Hyperliquid | Perps execution via EIP-712 signed orders from Rust |
| Solana | Treasury PDA, Anchor program, Token-2022 TransferFeeConfig |

## What Makes This Different

No prior Colosseum hackathon project combines autonomous research + multi-wing constitutional governance + on-chain constraint enforcement + self-funding economics in a single deployed system. The Anchor program has 10+ constraint rejection tests. The devnet loop has accumulated multiple autonomous cycles with real LLM-driven adaptation. The unified SOL cycle (SOL in → USDC on HL → SOL back to treasury) is auditable end-to-end: one asset on-chain, trustless conversion via Phantom bridge, USDC-margined positions only.

## Capital Flow

```
SOL fees → Phantom bridge → USDC → Hyperliquid perps → USDC yield → Phantom bridge → SOL → Treasury PDA → redistribute
```

Treasury PDA holds SOL reserves. HL clearinghouse holds USDC working capital. Judges can verify the full SOL balance on Solana Explorer.
