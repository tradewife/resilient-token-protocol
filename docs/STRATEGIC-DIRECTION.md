# RTP Strategic Direction — Bespoke Treasury Infrastructure

> Canonical strategy document. If session context conflicts with this file,
> this file wins. Source: owner's strategic assessment (2026-08-07) +
> subsequent decisions. Last updated: 2026-08-08 (venue decision).

## The Pivot

RTP evolves from "token-longevity infrastructure for token projects" to
**bespoke treasury infrastructure**: a high-value solo service where each
client gets a **manufactured strategy** — engineered to their mandate,
deployed on self-custodied, on-chain-verifiable rails.

- **Survivor 2.69 is the specimen**, not the product. It proves the factory
  works. Mass-distributing it would crowd the trade and decay the alpha.
- **The pipeline is the product**: client defines a mandate (accumulation
  target, risk budget, drawdown tolerance, horizon) → research wing
  manufactures a strategy → validation gates qualify it → it deploys on
  self-custodied rails with on-chain enforcement.
- Bespoke is **anti-dilutive by construction**: each client gets a distinct
  strategy, no crowding, edges persist, pricing stays premium.

## Philosophy — Bespoke First, Scale Later

Deliberately rejecting early scaling. This is a HIGH-value service;
bespoke is the best model for the early stage of a solo operation.
Premature scaling (self-serve infra, docs, onboarding for a pattern seen
once) is the actual early-stage killer. The scaling question is only
earned after the model is **nailed and profitable** (exit criteria below).

Why bespoke is structurally right here:
- **Alpha physics**: strategies have capacity limits. Three clients with
  distinct mandates is sustainable; five hundred self-serve users on the
  same engine destroys the edge and then the product.
- **Solo operator math**: two or three clients at a serious ticket beats a
  thousand users at $99/month — no support treadmill, cash flow from day
  one, development funded on our own terms.
- **Learning density**: high-touch engagements force solving the real
  problem (mandate definition, custody, reporting, risk sign-off). That
  knowledge IS the product spec. Self-serve guesses; bespoke knows.
- **Precedent**: the Palantir motion — forward-deployed, bespoke per
  client, productize years later once the common denominator is
  undeniable. How most quant firms start: prop capital first, outside
  money second.

## The Compounding Discipline

Bespoke for the client, compounding for us. Every engagement must leave
behind a **hardened module**: mandate intake spec, validation gate suite,
execution wrapper, reporting dashboard. The trap is bespoke without
extraction — client #4 costing the same effort as client #1. Each
engagement must make the next one cheaper; scaling later becomes assembly,
not invention.

**Standardize the process, customize the product.** If strategies vary per
client, quality variance is brand risk — one client's engine blowing up is
our headline. The validation gates, drawdown limits, and auto-suspension
logic must be identical across every engagement.

## Sequencing (agreed 2026-08-07, revised 2026-08-08 after Flash wind-down)

1. ~~**Get Survivor 2.69 healthy again.**~~ **SUPERSEDED** — Flash Trade is
   winding down (announced Aug 7). Funds fully extracted Aug 8, trader
   halted (`RTP_TRADER_DRY_RUN=1`). The 2.69 rehab track lost its venue;
   the specimen now serves only as a pipeline demonstration.
2. **Venue re-selection — DECIDED (2026-08-08): GMTrade.** The v8 venue
   ranking (`research/missions/s15_v8_venue_ranking.md`) tested the
   identical 10-gate suite across measured cost bases: **GMTrade 10/10
   (+59.9% OOS, best of all venues), Hyperliquid-direct maker-entry 10/10
   (+49.6%), Jupiter fails (6/10), Phantom wallet UI fails (6/10)**.
   GMTrade confirmed SOL-collateral longs with profits paid in SOL — the
   exact accumulation mechanic RB's mandate requires. HL-direct is the
   validated fallback.
