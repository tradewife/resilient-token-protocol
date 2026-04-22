# Multi-Token Scaling Architecture

Multi-token scaling: each adopting token mint has an AdopterRecord PDA tracking
cumulative fee contributions. Yield attribution is proportional:
`share_i = fees_i / Σfees × yield_pool`

Each token also gets its own Phantom agent wallet via `derivationIndex`, providing
isolated Solana/EVM addresses and a separate Hyperliquid perps account. The
`TradingState.token_wallet_map` (in `trading/types.rs`) maps token mints to
derivation indices. The swarm copy-trades the same validated strategy across all
tokens with isolated capital and isolated wallets.

Holder-level distribution for each adopter uses an SPL token balance snapshot
at redistribution time. Phase 1 uses a single adopter; Phase 2 adds factory
pattern for full multi-tenant isolation.

## On-Chain Accounts

```
AdopterRecord PDA (per token mint)
seeds: ["adopter", token_mint]
├── token_mint: Pubkey
├── fees_contributed_lamports: u64    ← incremented on every fee deposit
├── adopted_at: i64
├── last_deposit_ts: i64
├── deposit_count: u64
└── bump: u8

Treasury (shared)
└── total_fees_received_lamports: u64 ← sum of all adopter contributions
```

## Attribution Formula

```
adopter_yield_share = (fees_contributed / total_fees_received) × yield_pool

TokenA contributed 600 SOL → receives 60% of yield pool
TokenB contributed 400 SOL → receives 40% of yield pool
```

See `scripts/compute_adopter_yield_share.ts` for the pure TypeScript implementation.

## Instructions

| Instruction | Purpose |
|------------|---------|
| `register_adopter` | Create AdopterRecord PDA for a new token project (once per mint) |
| `record_fee_deposit` | Increment per-adopter and treasury fee totals (accounting hook) |

## Phase Roadmap

- **Phase 1 (current):** Single adopter, single treasury, full redistribution cycle proven on devnet.
- **Phase 2:** Factory pattern — `initialize_vault` per adopter, per-adopter yield isolation.
