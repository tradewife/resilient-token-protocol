# Squads Multisig Deep Dive — Production Security Hardening for RTP

> Research deliverable for integrating Squads v4 multisig as `treasury.authority` in the RTP on-chain program. All findings verified against Squads v4 on-chain program, REST API docs, and SDK references.

**Date:** April 2026
**Scope:** Can Squads serve as the authority gate for RTP's authority-gated instructions (`evolve_phase`, `register_strategy`, `force_retire_strategy`, `end_beta`)?

**Short answer: Yes.** Full details below.

---

## Table of Contents

1. [Squads PDA as Treasury Authority](#1-squads-pda-as-treasury-authority)
2. [Programmatic API (REST / Grid)](#2-programmatic-api-rest--grid)
3. [Spending Limits](#3-spending-limits)
4. [Rust SDK](#4-rust-sdk)
5. [Time Lock Mechanism](#5-time-lock-mechanism)
6. [Audits & Security](#6-audits--security)
7. [RTP Integration Plan](#7-rtp-integration-plan)

---

## 1. Squads PDA as Treasury Authority

**Can a Squads PDA be used as `treasury.authority` in an Anchor program?** **Yes.**

Squads v4 creates a multisig PDA with seeds:

```
["multisig", "multisig", createKey]
```

The multisig PDA can be set as the `authority` on any Solana account, including an Anchor program's `treasury.authority` field. Squads executes transactions as top-level instructions from the multisig vault PDA, so any instruction that checks `treasury.authority` will succeed when the Squads vault PDA is the signer.

### Vault PDA Derivation

```
seeds: ["multisig", multisig_key, "vault", vault_index]
program ID: SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf
```

Same program ID on mainnet and devnet.

### What This Means for RTP

- `treasury.authority = <squads_multisig_pda>` (the multisig account address)
- Any `evolve_phase`, `register_strategy`, `force_retire_strategy`, or `end_beta` call goes through Squads proposal → vote → execute flow
- The Squads vault PDA signs via `invoke_signed` with the vault seeds — standard Solana CPI signing

### Authority Flow

```
Agent/Human → Squads Proposal → Vote (2-of-3) → Time Lock (24h) → Execute
     │                                                           │
     │                              ┌─────────────────────────────┘
     │                              ▼
     │                    Vault PDA signs via invoke_signed
     │                              │
     │                              ▼
     └──────────────────► RTP Treasury Program
                          checks treasury.authority == vault PDA ✓
```

---

## 2. Programmatic API (REST / Grid)

Squads provides a REST API called **Grid** for programmatic multisig management.

### Base Configuration

| Field | Value |
|-------|-------|
| Base URL | `https://developer-api.squads.so/api/v1` |
| Auth | `Authorization: Bearer <API_KEY>` |
| Network Header | `x-squads-network: mainnet` or `devnet` |
| Idempotency | `x-idempotency-key` header for safe retries |

### Create Smart Account

```
POST /smart-accounts

{
  "smart_account_signers": [
    {
      "address": "<pubkey>",
      "permissions": ["CAN_INITIATE", "CAN_VOTE", "CAN_EXECUTE"]
    },
    {
      "address": "<pubkey>",
      "permissions": ["CAN_VOTE"]
    }
  ],
  "threshold": 2,
  "admin_address": "<optional>"
}
```

### Get Smart Account

```
GET /smart-accounts/{address}
```

### Patch Smart Account

Update time lock, members, or other account-level settings:

```
PATCH /smart-accounts/{address}

{
  "time_lock": 86400,
  "transaction_signers": ["<signer1>", "<signer2>"]
}
```

### Permissions

| Permission | Description |
|-----------|-------------|
| `CAN_INITIATE` | Create new proposals |
| `CAN_VOTE` | Vote on active proposals |
| `CAN_EXECUTE` | Execute approved proposals after time lock |

**Constraints:**
- At least one signer must have `CAN_INITIATE`
- Threshold cannot exceed number of `CAN_VOTE` signers

---

## 3. Spending Limits

Spending limits enforce per-period caps on token transfers through the multisig.

### API Endpoint

```
POST /smart-accounts/{address}/spending-limits

{
  "amount": "1000000000",
  "token_address": "<USDC_MINT_ADDRESS>",  // EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v on mainnet
  "period": "DAILY",
  "spending_limit_signers": ["<signer>"],
  "destinations": ["<allowed_dest>"],
  "transaction_signers": ["<signer1>", "<signer2>"]
}
```

### Period Options

| Period | Description |
|--------|-------------|
| `ONE_TIME` | Single-use limit |
| `DAILY` | Resets every 24h |
| `WEEKLY` | Resets every 7 days |
| `MONTHLY` | Resets every 30 days |
| `YEARLY` | Resets every 365 days |

### Field Reference

| Field | Type | Notes |
|-------|------|-------|
| `amount` | `string<uint64>` | Raw units (e.g., 6 decimals for USDC) |
| `token_address` | SPL mint | Use `11111111111111111111111111111111` for SOL |
| `destinations` | `Pubkey[]` | Optional — restrict to specific addresses |
| `expiration` | unix timestamp | Optional — auto-expire the limit |

### RTP Example: Per-Day USDC Cap

```json
{
  "amount": "10000000000",
  "token_address": "<USDC_MINT_ADDRESS>",  // EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v on mainnet
  "period": "DAILY",
  "spending_limit_signers": ["<agent_wallet>"],
  "destinations": ["<treasury_pda>"]
}
```

This caps USDC transfers at **10,000 USDC/day** to the treasury PDA.

**Note:** The same token can have multiple spending limits with different periods (e.g., a daily cap and a separate weekly cap).

---

## 4. Rust SDK

Squads provides both Rust and TypeScript SDKs:

| SDK | Package |
|-----|---------|
| **Rust** | `squads-multisig-program` on crates.io |
| **TypeScript** | `@sqds/multisig` on npm |

### Repository Structure

The v4 repo (`github.com/Squads-Protocol/v4`) contains:

```
├── programs/squads_multisig_program/   # On-chain Anchor program
├── sdk/                                # TypeScript SDK
└── cli/                                # CLI for multisig interaction
```

### RTP Relevance

The `squads-multisig-program` Rust crate provides **CPI helpers** for interacting with the Squads program from other Solana programs. This is critical for RTP — the treasury program can CPI into Squads for proposal creation, enabling the swarm to programmatically propose authority-gated actions without direct key access.

---

## 5. Time Lock Mechanism

Time lock is **per-account** (set on the multisig account), **not per-transaction**.

| Property | Details |
|----------|---------|
| Field | `time_lock: u32` on the Multisig account |
| Unit | Seconds |
| Scope | Applies to ALL transactions through this multisig |
| Granularity | Account-level only — no per-transaction time lock |

### Example

```
time_lock: 86400  →  24-hour delay between voting settlement and execution
```

### Lifecycle

```
Proposal Created → Voting Period → All Votes Collected
                                        │
                                        ▼  time_lock seconds
                                  Execution Window Opens
                                        │
                                        ▼
                                  Any CAN_EXECUTE signer can finalize
```

### RTP Implication

Setting `time_lock = 86400` means every authority-gated operation (`evolve_phase`, `register_strategy`, `force_retire_strategy`, `end_beta`) has a mandatory 24-hour delay after approval. This provides a safety window for human review and emergency intervention.

---

## 6. Audits & Security

Squads v4 has undergone extensive auditing:

| Auditor | Scope | Rounds |
|---------|-------|--------|
| OtterSec | Smart contract security | 3 |
| Neodyme | Smart contract security | 3 |
| Certora | Formal verification | 3 |
| Trail of Bits | Smart contract security | 1 |

**Final audited commit:** `64af7330413d5c85cbbccfd8c27a05d45b6e666f`

---

## 7. RTP Integration Plan

### Multisig Configuration

Create a **2-of-3 Squads multisig** with the following members:

| Signer | Role | Permissions | Key Type |
|--------|------|-------------|----------|
| Signer 1 | RTP Agent | `CAN_INITIATE`, `CAN_VOTE`, `CAN_EXECUTE` | Agent wallet (hot) |
| Signer 2 | Human Admin | `CAN_VOTE` | Cold wallet |
| Signer 3 | Emergency Recovery | `CAN_VOTE` | Backup key (offline) |

### Security Parameters

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Threshold | 2-of-3 | Agent + one human for ops, or both humans to override agent |
| Time Lock | 86400 (24h) | Mandatory delay on all authority-gated actions |
| Spending Limit (USDC) | $10,000/day | Cap on daily treasury outflows |

### Authority Mapping

| RTP Instruction | Squads Gate | Notes |
|-----------------|-------------|-------|
| `evolve_phase` | 2-of-3 + 24h time lock | Irreversible — requires full scrutiny |
| `register_strategy` | 2-of-3 + 24h time lock | Promotes strategy to Live |
| `force_retire_strategy` | 2-of-3 + 24h time lock | Emergency retirement |
| `end_beta` | 2-of-3 + 24h time lock | Manual beta adopter sunset |

### Implementation Path

1. **Create Squads multisig** via REST API with the 3 signers and threshold=2
2. **Set `treasury.authority`** to the Squads multisig PDA address
3. **Set time lock** to 86400 via PATCH endpoint
4. **Configure spending limits** for USDC transfers
5. **Add `squads-multisig-program`** Rust crate as dependency in `rtp/programs/rtp-treasury/Cargo.toml`
6. **Wire CPI helpers** for programmatic proposal creation from the swarm runtime

### CPI Integration (Rust)

The treasury program can use the `squads-multisig-program` crate to CPI into Squads for proposal creation. This enables the swarm to:

- Propose `register_strategy` after Night Shift validates a strategy
- Propose `evolve_phase` when on-chain thresholds are met
- Propose `force_retire_strategy` when the Security Wing detects anomalies

Each proposal still requires the 2-of-3 vote + 24h time lock — the swarm can initiate but cannot unilaterally execute.

---

## Summary

Squads v4 is production-ready for RTP's authority gating needs:

- ✅ PDA-as-authority works natively with Anchor constraint checks
- ✅ REST API for programmatic multisig management
- ✅ Spending limits for rate-capping treasury outflows
- ✅ Time lock for mandatory delay on critical operations
- ✅ Rust crate for CPI integration from the treasury program
- ✅ 4 auditors, 10 audit rounds, formal verification
- ✅ Same program ID on mainnet and devnet (seamless testing)
