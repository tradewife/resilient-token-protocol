# RTP Codebase Audit Report — April 27, 2026

## Executive Summary

The RTP codebase is in **solid shape** for the May 11 hackathon. The on-chain program (16 instructions, 12 frozen guards, zero-address rejection) is correctly implemented and all 307 Rust tests pass. The pipeline from Night Shift → bridge.rs → Trading Wing → Hyperliquid → Treasury is fully traced and intact. Security posture is clean — no secrets tracked, `.gitignore` coverage verified, all CI triggers correctly paused to `workflow_dispatch`.

**Two critical bugs** were found: (1) the dashboard reads the `frozen` field at byte offset 229 instead of the correct 225 (Phase enum is 1 byte in Borsh, not 5), and (2) `configs/.env.devnet` contains a stale program ID from a prior deployment. Four research docs are 100% aspirational (Server SDK/Squads/Hydra all deferred) and should be archived. Several doc-code inconsistencies exist (test counts, Squads program ID typos). One documented script (`compute_adopter_yield_share.ts`) was never created.

---

## Critical Issues (Must Fix Before May 11)

1. **Dashboard `frozenOffset` is wrong** — `dashboard/src/app/page.tsx:219`: offset is `229`, should be `225`. The `Phase` enum (unit variants only) serializes as 1 byte in Borsh, not 5 (`1+4`). The dashboard reads the wrong byte for the frozen flag. **Fix**: change `frozenOffset = 229` to `frozenOffset = 225` and update the comment at line 217.

2. **`configs/.env.devnet` has stale program ID** — Lines 5 and 16: `PROGRAM_ID=4LvsHbe9LLwgogcDbH7ieTsGcWZctjYFZkzZwaHDM8Ad` should be `PROGRAM_ID=8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB`. Explorer URL on line 16 also stale.

3. **`mcp_bridge_flow` hardcodes `di=0`** — `rtp/swarm/src/wings/trading/mod.rs:1200`: `let di = 0; // TODO: look up per-token derivation index from state`. Per-token wallet isolation is documented as "DONE" but the bridge flow doesn't use `TradingState.token_wallet_map`. For single-token demo this is fine; for multi-token it's broken.

4. **Missing `compute_adopter_yield_share.ts`** — Referenced in `MULTI_TOKEN_SCALING.md:42`, `SESSION-CONTEXT.md:494`, `README.md:444` but file does not exist. Either create it or remove doc references.

5. **No freeze/unfreeze tests** — Neither `treasury.ts` nor `strategy-lifecycle.ts` exercises `freeze_treasury` or `unfreeze_treasury`. No test verifies that a frozen treasury rejects operations. The feature is untested at the integration level.

---

## Doc-Code Inconsistencies