3. **Rebuild Survivor 2.69 on GMTrade for RB (client #1).** Owner decision
   2026-08-08: rebuild, don't just port. Sequence: (a) live-cost
   verification probe on GMTrade (on-chain fee parameter reads, SOL market
   OI/depth, small probe trades) — capital never deploys on docs-based
   costs alone; (b) GMTrade execution adapter; (c) re-run the gate suite
   on MEASURED live costs; (d) paper → live on RB's wallet.
4. **Website copy reframe — copy only, design untouched.** DONE in draft
   on `feat/mandate-diagnostic` (hero, §4, /diagnostic page, intake API).

## The Multi-Venue Doctrine (owner decision 2026-08-08)

GMTrade is the venue for **RB (client #1)** specifically — not a
platform-wide default. Future clients either **select/specify their own
venue** or we select one based on their specifics (mandate, collateral
asset, accumulation target, risk budget, chain preference).

This is deliberate, and it is a feature of the bespoke model, not overhead:
- It forces **continuous fee accounting across the full ecosystem** —
  every engagement refreshes our measured-cost basis on at least one venue.
  Fee schedules drift, venues add/remove mechanics (Flash v1→v2 proved
  this), and stale cost models are how edges silently die.
- The v8 gate suite is venue-agnostic by design: swap the cost model,
  re-run the gates. That loop is now an institutional capability, and AI
  tooling makes keeping current across venues cheap.
- The venue ranking doc is a living artifact — re-run it whenever a
  venue's measured costs or mechanics change, or a new venue enters scope.

Parallel de-risking: the RB engagement forces mandate definition,
custody setup, reporting, and the **venue-migration playbook** into
existence, while the GMTrade rebuild tests whether the pipeline handles
**regime change** (venue death + re-validation on new cost basis) — the
real capability an enterprise client is buying. Flash's wind-down turned
this from hypothetical into the first live exam.

## The Falsifiable Claim

"Bespoke edges manufactured on demand" is a claim until the pipeline
autonomously produces strategies beyond the hand-built one that clear the
same gates. Sample size must grow past n=1.

**Status (2026-08-08)**: S15 — the friend's engine — passed 10/10 gates
on 2-year data under measured Flash v2 fees
(`research/missions/s15_final_verdict.md`). Flash Trade announced
wind-down on Aug 7, 2026; the same config failed 4/10 under Jupiter's
measured fees, then the v8 venue ranking found it passes **10/10 on
GMTrade (docs-based costs) and 10/10 on Hyperliquid-direct**
(`research/missions/s15_v8_venue_ranking.md`). The claim survives *in
principle* and now has a selected venue, but the deployment bar remains:
the gate suite must pass on GMTrade's **MEASURED live costs** before RB
capital deploys. The measured-fee gate suite caught venue-incompatibility
before capital twice (Flash v1 model, then Jupiter) — that is the product
working.

## Exit Criteria for the Scaling Conversation

Only when ALL four are true do we earn the right to ask how to scale:
1. Three paying clients
2. Two pipeline-manufactured strategies live beyond Survivor
3. One repeatable onboarding runbook
4. Marginal effort per new client measurably falling

## Known Risks / Tensions

- **Regulatory surface**: bespoke treasury for individuals can read as
  unregistered investment advice (AU/US). Self-custody ≠ no burden if we
  manage strategies. Positioning leans toward *verifiable execution rails*
  (software/infrastructure) with the client retaining full custody and
  instant revocation. A crypto-savvy AU consult on "managed discretionary
  account" rules is on the pre-client checklist.
- **Quality variance**: mitigated by the standardized gate suite (above).
- **Revenue model**: setup fee + performance/AUM terms undecided —
  placeholder in `docs/mandate-intake-client1.md`, to be finalized with
  client #1.
- **Execution-venue dependency — NOW ACTIVE (Aug 2026)**: Flash Trade is
  winding down; the S15 config passes 10/10 on Flash's measured fees but
  only 4/10 on Jupiter's. Venue selection is now part of every mandate's
  engineering work (see Multi-Venue Doctrine above), and no engine is
  deployable without a gate pass on the chosen venue's measured cost
  basis. The measured-fee re-check process (`s15_v7` → `s15_v8` pattern)
  is the institutional answer and has been proven to catch
  venue-incompatibility before capital.
- **GMTrade operational risk (RB venue)**: docs-based fee numbers pending
  live verification; keeper-executed order fills must be probed (Flash
  lesson: UI and API paths can diverge); SOL-market depth and ADL
  thresholds unconfirmed; protocol age ~17 months vs Flash's wind-down
  precedent — venue health monitoring (TVL, volume, team activity) joins
  the per-client checklist.

## Artifacts

| Artifact | Path | Status |
|---|---|---|
| Client #1 intake template (RB) | `docs/mandate-intake-client1.md` | Draft — venue GMTrade, commercials TBD |
| Friend's engine (S15) verdict | `research/missions/s15_final_verdict.md` | DEPLOYABLE 10/10 (Flash fees) |
| Friend's engine machine config | `research/missions/s15_friend_engine_config.json` | Frozen pending leverage decision |
| v2 cost post-mortem | `research/missions/v2_cost_postmortem.py` | Edge survives v2 |
| Jupiter falsification | `research/missions/s15_v8_jupiter_verdict.md` | 4/10 — venue rejected |
| Venue ranking verdict | `research/missions/s15_v8_venue_ranking.md` | GMTrade #1, HL #2 |
| Live specimen | rtp-trader on Railway | HALTED (DRY_RUN) since Flash wind-down |
