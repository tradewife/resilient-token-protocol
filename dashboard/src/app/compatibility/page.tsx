"use client";

import React, { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import Topbar from "../Topbar";
import { formatPnlPct, summarizeTradePnl } from "../../lib/tradePnl";

type SpecimenTrade = {
  entry_price: number;
  exit_price: number;
  entry_time: number;
  exit_time: number;
  pnl_pct: number;
  size_usd?: number;
  side?: string;
};

type SpecimenState = {
  open_position: { side?: string; entry_price?: number; size_usd?: number } | null;
  trade_history: SpecimenTrade[];
  total_trades?: number;
};

/* ── Scorecard model (v5 Compatibility Check) ── */

type Step = "splash" | "questions" | "gate" | "results";

interface ScorecardForm {
  currentSituation: string;
  desiredOutcome: string;
  patienceMindset: string;
  expectedHorizon: string;
  solutionModel: string;
  name: string;
  email: string;
}

const EMPTY_SCORECARD: ScorecardForm = {
  currentSituation: "",
  desiredOutcome: "",
  patienceMindset: "",
  expectedHorizon: "",
  solutionModel: "",
  name: "",
  email: "",
};

const FIELD_KEYS: (keyof ScorecardForm)[] = [
  "currentSituation",
  "desiredOutcome",
  "patienceMindset",
  "expectedHorizon",
  "solutionModel",
];

const QUESTIONS = [
  {
    id: 1,
    title: "Where are your active crypto assets currently managed?",
    description:
      "Helps us assess exposure to centralized custody risk or unoptimized wallet setups.",
    options: [
      {
        value: "cex",
        label: "Centralized Exchanges (CEX)",
        desc: "Coinbase, Binance, Kraken, and similar venues.",
      },
      {
        value: "cold",
        label: "Cold Storage / Hardware Wallet",
        desc: "Ledger, Trezor, or keys held offline.",
      },
      {
        value: "hot_unsecured",
        label: "Active Web3 Hot Wallets",
        desc: "Phantom, Backpack, Solflare with manual on-chain activity.",
      },
      {
        value: "none",
        label: "New to Crypto Assets",
        desc: "No active holdings yet; establishing a secure start.",
      },
    ],
  },
  {
    id: 2,
    title: "What is your primary focus when deploying capital on-chain?",
    description:
      "Defines the guardrails and objective targets for a bespoke system.",
    options: [
      {
        value: "safety",
        label: "Absolute Capital Preservation",
        desc: "Accumulate with minimal drawdown exposure.",
      },
      {
        value: "custom",
        label: "Bespoke Automated Execution",
        desc: "Systematic strategies matched to personal risk parameters.",
      },
      {
        value: "yield",
        label: "Passive Yield & Liquidity",
        desc: "Lending, staking, or providing liquidity securely.",
      },
      {
        value: "education",
        label: "Systematic Learning",
        desc: "Venue mechanics, cost realities, and on-chain safety.",
      },
    ],
  },
  {
    id: 3,
    title: "How do you view systematic patience and market volatility?",
    description:
      "On-chain edge often means waiting for high-probability setups.",
    options: [
      {
        value: "flat_edge",
        label: 'Patience and sitting "Flat"',
        desc: "Preserve capital through noise; wait for structural alignment.",
      },
      {
        value: "active_trade",
        label: "High frequency / active trades",
        desc: "Accept higher costs and slippage for constant exposure.",
      },
      {
        value: "unsure",
        label: "No defined execution timeframe",
        desc: "Want guardrails that enforce patience programmatically.",
      },
    ],
  },
  {
    id: 4,
    title: "What horizon do you use to measure systematic success?",
    description:
      "Sophisticated systems are engineered for longevity, not short-term noise.",
    options: [
      {
        value: "long_term",
        label: "Multi-Year / Full Market Cycles",
        desc: "Capital survival and compounding over years.",
      },
      {
        value: "medium_term",
        label: "Quarterly to Semi-Annual",
        desc: "Structural out-of-sample metrics over 3–6 months.",
      },
      {
        value: "short_term",
        label: "Weekly to Monthly PnL",
        desc: "High-turnover targets and rapid feedback loops.",
      },
    ],
  },
  {
    id: 5,
    title: "Which development path fits your resources?",
    description:
      "We run 3–4 advisory builds at a time so edges stay uncrowded.",
    options: [
      {
        value: "advisory",
        label: "Bespoke Strategy Build · A$4,500",
        desc: "Structured intake, dedicated build, ten-gate validation, paper report, up to 4× implementation calls.",
      },
      {
        value: "developer",
        label: "Self-Serve Developer Docs",
        desc: "Open-source specifications; build on your own infrastructure.",
      },
    ],
  },
] as const;

/* ── Detailed paid intake (preserved secondary path) ── */

interface IntakeForm {
  name: string;
  email: string;
  capitalBand: string;
  objective: string;
  horizon: string;
  hardTarget: string;
  maxDrawdown: string;
  riskBudget: string;
  constraints: string;
  lossTolerance: string;
  venues: string;
  custody: string;
  reporting: string;
  cadence: string;
  existingStyles: string;
  regimes: string;
  otherContext: string;
  delivery: string;
  contact: string;
  deadline: string;
}

const EMPTY_INTAKE: IntakeForm = {
  name: "",
  email: "",
  capitalBand: "",
  objective: "",
  horizon: "",
  hardTarget: "",
  maxDrawdown: "",
  riskBudget: "",
  constraints: "",
  lossTolerance: "",
  venues: "",
  custody: "",
  reporting: "",
  cadence: "",
  existingStyles: "",
  regimes: "",
  otherContext: "",
  delivery: "",
  contact: "",
  deadline: "",
};

const TRADER_WALLET =
  process.env.NEXT_PUBLIC_RTP_TRADER_WALLET_PUBKEY ||
  "HDQ79fQ1YbL9CenS1DzfHizEWGrJdnmo99fgAWmdhuy5";

const PAYMENT_LINK =
  process.env.NEXT_PUBLIC_RTP_DIAGNOSTIC_PAY_URL ||
  "https://buy.stripe.com/8x2bIU7GH0AFbAQ6qVd7q00";

// Public Cal.com booking URL (30-min fit / kickoff). API keys stay server-side only.
const CAL_BOOK_URL =
  process.env.NEXT_PUBLIC_RTP_DIAGNOSTIC_CAL_URL ||
  "https://cal.com/kate-cooper/30min";

const EXPLORER_URL = `https://explorer.solana.com/address/${TRADER_WALLET}`;

function horizonLabel(h: string): string {
  if (h === "long_term") return "long-term market cycles";
  if (h === "medium_term") return "structural multi-month review windows";
  return "shorter feedback loops with tighter operational discipline";
}

/** Prefill Cal.com booker with scorecard / intake contact details. */
function calBookHref(name?: string, email?: string): string {
  try {
    const url = new URL(CAL_BOOK_URL);
    const n = (name || "").trim();
    const e = (email || "").trim();
    if (n) url.searchParams.set("name", n);
    if (e) url.searchParams.set("email", e);
    // Keep notes short — helps you spot Compatibility leads in the calendar.
    url.searchParams.set("notes", "RTP Compatibility Check · advisory path");
    return url.toString();
  } catch {
    return CAL_BOOK_URL;
  }
}

export default function DiagnosticPage() {
  const [step, setStep] = useState<Step>("splash");
  const [currentQ, setCurrentQ] = useState(1);
  const [scorecard, setScorecard] = useState<ScorecardForm>(EMPTY_SCORECARD);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);

  const [intake, setIntake] = useState<IntakeForm>(EMPTY_INTAKE);
  const [intakeStatus, setIntakeStatus] = useState<
    "idle" | "submitting" | "done" | "error"
  >("idle");
  const [specimen, setSpecimen] = useState<SpecimenState | null>(null);

  const totalQuestions = QUESTIONS.length;
  const isAdvisory = scorecard.solutionModel === "advisory";

  useEffect(() => {
    let alive = true;
    const load = async () => {
      try {
        const res = await fetch("/api/trader-status/", { cache: "no-store" });
        if (!res.ok) return;
        const data = (await res.json()) as SpecimenState;
        if (alive) setSpecimen(data);
      } catch {
        /* keep prior / empty specimen */
      }
    };
    load();
    const id = setInterval(load, 30_000);
    return () => {
      alive = false;
      clearInterval(id);
    };
  }, []);

  const specimenPnl = useMemo(
    () => summarizeTradePnl(specimen?.trade_history),
    [specimen]
  );
  const specimenOpen = Boolean(specimen?.open_position);
  const specimenStatusLabel = specimenOpen
    ? `${(specimen?.open_position?.side || "Long").toUpperCase()}`
    : "FLAT";
  const specimenStatusSub = specimenOpen
    ? "In position · managing exits"
    : "Waiting for multi-TF alignment";
  const specimenTitle = specimenOpen
    ? "SOL/USDT Survivor 2.69 · live position open"
    : "SOL/USDT Survivor 2.69 · waiting for alignment";

  const selectOption = (field: keyof ScorecardForm, value: string) => {
    setScorecard((prev) => ({ ...prev, [field]: value }));
    if (currentQ < totalQuestions) {
      setCurrentQ((q) => q + 1);
      if (typeof window !== "undefined") {
        window.scrollTo({ top: 0, behavior: "smooth" });
      }
    } else {
      setStep("gate");
      if (typeof window !== "undefined") {
        window.scrollTo({ top: 0, behavior: "smooth" });
      }
    }
  };

  const goBack = () => {
    if (step === "gate") {
      setStep("questions");
      setCurrentQ(totalQuestions);
      return;
    }
    if (currentQ > 1) {
      setCurrentQ((q) => q - 1);
    } else {
      setStep("splash");
    }
  };

  const submitScorecard = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!scorecard.name.trim() || !scorecard.email.trim()) return;
    setIsSubmitting(true);
    setSubmitError(null);
    try {
      const res = await fetch("/api/diagnostic-intake/", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          kind: "compatibility_v5",
          name: scorecard.name,
          email: scorecard.email,
          currentSituation: scorecard.currentSituation,
          desiredOutcome: scorecard.desiredOutcome,
          patienceMindset: scorecard.patienceMindset,
          expectedHorizon: scorecard.expectedHorizon,
          solutionModel: scorecard.solutionModel,
          objective: scorecard.desiredOutcome,
          horizon: scorecard.expectedHorizon,
          custody: scorecard.currentSituation,
        }),
      });
      if (!res.ok) throw new Error("submit failed");
      setIntake((prev) => ({
        ...prev,
        name: scorecard.name,
        email: scorecard.email,
        objective:
          scorecard.desiredOutcome === "safety"
            ? "Capital accumulation (grow the stack)"
            : scorecard.desiredOutcome === "custom"
              ? "Absolute return"
              : scorecard.desiredOutcome === "yield"
                ? "Income generation"
                : prev.objective,
        horizon:
          scorecard.expectedHorizon === "long_term"
            ? "3+ years"
            : scorecard.expectedHorizon === "medium_term"
              ? "6–12 months"
              : scorecard.expectedHorizon === "short_term"
                ? "3–6 months"
                : prev.horizon,
        custody:
          scorecard.currentSituation === "cold"
            ? "Self-custody (hardware wallet)"
            : scorecard.currentSituation === "hot_unsecured"
              ? "Self-custody (software wallet)"
              : scorecard.currentSituation === "cex"
                ? "Exchange / custodian"
                : prev.custody,
      }));
      setStep("results");
      if (typeof window !== "undefined") {
        window.scrollTo({ top: 0, behavior: "smooth" });
      }
    } catch {
      setSubmitError(
        "Submission failed. Email hello@resilientprotocol.xyz with your answers instead."
      );
    } finally {
      setIsSubmitting(false);
    }
  };

  const setIntakeField =
    (k: keyof IntakeForm) =>
    (
      e: React.ChangeEvent<
        HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement
      >
    ) =>
      setIntake((f) => ({ ...f, [k]: e.target.value }));

  const submitIntake = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!intake.name.trim() || !intake.email.trim()) return;
    setIntakeStatus("submitting");
    try {
      const res = await fetch("/api/diagnostic-intake/", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ kind: "mandate_intake", ...intake }),
      });
      if (!res.ok) throw new Error("submit failed");
      setIntakeStatus("done");
    } catch {
      setIntakeStatus("error");
    }
  };

  return (
    <div className="page">
      <Topbar activePage="compatibility" />

      {/* ════════ SPLASH ════════ */}
      {step === "splash" && (
        <section className="compat-shell compat-shell--wide">
          <header className="compat-hero">
            <div className="compat-hero-copy">
              <div className="sys2-sect-eyebrow">ON-CHAIN COMPATIBILITY CHECK</div>
              <h1 className="compat-title">
                Map your custody, horizon, and risk posture before anything
                touches capital.
              </h1>
              <p className="compat-lede">
                Five questions. Ninety seconds. A blueprint that forks you to a
                Bespoke Strategy Build or the open specs — no hype, no
                hand-holding, measured against how institutional on-chain
                execution actually runs.
              </p>
              <div className="compat-hero-meta">
                <span className="sys2-status-pill watching">
                  <span className="sys2-status-dot" />
                  Systematic onboarding · v5
                </span>
                <span className="compat-meta-note">5 questions · ~90 seconds</span>
              </div>
            </div>
          </header>

          <div className="compat-pillars">
            <div className="compat-pillar">
              <div className="compat-pillar-kicker">01</div>
              <div className="compat-pillar-title">Sovereign Custody</div>
              <div className="compat-pillar-body">
                Zero-custody setups via on-chain program vaults, with no private
                keys held by operators.
              </div>
            </div>
            <div className="compat-pillar">
              <div className="compat-pillar-kicker">02</div>
              <div className="compat-pillar-title">Execution Safety</div>
              <div className="compat-pillar-body">
                Guardrails against latency, venue failure, and fee-decay traps
                measured on live rails.
              </div>
            </div>
            <div className="compat-pillar">
              <div className="compat-pillar-kicker">03</div>
              <div className="compat-pillar-title">Bespoke Alignment</div>
              <div className="compat-pillar-body">
                Parameters built around your risk budget, not a shared template
                edge.
              </div>
            </div>
          </div>

          <div className="compat-actions">
            <button
              type="button"
              className="sys2-cta-primary"
              onClick={() => {
                setStep("questions");
                setCurrentQ(1);
              }}
            >
              Start Compatibility Check →
            </button>
            <span className="compat-meta-note">
              No account required until the blueprint.
            </span>
          </div>
        </section>
      )}

      {/* ════════ QUESTIONS ════════ */}
      {step === "questions" && (
        <section className="compat-shell compat-shell--stage">
          <div className="compat-stepbar">
            <button type="button" onClick={goBack} className="compat-back">
              ← Back
            </button>
            <div className="compat-step-meta">
              <span className="compat-step-count">
                Step {currentQ} of {totalQuestions}
              </span>
              <div className="compat-progress" aria-hidden>
                <div
                  className="compat-progress-fill"
                  style={{ width: `${(currentQ / totalQuestions) * 100}%` }}
                />
              </div>
            </div>
          </div>

          {QUESTIONS.map((q) => {
            if (q.id !== currentQ) return null;
            const fieldKey = FIELD_KEYS[q.id - 1];
            return (
              <div key={q.id} className="compat-q">
                <div className="sys2-sect-eyebrow">COMPATIBILITY · Q{q.id}</div>
                <h2 className="compat-q-title">{q.title}</h2>
                <p className="compat-q-desc">{q.description}</p>

                <div className="compat-options" role="listbox" aria-label={q.title}>
                  {q.options.map((opt) => {
                    const selected = scorecard[fieldKey] === opt.value;
                    return (
                      <button
                        key={opt.value}
                        type="button"
                        role="option"
                        aria-selected={selected}
                        onClick={() => selectOption(fieldKey, opt.value)}
                        className={`compat-option${selected ? " is-selected" : ""}`}
                      >
                        <span className="compat-option-radio" aria-hidden />
                        <span className="compat-option-copy">
                          <span className="compat-option-label">{opt.label}</span>
                          <span className="compat-option-desc">{opt.desc}</span>
                        </span>
                      </button>
                    );
                  })}
                </div>
              </div>
            );
          })}
        </section>
      )}

      {/* ════════ EMAIL GATE ════════ */}
      {step === "gate" && (
        <section className="compat-shell compat-shell--narrow compat-shell--stage">
          <div className="compat-stepbar">
            <button type="button" onClick={goBack} className="compat-back">
              ← Back
            </button>
            <span className="compat-step-count">Blueprint gate</span>
          </div>

          <div className="compat-gate-card">
            <div className="sys2-sect-eyebrow">PROFILE READY</div>
            <h2 className="compat-gate-title">Your compatibility profile is ready</h2>
            <p className="compat-gate-lede">
              Enter contact details to unlock your systematic setup
              recommendations. One blueprint, then silence unless you choose a
              path.
            </p>

            <form onSubmit={submitScorecard} className="compat-gate-form">
              <div className="form-group">
                <label className="form-label" htmlFor="compat-name">
                  Your name
                </label>
                <input
                  id="compat-name"
                  className="form-input"
                  required
                  value={scorecard.name}
                  onChange={(e) =>
                    setScorecard((p) => ({ ...p, name: e.target.value }))
                  }
                  placeholder="Enter your name"
                  autoComplete="name"
                />
              </div>
              <div className="form-group">
                <label className="form-label" htmlFor="compat-email">
                  Corporate or primary email
                </label>
                <input
                  id="compat-email"
                  className="form-input"
                  type="email"
                  required
                  value={scorecard.email}
                  onChange={(e) =>
                    setScorecard((p) => ({ ...p, email: e.target.value }))
                  }
                  placeholder="name@company.com"
                  autoComplete="email"
                />
              </div>

              <button
                type="submit"
                className="sys2-cta-primary compat-full-cta"
                disabled={isSubmitting}
              >
                {isSubmitting
                  ? "Generating blueprint…"
                  : "Generate Compatibility Blueprint →"}
              </button>

              {submitError && <div className="compat-error">{submitError}</div>}
            </form>
          </div>
        </section>
      )}

      {/* ════════ RESULTS ════════ */}
      {step === "results" && (
        <>
          <section className="compat-shell compat-shell--wide">
            <header className="compat-hero">
              <div className="compat-hero-copy">
                <div className="sys2-sect-eyebrow">DYNAMIC ANALYSIS VERDICT</div>
                <h1 className="compat-title compat-title--result">
                  {isAdvisory
                    ? "Sovereign Build Candidate"
                    : "Developer-Guided Setup"}
                </h1>
                <p className="compat-lede">
                  Based on your preferences, you need a system that values{" "}
                  {horizonLabel(scorecard.expectedHorizon)} with custody
                  architectures built on trustless-by-design principles.
                </p>
              </div>
              <div className="compat-hero-side">
                <span className="sys2-status-pill watching">
                  <span className="sys2-status-dot" />
                  Cohort · 3/4 slots booked
                </span>
              </div>
            </header>

            <div className="compat-results-grid">
              <div className="compat-results-main">
                <div className="compat-specimen">
                  <div className="compat-specimen-head">
                    <span className="validated-tag">LIVE SPECIMEN</span>
                    <span className="compat-specimen-title">{specimenTitle}</span>
                  </div>
                  <div className="compat-metrics">
                    <div className="compat-metric">
                      <span className="compat-metric-val">{specimenStatusLabel}</span>
                      <span className="compat-metric-lab">{specimenStatusSub}</span>
                    </div>
                    <div className="compat-metric">
                      <span
                        className="compat-metric-val"
                        style={{
                          color:
                            specimenPnl.tradeCount === 0
                              ? undefined
                              : specimenPnl.totalNetPct >= 0
                                ? "var(--emerald)"
                                : "var(--coral)",
                        }}
                      >
                        {specimenPnl.tradeCount === 0
                          ? "—"
                          : formatPnlPct(specimenPnl.totalNetPct)}
                      </span>
                      <span className="compat-metric-lab">
                        Net closed PnL · measured fees
                      </span>
                    </div>
                    <div className="compat-metric">
                      <span className="compat-metric-val">
                        {specimenPnl.tradeCount || specimen?.total_trades || "—"}
                      </span>
                      <span className="compat-metric-lab">Closed trades</span>
                    </div>
                    <div className="compat-metric">
                      <span className="compat-metric-val">44.89</span>
                      <span className="compat-metric-lab">Calmar (research)</span>
                    </div>
                  </div>
                  <p className="compat-specimen-note">
                    {specimenOpen
                      ? "Live mainnet specimen on GMTrade. Closed-trade PnL is net of measured venue round-trip fees (0.022%); open mark-to-market is managed by the exit stack, not forced activity."
                      : "Flat is a feature. The system sits out noise until multiple clean timeframes align: capital preservation over forced activity. Closed-trade PnL is net of measured GMTrade fees."}
                  </p>
                </div>

                <div className="compat-insight">
                  <div className="compat-insight-title">
                    Sovereign custody (program-derived addresses)
                  </div>
                  <div className="compat-insight-body">
                    No human holds private keys to the treasury. The program is
                    the sole authority; your wallet retains an on-chain
                    kill-switch to freeze trading instantly.
                  </div>
                </div>
                <div className="compat-insight">
                  <div className="compat-insight-title">
                    Out-of-sample walk-forward validation
                  </div>
                  <div className="compat-insight-body">
                    Engines clear walk-forward folds on isolated history before
                    touching live Solana capital. Protection against overfitted
                    backtests, not a marketing chart.
                  </div>
                </div>
              </div>

              <aside className="compat-fork">
                {isAdvisory ? (
                  <>
                    <div className="sys2-sect-eyebrow">BESPOKE STRATEGY BUILD</div>
                    <h2 className="compat-fork-title">
                      Reserve your Bespoke Strategy Build
                    </h2>
                    <p className="compat-fork-lede">
                      Your check qualifies you for a limited cohort slot. Pay once,
                      lay out your terms, and we map them to a paper-validated strategy —
                      with up to four 1-on-1 implementation calls included.
                    </p>
                    <ul className="compat-checks">
                      <li>Bespoke Strategy Build: A$4,500, one-time</li>
                      <li>Paper report at measured venue fees + full config</li>
                      <li>Ten-gate verification on historical data</li>
                      <li>Up to 4× 45–60 min implementation consultations</li>
                      <li>Your custody throughout · no capital moves</li>
                      <li>Initial report 5–8 business days after intake</li>
                    </ul>
                    <div className="compat-fork-actions">
                      <a
                        href={PAYMENT_LINK}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="sys2-cta-primary"
                      >
                        Reserve slot · A$4,500
                      </a>
                      <a
                        href={calBookHref(scorecard.name, scorecard.email)}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="sys2-cta-secondary"
                      >
                        Book a 30-min fit call
                      </a>
                      <a href="#intake" className="sys2-cta-secondary">
                        Already paid? Lay out your terms →
                      </a>
                    </div>
                  </>
                ) : (
                  <>
                    <div className="sys2-sect-eyebrow">DEVELOPER PATH</div>
                    <h2 className="compat-fork-title">
                      Access open-source specifications
                    </h2>
                    <p className="compat-fork-lede">
                      Self-directed configuration starts with the core docs and
                      the live wallet ledger on Solana Explorer.
                    </p>
                    <div className="compat-fork-actions">
                      <Link href="/docs" className="sys2-cta-primary">
                        Technical documentation →
                      </Link>
                      <a
                        href={EXPLORER_URL}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="sys2-cta-secondary"
                      >
                        Solana Explorer wallet ledger ↗
                      </a>
                      <Link href="/" className="sys2-cta-secondary">
                        Main dashboard
                      </Link>
                    </div>
                    <p className="compat-fork-aside">
                      Prefer a manufactured build later?{" "}
                      <a
                        href={PAYMENT_LINK}
                        target="_blank"
                        rel="noopener noreferrer"
                      >
                        Bespoke Strategy Build slots remain open at A$4,500
                      </a>
                      .
                    </p>
                  </>
                )}

                <div className="compat-fork-foot">
                  <span>RTP-COMPAT-V5</span>
                  <button
                    type="button"
                    className="compat-retake"
                    onClick={() => {
                      setStep("splash");
                      setCurrentQ(1);
                      setScorecard(EMPTY_SCORECARD);
                      setSubmitError(null);
                    }}
                  >
                    Retake check
                  </button>
                </div>
              </aside>
            </div>
          </section>

          {isAdvisory && (
            <section className="compat-shell compat-shell--wide" id="intake">
              <header className="compat-hero">
                <div className="compat-hero-copy">
                  <div className="sys2-sect-eyebrow">MANDATE INTAKE</div>
                  <h2 className="compat-title compat-title--result">
                    Lay out your terms.
                  </h2>
                  <p className="compat-lede">
                    After payment clears, complete this form so the research wing
                    can engineer around your risk budget. Ten minutes. Precision
                    sharpens the verdict.
                  </p>
                </div>
              </header>

              {intakeStatus === "done" ? (
                <div className="compat-gate-card">
                  <div className="sys2-sect-eyebrow">RECEIVED</div>
                  <h2 className="compat-gate-title">Terms received.</h2>
                  <p className="compat-gate-lede">
                    RTP will review and reply by email with scope confirmation.
                    Initial report typically ships 5–8 business days after intake.
                    Nothing proceeds until you agree to terms in writing.
                  </p>
                  <div className="compat-fork-actions" style={{ marginTop: "var(--space-md)" }}>
                    <a
                      href={calBookHref(intake.name || scorecard.name, intake.email || scorecard.email)}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="sys2-cta-primary"
                    >
                      Book kickoff call · 30 min
                    </a>
                  </div>
                </div>
              ) : (
                <form className="launch-form compat-intake" onSubmit={submitIntake}>
                  <div className="form-group">
                    <label className="form-label">A · Capital & Objectives</label>
                    <div className="form-row">
                      <div>
                        <label className="form-label form-label--sm">Name *</label>
                        <input
                          className="form-input"
                          required
                          value={intake.name}
                          onChange={setIntakeField("name")}
                          placeholder="Your name"
                        />
                      </div>
                      <div>
                        <label className="form-label form-label--sm">Email *</label>
                        <input
                          className="form-input"
                          type="email"
                          required
                          value={intake.email}
                          onChange={setIntakeField("email")}
                          placeholder="you@example.com"
                        />
                      </div>
                    </div>
                    <div className="form-row">
                      <div>
                        <label className="form-label form-label--sm">
                          Approximate capital (band)
                        </label>
                        <select
                          className="form-input"
                          value={intake.capitalBand}
                          onChange={setIntakeField("capitalBand")}
                        >
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
                        <label className="form-label form-label--sm">
                          Primary objective
                        </label>
                        <select
                          className="form-input"
                          value={intake.objective}
                          onChange={setIntakeField("objective")}
                        >
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
                        <label className="form-label form-label--sm">
                          Time horizon
                        </label>
                        <select
                          className="form-input"
                          value={intake.horizon}
                          onChange={setIntakeField("horizon")}
                        >
                          <option value="">Select…</option>
                          <option>3–6 months</option>
                          <option>6–12 months</option>
                          <option>1–3 years</option>
                          <option>3+ years</option>
                        </select>
                      </div>
                      <div>
                        <label className="form-label form-label--sm">
                          Hard target (optional)
                        </label>
                        <input
                          className="form-input"
                          value={intake.hardTarget}
                          onChange={setIntakeField("hardTarget")}
                          placeholder="e.g. +25% SOL terms"
                        />
                      </div>
                    </div>
                  </div>

                  <div className="form-group">
                    <label className="form-label">B · Risk Parameters</label>
                    <div className="form-row">
                      <div>
                        <label className="form-label form-label--sm">
                          Max drawdown (hard limit) *
                        </label>
                        <select
                          className="form-input"
                          required
                          value={intake.maxDrawdown}
                          onChange={setIntakeField("maxDrawdown")}
                        >
                          <option value="">Select…</option>
                          <option>5%</option>
                          <option>10%</option>
                          <option>15%</option>
                          <option>20%</option>
                          <option>25%</option>
                        </select>
                      </div>
                      <div>
                        <label className="form-label form-label--sm">
                          Loss tolerance
                        </label>
                        <select
                          className="form-input"
                          value={intake.lossTolerance}
                          onChange={setIntakeField("lossTolerance")}
                        >
                          <option value="">Select…</option>
                          <option>Temporary unrealised losses acceptable</option>
                          <option>Prefer to realise losses quickly</option>
                          <option>Discuss</option>
                        </select>
                      </div>
                    </div>
                    <div>
                      <label className="form-label form-label--sm">
                        Risk budget description
                      </label>
                      <textarea
                        className="form-input"
                        rows={2}
                        value={intake.riskBudget}
                        onChange={setIntakeField("riskBudget")}
                        placeholder="How much volatility can this capital absorb, and for how long?"
                      />
                    </div>
                    <div>
                      <label className="form-label form-label--sm">
                        Absolute constraints
                      </label>
                      <textarea
                        className="form-input"
                        rows={2}
                        value={intake.constraints}
                        onChange={setIntakeField("constraints")}
                        placeholder="e.g. no leverage above 3x, max position size, excluded assets"
                      />
                    </div>
                  </div>

                  <div className="form-group">
                    <label className="form-label">C · Operational & Custody</label>
                    <div className="form-row">
                      <div>
                        <label className="form-label form-label--sm">
                          Current custody setup
                        </label>
                        <select
                          className="form-input"
                          value={intake.custody}
                          onChange={setIntakeField("custody")}
                        >
                          <option value="">Select…</option>
                          <option>Self-custody (hardware wallet)</option>
                          <option>Self-custody (software wallet)</option>
                          <option>Multisig</option>
                          <option>Exchange / custodian</option>
                          <option>Other</option>
                        </select>
                      </div>
                      <div>
                        <label className="form-label form-label--sm">
                          Preferred chains / venues
                        </label>
                        <input
                          className="form-input"
                          value={intake.venues}
                          onChange={setIntakeField("venues")}
                          placeholder="e.g. Solana perps / GMTrade, or let us measure and choose"
                        />
                      </div>
                    </div>
                    <div className="form-row">
                      <div>
                        <label className="form-label form-label--sm">
                          Reporting requirements
                        </label>
                        <input
                          className="form-input"
                          value={intake.reporting}
                          onChange={setIntakeField("reporting")}
                          placeholder="e.g. weekly summary, on-chain proofs"
                        />
                      </div>
                      <div>
                        <label className="form-label form-label--sm">
                          Communication cadence
                        </label>
                        <select
                          className="form-input"
                          value={intake.cadence}
                          onChange={setIntakeField("cadence")}
                        >
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
                      <label className="form-label form-label--sm">
                        Existing strategies or styles
                      </label>
                      <textarea
                        className="form-input"
                        rows={2}
                        value={intake.existingStyles}
                        onChange={setIntakeField("existingStyles")}
                        placeholder="Anything you already run, or have ruled out"
                      />
                    </div>
                    <div className="form-row">
                      <div>
                        <label className="form-label form-label--sm">
                          Regimes you must survive
                        </label>
                        <input
                          className="form-input"
                          value={intake.regimes}
                          onChange={setIntakeField("regimes")}
                          placeholder="e.g. extended ranges, trend reversals"
                        />
                      </div>
                      <div>
                        <label className="form-label form-label--sm">
                          Anything else
                        </label>
                        <input
                          className="form-input"
                          value={intake.otherContext}
                          onChange={setIntakeField("otherContext")}
                          placeholder="Context the research wing should know"
                        />
                      </div>
                    </div>
                  </div>

                  <div className="form-group">
                    <label className="form-label">E · Logistics</label>
                    <div className="form-row">
                      <div>
                        <label className="form-label form-label--sm">
                          Preferred delivery format
                        </label>
                        <select
                          className="form-input"
                          value={intake.delivery}
                          onChange={setIntakeField("delivery")}
                        >
                          <option value="">Select…</option>
                          <option>PDF report + config files</option>
                          <option>Notion / shared doc</option>
                          <option>Call walkthrough only</option>
                        </select>
                      </div>
                      <div>
                        <label className="form-label form-label--sm">
                          Hard deadline (optional)
                        </label>
                        <input
                          className="form-input"
                          value={intake.deadline}
                          onChange={setIntakeField("deadline")}
                          placeholder="e.g. before end of quarter"
                        />
                      </div>
                    </div>
                    <div>
                      <label className="form-label form-label--sm">
                        Best contact method & timezone
                      </label>
                      <input
                        className="form-input"
                        value={intake.contact}
                        onChange={setIntakeField("contact")}
                        placeholder="e.g. email, AEST"
                      />
                    </div>
                  </div>

                  <div className="compat-actions">
                    <button
                      type="submit"
                      className="sys2-cta-primary"
                      disabled={intakeStatus === "submitting"}
                    >
                      {intakeStatus === "submitting"
                        ? "Submitting…"
                        : "Submit your terms"}
                    </button>
                    <span className="compat-meta-note">
                      Research output only. No capital moves. No discretionary
                      management.
                    </span>
                  </div>
                  {intakeStatus === "error" && (
                    <div className="compat-error">
                      Submission failed. Email your terms to
                      hello@resilientprotocol.xyz instead.
                    </div>
                  )}
                </form>
              )}
            </section>
          )}
        </>
      )}
    </div>
  );
}
