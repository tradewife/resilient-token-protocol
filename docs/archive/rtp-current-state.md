# RTP Current State — MCP → Server SDK + Squads + Hydra Migration Map

**Date**: 2026-04-26  
**Purpose**: Research deliverable documenting every Phantom MCP reference, function call site, and documentation mention that must change when upgrading from the MCP subprocess pattern to the Phantom Server SDK, adding Squads multisig treasury authority, and integrating Hydra for automated crank execution.

---

## Priority Definitions

| Priority | Meaning |
|----------|---------|
| **P0** | Must change — security hardening or blocking architectural change |
| **P1** | Should change — functional correctness, new capability, or removes tech debt |
| **P2** | Nice to have — documentation polish, CI convenience, non-blocking improvements |

---

## 1. Source Code Files

### 1.1 `rtp/swarm/src/wings/trading/phantom_mcp.rs` — **P0**

**Scope**: Entire file (~500+ lines). This is the core MCP client.

**Current architecture**:
- `PhantomMcpClient` struct spawns `@phantom/mcp-server` as a subprocess
- Communicates via JSON-RPC over stdio (stdin/stdout)
- Every public function takes `di: u32` (derivation index) for per-token wallet isolation

**Functions that must be replaced or refactored**:

| Function | Purpose | Migration Path |
|----------|---------|----------------|
| `new()` | Starts MCP subprocess, discovers server PID | Replace with Server SDK HTTP client initialization |
| `quote_sol_to_usdc(di, amount)` | SOL → USDC swap quote | Server SDK `swap/quote` endpoint |
| `swap_sol_to_usdc(di, amount)` | Execute SOL → USDC swap | Server SDK `swap/execute` endpoint |
| `quote_usdc_to_sol(di, amount)` | USDC → SOL swap quote | Server SDK `swap/quote` endpoint |
| `swap_usdc_to_sol(di, amount)` | Execute USDC → SOL swap | Server SDK `swap/execute` endpoint |
| `quote_deposit_to_hl(di, amount)` | HL deposit quote (cross-chain) | Server SDK `bridge/quote` endpoint |
| `deposit_to_hl(di, amount)` | Bridge USDC to Hyperliquid | Server SDK `bridge/execute` endpoint |
| `withdraw_from_hl(di, amount, chain)` | Withdraw USDC from HL | Server SDK `bridge/execute` (reverse) |
| `transfer_spot_to_perps(di, amount)` | HL spot → perps internal transfer | Server SDK perps transfer endpoint |
| `get_perps_account(di)` | HL perps account balance | Server SDK perps account endpoint |
| `get_perps_positions(di)` | Open perps positions | Server SDK perps positions endpoint |
| `get_perp_orders(di)` | Open perps orders | Server SDK perps orders endpoint |
| `get_perp_trade_history(di)` | Historical trades | Server SDK perps history endpoint |
| `get_perp_markets()` | Available HL markets | Server SDK perps markets endpoint |
| `open_perp_position(di, market, direction, size, leverage, order_type)` | Open perp position | Server SDK perps open endpoint |
| `close_perp_position(di, market, size_pct)` | Close perp position | Server SDK perps close endpoint |
| `cancel_perp_order(di, market, order_id)` | Cancel perp order | Server SDK perps cancel endpoint |
| `update_perp_leverage(di, market, leverage, margin_type)` | Update leverage | Server SDK perps leverage endpoint |
| `get_wallet_addresses(di)` | Get Solana/EVM addresses | Server SDK wallet addresses endpoint |
| `get_token_balances(di, networks)` | Token balances across chains | Server SDK balances endpoint |
| `transfer_tokens(di, network, to, amount, mint)` | Transfer tokens | Server SDK transfer endpoint |
| `send_solana_transaction(di, tx_b64, network)` | Sign + broadcast Solana tx | Server SDK `solana/send` endpoint |
| `simulate_transaction(di, chain, type, params)` | Simulate tx before signing | Server SDK simulate endpoint |

**Key change**: The entire JSON-RPC-over-stdio transport layer is replaced by HTTP calls to the Server SDK. The `di: u32` parameter pattern is preserved — Server SDK supports the same `derivationIndex` concept.

