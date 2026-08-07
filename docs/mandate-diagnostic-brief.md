# RTP Mandate Diagnostic — Product Brief

> Canonical product definition for the first Stripe-billable offer.
> Aligned with `docs/STRATEGIC-DIRECTION.md`. Prepared for Stripe Build
> Day (Aug 18, ICC Sydney). Last updated: 2026-08-08.

## What it is

A one-time, high-ticket research engagement. The client states a treasury
mandate; RTP's research pipeline manufactures a strategy against it, runs
it through the standardized validation gate suite on historical data under
**measured** venue fees, and delivers a paper-traded verdict package.

**Nothing live. No capital moves. No discretionary management.** The
client retains full custody and control at every stage.

## Positioning (regulatory-safe language)

> A structured research engagement that maps your stated treasury mandate
> against a manufactured strategy, validates it through a fixed gate suite
> on historical data, and returns a paper-traded verdict with full
> configuration. You retain full custody and control at every step. This
> is infrastructure and research output, not discretionary management or
> financial advice.

Use this framing everywhere (site, invoice, emails). The deliverable is
**research + configuration**, never returns management. AU regulatory
consult remains on the pre-live-engagement checklist (see STRATEGIC-DIRECTION.md).

## What the client receives

1. Structured mandate intake (capital band, risk budget, max drawdown,
   horizon, hard constraints, custody/liquidity preferences)
2. Manufactured strategy configuration produced by the research pipeline
3. Full run through the standardized gate suite (the same 10-gate battery
   used on the S15 client engine)
4. Paper-traded performance package under measured Flash Trade v2 fees
5. Written verdict — pass / conditional / fail — with supporting analysis
6. Machine-readable config + risk report (independently verifiable)
7. 45–60 minute debrief call

**Specimen on hand**: the S15 final verdict
(`research/missions/s15_final_verdict.md`) is a genuine example of the
exact deliverable shape, produced for a real mandate. Show it during the
debrief as proof of the factory — never as a promise of results.

## Pricing

- **A$4,500** one-time (Payment Link or Invoice; no subscription yet)
- Slots capped at **3–4 total**, and **none sold until the friend
  engagement's onboarding runbook exists** (standardize the process before
  selling it)
- Optional later: live deployment support as a separate commercial
  conversation, only after the paper verdict is accepted

## Process (bounded effort)

1. Stripe payment → access to intake form
2. Client completes mandate form (RTP-owned template)
3. Research wing manufactures + gates — **target turnaround 5–8 business
   days** (honest caveat: diagnostic #1 will run slower while the loop is
   being hardened; do not promise the turnaround until it has been
   measured once)
4. Deliver package + schedule debrief
5. Extract any new edge cases into the hardened modules (compounding
   discipline: diagnostic #4 must cost less effort than #1)

## Intake form sections (v1)

- **A. Capital & objectives**: size/band, primary objective, horizon,
  hard targets
- **B. Risk parameters**: max drawdown (hard), risk budget, absolute
  constraints (leverage, position size, excluded assets), unrealized vs
  realized loss tolerance
- **C. Operational & custody**: chains/venues, current custody setup,
  reporting requirements, communication cadence
- **D. Style & context** (optional): existing strategies/styles, regimes
  they must survive
- **E. Logistics**: delivery format, contact/timezone, deadlines

Template origin: `docs/mandate-intake-client1.md` (client-1 draft) — the
diagnostic intake is the generalized version of it.

## Stripe product description (paste-ready)

**Product name**: RTP Mandate Diagnostic
**Short**: Mandate Fit Diagnostic + Paper Engine

**Long**:

> A structured research engagement that maps your treasury mandate against
> a manufactured strategy and validates it through a fixed gate suite on
> historical data.
>
> You receive:
> - A complete paper-traded performance package under current measured
>   venue fees
> - Full strategy configuration
> - Clear pass / conditional / fail verdict with supporting analysis
> - 45–60 minute debrief
>
> You retain full self-custody and control at every stage. This is
> research and infrastructure output only — no capital is moved and no
> discretionary management is provided.
>
> Limited slots. Turnaround typically 5–8 business days after the mandate
> form is completed.

**Price**: A$4,500 one-time.

## Strategy alignment checklist

- [x] Bespoke, not mass distribution — one manufactured strategy per mandate
- [x] Pipeline is the product — the diagnostic IS a pipeline run
- [x] Hardened module extraction — intake form, gate-runner, verdict
      template become reusable
- [x] Verifiable execution rails positioning — custody and language safe
- [x] Friend remains client #1 (full engagement) — NOT a diagnostic slot
- [x] Volume capped — no alpha crowding, no support treadmill
- [x] Regulatory-safe copy throughout

## Relationship to other tracks

- **Survivor 2.69 rehab (Track 1)**: unchanged main technical track; the
  specimen must look healthy — diagnostics convert better with a live,
  credible specimen behind them
- **Friend engagement (Track 4)**: the diagnostic is the truncated,
  paper-only version of the same shape; the friend gets the full path
- **Website (Track 3)**: one product page, copy-only, design untouched —
  lands with this branch, not before
