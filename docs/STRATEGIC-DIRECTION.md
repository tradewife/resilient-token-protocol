# RTP Strategic Direction — Bespoke Treasury Infrastructure

> Canonical strategy document. If session context conflicts with this file,
> this file wins. Source: owner's strategic assessment (2026-08-07) +
> subsequent decisions. Last updated: 2026-08-07.

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
2. **Venue re-selection + re-forge.** The client's core ask — post SOL as
   collateral and *accumulate more SOL* — defines the venue filter.
   Jupiter Perps measured costs falsify the current engine config
   (v8 verdict), so the immediate work is either (a) re-forge a family
   on Jupiter's cost basis (overnight search), or (b) verify GMTrade's
   reported 0.4–0.6 bps fees and SOL-collateral mechanics. Capital never
   deploys until the gate suite passes on the venue's MEASURED fees.
3. **Website copy reframe — copy only, design untouched.** DONE in draft
   on `feat/mandate-diagnostic` (hero, §4, /diagnostic page, intake API).
4. **First client = a close friend.** Real mandate, forgiving feedback
   loop, case study without a formal sales process. Paper-first — capital
   deployment waits on a cost-validated engine from step 2.

Parallel de-risking: the friend engagement forces mandate definition,
custody setup, and reporting into existence, while the venue re-forge
tests whether the pipeline handles **regime change** (venue death +
re-validation on new cost basis) — the real capability an enterprise
client is buying. Flash's wind-down turned this from hypothetical into
the first live exam.

## The Falsifiable Claim

"Bespoke edges manufactured on demand" is a claim until the pipeline
autonomously produces strategies beyond the hand-built one that clear the
same gates. Sample size must grow past n=1.

**Status (2026-08-08)**: S15 — the friend's engine — passed 10/10 gates
on 2-year data under measured Flash v2 fees
(`research/missions/s15_final_verdict.md`). But Flash Trade announced
wind-down on Aug 7, 2026, and the same config **fails 4/10 gates under
Jupiter Perps' measured fees** (`research/missions/s15_v8_jupiter_verdict.md`:
OOS −28.5%, short leg destroyed by USDC-collateral swap fees). The claim
survives *in principle* but currently has **zero cost-valid deployment
venues**. The measured-fee gate suite caught this before capital — that
is the product working. The next step is a re-forge on the chosen venue's
cost basis, not deployment of the existing config.

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
  engineering work, and no engine is deployable without a gate pass on
  the chosen venue's measured cost basis. The measured-fee re-check
  process (`s15_v7` → `s15_v8` pattern) is the institutional answer and
  has been proven to catch venue-incompatibility before capital.

## Artifacts

| Artifact | Path | Status |
|---|---|---|
| Client #1 intake template | `docs/mandate-intake-client1.md` | Draft — commercials TBD |
| Friend's engine (S15) verdict | `research/missions/s15_final_verdict.md` | DEPLOYABLE 10/10 |
| Friend's engine machine config | `research/missions/s15_friend_engine_config.json` | Frozen pending leverage decision |
| v2 cost post-mortem | `research/missions/v2_cost_postmortem.py` | Edge survives v2 |
| Live specimen | rtp-trader on Railway | Fee ledger live since Aug 7 |
