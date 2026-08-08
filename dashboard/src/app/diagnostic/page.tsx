"use client";

import React, { useState } from "react";
import Link from "next/link";
import Topbar from "../Topbar";

/* ── Intake form model ── */

interface IntakeForm {
  // Contact
  name: string;
  email: string;
  // A. Capital & objectives
  capitalBand: string;
  objective: string;
  horizon: string;
  hardTarget: string;
  // B. Risk parameters
  maxDrawdown: string;
  riskBudget: string;
  constraints: string;
  lossTolerance: string;
  // C. Operational & custody
  venues: string;
  custody: string;
  reporting: string;
  cadence: string;
  // D. Style & context (optional)
  existingStyles: string;
  regimes: string;
  otherContext: string;
  // E. Logistics
  delivery: string;
  contact: string;
  deadline: string;
}

const EMPTY: IntakeForm = {
  name: "", email: "",
  capitalBand: "", objective: "", horizon: "", hardTarget: "",
  maxDrawdown: "", riskBudget: "", constraints: "", lossTolerance: "",
  venues: "", custody: "", reporting: "", cadence: "",
  existingStyles: "", regimes: "", otherContext: "",
  delivery: "", contact: "", deadline: "",
};

const DELIVERABLES = [
  { v: "01", l: "Mandate intake", d: "Structured intake: capital band, risk budget, drawdown limit, horizon, hard constraints." },
  { v: "02", l: "Manufactured config", d: "A strategy produced by the research pipeline against your mandate — not a shared template." },
  { v: "03", l: "10-gate validation", d: "The same battery every engine must clear: OOS PnL, consistency, attribution, sensitivity, latency, drawdown, zero liquidations." },
  { v: "04", l: "Measured fee basis", d: "Paper performance priced at real, current venue costs — measured from the execution venue, not assumed." },
  { v: "05", l: "Written verdict", d: "Pass / conditional / fail with supporting analysis, plus a machine-readable config you can verify independently." },
  { v: "06", l: "Debrief", d: "45–60 minutes walking through the verdict, the risk envelope, and what deployment would require." },
] as const;

const PROCESS = [
  { n: "01", t: "Reserve a slot", d: "One-time payment below. Limited to 3–4 engagements; your slot is held the moment payment clears." },
  { n: "02", t: "Submit your mandate", d: "The intake form below. Ten minutes. You keep a copy." },
  { n: "03", t: "The factory runs", d: "Research pipeline manufactures a strategy for your mandate and runs the full gate suite on two years of historical data." },
  { n: "04", t: "Paper verdict", d: "A complete paper-traded package under measured venue fees. Nothing live, nothing moves." },
  { n: "05", t: "Debrief", d: "You receive the verdict, the config, and the risk report — and decide what happens next. If nothing, the mandate simply ends." },
] as const;

// Sandbox Payment Link (test mode). Swap for the live link when the
// account moves to production — see .secrets/stripe-sandbox-resources.
const PAYMENT_LINK = process.env.NEXT_PUBLIC_RTP_DIAGNOSTIC_PAY_URL ||
  "https://buy.stripe.com/test_8x2cN6dajgfm0JK33S57W00";

