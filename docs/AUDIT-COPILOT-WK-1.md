# Audit: Colosseum Copilot Week 1 — Pre-Submission Polish Pass

**Date:** 2026-04-13
**Deadline:** May 11, 2026 (Frontier, COLOSSEUM)
**Context:** Solo builder, ~28 days remaining. Judge has exactly 3 minutes.

---

## JUDGE-READINESS SCORE

### 1. On-chain constraint rejection — 4/10

**What currently proves it:** The Anchor program at `4LvsHb...M8Ad` genuinely enforces `BelowThreshold` in `evolve_phase` (lib.rs line 408-411: `require!(vault_balance >= SUSTENANCE_CAP)`), and the Anchor test suite (`treasury.ts` lines 777-793) sends a real `evolvePhase` TX that fails with `BelowThreshold`. The `devnet-demo.ts` script (line 531) also calls `evolvePhase` against the deployed devnet program and catches the rejection. The redistribution tx `9HzWgB...` on explorer proves 70/20/10 enforcement.

**Why it's a 4 not higher:** In `demo.sh`, constraint rejection is **simulated in Rust** (`simulate_below_threshold_withdrawal()` at demo.rs:414 returns a hardcoded `Err("BelowThreshold...")`). A judge running `./demo.sh` sees a Rust `println!`, not an actual on-chain TX failure. The real on-chain rejection is only visible if the judge runs `npm run demo:devnet` separately — and demo.sh Layer 3 falls through with instructions if neither localhost:8899 nor devnet is reachable via curl. The `demo-flow.md` script doesn't explicitly tell the presenter to show the on-chain rejection TX on explorer.

**Single highest-leverage action:** Add a `curl` call in demo.sh Layer 3 that sends a real `evolvePhase` transaction to the deployed devnet program (using a pre-signed TX or the demo keypair), so the judge sees the actual on-chain `BelowThreshold` error in terminal, and add the explorer link to that specific failed TX in the output.

### 2. Autonomous operation — 8/10

**What currently proves it:** The `devnet-loop.yml` GitHub Action runs every 6h via cron (`0 */6 * * *`), executes `cargo run --release --bin rtp-daemon`, commits cycle output to `data/devnet-cycles/`, and pushes. Three completed cycles exist (`2026-04-12T21`, `T22`, `T23`) in `data/devnet-cycles/latest/cycle.json` showing real LLM-driven mutations (GLM-5.1, `used_llm: true`). The `night_shift.yml` CI runs 30K config evaluations. The daemon cycle output shows accepted/rejected mutations with rationale.

**Why it's an 8 not 10:** The devnet loop only has 3 cycles committed so far — could look like a manual test. No heartbeat monitoring or alerting visible. The cycle JSON files reference `/tmp/rtp-demo-memory/` paths that don't survive between CI runs (tmpfs is ephemeral on GitHub runners).

**Single highest-leverage action:** Ensure at least 10+ cycle commits accumulate before May 11 (the 6h cron will produce ~4/day, so ~100+ by deadline if CI stays green). Add a cycle count badge or summary to README showing accumulated cycles.

### 3. Persistent memory — 5/10

**What currently proves it:** The Rust demo (`run_two_cycle_demo`) writes memory files to `/tmp/rtp-demo-memory/` (working + project tiers), and the print output lists them: `[MEMORY] files written to: /tmp/rtp-demo-memory/project`. The devnet cycle JSON includes a `memory_files` array showing 14 files across working/project tiers. The `MemoryPromotion` module (`memory_promotion.rs`) implements a real promotion ladder from working → project → overview tiers.

**Why it's a 5 not higher:** `/tmp/` is not persistent — it's wiped on reboot. The devnet cycle commits `cycle.json` but the actual memory files referenced (`/tmp/rtp-demo-memory/...`) are ephemeral on CI runners. A judge who inspects the paths will find they don't exist. There is no Arweave, IPFS, or on-chain memory storage. The "persistence" is only proven within a single demo run, not across restarts.

