# RTP Remediation Spec

Goal: make the hackathon submission defensible as a real autonomous Solana/DeFi protocol, not a demo pipeline.

This spec covers the weaknesses found in the April 29, 2026 Colosseum Copilot audit pass. It is intentionally implementation-oriented.

## P0: Fix Flash Trade CPI Account Validation ✅ DONE

Problem: `open_flash_position` documents the Flash Trade program at `remaining[16]` but validates `remaining[15]`.

Files:

- `rtp/programs/rtp-treasury/programs/rtp-treasury/src/lib.rs`
- `rtp/programs/rtp-treasury/tests/flash-trade-cpi.ts`
- `sdk/idl.ts`
- `dashboard/src/lib/sdk/idl.ts`

Tasks:

- [x] Change `open_flash_position` program validation from `remaining[15]` to `remaining[16]`.
- [x] Add validation that `remaining[15]` is the expected Flash Trade event authority PDA.
- [x] Add validation that `remaining[13]` and `remaining[14]` are expected system/token programs.
- [x] Add tests:
   - valid 19-account layout passes pre-CPI validation
   - wrong `remaining[16]` fails with `InvalidFlashProgramId`
   - wrong `remaining[15]` fails with a new `InvalidFlashEventAuthority`
   - wrong `remaining[13]` fails with `InvalidFlashSystemProgram`
   - wrong `remaining[14]` fails with `InvalidFlashTokenProgram`
   - Token-2022 accepted at `remaining[14]`
   - too few accounts fails with `FlashCpiFailed`
- [x] Regenerate IDLs after Anchor build (sdk/idl.ts, dashboard/src/lib/sdk/idl.ts).

Acceptance:

- `anchor test` catches the wrong program index. ✅ (7 new tests added in flash-trade-cpi.ts)
- Code and IDL comments agree on every remaining account index. ✅ (validation now matches IDL v15.2.0 doc comments)

## P0: Make Daemon Actually Execute Transactions ✅ DONE

Problem: `RTP_MAINNET_EXECUTE` only changes logs. The daemon runs `run_two_cycle_demo()` and never submits `open_flash_position` or `close_flash_position`.

Files:

- `rtp/swarm/src/bin/rtp-daemon.rs`
- `rtp/swarm/src/wings/trading/mod.rs`
- Add `rtp/swarm/src/chain_client.rs`

Tasks:

1. [x] Add env-driven config:
   - `RTP_PROGRAM_ID`
   - `RTP_MINT`
   - `RTP_TREASURY_PDA`
   - `RTP_STRATEGY_ID`
   - `RTP_AUTHORITY_KEYPAIR`
   - `SOLANA_RPC_URL`
   - `RTP_EXECUTION_MODE=simulate|devnet|mainnet`
2. [x] Implement a Rust chain client that:
   - loads authority keypair
   - derives treasury, vault, and strategy PDA
   - builds Anchor instruction data for `open_flash_position`
   - derives Flash Trade remaining accounts
   - sends transaction with retry and confirmation
3. [x] Replace daemon `run_two_cycle_demo()` execution path with:
   - read live strategy from chain
   - read Night Shift candidate
   - run security/audit gates
   - build `open_flash_position`
   - submit or simulate based on `RTP_EXECUTION_MODE`
4. [x] Add close path:
   - query Flash Trade positions
   - close positions older than `max_hold_hours`
   - call `close_flash_position`
5. [x] Keep demo loop only behind `RTP_DEMO_MODE=1`.

Acceptance:

- In `simulate`, daemon prints actual serialized transaction and RPC simulation result.
- In `devnet`, daemon submits a real transaction or fails with a precise on-chain error.
- No "real transactions will be sent" log unless a send path exists.

## P0: Remove Hardcoded Treasury PDAs ✅ DONE

Problem: daemon uses inconsistent hardcoded PDAs.

Files:

- `rtp/swarm/src/bin/rtp-daemon.rs`