**Internal helpers to remove/replace**:
- `start_server()` — subprocess spawn logic
- `send_request(method, params)` — JSON-RPC framing
- `read_response()` — JSON-RPC response parsing
- `wait_for_server_ready()` — health-check polling
- All JSON-RPC error code handling (`METHOD_NOT_FOUND`, `INVALID_PARAMS`, etc.)

---

### 1.2 `rtp/swarm/src/wings/trading/mod.rs` — **P0**

**Scope**: ~3068 lines. Trading Wing coordinator and execution logic.

**Call sites that reference `PhantomMcpClient`**:

| Location (function) | MCP Call | Change Required |
|----------------------|----------|-----------------|
| `mcp_bridge_flow()` | `client.quote_sol_to_usdc(di, amount)` | Update to Server SDK client |
| `mcp_bridge_flow()` | `client.swap_sol_to_usdc(di, amount)` | Update to Server SDK client |
| `mcp_bridge_flow()` | `client.deposit_to_hl(di, usdc_amount)` | Update to Server SDK client |
| `mcp_bridge_flow()` | `client.transfer_spot_to_perps(di, amount)` | Update to Server SDK client |
| `mcp_bridge_flow()` | `client.get_perps_account(di)` | Update to Server SDK client |
| `handle_execute_permit()` | Dispatches to `mcp_bridge_flow()` or HL direct | Update dispatch logic for new client |
| `deposit_sol_yield_to_treasury()` | Signs with local keypair | Integrate Squads multisig proposal flow |
| `TradingWing::new()` | Creates `PhantomMcpClient::new()` | Replace with Server SDK client init |

**Additional changes**:
- Signing cascade logic (lines referencing Phantom MCP → Phantom KMS → local keypair) must be updated to: Server SDK → Squads proposal → Hydra crank execution
- Error handling: JSON-RPC error codes → HTTP status codes + Server SDK error format
- `devnet_fund_stub()` (cfg devnet only) — no change needed, remains as simulation stub

---

### 1.3 `rtp/swarm/src/wings/trading/types.rs` — **P2**

**Scope**: Trading types and state management.

| Struct/Field | Change Required |
|--------------|-----------------|
| `TradingState.token_wallet_map: HashMap<String, u32>` | No change — derivation index concept is transport-agnostic |
| `TradingState.next_derivation_index: u32` | No change |
| `assign_derivation_index(&mut self, token_mint: &str)` | No change |
| `derivation_index_for(&self, token_mint: &str)` | No change |

**Verdict**: This file requires **no changes**. The per-token wallet isolation model maps cleanly to Server SDK's `derivationIndex` parameter.

---

### 1.4 `rtp/swarm/Cargo.toml` — **P1**

**Current dependencies** (relevant):
```toml
reqwest = { version = "0.12", features = ["json"] }
sha3 = "0.10"
secp256k1 = "0.29"
rmp-serde = "1.3"
rmp = "0.8"
serde_json = { version = "1.0", features = ["preserve_order"] }
tokio = { version = "1", features = ["full"] }
```

**Changes needed**:

| Change | Priority | Rationale |
|--------|----------|-----------|
| Add `squads-multisig` dependency | P1 | Squads SDK for multisig proposal creation |
| Add `hydra-sdk` or `hydra-api` dependency | P1 | Hydra crank scheduling and management |
| Remove `rmp-serde`, `rmp` if no longer needed | P2 | MessagePack was for MCP subprocess framing — HTTP/JSON replaces it |
| Consider adding `anchor-lang` to swarm crate | P2 | If Squads CPI helpers are needed at runtime |
| Update `reqwest` features if Server SDK requires specific auth | P1 | May need `rustls-tls` or cookie-based auth |

---

### 1.5 `rtp/programs/rtp-treasury/programs/rtp-treasury/src/lib.rs` — **P0**

**Scope**: ~1370 lines. On-chain treasury program (Anchor).

**Current authority model**:
- `Treasury.authority` is a `Signer` pubkey (single-key authority)
- Authority-gated instructions check `ctx.accounts.authority` against `treasury.authority`

**Changes needed**:

| Change | Priority | Details |
|--------|----------|---------|
| Add zero-address rejection guard on `initialize` | P0 | Reject `Pubkey::default()` as authority — prevents misconfiguration |
| Add `emergency_freeze(ctx)` instruction | P0 | Authority can freeze all treasury operations (emergency halt) |
| Add `emergency_unfreeze(ctx)` instruction | P0 | Authority can resume operations after freeze |
| Support Squads multisig PDA as authority | P1 | `Treasury.authority` can be a Squas multisig PDA — Anchor `Signer` check works unchanged since Squads signs via CPI |
| Add `set_authority(ctx, new_authority)` instruction | P1 | Allows migrating from single-key to Squads multisig PDA |
| Add `is_frozen` flag to `Treasury` account | P0 | Checked by all state-mutating instructions |
| Add `freeze_reason: String` to `Treasury` account | P2 | Human-readable audit trail for freeze events |
| Emit `AuthorityChanged` event | P2 | Audit trail for authority transitions |
| Emit `TreasuryFrozen` / `TreasuryUnfrozen` events | P1 | Audit trail for emergency actions |

**Affected instructions** (must add `is_frozen` guard):
- `initialize` — N/A (sets state)
- `evolve_phase` — add frozen check
- `register_strategy` — add frozen check
- `force_retire_strategy` — add frozen check
- `end_beta` — add frozen check
- `create_swarm_vault` — add frozen check
- `withdraw_fees` — add frozen check (permissionless but still state-mutating)
- `check_redistribute` — add frozen check (permissionless but moves funds)
- `hydrate_swarm` — add frozen check
- `record_fee_deposit` — add frozen check
- `update_strategy_performance` — add frozen check (write-only, debatable)

**Account size**: `Treasury` struct grows by ~64 bytes (`is_frozen: bool` + padding + `freeze_reason: String`). Account reallocation needed.

---

### 1.6 `sdk/index.ts` — **P1**

**Scope**: ~630 lines. TypeScript SDK for frontend/dashboard integration.

**Current exports**:
```typescript
registerWithRTP(connection, wallet, params)
withdrawAndRedistribute(connection, wallet, treasuryPda)
fetchTreasuryState(connection, treasuryPda)
```

**Changes needed**:

| Change | Priority | Details |
|--------|----------|---------|
| Add `setSquadsAuthority(connection, wallet, multisigPda)` | P1 | Helper to migrate treasury authority to Squads multisig |
| Add `freezeTreasury(connection, wallet, treasuryPda)` | P0 | Emergency freeze UI helper |
| Add `unfreezeTreasury(connection, wallet, treasuryPda)` | P0 | Emergency unfreeze UI helper |
| Add `fetchMultisigStatus(connection, multisigPda)` | P1 | Query Squads multisig state (pending proposals, active members) |
| Update `TreasuryState` type | P1 | Add `isFrozen`, `freezeReason`, `authorityType` fields |
| Update IDL import | P1 | Regenerate from updated Anchor program |

---

### 1.7 `dashboard/src/app/page.tsx` — **P1**

**Scope**: ~708 lines. Main dashboard page.

**Changes needed**:

| Change | Priority | Details |
|--------|----------|---------|
| Add multisig status indicator | P1 | Show Squads multisig state (active/pending proposals/member count) |
| Add emergency freeze banner | P0 | Visual indicator when treasury is frozen |
| Add Hydra crank status widget | P1 | Show crank queue depth, next execution time |
| Show authority type | P1 | Display "Single Key" vs "Squads Multisig (N/M)" |
| Update treasury state display | P1 | Show `isFrozen` field from on-chain state |

---

### 1.8 `rtp/swarm/src/demo.rs` — **P1**

**Scope**: End-to-end demo loop (8-step pipeline).

**Changes needed**:

| Change | Priority | Details |
|--------|----------|---------|
| Update MCP bridge demo step | P1 | Replace MCP subprocess call with Server SDK HTTP call |
| Add Squads proposal step | P2 | Demo submitting a proposal via Squads multisig |
| Add Hydra crank visualization | P2 | Show crank creation and execution in demo output |

---

### 1.9 `rtp/swarm/src/bin/rtp-daemon.rs` — **P1**

**Scope**: Devnet loop daemon, single-cycle execution, 6h cron.

**Changes needed**:

