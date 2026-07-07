# RTP — Resilient Token Protocol

**Token projects route trading fees to RTP → RTP generates yield via on-chain perps → yield flows back to holders.**

A Solana-native, self-funding treasury protocol. Any token project adopts RTP — their trading fees route to a program-owned treasury that autonomously generates yield via Flash Trade on-chain perpetuals (CPI via `invoke_signed`, mainnet-proven) and redistributes it back to the project and holders (70/20/10 on-chain split). Funded by its own yield, forever. No RTP token. RTP is infrastructure.

> Even after a token dies, RTP keeps paying its holders. The treasury compounds in perpetuity.

```
                    ┌─────────────────────────────┐
                    │     RTP SWARM COORDINATOR    │
                    │   (soulcontract governance)   │
                    └──────────┬──────────────────┘
                               │
          ┌────────────┬───────┼───────┬───────────┬────────────┐
          │            │       │       │           │            │
     ┌────▼────┐ ┌────▼───┐ ┌▼─────┐ ┌▼────────┐ ┌▼────────┐ ┌▼────────┐
     │TRADING  │ │SECURITY│ │EVOLVE│ │KNOWLEDGE │ │AUDIT    │ │FUTURE   │
     │WING     │ │WING    │ │WING  │ │WING      │ │WING     │ │PROOF    │
     │         │ │        │       │ │          │ │         │ │WING     │
     │Yield    │ │Threat  │ │Self- │ │Persist.  │ │3-agent  │ │Deprec.  │
     │gen +    │ │detect  │ │modify│ │knowledge │ │tribunal │ │monitor  │
     │exec     │ │defend  │ │adapt │ │store     │ │consensus│ │+heartbeat│
     └────┬────┘ └────┬───┘ └──┬───┘ └────┬─────┘ └────┬────┘ └────┬────┘
          │           │        │          │            │           │
          └───────────┴────────┴────────┴────────────┴───────────┘
                               │
                    ┌──────────▼──────────────────┐
                    │     SOLANA TREASURY PDA      │
                    │  fees → yield → redistribute │
                    │  self-hydrate → run forever  │
                    └─────────────────────────────┘
```

## Mainnet Proof

The Treasury PDA opens and closes Flash Trade positions on Solana mainnet via `invoke_signed` — no human keypair involved.