export default function DiagnosticPage() {
  const [form, setForm] = useState<IntakeForm>(EMPTY);
  const [status, setStatus] = useState<"idle" | "submitting" | "done" | "error">("idle");

  const set = (k: keyof IntakeForm) => (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>) =>
    setForm((f) => ({ ...f, [k]: e.target.value }));

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!form.name.trim() || !form.email.trim()) return;
    setStatus("submitting");
    try {
      const res = await fetch("/api/diagnostic-intake/", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(form),
      });
      if (!res.ok) throw new Error("submit failed");
      setStatus("done");
    } catch {
      setStatus("error");
    }
  };

  return (
    <div className="page">
      <Topbar activePage="diagnostic" />

      {/* ════════ HERO ════════ */}
      <section className="sys2-section" style={{ marginTop: "var(--space-xl)" }}>
        <header className="sys2-sect-head">
          <div>
            <div className="sys2-sect-eyebrow">RTP MANDATE DIAGNOSTIC · PAPER ENGINE</div>
            <h1 className="hero-title" style={{ fontSize: "clamp(2rem, 4.5vw, 3.5rem)" }}>
              Your mandate, manufactured.
              <br />
              On paper first.
            </h1>
            <p className="sys2-sect-lede" style={{ marginTop: "var(--space-md)" }}>
              A structured research engagement that maps your treasury mandate against a
              manufactured strategy, validates it through a fixed gate suite on historical
              data, and returns a paper-traded verdict with the full configuration.
              You retain full custody and control at every stage. This is research and
              infrastructure output — no capital moves, no discretionary management.
            </p>
          </div>
          <div className="sys2-sect-side">
            <span className="sys2-status-pill watching">
              <span className="sys2-status-dot" />
              Limited slots · 3–4 engagements
            </span>
          </div>
        </header>

        <div style={{ marginTop: "var(--space-lg)", display: "flex", gap: "var(--space-md)", alignItems: "center", flexWrap: "wrap" }}>
          <a href={PAYMENT_LINK} target="_blank" rel="noopener noreferrer" className="sys2-cta-primary">
            Reserve a slot · A$4,500
          </a>
          <a href="#intake" className="sys2-cta-secondary">Already paid? Submit your mandate →</a>
        </div>

        <div className="validated-card" style={{ marginTop: "var(--space-xl)" }}>
          <div className="validated-head">
            <span className="validated-tag">ENGAGEMENT</span>
            <span className="validated-title">A$4,500 · one-time · paper only</span>
          </div>
          <div className="validated-grid">
            {[
              { v: "A$4,500", l: "One-time, all deliverables" },
              { v: "3–4", l: "Slots, strictly limited" },
              { v: "10", l: "Validation gates applied" },
              { v: "0", l: "Capital at risk — paper only" },
              { v: "100%", l: "Your custody, throughout" },
              { v: "1", l: "Distinct strategy per mandate" },
            ].map((m) => (
              <div key={m.l} className="validated-cell">
                <span className="validated-val">{m.v}</span>
                <span className="validated-lab">{m.l}</span>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* ════════ DELIVERABLES ════════ */}
      <section className="sys2-section" style={{ marginTop: "var(--space-3xl)" }}>
        <header className="sys2-sect-head">
          <div>
            <div className="sys2-sect-eyebrow">WHAT YOU RECEIVE</div>
            <h2 className="sys2-sect-title">Six deliverables. One verdict.</h2>
          </div>
        </header>
        <div className="arch2-layer-cells">
          {DELIVERABLES.map((d) => (
            <div key={d.v} className="arch2-cell" style={{ borderLeft: "2px solid var(--emerald-dim)" }}>
              <div className="arch2-cell-title">{d.v} · {d.l}</div>
              <div className="arch2-cell-sub">{d.d}</div>
            </div>
          ))}
        </div>
      </section>

      {/* ════════ PROCESS ════════ */}
      <section className="sys2-section" style={{ marginTop: "var(--space-4xl)" }}>
        <header className="sys2-sect-head">
          <div>
            <div className="sys2-sect-eyebrow">HOW IT RUNS</div>
            <h2 className="sys2-sect-title">The factory, run once, against your mandate.</h2>
            <p className="sys2-sect-lede">
              The same research pipeline that produced the live specimen on this dashboard —
              validated under measured venue fees, gated by the same battery, delivered as a
              paper verdict. The specimen is proof the factory works. Your mandate is the next run.
            </p>
          </div>
        </header>
        <ol className="pipe2-steps">
          {PROCESS.map((s, i) => (
            <li key={s.n} className="pipe2-step">
              <div className="pipe2-num">{s.n}</div>
              <div className="pipe2-body">
                <div className="pipe2-title">{s.t}</div>
                <div className="pipe2-desc">{s.d}</div>
              </div>
              {i < PROCESS.length - 1 && <div className="pipe2-tick">▾</div>}
            </li>
          ))}
        </ol>
      </section>

      {/* ════════ INTAKE FORM ════════ */}
      <section className="sys2-section" id="intake" style={{ marginTop: "var(--space-4xl)" }}>
        <header className="sys2-sect-head">
          <div>
            <div className="sys2-sect-eyebrow">MANDATE INTAKE</div>
            <h2 className="sys2-sect-title">State the mandate.</h2>
            <p className="sys2-sect-lede">
              Ten minutes. The more precise the risk parameters, the more meaningful the verdict.
              Submission reaches RTP directly — no third parties, nothing stored beyond this engagement.
            </p>
          </div>
        </header>

        {status === "done" ? (
          <div className="cta2-card">
            <div className="cta2-content" style={{ textAlign: "center" }}>
              <div className="sys2-sect-eyebrow">RECEIVED</div>
              <h2 className="cta2-title">Mandate received.</h2>
              <p className="cta2-lede">
                RTP will review the mandate and reply by email with scope confirmation and
                payment instructions. Nothing proceeds until you agree to terms in writing.
              </p>
            </div>
          </div>
        ) : (
          <form className="launch-form" onSubmit={submit} style={{ maxWidth: "860px" }}>
            <div className="form-group">
              <label className="form-label">A · Capital & Objectives</label>
              <div className="form-row">
                <div>
                  <label className="form-label" style={{ fontSize: "0.75rem" }}>Name *</label>
                  <input className="form-input" required value={form.name} onChange={set("name")} placeholder="Your name" />
                </div>
                <div>
                  <label className="form-label" style={{ fontSize: "0.75rem" }}>Email *</label>
                  <input className="form-input" type="email" required value={form.email} onChange={set("email")} placeholder="you@example.com" />
                </div>
              </div>
              <div className="form-row">
                <div>
                  <label className="form-label" style={{ fontSize: "0.75rem" }}>Approximate capital (band)</label>
                  <select className="form-input" value={form.capitalBand} onChange={set("capitalBand")}>
                    <option value="">Select…</option>
                    <option>Under 10 SOL</option>
                    <option>10–50 SOL</option>
                    <option>50–250 SOL</option>
                    <option>250–1,000 SOL</option>
                    <option>1,000+ SOL</option>
                    <option>Prefer to discuss</option>
                  </select>
                </div>
                <div>
                  <label className="form-label" style={{ fontSize: "0.75rem" }}>Primary objective</label>
                  <select className="form-input" value={form.objective} onChange={set("objective")}>
                    <option value="">Select…</option>
                    <option>Capital accumulation (grow the stack)</option>
                    <option>Absolute return</option>
                    <option>Income generation</option>
                    <option>Other</option>
                  </select>
                </div>
              </div>
              <div className="form-row">
                <div>
                  <label className="form-label" style={{ fontSize: "0.75rem" }}>Time horizon</label>
                  <select className="form-input" value={form.horizon} onChange={set("horizon")}>
                    <option value="">Select…</option>
                    <option>3–6 months</option>
                    <option>6–12 months</option>
                    <option>1–3 years</option>
                    <option>3+ years</option>
                  </select>
                </div>
                <div>
                  <label className="form-label" style={{ fontSize: "0.75rem" }}>Hard target (optional)</label>
                  <input className="form-input" value={form.hardTarget} onChange={set("hardTarget")} placeholder="e.g. +25% SOL terms" />
                </div>
              </div>
            </div>

            <div className="form-group">
              <label className="form-label">B · Risk Parameters</label>
              <div className="form-row">
                <div>
                  <label className="form-label" style={{ fontSize: "0.75rem" }}>Max drawdown (hard limit) *</label>
                  <select className="form-input" required value={form.maxDrawdown} onChange={set("maxDrawdown")}>
                    <option value="">Select…</option>
                    <option>5%</option>
                    <option>10%</option>
                    <option>15%</option>
                    <option>20%</option>
                    <option>25%</option>
                  </select>
                </div>
                <div>
                  <label className="form-label" style={{ fontSize: "0.75rem" }}>Loss tolerance</label>
                  <select className="form-input" value={form.lossTolerance} onChange={set("lossTolerance")}>
                    <option value="">Select…</option>
                    <option>Temporary unrealised losses acceptable</option>
                    <option>Prefer to realise losses quickly</option>
                    <option>Discuss</option>
                  </select>
                </div>
              </div>
              <div>
                <label className="form-label" style={{ fontSize: "0.75rem" }}>Risk budget description</label>
                <textarea className="form-input" rows={2} value={form.riskBudget} onChange={set("riskBudget")} placeholder="How much volatility can this capital absorb, and for how long?" />
              </div>
              <div>
                <label className="form-label" style={{ fontSize: "0.75rem" }}>Absolute constraints</label>
                <textarea className="form-input" rows={2} value={form.constraints} onChange={set("constraints")} placeholder="e.g. no leverage above 3x, max position size, excluded assets, liquidity requirements" />
              </div>
            </div>

            <div className="form-group">
              <label className="form-label">C · Operational & Custody</label>
              <div className="form-row">
                <div>
                  <label className="form-label" style={{ fontSize: "0.75rem" }}>Current custody setup</label>
                  <select className="form-input" value={form.custody} onChange={set("custody")}>
                    <option value="">Select…</option>
                    <option>Self-custody (hardware wallet)</option>
                    <option>Self-custody (software wallet)</option>
                    <option>Multisig</option>
                    <option>Exchange / custodian</option>
                    <option>Other</option>
                  </select>
                </div>
                <div>
                  <label className="form-label" style={{ fontSize: "0.75rem" }}>Preferred chains / venues</label>
                  <input className="form-input" value={form.venues} onChange={set("venues")} placeholder="e.g. Solana perps / GMTrade — or let us measure and choose" />
                </div>
              </div>
              <div className="form-row">
                <div>
                  <label className="form-label" style={{ fontSize: "0.75rem" }}>Reporting requirements</label>
                  <input className="form-input" value={form.reporting} onChange={set("reporting")} placeholder="e.g. weekly summary, on-chain proofs" />
                </div>
                <div>
                  <label className="form-label" style={{ fontSize: "0.75rem" }}>Communication cadence</label>
                  <select className="form-input" value={form.cadence} onChange={set("cadence")}>
                    <option value="">Select…</option>
                    <option>Async only (email)</option>
                    <option>Weekly check-in</option>
                    <option>Milestones only</option>
                  </select>
                </div>
              </div>
            </div>

            <div className="form-group">
              <label className="form-label">D · Style & Context (optional)</label>
              <div>
                <label className="form-label" style={{ fontSize: "0.75rem" }}>Existing strategies or styles</label>
                <textarea className="form-input" rows={2} value={form.existingStyles} onChange={set("existingStyles")} placeholder="Anything you already run, or have ruled out" />
              </div>
              <div className="form-row">
                <div>
                  <label className="form-label" style={{ fontSize: "0.75rem" }}>Regimes you must survive</label>
                  <input className="form-input" value={form.regimes} onChange={set("regimes")} placeholder="e.g. extended ranges, trend reversals" />
                </div>
                <div>
                  <label className="form-label" style={{ fontSize: "0.75rem" }}>Anything else</label>
                  <input className="form-input" value={form.otherContext} onChange={set("otherContext")} placeholder="Context the research wing should know" />
                </div>
              </div>
            </div>

            <div className="form-group">
              <label className="form-label">E · Logistics</label>
              <div className="form-row">
                <div>
                  <label className="form-label" style={{ fontSize: "0.75rem" }}>Preferred delivery format</label>
                  <select className="form-input" value={form.delivery} onChange={set("delivery")}>
                    <option value="">Select…</option>
                    <option>PDF report + config files</option>
                    <option>Notion / shared doc</option>
                    <option>Call walkthrough only</option>
                  </select>
                </div>
                <div>
                  <label className="form-label" style={{ fontSize: "0.75rem" }}>Hard deadline (optional)</label>
                  <input className="form-input" value={form.deadline} onChange={set("deadline")} placeholder="e.g. before end of quarter" />
                </div>
              </div>
              <div>
                <label className="form-label" style={{ fontSize: "0.75rem" }}>Best contact method & timezone</label>
                <input className="form-input" value={form.contact} onChange={set("contact")} placeholder="e.g. email, AEST" />
              </div>
            </div>

            <div style={{ marginTop: "var(--space-lg)", display: "flex", gap: "var(--space-md)", alignItems: "center", flexWrap: "wrap" }}>
              <button type="submit" className="sys2-cta-primary" disabled={status === "submitting"}>
                {status === "submitting" ? "Submitting…" : "Submit mandate"}
              </button>
              <span style={{ fontSize: "0.8125rem", color: "var(--text-tertiary)" }}>
                Research and infrastructure output only. No capital moves. No discretionary management.
              </span>
            </div>
            {status === "error" && (
              <div style={{ color: "var(--coral)", fontSize: "0.8125rem", marginTop: "var(--space-sm)" }}>
                Submission failed — email the mandate to hello@resilientprotocol.xyz instead.
              </div>
            )}
          </form>
        )}
      </section>

      {/* ════════ CLOSING ════════ */}
      <section className="sys2-section sys2-cta-section" style={{ marginTop: "var(--space-4xl)" }}>
        <div className="cta2-card">
          <div className="cta2-content">
            <div className="sys2-sect-eyebrow">THE SPECIMEN</div>
            <h2 className="cta2-title">The engine on this dashboard is proof the factory works.</h2>
            <p className="cta2-lede">
              Survivor 2.69 is the reference specimen — validated through the same gate suite,
              run on real capital since May 2026, every position verifiable on Solana Explorer.
              When its venue wound down, the pipeline re-validated it on the next venue&apos;s
              measured on-chain costs in a single session. That migration is the product working.
              Bespoke by construction: one strategy per mandate, no shared edges, nothing crowded.
            </p>
            <div className="cta2-actions">
              <a href="#intake" className="sys2-cta-primary">Submit a mandate</a>
              <Link href="/docs" className="sys2-cta-secondary">Read the docs →</Link>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
