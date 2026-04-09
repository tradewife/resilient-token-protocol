# RTP Evaluator — Protocol Survival Objective

> The evaluator defines what "winning" means for the agent swarm.
> It is the single source of truth for memory promotion, heartbeat triggers,
> stagnation detection, and improvement claims.

---

## 1. Mission

**Maximize sustainable treasury growth while never violating survival constraints.**

The swarm does not exist to maximize yield at any cost. It exists to keep the
treasury alive, growing, and improving — in that order of priority.

---

## 2. Primary Metric — Treasury Survival Index (TSI)

A single scalar score, computable entirely from on-chain state + bridge outputs:

```
TSI = growth × safety × reliability

growth      = ln(vault_balance / min_runway_balance)    if vault_balance > min_runway
              0                                          otherwise

safety      = max(0, 1 - drawdown / MAX_DRAWDOWN)

reliability = consistency × confidence                  from bridge response
```

### Rationale

| Factor | What it measures | Why it matters |
|--------|-----------------|----------------|
| `growth` | Distance above the runway floor | Below runway = system cannot pay for itself. ln() compresses so a $200k vault isn't 4× "better" than $50k in a way that drowns safety. |
| `safety` | Principal preservation | Core Value #5: "yield generation must never risk the principal beyond defined risk budgets." Zero when drawdown hits the cap — the protocol is not "kind of safe." |
| `reliability` | Strategy robustness | A high-yield strategy that fails 60% of the time is worse than a moderate strategy that works 90% of the time. |

### Scoring characteristics

- **TSI = 0**: Survival threat. Vault at or below runway, or drawdown at max.
- **TSI < 1.0**: Subsistence. System is alive but not thriving.
- **TSI ≈ 1.0–2.0**: Healthy. Growing safely with reliable strategies.
- **TSI > 2.0**: Strong growth. Significant distance above runway, low drawdown, high consistency.

### Constants

```
MAX_DRAWDOWN = 0.20    (20% — hard cap from soulcontract "risk budget")
```

This is the same cap the Evolve Wing's rollback threshold (5% degradation)
is measured against. A strategy hitting 20% drawdown is not degraded — it is
failed.

---

## 3. Secondary Metrics

These are not composited into TSI but are tracked and dashboarded independently.

| Metric | Source | What it signals |
|--------|--------|----------------|
| **Treasury NAV** | `vault_balance` (on-chain) | Absolute treasury size. Phase transition trigger. |
| **Fee Accumulation Rate** | `Δ(total_fees_withdrawn) / Δt` | Is the adopting token actually generating fee revenue? |
| **Distribution Efficiency** | `total_distributed / total_fees_withdrawn` | What fraction of fees reach beneficiaries? Target: >90%. |
| **Self-Hydration Ratio** | `total_hydration / total_fees_withdrawn` | What fraction funds the swarm? Should stay below 10% (Phase-dependent). |
| **Price Floor Distance** | `vault_balance / min_runway_balance` | How far above the survival line? Below 1.0 = danger. |
| **LP Depth** | DEX liquidity for the adopting token | Can distributions be absorbed without price impact? (stretch) |
| **Strategy Yield (annualized)** | `bridge.yield_estimate` | Raw yield from the current strategy. |
| **Validation Breadth** | `bridge.folds_validated` | How many independent windows validated? Target: ≥9. |
| **Phase Progress** | `treasury.phase` | Sustenance → Ecosystem → Humanity. Irreversible. |

---

## 4. Hard Constraints

The evaluator must **never** permit these, regardless of TSI score:

1. **Drawdown exceeding 20%** — violate Core Value #5
2. **Vault balance below min_runway_balance after any action** — violate invariant #9
3. **Self-hydration above 50% of available excess** — system must deliver value to beneficiaries
4. **Execution without audit approval** — every action passes the 3-stage quality gate
5. **Phase reversal** — transitions are irreversible by soulcontract
6. **Soulcontract amendment without human signature** — Core Value #3

These are enforced by the Anchor program (Layer 1), Soulguard (Layer 2), and the
Audit Wing tribunal. The evaluator does not enforce them — it **checks** that
they are being enforced and flags violations as terminal states.