| File | Line | Current Text | Should Say | Priority |
|------|------|-------------|------------|----------|
| `configs/.env.devnet` | 5, 16 | `PROGRAM_ID=4LvsHbe9LLw...` | `PROGRAM_ID=8rt6yiBnRTy...` | **P0** |
| `dashboard/src/app/page.tsx` | 217, 219 | `frozenOffset = 229` (phase = 1+4) | `frozenOffset = 225` (phase = 1 byte) | **P0** |
| `SECURITY-HARDENING-SPEC.md` | 152 | `SQDS4ep65T869zMMBKyuUq6a6EgTu8ps...` | `SQDS4ep65T869zMMBKyuUq6a**D**6EgTu8ps...` (missing `D`) | P1 |
| `SECURITY-HARDENING-SPEC.md` | 245 | Same typo | Same fix | P1 |
| `SECURITY-HARDENING-SPEC.md` | 917 | Same typo | Same fix | P1 |
| `AGENTIC-ENGINEERING-GRANT-APPLICATION.md` | 28 | `306 Rust tests passing` | `307 Rust tests passing` | P1 |
| `AGENTIC-ENGINEERING-GRANT-APPLICATION.md` | 39 | `306 passing Rust tests` | `307 passing Rust tests` | P1 |
| `dashboard/src/app/docs/page.tsx` | 104 | "14 instructions" | "16 instructions" | P1 |
| `dashboard/src/app/docs/page.tsx` | 539-550 | `TreasuryState` type missing `isFrozen` | Add `isFrozen: boolean` | P1 |
| `CLAUDE.md` Trust Model | — | `create_swarm_vault` listed as **Authority-gated** | Actually **Permissionless** (no `has_one` check) | P1 |
| `SESSION-CONTEXT.md` | 355, 389, 420 | `306 Rust tests` / `306 tests` | `307` | P2 |
| `SECURITY-HARDENING-SPEC.md` | 264 | `Create squads_client.rs` | File never created — add DEFERRED | P2 |
| `SECURITY-HARDENING-SPEC.md` | 463 | `Create hydra_crank.rs` | File never created — add DEFERRED | P2 |
| `SECURITY-HARDENING-SPEC.md` | 910-919 | Doc Update Checklist — all items unchecked | All deferred — mark as POST-HACKATHON | P2 |
| `dashboard/src/app/docs/page.tsx` | 496-591 | SDK Reference: 3 functions documented | 9 exported — missing freeze/unfreeze/beta | P2 |
| `docs/architecture.md` | — | No mention of freeze/unfreeze or per-token isolation | Update or add reference to CLAUDE.md | P2 |

---

## Stale Files Recommended for Archive/Deletion

| File | Size | Action | Priority | Reason |
|------|------|--------|----------|--------|
| `research/rtp-current-state.md` | 19.5 KB | **ARCHIVE** → `docs/archive/` | P2 | 100% aspirational Server SDK/Squads/Hydra migration map, all deferred |
| `research/phantom-server-sdk-findings.md` | 12.7 KB | **ARCHIVE** → `docs/archive/` | P2 | Recommends deprecated Server SDK path |
| `research/squads-findings.md` | 10.6 KB | **ARCHIVE** → `docs/archive/` | P2 | Squads integration deferred post-hackathon |
| `research/hydra-findings.md` | 10.7 KB | **ARCHIVE** → `docs/archive/` | P2 | Hydra integration deferred post-hackathon |
| `scripts/hl_testnet_demo.py` | 3.5 KB | **ARCHIVE** → `scripts/archive/` | P2 | Self-labeled DEPRECATED in header |
| `configs/turnkey.json` | 249 B | **DELETE** | P1 | Zero references, abandoned signing approach, gitignored |
| `package.json` (repo root) | 622 B | **DELETE** | P1 | Empty npm stub, no scripts, no deps. Dashboard has its own. Creates misleading `node_modules/` |
| `package-lock.json` (repo root) | — | **DELETE** | P1 | Companion to empty root package.json |

**KEEP (verified):** `demo.sh`, `research/dead_ends.md`, `MULTI_TOKEN_SCALING.md`, `video/` (gitignored), `data/autoresearch_*.json` (gitignored, used by autoresearch.py), `docs/archive/PLAN-MULTI-PLATFORM-LAUNCHER.md`.

---

## Pipeline Gaps

1. **Dashboard frozen offset** — `page.tsx:219` reads byte 229 instead of 225. The frozen indicator will read a wrong byte (likely `bump` or trailing data), showing incorrect frozen/unfrozen state. **Impact**: cosmetic only (on-chain enforcement is correct), but judges could notice a non-functional freeze indicator.

2. **Launch page no frozen awareness** — `dashboard/src/app/launch/page.tsx` does not check `isTreasuryFrozen` before allowing token registration. TX will fail on-chain but UI gives no warning. **Impact**: poor UX if treasury is frozen during demo.

3. **`isFrozen` polled but never rendered** — `page.tsx:81` stores `isFrozen` state, polls every 30s, but the JSX never shows a frozen banner or indicator. Dead state variable.