| Change | Priority | Details |
|--------|----------|---------|
| Replace MCP client initialization | P1 | Use Server SDK HTTP client instead of subprocess |
| Integrate Hydra crank creation | P1 | After yield operations, schedule crank for treasury distribution |
| Integrate Squads proposal submission | P1 | For authority-gated operations, submit via Squads multisig |
| Update signing cascade | P1 | Server SDK → Squads → Hydra flow |

---

## 2. Documentation Files

### 2.1 `SESSION-CONTEXT.md` — **P2**

**Scope**: ~916 lines. Compressed project memory.

**References to update** (~30+ occurrences):

| Reference Pattern | Count | Change |
|-------------------|-------|--------|
| `PhantomMcpClient` | ~8 | Replace with Server SDK client name |
| `phantom_mcp.rs` / `phantom_mcp` | ~12 | Update file/function references |
| `MCP subprocess` / `MCP server` | ~6 | Replace with "Server SDK HTTP" |
| `@phantom/mcp-server` | ~3 | Remove or mark as deprecated |
| Signing cascade mentions | ~4 | Update to Server SDK → Squads → Hydra |
| Agent wallet address references | ~3 | Verify still valid (derivation index unchanged) |

---

### 2.2 `CLAUDE.md` — **P1**

**Scope**: Project instructions for AI coding agents.

**Sections to update**:

| Section | Change |
|---------|--------|
| Signing Architecture | Replace MCP subprocess diagram with Server SDK + Squads + Hydra flow |
| Key Files table | Update `phantom_mcp.rs` description to reflect Server SDK |
| Repo Layout diagram | Update MCP references in Trading Wing description |
| Commands | Update `cargo test --lib trading::phantom_mcp::tests` to new module path |
| Hackathon Resources | Add Squads Multisig details, Hydra crate link |
| Devnet Limitations | Update — Server SDK may have different devnet support than MCP subprocess |
| Key Invariants | Add Squads multisig invariants, Hydra crank scheduling invariants |

---

### 2.3 `README.md` — **P2**

**Scope**: Public-facing project documentation.

**Changes needed**:
- Update Phantom Connect architecture description
- Add Squads Multisig section
- Add Hydra crank automation section
- Update signing flow diagram
- Update architecture diagram (three-layer stack)

---

### 2.4 `SOULCONTRACT.md` — **P1**

**Scope**: Constitutional governance document.

**Changes needed**:

| Change | Priority | Details |
|--------|----------|---------|
| Add Squads multisig constraints | P1 | Authority must be Squads multisig PDA after initialization, minimum M-of-N |
| Add emergency freeze/unfreeze rules | P0 | Define who can freeze, duration limits, unfreeze conditions |
| Update capital flow constraints | P1 | Reference Hydra crank scheduling instead of manual signing |
| Add Hydra crank scheduling rules | P1 | Define crank frequency, retry policy, failure handling |
| Update signing cascade description | P1 | Server SDK → Squads proposal → Hydra crank execution |

---

### 2.5 `docs/RESOURCES.md` — **P2**

**Scope**: Links to all sponsor resources and SDKs.

**Changes needed**:
- Add Squads API documentation link
- Add Hydra crate documentation link
- Update Phantom Connect links (Server SDK docs vs MCP server docs)
- Add Server SDK GitHub repo link

---

### 2.6 `docs/SECURITY_AUDIT_2026-04-07.md` — **P2**

**Scope**: Full security audit — 18 findings.

**Changes needed**:
- Cross-reference findings with new Squads/Hydra mitigations
- Update signing-related findings to reflect Server SDK
- Note which findings are now resolved by Squads multisig authority

---

## 3. CI/CD Workflows

### 3.1 `.github/workflows/devnet-loop.yml` — **P1**

**Scope**: 6h cron for daemon execution.

**Changes needed**:

| Change | Priority | Details |
|--------|----------|---------|
| Add Hydra cranker deployment step | P1 | Deploy or verify Hydra cranker service alongside daemon |
| Update environment variables | P1 | Server SDK auth tokens, Squads multisig address |
| Add Squads proposal status check | P2 | Verify pending proposals before daemon cycle |

---

### 3.2 `.github/workflows/swarm-ci.yml` — **P2**

**Scope**: Cargo build + test + clippy.