---

## 5. State Inputs

### On-chain (trusted, every evaluation cycle)

```rust
struct OnChainState {
    vault_balance: u64,            // treasury_vault.amount
    total_fees_withdrawn: u64,     // treasury.total_fees_withdrawn
    total_distributed_holders: u64,
    total_distributed_dev: u64,
    total_distributed_ecosystem: u64,
    total_hydration: u64,
    phase: Phase,                  // Sustenance | Ecosystem | Humanity
    min_runway_balance: u64,       // 90-day ops floor
}
```

Read via: `getConnection().getAccountInfo(treasury_pda)` → deserialize `Treasury` struct.

### Off-chain (bridge response, when strategy runs)

```rust
struct BridgeMetrics {
    yield_estimate: f64,           // annualized USDC yield
    confidence: f64,               // 0.0–1.0
    consistency: f64,              // 0.0–1.0 (fold consistency)
    folds_validated: u32,          // independent validation windows
    strategy: String,              // strategy identifier
    max_drawdown: f64,             // observed max drawdown
}
```

Read via: `bridge::call_bridge()` → `BridgeResponse`.

### Derived (computed per cycle)

```rust
struct DerivedMetrics {
    delta_fees: u64,               // fees since last evaluation
    growth_rate: f64,              // delta_fees / prev_total_fees
    distribution_efficiency: f64,  // distributed / withdrawn
    hydration_ratio: f64,          // hydration / withdrawn
    price_floor_distance: f64,     // vault / min_runway
}
```

---

## 6. Scoring Function

Implemented as `evaluator.rs` in the swarm runtime:

```rust
pub fn compute_tsi(
    vault_balance: u64,
    min_runway_balance: u64,
    drawdown: f64,
    consistency: f64,
    confidence: f64,
) -> f64 {
    const MAX_DRAWDOWN: f64 = 0.20;

    // Growth: ln(vault / runway_floor). Zero if at or below floor.
    let growth = if vault_balance > min_runway_balance && min_runway_balance > 0 {
        (vault_balance as f64 / min_runway_balance as f64).ln()
    } else {
        0.0
    };

    // Safety: 1 - (drawdown / cap). Clamped to [0, 1].
    let safety = (1.0 - drawdown / MAX_DRAWDOWN).clamp(0.0, 1.0);

    // Reliability: consistency × confidence.
    let reliability = consistency * confidence;

    growth * safety * reliability
}
```

### Degraded mode (bridge unavailable)

When the bridge binary is unreachable or returns errors, fall back to an
on-chain-only score:

```rust
pub fn compute_tsi_onchain(
    vault_balance: u64,
    min_runway_balance: u64,
    delta_fees: u64,
    prev_total_fees: u64,
) -> f64 {
    let growth = if vault_balance > min_runway_balance && min_runway_balance > 0 {
        (vault_balance as f64 / min_runway_balance as f64).ln()
    } else {
        0.0
    };

    // Use fee accumulation as a proxy for reliability.
    // Growing fees → system is functioning. Stagnant → degraded.
    let fee_momentum = if prev_total_fees > 0 {
        (delta_fees as f64 / prev_total_fees as f64).min(1.0)
    } else {
        0.0
    };

    // On-chain has no drawdown data, so assume safety = 1.0
    // (conservative: no penalty without evidence).
    growth * fee_momentum
}
```

This degraded score is always ≤ the full score and is flagged on the dashboard.

---

## 7. Trigger Conditions

An evaluation runs when any of these events occur:

| Trigger | Source | Evaluation scope |
|---------|--------|-----------------|
| **Heartbeat tick** | Coordinator lifecycle (every `check_interval`) | Full TSI + all secondaries |
| **Post-execution** | Trading Wing after ExecutePermit | Full TSI + bridge metrics |
| **Post-distribution** | After `check_redistribute` on-chain tx | On-chain TSI + distribution metrics |
| **Post-hydration** | After `hydrate_swarm` on-chain tx | On-chain TSI + runway metrics |
| **Phase transition** | After `evolve_phase` on-chain tx | Full TSI + phase progress |
| **Stagnation check** | Every N heartbeat ticks (see §8) | TSI trend + delta analysis |
| **Manual** | Dashboard or CLI trigger | Full evaluation |