**Single highest-leverage action:** Change the memory persistence root from `/tmp/rtp-demo-memory/` to a committed directory (e.g. `data/swarm-memory/`) in the daemon config, so memory files are committed alongside cycle output. The judge can then `ls data/swarm-memory/` and see files that persisted across CI cycles.

### 4. Visible adaptation — 7/10

**What currently proves it:** The devnet cycle data shows real strategy mutation: `signal_threshold` changed from 0.28 → 0.25, `trailing_stop_atr` from 0.4 → 0.8, with LLM-generated rationale ("lower threshold to increase trade frequency and break stagnation"). The `rtp-daemon.rs` logs accepted vs rejected mutations. The `validate_mutation_bounds` function enforces soulcontract bounds, and the `SOULCONTRACT_BOUNDS` table in `evolve/mod.rs` defines hard limits per parameter.

**Why it's a 7 not 10:** The adaptation is only visible in JSON files — a judge has to `cat data/devnet-cycles/latest/cycle.json` to see it. There is no dashboard visualization of param changes over time. The demo.sh output mentions `[EVOLVE] mutation:` lines but doesn't highlight the before/after diff visually.

**Single highest-leverage action:** Add a `diff`-style output to demo.sh Layer 2 showing `params_used` vs `params_next` side-by-side, and/or add a simple ASCII sparkline of param evolution across all committed cycles in `data/devnet-cycles/`.

### 5. Observable treasury state — 6/10

**What currently proves it:** The dashboard fetches live SOL balance from devnet RPC every 10s. The explorer link to `FNQbK1Vw77aT7qM1EMSmeEPDGizSNhX4rkkYBKQNFotF` works. The redistribution TX `9HzWgB...` is visible on explorer. Program ID and PDA are displayed in the dashboard footer.

**Why it's a 6 not 10:** USDC balance (the actual yield currency) is NOT queried — only SOL balance. The hero metrics are hardcoded: "89.90 USDC Reserves", "+12.1% Monthly Yield", "298 Tests Passing". The feed is 12 static lines. The wings status is hardcoded. The "Connect Phantom" button does nothing. If the judge clicks the explorer link and sees a different SOL balance than what's shown, or notices the USDC claim is fabricated, credibility collapses.

**Single highest-leverage action:** Remove the three hardcoded hero metrics and replace them with a single live metric ("Treasury Balance: X SOL" from RPC), or clearly label them as "projected" / "simulated". At minimum, delete "89.90 USDC Reserves" — there is no USDC token account query to back this number.

---

## POLISH TASKS

### 1. Make constraint rejection provably on-chain in demo.sh
- **Priority:** HIGH
- **File:** `demo.sh` ~line 160 (Layer 3 section, after "Devnet reachable" check)
- **Change:** When devnet is reachable, execute the on-chain `evolvePhase` rejection via the existing `devnet-demo.ts` script (which already does this at line 531). Ensure the output is piped through with visible `BelowThreshold` error. If devnet is unreachable, print the explorer link to the redistribution TX and the specific Anchor test line that proves it.
- **Estimated time:** 2h
- **Demo point(s):** 1 (on-chain constraint rejection)

### 2. Remove fabricated hero metrics from dashboard
- **Priority:** HIGH
- **File:** `dashboard/src/app/page.tsx` ~line 85-98 (hero-metrics div)
- **Change:** Replace the three hardcoded metrics (`89.90`, `+12.1%`, `298`) with live SOL balance (already fetched) and a "Devnet" badge instead of fake yield numbers.
- **Estimated time:** 1h
- **Demo point(s):** 5 (observable treasury state)

### 3. Fix Phantom button — either wire it up or remove it
- **Priority:** HIGH
- **File:** `dashboard/src/app/page.tsx` ~line 70
- **Change:** Replace the button text with "Phantom Connect (coming soon)" and disable it, or implement a minimal `onClick` that calls `window.solana.connect()`.
- **Estimated time:** 0.5h
- **Demo point(s):** 5