Tasks:

1. [x] Delete hardcoded PDA strings.
2. [x] Derive treasury PDA from `RTP_MINT` and `RTP_PROGRAM_ID`.
3. [x] Use the same derived PDA for:
   - frozen check
   - stale-position query
   - open/close transaction construction
4. [x] Add startup log:
   - program id
   - mint
   - derived treasury PDA
   - RPC URL
   - execution mode

Acceptance:

- `rg "FNQb|7oZT|FumRW" rtp/swarm/src` returns zero hardcoded operational addresses except test fixtures/docs.

## P0: Enforce `AdopterRecord.treasury` Everywhere ✅ DONE

Problem: the back-reference exists but is not enforced in critical account contexts.

Files:

- `rtp/programs/rtp-treasury/programs/rtp-treasury/src/lib.rs`
- `rtp/programs/rtp-treasury/tests/treasury.ts`

Tasks:

1. [x] Add constraints:
   - `HydrateSwarm.adopter_record.treasury == treasury.key()`
   - `RecordFeeDeposit.adopter_record.treasury == treasury.key()`
   - `EndBeta.adopter_record.treasury == treasury.key()`
2. [x] Add error: `AdopterTreasuryMismatch`.
3. [x] Add tests using an adopter record from another treasury/mint:
   - `hydrate_swarm` rejects
   - `record_fee_deposit` rejects
   - `end_beta` rejects

Acceptance:

- Cross-treasury adopter records cannot influence funding, accounting, or beta expiry.

## P0: Make Fee Attribution Non-Gameable ✅ DONE

Problem: `record_fee_deposit` allows arbitrary signer and arbitrary amount.

Recommended implementation:

1. [x] Change `record_fee_deposit` to require `authority.key() == treasury.authority`.
2. Add `source_tx: Pubkey` or `source_slot: u64` argument for audit trace.
3. Longer-term: combine fee withdrawal and adopter attribution in one instruction.

Alternative:

- Remove public `record_fee_deposit` and account for deposits only inside `withdraw_fees`.

Acceptance:

- Random signer cannot inflate adopter contribution.
- Tests prove unauthorized `record_fee_deposit` fails.

## P1: Fix Night Shift Handoff Across Railway Services

Problem: data is baked into images or absent from the daemon image.

Files:

- `rtp/swarm/Dockerfile.daemon`
- `scripts/Dockerfile.promote`
- `rtp/swarm/src/bridge.rs`

Tasks:

1. Stop relying on baked `data/night_results`.
2. Add shared storage backend:
   - simplest: Railway volume mounted at `/data/night_results`
   - fallback: GitHub raw fetch by commit SHA
3. Set `NIGHT_RESULTS_DIR=/data/night_results`.
4. Update Night Shift Docker service to write directly to mounted path.
5. Update promote and daemon services to read that same path.
6. Keep git commit as archival only, not primary handoff.

Acceptance:

- Night Shift writes `summary.json`.
- Promote service reads the same file without image rebuild.
- Daemon reads the same file without image rebuild.

## P1: Replace Demo Integration Tests With Real Pipeline Tests

Problem: current 5 tests are not end-to-end.

Add tests:

1. `night_shift_summary_to_promotion_dry_run`
   - create temp `summary.json`
   - run promotion gate
   - assert expected strategy id and Sharpe
2. `promotion_registers_on_local_validator`
   - local validator
   - initialize treasury
   - run promote script
   - fetch `StrategyRecord`
3. `daemon_simulates_open_position`
   - mock Flash accounts
   - `RTP_EXECUTION_MODE=simulate`
   - assert daemon builds `open_flash_position`
4. `stale_position_triggers_close_simulation`
   - mock Flash Trade position API
   - assert close transaction is built
5. `night_shift_to_daemon_config`
   - temp shared `NIGHT_RESULTS_DIR`
   - daemon reads latest candidate and applies params

Acceptance:

- Tests fail if the pipeline reverts to demo-only behavior.

## P1: Make Knowledge Wing Persistence Real in Railway

Problem: persistence API exists, but daemon/demo mostly use `KnowledgeWing::new()` and temp tests.

Tasks:

1. Add env var `RTP_KNOWLEDGE_PATH`.
2. In daemon, instantiate `KnowledgeWing::new_with_persistence(path)` when set.
3. Default Railway path: `/data/swarm-memory/knowledge.json`.
4. Mount Railway volume at `/data`.
5. Add restart test:
   - write knowledge
   - recreate daemon/wing
   - assert reload

Acceptance:

- Knowledge survives container restart on Railway volume.

## P1: Emergency Controls Must Actually Unwind

Problem: `emergency_close_all_positions` only zeroes counters.

Tasks:

1. Rename current instruction/event to `emergency_reset_position_counters`.
2. Add CLI runbook commands:
   - `rtp freeze`
   - `rtp positions list`
   - `rtp positions close --all`
   - `rtp positions reset-counters`
3. Implement `positions close --all` to call real `close_flash_position` per open Flash Trade position.
4. Add warning if counters reset while Flash Trade positions remain open.

Acceptance:

- No UI/CLI copy implies reset equals unwind.
- Emergency path can freeze and submit close transactions from cold start.

## P1: Make Redistribution Semantics Honest

Problem: `check_redistribute` transfers to three wallets, not individual holders.

Tasks:

1. Rename docs from "70% to holders" to "70% to holders recipient wallet" unless real holder distribution exists.
2. If actual holder distribution is required:
   - implement Merkle distributor or claimable balance program
   - derive holder allocation snapshots off-chain
   - write root on-chain
3. For the hackathon, prefer honest wording unless the real holder distribution can be completed and tested.

Acceptance:

- No claim that individual holders are paid unless implemented.

## P2: Harden Promotion Idempotency

Problem: `makeStrategyId("SOL", 2.69)` creates collisions.

Tasks:

1. Strategy ID should include date or hash:
   - `SOL_260429_A1`
   - first 6 chars of params hash
2. Store `summary_hash` or `params_hash` in strategy metadata if space allows.
3. Do not treat all `0x0` errors as "already exists."

Acceptance:

- Same symbol/score with different params does not collide silently.

## P2: Verify Railway Claims Programmatically

Tasks:

1. Add `scripts/check-railway-services.ts`.
2. Query Railway GraphQL for:
   - service exists
   - latest deployment status
   - cron schedule
   - env vars present, without printing secrets
3. CI/manual command outputs a red/green table.

Acceptance:

- "6 services green" can be reproduced from repo tooling.

## P2: Documentation Cleanup

Files:

- `README.md`
- `CLAUDE.md`
- `SOULCONTRACT.md`
- dashboard docs

Tasks:

1. Replace "autonomously executes" with accurate status until P0 daemon execution is done.
2. Document human dependencies:
   - authority key custody
   - freeze/unfreeze
   - emergency close
   - promotion keypair
3. Add architecture diagram showing actual data paths.

Acceptance:

- A judge cannot point to a documented autonomy claim that code does not satisfy.

## Definition Of Done

Run and attach outputs for:

```bash
rg "FNQb|7oZT" rtp/swarm/src
cd rtp/programs/rtp-treasury && anchor test
cd rtp/swarm && cargo test
cd rtp/swarm && RTP_EXECUTION_MODE=simulate cargo run --bin rtp-daemon
npx tsx scripts/promote-strategy.ts --dry-run
npx tsx scripts/check-railway-services.ts
```

Final demo must show:

1. Night Shift writes `summary.json`.
2. Promote reads it and registers a Live strategy.
3. Daemon reads that strategy.
4. Daemon builds and simulates or submits `open_flash_position`.
5. Stale position logic builds or submits `close_flash_position`.
6. Freeze prevents mutation.
7. Emergency close path is operational.
