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

## Sequencing (agreed 2026-08-07)

1. **Get Survivor 2.69 healthy again.** Lost momentum to the Flash Trade v2
   change. "Healthy" = positive cumulative SOL PnL is the bare minimum
   (layman definition); we must strive for **impressive**. Definition must
   be evidence-based (see Track 1c checkpoints), not vibes.
2. **Website copy reframe — copy only, design untouched.** Happens AFTER
   2.69 has clean post-v2 trades. Copy is reversible; strategy PnL is not.
3. **First client = a close friend.** Real mandate, forgiving feedback
   loop, case study without a formal sales process. Onboarding work can
   proceed in parallel with 2.69 rehab. This mandate IS the S15 lineage —
   it began in `perplexity-strat.md` (the marubozu idea).

Parallel de-risking: the friend engagement forces mandate definition,
custody setup, and reporting into existence, while the 2.69 rehab tests
whether the pipeline handles **regime change** (venue v1→v2) — the real
capability an enterprise client is buying.

## The Falsifiable Claim

"Bespoke edges manufactured on demand" is a claim until the pipeline
autonomously produces strategies beyond the hand-built one that clear the
same gates. Sample size must grow past n=1.

**Status (2026-08-07)**: S15 — the friend's engine — passed 10/10 gates
on 2-year data under measured Flash v2 fees
(`research/missions/s15_final_verdict.md`). That is pipeline-produced
strategy #2. The claim is now two-for-two in principle; live execution
verification is the remaining step (limit-at-zone fills on Flash).

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
- **Execution-venue dependency**: engine is Flash Trade v2-native; venue
  changes require re-validation (learned once already, Aug 2026). The
  research pipeline's measured-fee re-check process is the institutional
  answer.

## Artifacts

| Artifact | Path | Status |
|---|---|---|
| Client #1 intake template | `docs/mandate-intake-client1.md` | Draft — commercials TBD |
| Friend's engine (S15) verdict | `research/missions/s15_final_verdict.md` | DEPLOYABLE 10/10 |
| Friend's engine machine config | `research/missions/s15_friend_engine_config.json` | Frozen pending leverage decision |
| v2 cost post-mortem | `research/missions/v2_cost_postmortem.py` | Edge survives v2 |
| Live specimen | rtp-trader on Railway | Fee ledger live since Aug 7 |