4. **Daemon has no frozen pre-check** — `rtp-daemon.rs` does not check treasury frozen state before running a cycle. On-chain CPI will reject, but the daemon wastes a full cycle of work. Minor — won't break demo.

5. **`mcp_bridge_flow` ignores per-token wallets** — `trading/mod.rs:1200` hardcodes `di=0` with a TODO comment. Multi-token demo would use wrong wallet. Acceptable for single-token hackathon demo.

---

## Dead Code / Unused Imports

| File | Line | Item | Safe to Remove? |
|------|------|------|-----------------|
| `trading/mod.rs` | 1-2 | Duplicate doc comment line | Yes |
| `trading/mod.rs` | 251-258 | `#[deprecated] #[allow(dead_code)] sign_action()` | Yes — replaced by EIP-712 signing |
| `Cargo.toml` | dep `aes-gcm` | No usage found in `src/` | Verify — may be used in integration tests |
| `Cargo.toml` | dep `rmp-serde` | No `rmp_serde::` call found; only `rmp` used directly | Likely safe, but verify no derive usage |
| `security/mod.rs` | 37 | `#[allow(dead_code)]` on `AlertEntry` fields | Intentional forward-looking — KEEP |
| `coordinator/lifecycle.rs` | 104 | `_metrics` parameter unused in `heartbeat()` | Intentionally underscore-prefixed — KEEP |

---

## Security Findings

1. **Severity: PASS** — `git ls-files configs/` returns empty. All config files gitignored. ✅
2. **Severity: PASS** — No `.env` files tracked. No `*key*`/`*secret*`/`*token*` files tracked. ✅
3. **Severity: PASS** — `.gitignore` covers `configs/*.json`, `configs/.env*`, `video/`, `data/*`, `node_modules/`, `.env*`, `*.key`, `*.pem`. ✅
4. **Severity: PASS** — Grep for leaked keys in `.md` files: all matches are env var names or placeholders, no real secrets. ✅
5. **Severity: PASS** — `dashboard/.env` and `dashboard/.env.local` do not exist. ✅
6. **Severity: PASS** — `SECURITY-HARDENING-SPEC.md` code examples contain only public addresses, program IDs, and `process.env.*` references. ✅
7. **Severity: INFO** — `lib.rs:807`: `deposit_count += 1` uses raw `+=` instead of `saturating_add`. Practically impossible to overflow (u64), but breaks the consistency pattern. All other counters use `saturating_add` or `checked_add`.

---

## CI/CD Issues

1. **PASS** — All 4 workflows use `workflow_dispatch` as the only active trigger. Push/PR/cron triggers are commented out per CLAUDE.md policy. ✅
2. **PASS** — `swarm-ci.yml`: `cargo test` (full, not `--lib`), `cargo clippy -- -D warnings`, `cargo fmt --check`, AVM 1.0.0. ✅
3. **PASS** — `deploy-dashboard.yml`: push triggers commented out, `workflow_dispatch` only. ✅
4. **LOW** — `devnet-loop.yml:34`: `git push` has no error handling. If push fails, workflow fails silently. `night_shift.yml:89` handles this correctly with `git push || echo "Push failed..."`. Consider mirroring that pattern.
5. **PASS** — `night_shift.yml`: module paths match current layout, 300-min timeout. ✅

---

## On-Chain Program Verification

| Check | Result |
|-------|--------|
| Frozen guards (12/12) | ✅ All 12 state-mutating instructions have `require!(!treasury.frozen)` |
| Zero-address guard (5/5 fields) | ✅ authority, mint, holders_wallet, project_dev_wallet, ecosystem_wallet |
| `#[derive(InitSpace)]` for `frozen: bool` | ✅ Auto-computed, no manual space math |
| Event emission (TreasuryFrozen/Unfrozen) | ✅ Correct fields: mint, authority, timestamp |
| Error variants (AlreadyFrozen/NotFrozen) | ✅ Correct guard logic |
| `create_swarm_vault` frozen guard | ✅ Present at lib.rs:718 |
| Overflow safety | ✅ All counters use `saturating_add`/`checked_add` except `deposit_count` (raw `+=`, u64) |
| Instruction count | ✅ 16 instructions match CLAUDE.md |
| 70/20/10 redistribution split | ✅ Correct with remainder approach |
| Tests for freeze/unfreeze | ❌ **None exist** |