### Minimum evaluation frequency

For the hackathon demo: every 30 seconds (heartbeat interval).
For production: every 5 minutes, with on-chain event-driven triggers.

---

## 8. Stagnation Definition

**The heartbeat redirect triggers when TSI has not improved over 3 consecutive
evaluation cycles.**

Detection logic:

```rust
pub fn is_stagnant(tsi_history: &[f64]) -> bool {
    if tsi_history.len() < 3 {
        return false; // Not enough data to judge.
    }
    let recent: Vec<f64> = tsi_history.iter().rev().take(3).cloned().collect();
    // Stagnant if no improvement across all 3 readings.
    // "Improvement" means strictly greater than the previous reading.
    recent[0] <= recent[1] && recent[1] <= recent[2]
}
```

When stagnant, the Coordinator triggers a **redirect heartbeat** (CORAL §3.3):
- The Knowledge Wing surfaces prior-cycle insights for the Trading Wing
- The Evolve Wing proposes parameter changes
- The swarm re-selects its computational focus rather than repeating failed patterns

### Dead vs. stagnant

- **Stagnant**: TSI flat or declining, but > 0. Swarm should adapt.
- **Degraded** (5% drop): Rollback trigger from Evolve Wing assessor.
- **Dead**: TSI = 0 for 2+ consecutive cycles. See §9.

---

## 9. Failure Definition

A **terminal bad state** is any of:

1. **TSI = 0 for 2+ consecutive cycles** — vault below runway, or drawdown at cap
2. **Hard constraint violation** (§4) — enforcement failure, not just a bad score
3. **All wings reporting Unhealthy** — total system failure
4. **Bridge unreachable for 6+ consecutive evaluation cycles** — the swarm cannot
   generate strategies and is operating blind

When a terminal state is detected:

1. Log the failure with full state snapshot
2. Halt new strategy proposals (defensive posture)
3. Alert via dashboard (critical priority)
4. Do NOT auto-recover without human acknowledgment (Core Value #3)

### Recovery after failure

A human operator must:
1. Diagnose the root cause
2. Fix the issue (fund the vault, adjust parameters, restart bridge)
3. Manually trigger a fresh evaluation
4. Confirm TSI > 0 before re-enabling autonomous operation

---

## 10. Fallback Behavior

When evaluator data is partially unavailable:

| Missing data | Fallback | Impact |
|-------------|----------|--------|
| Bridge response | Use `compute_tsi_onchain()` | No drawdown penalty, no reliability factor. Flagged as degraded. |
| On-chain vault data | Halt evaluation. Retry with backoff (1s, 2s, 4s, 8s, max 30s). | System cannot evaluate safety — do not guess. |
| Individual wing metrics | Exclude that wing from assessment. | Assessor scores other wings. |
| All data | Enter safe mode. No new proposals. Heartbeat only. | Defensive posture until data returns. |

### Safe mode rules

In safe mode (all data unavailable):
- No new strategy proposals
- No hydrations
- No distributions
- Heartbeat continues (to detect recovery)
- Dashboard shows "SAFE MODE — awaiting data" in red

---

## 11. Demo-Visible Metrics

For the 3-minute hackathon demo, these metrics render on the dashboard:

### Primary display

```
┌─────────────────────────────────────────┐
│  TREASURY SURVIVAL INDEX: 1.47  ▲ +0.12 │
│  (improving · 3 cycles positive trend)  │
└─────────────────────────────────────────┘
```

### Secondary panel

| Metric | Value | Trend |
|--------|-------|-------|
| Treasury NAV | $47,230 | ▲ +$2,100 |
| Fee Rate | $342/day | ▲ +$18 |
| Runway | 283 days | ▼ -4 days |
| Strategy | mr_rsi_bb | — |
| Yield (ann.) | 118.3% | — |
| Drawdown | 3.2% | ▼ -0.1% |
| Consistency | 78% | ▲ +2% |
| Phase | Sustenance | — |
| Distribution | 70/20/10 | — |

### Memory adaptation display (Demo Requirement #3 and #4)

```
┌──────────────────────────────────────────────────────┐
│  CYCLE MEMORY                                         │
│                                                       │
│  Prior best: mr_bb (TSI 1.21, consistency 0.71)      │
│  Current:    mr_rsi_bb (TSI 1.47, consistency 0.78)  │
│  Improvement: +21.5%                                  │
│  Knowledge: "RSI filter improves mean-reversion by    │
│              reducing false entries in low-vol"        │
│  Source: Cycle 12 assessment → promoted to project     │
└──────────────────────────────────────────────────────┘
```

### Stagnation event display (Demo Requirement #4)

When stagnation is detected, show:
```
⚠️  STAGNATION DETECTED: TSI flat for 3 cycles (1.21, 1.20, 1.19)
    → Redirect heartbeat dispatched
    → Knowledge Wing surfacing Cycle 8-11 insights
    → Evolve Wing proposing parameter adjustment
```

### Constraint rejection display (Demo Requirement #1)

When a hard constraint is violated:
```
🔴 CONSTRAINT REJECTED: withdraw_fees below threshold
    → TreasuryError::BelowThreshold
    → Vault: $8,200  Threshold: $10,000
    → Action blocked by Anchor program (Ring 1)
```

---

## 12. Stretch Goals (Post-Hackathon)

These are explicitly NOT in scope for the hackathon but define the evaluation
architecture's growth path:

| Stretch metric | What it measures | Why it's stretch |
|---------------|-----------------|-----------------|
| **Sharpe ratio** | Risk-adjusted yield | Requires reliable price feed oracle |
| **Adopting token price impact** | Does RTP improve the token's market? | Requires DEX price history oracle |
| **Multi-token TSI** | Aggregate score across all adopting tokens | Requires multiple initialized treasuries |
| **Gas efficiency score** | Compute units per evaluation cycle | Solana runtime metrics, not treasury data |
| **Adversarial robustness** | Performance under simulated attacks | Requires red-team agent (Futureproof Wing v2) |
| **Cross-cycle skill promotion** | Strategy knowledge surviving >1 session | Requires Prologue memory integration |

---

## Relationship to Existing Systems

```
┌─────────────────────────────────────────────────────────┐
│                    EVALUATOR                             │
│  compute_tsi() → single scalar + secondary metrics      │
│  is_stagnant() → heartbeat redirect trigger             │
│  is_terminal() → failure detection                      │
└────────┬────────────────┬────────────────┬──────────────┘
         │                │                │
    ┌────▼────┐     ┌─────▼─────┐    ┌────▼─────────┐
    │ Assessor │     │ Knowledge │    │ Dashboard    │
    │ (wings)  │     │  Wing     │    │ (demo viz)   │
    └─────────┘     └───────────┘    └──────────────┘
         │                │
    Wing-level      Memory promotion
    scores feed     confidence =
    into TSI        TSI_improvement
                    / baseline_TSI
```

The **Assessor** evaluates individual wings (already implemented in
`wings/evolve/assessor.rs`). The **Evaluator** evaluates the protocol as a
whole. They are separate layers:

- Assessor: "Is the Trading Wing performing well?" → per-wing score
- Evaluator: "Is the protocol surviving and improving?" → TSI

The Knowledge Wing uses TSI improvement as the confidence score for memory
promotion: if a strategy raised TSI, it's worth remembering.

---

## Implementation Checklist

- [ ] `rtp/swarm/src/evaluator.rs` — compute_tsi, compute_tsi_onchain, is_stagnant, is_terminal
- [ ] Wire evaluator into Coordinator heartbeat cycle
- [ ] Knowledge Wing: use TSI delta for memory promotion confidence
- [ ] Evolve Wing: use TSI trend for redirect trigger (replaces hardcoded heartbeat count)
- [ ] Dashboard: render TSI + secondary panel + stagnation events
- [ ] Demo script: show TSI improving, stagnation detected, constraint rejected

---

*This document resolves the CRITICAL BLOCKER identified in SESSION-CONTEXT.md §7.*
*Last updated: 2026-04-09 — evaluator definition session.*