### 4. Change memory persistence path from /tmp to committed directory
- **Priority:** HIGH
- **File:** `rtp/swarm/src/orchestrator.rs` ~line 190, `rtp/swarm/src/memory_promotion.rs` ~line 211
- **Change:** Change default memory root from `/tmp/rtp-demo-memory/` to `data/swarm-memory/` (relative to repo root). Add to git.
- **Estimated time:** 1h
- **Demo point(s):** 3 (persistent memory)

### 5. Add visible param diff output to demo.sh Layer 2
- **Priority:** MEDIUM
- **File:** `demo.sh` ~line 135 (after cargo run --bin rtp-demo)
- **Change:** After the rtp-demo output, add a python3 one-liner that reads `data/devnet-cycles/latest/cycle.json` and prints `params_used` vs `params_next` with arrows for changes.
- **Estimated time:** 0.5h
- **Demo point(s):** 4 (visible adaptation)

### 6. Add architecture diagram to docs/
- **Priority:** MEDIUM
- **File:** Create `docs/architecture.md`
- **Change:** Create a Mermaid diagram showing the three-layer stack with data flow arrows.
- **Estimated time:** 1h
- **Demo point(s):** Indirect — judge comprehension

### 7. Add judge-facing one-pager
- **Priority:** MEDIUM
- **File:** Create `docs/JUDGE-ONE-PAGER.md`
- **Change:** Single-page document with one-liner, five demo points with proof locations, explorer links, key metrics.
- **Estimated time:** 1h
- **Demo point(s):** All five — judge comprehension

### 8. Make feed and wings dynamic (or clearly label as demo)
- **Priority:** MEDIUM
- **File:** `dashboard/src/app/page.tsx` ~line 17 (FEED_LINES), ~line 32 (WINGS)
- **Change:** Change "Live" feed header to "Demo Replay" and add disclaimer. Or populate from latest cycle.json.
- **Estimated time:** 0.5h
- **Demo point(s):** 5

### 9. Ensure demo.sh fails loudly on silent errors
- **Priority:** HIGH
- **File:** `demo.sh` throughout
- **Change:** Add fallbacks for missing night_shift.bin (read from cycle.json), label hardcoded test count, color-code Layer 3 skip.
- **Estimated time:** 1h
- **Demo point(s):** All — demo reliability

### 10. Add devnet cycle history summary to demo.sh
- **Priority:** LOW
- **File:** `demo.sh` ~line 95 (before Layer 2)
- **Change:** Count committed cycle directories and print summary.
- **Estimated time:** 0.5h
- **Demo point(s):** 2, 4

### 11. Add differentiation section to README
- **Priority:** MEDIUM
- **File:** `README.md` ~line 30
- **Change:** Add "Why RTP is Different" section based on Colosseum Copilot competitive analysis.
- **Estimated time:** 1h
- **Demo point(s):** Indirect — judge perception

### 12. Wire USDC balance query to dashboard (if feasible)
- **Priority:** LOW
- **File:** `dashboard/src/app/page.tsx` ~line 85
- **Change:** If Treasury PDA has a USDC token account, fetch its balance. Otherwise, remove the USDC number.
- **Estimated time:** 1-2h
- **Demo point(s):** 5

---

## RED FLAGS

### RF-1. `simulate_below_threshold_withdrawal()` is a hardcoded Err string, not an on-chain call
- **Severity:** CRITICAL
- **File:** `rtp/swarm/src/demo.rs` line 414-416
- **Fix:** Add transparency that it's a replay of a real on-chain event. Augment demo.sh to call the on-chain devnet-demo.ts when devnet is reachable.

### RF-2. Hardcoded "89.90 USDC Reserves" may contradict on-chain state
- **Severity:** CRITICAL
- **File:** `dashboard/src/app/page.tsx` line 92
- **Fix:** Replace with live SOL balance. Remove USDC number or label "(projected)".

### RF-3. "Connect Phantom" button with no onClick handler
- **Severity:** HIGH
- **File:** `dashboard/src/app/page.tsx` line 70
- **Fix:** Disable with "coming soon" label or implement minimal connect.

### RF-4. bg-flower.jpg exists (confirmed) but loaded without Next.js Image
- **Severity:** MEDIUM
- **File:** `dashboard/public/bg-flower.jpg`
- **Fix:** Low priority. Optionally use Next.js `<Image>` component.

