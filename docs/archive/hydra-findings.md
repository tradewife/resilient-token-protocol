# Hydra Deep Dive Findings

**Repository:** [github.com/magicblock-labs/hydra](https://github.com/magicblock-labs/hydra)  
**Program ID:** `Hydra17i1feui9deaxu6d1TzSQMRNHeBRkDR1Awy7zea`  
**License:** MIT  
**Latest version:** v0.1.1 (Apr 25, 2026)  
**Language:** Rust 100%  

---

## What Hydra Is

Hydra is a **permissionless Solana crank** for scheduling instructions with minimum overhead. It stores a scheduled instruction in a crank PDA and lets anyone trigger it when the schedule is due.

---

## 1. CreateArgs Format

### Scheduling `check_redistribute`

```rust
use hydra_api::instruction::{self as ix, CreateArgs};

let seed = [0x42u8; 32]; // unique seed for this crank
let (crank, _bump) = ix::find_crank_pda(&seed);

// RTP treasury program: 8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB
// check_redistribute instruction discriminator + accounts

let create = ix::create(
    payer_pubkey,
    crank,
    &CreateArgs {
        seed,
        authority: [0u8; 32], // all-zeros = unkillable (no cancel authority)
        start_slot: 0,         // start immediately
        interval_slots: 400,   // ~every 400 slots (~2-3 min on mainnet)
        remaining: 0,          // 0 = infinite runs
        priority_tip: 2_500,   // lamports paid to cranker per trigger
        cu_limit: 0,           // inherit default (200k)
        scheduled_program_id: rtp_program_id,
        scheduled_metas: &[
            SchedMeta::writable(treasury_pda),      // treasury account
            SchedMeta::readonly(token_program),      // Token-2022 program
            SchedMeta::readonly(mint_pubkey),         // mint account
        ],
        scheduled_data: &check_redistribute_data,    // instruction discriminator + args
    },
);
```

### Scheduling `withdraw_fees`

Same structure, different instruction data and accounts:

```rust
let withdraw_create = ix::create(
    payer_pubkey,
    crank,
    &CreateArgs {
        seed: [0x43u8; 32], // different seed for this crank
        authority: [0u8; 32],
        start_slot: 0,
        interval_slots: 800, // ~every 5 min
        remaining: 0,
        priority_tip: 2_500,
        cu_limit: 0,
        scheduled_program_id: rtp_program_id,
        scheduled_metas: &[
            SchedMeta::writable(treasury_pda),       // treasury account
            SchedMeta::writable(vault_ata),           // vault ATA (writable)
            SchedMeta::readonly(mint_pubkey),          // mint account
            SchedMeta::readonly(token_program),        // Token-2022 program
        ],
        scheduled_data: &withdraw_fees_data,
    },
);
```

### CreateArgs Fields

| Field | Type | Description |
|-------|------|-------------|
| `seed` | `[u8; 32]` | Unique identifier for PDA derivation |
| `authority` | `[u8; 32]` | All-zeros = no cancel authority (permanent crank) |
| `start_slot` | `u64` | Slot to start (0 = immediately) |
| `interval_slots` | `u64` | Recurrence interval in slots (~400ms per slot on mainnet) |
| `remaining` | `u64` | 0 = infinite, else exact number of runs |
| `priority_tip` | `u64` | Lamports paid to cranker per trigger |
| `cu_limit` | `u32` | Compute units (0 = default 200k, max 1.4M) |
| `scheduled_program_id` | `Pubkey` | Target program ID |
| `scheduled_metas` | `&[SchedMeta]` | Account metas (no signer metas allowed) |
| `scheduled_data` | `&[u8]` | Instruction data bytes (max 1024 bytes) |

---

## 2. Cranker Architecture

The cranker is **long-running**, not one-shot. It is an event-driven daemon:

- Uses **WebSocket subscriptions** for account and slot updates
- Optional **Yellowstone gRPC** for redundancy and lower latency
- Supports **Prometheus metrics** at a configurable port
- Runs continuously, triggering eligible cranks as slots advance

```bash
hydra-cranker --keypair ~/.config/solana/cranker.json \
  --rpc-url https://api.mainnet-beta.solana.com \
  --ws-url wss://api.mainnet-beta.solana.com \
  --prometheus-port 9100
```

---

## 3. Programmatic Crank Creation from Rust

**Yes — fully supported.** The `hydra-api` crate (published on crates.io, version 0.1.0) provides:

| Feature | Purpose |
|---------|---------|
| `client` | `Instruction` builders for off-chain / client use |
| `cpi-native` | CPI helpers for Anchor programs |
| `cpi-pinocchio` | CPI helpers for Pinocchio programs |

```toml
# Add to Cargo.toml
[dependencies]
hydra-api = { version = "0.1.0", features = ["client"] }
```

The `create()` function returns a `solana_instruction::Instruction` that can be submitted as a normal Solana transaction — no special integration needed.

---

## 4. Failure Handling — Atomic Rollback

The trigger transaction contains two instructions:

```
ix[k]   = Hydra.Trigger
ix[k+1] = scheduled instruction
```

If the scheduled instruction fails, the **entire transaction rolls back** (Solana atomicity). This means:

- Hydra's payout does NOT advance
- Crank state does NOT change
- The crank **remains eligible** for the next trigger attempt
- The cranker will retry on the next eligible slot

**Design insight:** Hydra verifies `ix[k+1]` against the bytes stored in the crank account via `memcmp` (~60 CU), then lets the runtime execute it as a sibling instruction. **No CPI overhead.**

This is ideal for RTP: if `check_redistribute` hits a no-op condition (nothing to redistribute), the transaction fails atomically and the crank simply retries next cycle. No partial state corruption is possible.

---

## 5. Costs

| Item | Amount | Details |
|------|--------|---------|
| Rent deposit | ~0.002 SOL | Refunded on close |
| Create tx fee | 5,000 lamports | Standard Solana base fee |
| Per trigger | 10,000 + `priority_tip` lamports | Deducted from crank balance, paid to cranker |

**Funding:** Transfer SOL to the crank account via `system_program::transfer`, sized to:

```
fund_amount = expected_runs × (10,000 + priority_tip) lamports
```

For an infinite crank (`remaining: 0`), fund for ~30 days and top up periodically.

---

## 6. Limits

| Constraint | Value |
|------------|-------|
| `Trigger` scope | Top-level only (no CPI) |
| Signer metas | **Not allowed** in scheduled instructions |
| `MAX_ACCOUNTS` | 32 |
| `MAX_DATA_LEN` | 1024 bytes |
| Reward per trigger | Fixed: 10,000 lamports + `priority_tip` |

**RTP implication:** Only **permissionless** instructions (no signer required) can be Hydra-scheduled. This aligns perfectly with RTP's trust model — `withdraw_fees`, `check_redistribute`, `record_fee_deposit`, and `update_strategy_performance` are all permissionless.

---

## 7. Compute Units

| Instruction | CU |
|-------------|-----|
| Create | 3,292 |
| Trigger | 464 |
| Trigger (reject) | 378 |
| Cancel | 128 |
| Close | 139 |

Trigger overhead is negligible (~464 CU). Combined with RTP's `check_redistribute` (~15k CU estimated), a single trigger fits well within the default 200k CU budget.

---

## 8. RTP Integration Plan

### Eligible for Hydra Scheduling (permissionless, no signer)

| Instruction | Interval | Purpose |
|-------------|----------|---------|
| `check_redistribute` | ~1,600 slots (~10 min) | Triggers the 70/20/10 split deterministically. Anyone can call; on-chain enforces the split logic. |
| `withdraw_fees` | ~800 slots (~5 min) | Pulls TransferFeeConfig fees into the treasury vault. Permissionless — no authority check. |
| `record_fee_deposit` | ~800 slots (~5 min) | Records fee accounting counters (no fund movement). Optional — depends on accounting cadence. |
| `update_strategy_performance` | ~4,000 slots (~25 min) | Writes strategy metrics. Enforcement is on-chain via `hydrate_swarm` gate. |

### NOT Eligible for Hydra (authority-gated, requires signer)

| Instruction | Reason |
|-------------|--------|
| `initialize` | Requires `treasury.authority` |
| `evolve_phase` | Requires `treasury.authority`, irreversible |
| `register_strategy` | Requires `treasury.authority` |
| `force_retire_strategy` | Requires `treasury.authority` |
| `end_beta` | Requires `treasury.authority` |
| `create_swarm_vault` | Authority is treasury PDA (via CPI) |
| `hydrate_swarm` | Gated by strategy Live status + beta check + runway invariant |

### Recommended Crank Configuration

```rust
// Crank 1: withdraw_fees — frequent, lightweight
let withdraw_fees_crank = CreateArgs {
    seed: hash("rtp-withdraw-fees-v1"),
    authority: [0u8; 32],      // permanent
    start_slot: 0,
    interval_slots: 800,       // ~5 min
    remaining: 0,              // infinite
    priority_tip: 2_500,       // 0.0000025 SOL per trigger
    cu_limit: 0,               // default 200k
    scheduled_program_id: rtp_program_id,
    scheduled_metas: &[/* treasury, vault_ata, mint, token_program */],
    scheduled_data: &withdraw_fees_discriminator,
};

// Crank 2: check_redistribute — slightly less frequent
let redistribute_crank = CreateArgs {
    seed: hash("rtp-redistribute-v1"),
    authority: [0u8; 32],      // permanent
    start_slot: 0,
    interval_slots: 1_600,     // ~10 min
    remaining: 0,              // infinite
    priority_tip: 2_500,
    cu_limit: 0,
    scheduled_program_id: rtp_program_id,
    scheduled_metas: &[/* treasury, token_program, mint */],
    scheduled_data: &check_redistribute_discriminator,
};
```

### Cost Estimation (per crank, 30 days)

```
Slots per day:    ~216,000 (400ms avg)
Triggers/day:     ~135 (withdraw_fees at 800 slots)
                   ~135 (check_redistribute at 1,600 slots)

Cost per trigger:  10,000 + 2,500 = 12,500 lamports
Daily cost/crank:  135 × 12,500 = 1,687,500 lamports ≈ 0.00169 SOL
30-day cost/crank: ~0.05 SOL

Total (2 cranks):  ~0.10 SOL/month
```

### Deployment Steps

1. **Build the crank creation transactions** using `hydra-api` client feature
2. **Fund each crank** with ~0.05 SOL for 30 days of operation
3. **Deploy a cranker** (or rely on existing Hydra cranker fleet on mainnet)
4. **Monitor** crank balance and top up monthly
5. **Optional:** Run own cranker with Prometheus metrics for operational visibility

---

## Summary

Hydra is an excellent fit for RTP's permissionless on-chain operations. It provides:

- **Deterministic scheduling** without requiring a trusted operator
- **Atomic failure handling** — no partial state on revert
- **Minimal CU overhead** (~464 CU per trigger)
- **Low cost** (~0.10 SOL/month for two cranks)
- **Rust-native integration** via `hydra-api` crate
- **Permanent cranks** via all-zeros authority (no one can cancel)

The two permissionless hot-path operations — `withdraw_fees` and `check_redistribute` — are the primary candidates. Authority-gated operations (`hydrate_swarm`, `evolve_phase`) remain under swarm runtime control via the Coordinator.