---

## Prioritized Action List

| # | Action | Effort | Files | Priority |
|---|--------|--------|-------|----------|
| 1 | Fix dashboard `frozenOffset` from 229 → 225 | **S** | `dashboard/src/app/page.tsx:217-219` | **P0** |
| 2 | Update `configs/.env.devnet` program ID | **S** | `configs/.env.devnet:5,16` | **P0** |
| 3 | Add frozen banner to dashboard UI | **S** | `dashboard/src/app/page.tsx` (JSX section) | **P0** |
| 4 | Add `isFrozen` check to launch page | **S** | `dashboard/src/app/launch/page.tsx` | **P1** |
| 5 | Fix Squads program ID typo (3 places) | **S** | `SECURITY-HARDENING-SPEC.md:152,245,917` | **P1** |
| 6 | Fix test count "306" → "307" (5 places) | **S** | `AGENTIC-ENGINEERING-GRANT-APPLICATION.md:28,39`, `SESSION-CONTEXT.md:355,389,420` | **P1** |
| 7 | Fix docs page instruction count "14" → "16" | **S** | `dashboard/src/app/docs/page.tsx:104` | **P1** |
| 8 | Add `isFrozen` to docs `TreasuryState` type | **S** | `dashboard/src/app/docs/page.tsx:539-550` | **P1** |
| 9 | Create `scripts/compute_adopter_yield_share.ts` or remove 3 doc references | **S** | `MULTI_TOKEN_SCALING.md:42`, `SESSION-CONTEXT.md:494`, `README.md:444` | **P1** |
| 10 | Delete orphaned `configs/turnkey.json`, root `package.json`/`package-lock.json` | **S** | Root directory | **P1** |
| 11 | Write freeze/unfreeze Anchor tests | **M** | `rtp/programs/rtp-treasury/tests/treasury.ts` | **P1** |
| 12 | Archive 4 deferred research docs to `docs/archive/` | **S** | `research/rtp-current-state.md`, `phantom-server-sdk-findings.md`, `squads-findings.md`, `hydra-findings.md` | **P2** |
| 13 | Archive deprecated `scripts/hl_testnet_demo.py` | **S** | `scripts/hl_testnet_demo.py` | **P2** |
| 14 | Document freeze/unfreeze + remaining SDK functions in docs page | **M** | `dashboard/src/app/docs/page.tsx:496-591` | **P2** |
| 15 | Fix `CLAUDE.md` Trust Model: `create_swarm_vault` is permissionless | **S** | `CLAUDE.md` | **P2** |
| 16 | Mark SECURITY-HARDENING-SPEC.md checklist items as DEFERRED | **S** | `SECURITY-HARDENING-SPEC.md:910-919` | **P2** |
| 17 | Resolve `mcp_bridge_flow` `di=0` TODO for multi-token | **M** | `rtp/swarm/src/wings/trading/mod.rs:1200` | **P3** |
| 18 | Add frozen pre-check to rtp-daemon | **S** | `rtp/swarm/src/bin/rtp-daemon.rs` | **P3** |
| 19 | Verify/remove `aes-gcm` and `rmp-serde` deps | **S** | `rtp/swarm/Cargo.toml` | **P3** |
| 20 | Add error handling to devnet-loop git push | **S** | `.github/workflows/devnet-loop.yml:34` | **P3** |

**Estimated total effort**: ~4-6 hours for P0+P1 items (12 actions, mostly S). P2 items are another ~2 hours. P3 items are nice-to-have.