### RF-5. Feed labeled "Live" but contains static timestamps
- **Severity:** HIGH
- **File:** `dashboard/src/app/page.tsx` line 17-30, line 113
- **Fix:** Change "Live" to "Demo Replay".

### RF-6. Paper trader state.json may not exist on fresh clone
- **Severity:** MEDIUM
- **File:** `demo.sh` line 77
- **Fix:** Point to committed data in data/night_results/ as fallback.

### RF-7. devnet-loop.yml memory_files reference /tmp/ paths that don't persist
- **Severity:** HIGH
- **File:** `data/devnet-cycles/latest/cycle.json` lines 19-32
- **Fix:** Change memory root to `data/swarm-memory/` and commit files.

### RF-8. demo.sh hardcoded fallback test count "238"
- **Severity:** MEDIUM
- **File:** `demo.sh` line 127
- **Fix:** Change fallback to honest "unknown (cargo test parse failed)".

### RF-9. No python-tests.yml despite CLAUDE.md referencing it
- **Severity:** MEDIUM
- **File:** `.github/workflows/`
- **Fix:** Verify Python tests run in existing workflow or remove reference.

### RF-10. Anchor program may be garbage-collected on devnet
- **Severity:** MEDIUM
- **File:** `rtp/programs/rtp-treasury/`
- **Fix:** Add program liveness check to demo.sh Layer 3.

---

## DASHBOARD RECOMMENDATION: A) UPGRADE (limited scope, ~2.5h)

| Order | Change | Time | Impact |
|-------|--------|------|--------|
| 1 | Remove "89.90 USDC Reserves" and "+12.1% Monthly Yield" — replace with live SOL balance in hero | 30min | Eliminates credibility risk |
| 2 | Change "Live" feed label to "Demo Replay" or populate from latest cycle.json | 30min | Honesty |
| 3 | Disable "Connect Phantom" button or label "coming soon" | 15min | Removes dead interaction |
| 4 | Change "298 Tests Passing" to read from a static build-time JSON | 30min | Accuracy |
| 5 | (Optional) Fetch USDC token account balance if one exists on PDA | 1h | Completes Point 5 |

Do NOT replace with static HTML — the live SOL balance fetch is the single most convincing "real on-chain data" element.

---

## COMPETITIVE DIFFERENTIATION (Colosseum Copilot analysis)

| RTP Feature | Gap Classification | Closest Prior Art | RTP Differentiation |
|---|---|---|---|
| Soulcontract constitutional governance | **Full gap** | `ai-powered-dao-strategy-assistant` (Cypherpunk) | Enforced in Rust (soulguard.rs) AND on-chain (Anchor require!) |
| Multi-wing swarm with Byzantine audit | **Full gap** | `eremos-2` (Cypherpunk) | 6 wings + 3-agent tribunal consensus |
| Self-funding treasury with phase evolution | **Full gap** | `firebird` (Radar) | Generates own yield, irreversible phase transitions |
| Walk-forward validated trading (30K configs) | **Partial gap** | `algoth.ai` (Radar) | 9-fold WFA + fee-aware simulation + live HL execution |
| On-chain constraint rejection proof | **Full gap** | No prior project shows deliberate TX failure as a feature | 10+ rejection tests in Anchor program |
| Persistent memory across cycles | **Partial gap** | `plonk` (Breakout) | Memory promotion ladder (currently /tmp — fixing) |
| Hyperliquid perps execution | **Full gap** | No prior Colosseum project integrates Hyperliquid | EIP-712 signing from Rust, USDC yield to Solana PDA |

**Framing:** "Prior hackathon projects built individual components — treasury managers (`firebird`), AI agents (`plonk`, `eremos-2`), yield aggregators (`stratos-defi-pod`), and backtesting tools (`algoth.ai`). RTP is the first to combine autonomous research + multi-wing constitutional governance + on-chain constraint enforcement + self-funding economics in a single integrated system with a deployed, tested Anchor program on devnet."