| Proof | Explorer Link |
|-------|---------------|
| **Open position** (CPI via invoke_signed) | [View on Solana Explorer](https://explorer.solana.com/tx/2bLg1FuJ6iqwYq6SKi5EcZQWszarDZhS68bCbGTRLKMwhYqsU7G57fTtG4G6GFx3ZKN15qhb85zy28pGJvSdrnG3) |
| **Close position** (SOL returned to treasury) | [View on Solana Explorer](https://explorer.solana.com/tx/dFqkoP2wX2meR8Mv8CngujJJUNBYuv5peCyzRYFPBvpN3uqCqXqRCy4TPyw5JbAZhumCaJdGaJoQvJrJGJzxfHF) |
| **Program** (Treasury on devnet) | [View on Solana Explorer](https://explorer.solana.com/address/8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB?cluster=devnet) |

Every position open/close is an on-chain transaction. The Treasury PDA signs via `invoke_signed` — the program IS the only authority. No private key exists for trading.

## Live Autonomous Trading

The Survivor 2.69 strategy runs autonomously 24/7 on Railway. A Rust binary (`rtp-trader`) polls Flash Trade every 5 minutes, computes the multi-timeframe signal (ATR/RSI/SMA/Bollinger Band), and executes positions when conditions are met — no human in the loop.

| Component | Detail |
|-----------|--------|
| Strategy | SOL/USDT Survivor 2.69 (9x Calmar-optimized) — Calmar 44.89, 100% consistency |
| Execution | Flash SDK v2 (`@flash_trade/flash-sdk-v2@1.0.36`) via Node child process · REST `/transaction-builder/*` fallback retained |
| Position sizing | 20% of capital per trade, 9x leverage |
| Stops | TP: 6.0× ATR, SL: 2.5× ATR, Trailing: 1.0× ATR |
| Signal | threshold=0.3 with 3+ aligned timeframes, max hold 96h |

**Confirmed mainnet transactions:**

| Proof | Explorer Link |
|-------|---------------|
| **Open position** (SOL LONG) | [TX `YtGKq46w...`](https://explorer.solana.com/tx/YtGKq46wEgeUqoWouV5LXvv6mAxb5dCYmRHy622i7UtP5UoXsKZJtqscJf9fWLjzjZwCZhGw7r4EMgKV3wU2CBg) |
| **Close position** (SOL returned) | [TX `56PLUQA...`](https://explorer.solana.com/tx/56PLUQAPGqtAcvRUgJBreMrubAETZkpFCoyHzkwt3jCGCwZYHeonbxcJp244ZipeHuNBAwAX6r1wWkcR9LFcdmM6) |

## Why This Is Different

- **Constitutional governance** — soulcontract enforced in Rust (soulguard.rs) AND on-chain (Anchor `require!`). No other project has both.
- **Self-funding economics** — treasury generates its own yield via Flash Trade on-chain perps (CPI via `invoke_signed`), with irreversible phase evolution (Sustenance → Ecosystem → Humanity). No VC dependency.
- **Proven research engine** — 30K configs/night, 9-fold walk-forward validation, fee-aware simulation. Out-of-sample results across 9 independent time windows.
- **Flash Trade on-chain execution** — PDA-signed CPI into Flash Trade Perpetuals. No human keypair. No cross-chain bridge. Fully auditable on Explorer. Mainnet-proven (TX `2bLg1Fu...`, 99,214 CU).

## Research Results

| Symbol | Production PnL | Optimized PnL | Consistency | Trades |
|--------|---------------|--------------|-------------|--------|
| SOL/USDT | +36.9% | **+554%** (9x) | 100% | 429 |
| BNB/USDT | +49.6% | — | 67% | 178 |
| ETH/USDT | +48.1% | — | 78% | 155 |
| BTC/USDT | +17.5% | — | 67% | 153 |

30,000 parameter combinations tested per symbol per night. 9-fold expanding-window walk-forward validation. Full-sim ground truth with 0.1% fees and 10bps slippage.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    ON-CHAIN (Solana / Anchor)                    │
│  Treasury PDA: fees → yield → redistribute (70/20/10)          │
│  19 instructions · PDA-owned · CPI-only transfers               │
├─────────────────────────────────────────────────────────────────┤
│                    SWARM RUNTIME (Rust · 362 tests)              │
│  Coordinator → message bus → 6 wings                            │
│  Trading → Flash Trade CPI → on-chain perps → SOL               │
│  Security · Evolve · Knowledge · Audit · Futureproof             │
├─────────────────────────────────────────────────────────────────┤
│                    RESEARCH LAYER (Python)                        │
│  Night Shift: 30K configs → WFA → Darwinian                     │
│  Validated: SOL/USDT Calmar 44.89, +554% at 9x                  │
└─────────────────────────────────────────────────────────────────┘
  Signing: Treasury PDA (invoke_signed, no private key)
  Capital: SOL → Treasury PDA → Flash Trade CPI → SOL yield → PDA
```

### Capital Flow

```
SOL fees → Treasury PDA → invoke_signed → Flash Trade CPI (on-chain perps) → SOL returned → Treasury PDA → redistribute
```

Single asset throughout (SOL). Single chain (Solana). No cross-chain bridge. PDA-signed execution.

### Per-Token Isolation

Every token that adopts RTP gets its own isolated treasury — a separate PDA and vault. No shared pool. No honeypot. One token's trading loss cannot affect another's reserves.

```
Token A → Treasury PDA_A → Flash Trade CPI_A → Yield_A → 70/20/10 split_A
Token B → Treasury PDA_B → Flash Trade CPI_B → Yield_B → 70/20/10 split_B
```

The swarm copy-trades the same validated strategy across all tokens with isolated capital.

## Treasury Capital Model

| Step | Asset | Location | Mechanism |
|------|-------|----------|-----------|
| 1. Fees arrive | SOL | Treasury PDA (Solana) | Platform creator fees → treasury PDA |
| 2. Execute strategies | SOL | Flash Trade (on-chain CPI) | Treasury PDA invoke_signed → perps position |
| 3. Yield returns | SOL | Treasury PDA (Solana) | Close position → SOL returned via CPI |
| 4. Redistribute | SOL | Treasury PDA | 70% holders / 20% dev / 10% ecosystem (on-chain) |

## Fee Routing

| Platform | Routing Method | Flexibility |
|----------|---------------|-------------|
| Pump.fun | One-time fee redirect to treasury PDA | Once only |
| Bags.fm | Multi-claimer fee sharing with treasury PDA | Updateable anytime |
| Raydium | Creator fees → pool_creator wallet → forward to treasury PDA | Manual |

## SDK — One Function Call

```bash
npm install @resilient-protocol/sdk @solana/web3.js @coral-xyz/anchor
```

```typescript
import { registerWithRTP } from "@resilient-protocol/sdk";

const result = await registerWithRTP(connection, wallet, {
  authority: publicKey,
});
// result.treasuryPDA, result.adopterPDA, result.signature
```

Core functions: `registerWithRTP()`, `fetchTreasuryState()`, `depositSol()`, `checkRedistribute()`, `registerAdopterBeta()`, `fetchAdopterState()`. Works with both Keypair and WalletAdapter.

## Operator CLI

```bash
npx tsx cli/bin/rtp.ts init          # Interactive setup
npx tsx cli/bin/rtp.ts demo           # Full 8-step demo
npx tsx cli/bin/rtp.ts accounts derive --mint <PUBKEY>  # Derive PDAs
npx tsx cli/bin/rtp.ts crank fees --mint <PUBKEY>       # Sweep fees
npx tsx cli/bin/rtp.ts freeze --mint <PUBKEY> --yes     # Emergency freeze
npx tsx cli/bin/rtp.ts status --all   # Protocol health
```

14 commands across 7 groups. All support `--json`, `--quiet`, `--cluster <devnet|mainnet>`.

## What's Built

| Component | Status |
|-----------|--------|
| Anchor treasury program (19 instructions, redistribution, Flash Trade CPI) | Built — devnet deployed, audit remediated |
| Rust swarm runtime (6 wings, Coordinator, soulguard) | 362 tests passing |
| Flash Trade CPI execution (mainnet-proven invoke_signed) | M0–M5 complete |
| Live autonomous trader (rtp-trader, 24/7 on Railway) | Running — Survivor 2.69 at 9x |
| Research engine (Night Shift, 30K configs/night, 9-fold WFA) | Shipping |
| Operator CLI (14 commands, Commander.js) | Built |
| SDK (`@resilient-protocol/sdk`) | Shipped |
| Dashboard (resilientprotocol.xyz, wallet connect) | Live |

## Live on Devnet

| Item | Value |
|------|-------|
| Program ID | `8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB` |
| Treasury PDA (demo) | `6PYPAnwiMoZvzphAWEu3EsNz3PpwjJ6YcZabj34qVQ4Z` |
| Explorer | [View demo treasury](https://explorer.solana.com/address/6PYPAnwiMoZvzphAWEu3EsNz3PpwjJ6YcZabj34qVQ4Z?cluster=devnet) |
| Init tx | [View transaction](https://explorer.solana.com/tx/4RVehmPVpnFYHrsF6N64RjVh7mszRzKF9DQVHd8TUqBHwrnyDYavf3TnDYJC4b5PrJWVSubZkNuyVkF1oJzk71RT?cluster=devnet) |

## Quick Start

```bash
# Python research
python -m venv .venv && source .venv/bin/activate
pip install pandas numpy ccxt pyarrow redis
python -m research.orchestration.night_shift --skip-fetch

# Rust swarm
cd rtp/swarm && cargo build --release
cargo run --bin rtp-demo      # 8-step demo
cargo run --bin rtp-trader    # Live autonomous trader

# Operator CLI
npx tsx cli/bin/rtp.ts demo
npx tsx cli/bin/rtp.ts init

# Solana program
cd rtp/programs/rtp-treasury && anchor build
anchor test --provider.cluster devnet
```


## License

Business Source License 1.1 (BSL 1.1). Source code is publicly visible for evaluation, testing, academic research, and hackathon participation. Production deployment or commercial use requires a separate commercial license. On 2030-05-11, converts to Apache License, Version 2.0. See [LICENSE](./LICENSE) for full terms.

Copyright (c) 2024-2026 Resilient Token Protocol Contributors. All rights reserved.