**Changes needed**:

| Change | Priority | Details |
|--------|----------|---------|
| Add Hydra test validator setup | P2 | If Hydra requires local validator for integration tests |
| Update test commands | P2 | Module path changes if `phantom_mcp.rs` is renamed |
| Add Squads program to test validator | P2 | Deploy Squads program to local validator for CPI tests |

---

## 4. Summary Statistics

| Category | P0 | P1 | P2 | Total |
|----------|----|----|-----|-------|
| Source code files | 3 | 5 | 2 | 10 |
| Documentation files | 0 | 2 | 4 | 6 |
| CI/CD workflows | 0 | 1 | 1 | 2 |
| **Total** | **3** | **8** | **7** | **18** |

### P0 (Security — Must Change)
1. `phantom_mcp.rs` — entire MCP transport layer replaced by Server SDK
2. `trading/mod.rs` — all MCP call sites updated, signing cascade rewritten
3. `rtp-treasury/src/lib.rs` — emergency freeze/unfreeze, zero-address guard, frozen checks

### P1 (Functional — Should Change)
4. `Cargo.toml` — new dependencies for Squads, Hydra
5. `sdk/index.ts` — Squads helpers, freeze/unfreeze SDK functions
6. `dashboard/page.tsx` — multisig status, freeze indicator, Hydra widget
7. `demo.rs` — updated demo flow
8. `rtp-daemon.rs` — Server SDK + Squads + Hydra integration
9. `CLAUDE.md` — updated architecture docs
10. `SOULCONTRACT.md` — new governance constraints
11. `devnet-loop.yml` — Hydra cranker deployment

### P2 (Polish — Nice to Have)
12. `trading/types.rs` — no change needed
13. `SESSION-CONTEXT.md` — reference updates
14. `README.md` — public docs update
15. `docs/RESOURCES.md` — link updates
16. `docs/SECURITY_AUDIT_2026-04-07.md` — cross-reference updates
17. `swarm-ci.yml` — test validator setup

---

## 5. Migration Order (Recommended)

```
Phase 1 — Security Hardening (P0)
  ├── 1a. Add emergency freeze/unfreeze to treasury program
  ├── 1b. Add zero-address rejection guard
  ├── 1c. Add is_frozen checks to all state-mutating instructions
  └── 1d. Deploy updated program to devnet, verify

Phase 2 — Server SDK Migration (P0)
  ├── 2a. Create new Server SDK HTTP client module (replace phantom_mcp.rs)
  ├── 2b. Port all 22 MCP functions to Server SDK HTTP equivalents
  ├── 2c. Update trading/mod.rs call sites
  ├── 2d. Update signing cascade in trading wing
  └── 2e. Run full test suite (307+ tests)

Phase 3 — Squads Multisig (P1)
  ├── 3a. Add set_authority instruction to treasury program
  ├── 3b. Add Squads proposal creation to daemon
  ├── 3c. Update SDK (sdk/index.ts) with Squads helpers
  └── 3d. Update dashboard with multisig status

Phase 4 — Hydra Crank (P1)
  ├── 4a. Add Hydra dependency to Cargo.toml
  ├── 4b. Integrate crank creation in daemon post-yield
  ├── 4c. Add crank status to dashboard
  └── 4d. Update CI workflows for Hydra cranker

Phase 5 — Documentation & Polish (P2)
  ├── 5a. Update SESSION-CONTEXT.md
  ├── 5b. Update CLAUDE.md
  ├── 5c. Update README.md
  ├── 5d. Update SOULCONTRACT.md
  ├── 5e. Update docs/RESOURCES.md
  └── 5f. Update CI workflows
```

---

## 6. Key Architectural Decision: Derivation Index Preservation

The `derivationIndex: u32` pattern used throughout the current MCP implementation maps directly to the Server SDK's `derivationIndex` parameter. This means:

- `TradingState.token_wallet_map` — **no change**
- `TradingState.next_derivation_index` — **no change**
- `assign_derivation_index()` — **no change**
- `derivation_index_for()` — **no change**

Every function in the new Server SDK client will continue to accept `di: u32` as the first parameter. The per-token wallet isolation model is preserved exactly.

---

*End of RTP Current State migration map.*
