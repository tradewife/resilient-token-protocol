"use client";

import React, { useEffect, useMemo, useState } from "react";
import Topbar from "../Topbar";
import { formatPnlPct, summarizeTradePnl } from "../../lib/tradePnl";

/* ── Types ── */

type Step = "splash" | "questions" | "gate" | "profile" | "results";

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

interface BlueprintAnswers {
  q1_venues: string[];
  q2_account_size: string;
  q3_activity: string;
  q4_drawdown: string;
  q5_pain_points: string[];
  q6_risk_orientation: string;
  q7_custody_comfort: string;
  q8_custody_setup: string;
  q9_cadence: string;
  q10_goal: string;
  q11_do_not_do: string[];
  q12_commitment: string;
}

interface BlueprintProfile {
  onChainReadiness: number;
  riskTolerance: number;
  complexityAppetite: number;
  commitmentReadiness: number;
  onChainLabel: string;
  onChainExplanation: string;
  riskSummary: string;
  archetype: string;
  archetypeDescription: string;
  custodyStance: string;
  commitmentHint: string;
}

interface GateFields {
  name: string;
  email: string;
  telegram: string;
  source: string;
}

const EMPTY_ANSWERS: BlueprintAnswers = {
  q1_venues: [],
  q2_account_size: "",
  q3_activity: "",
  q4_drawdown: "",
  q5_pain_points: [],
  q6_risk_orientation: "",
  q7_custody_comfort: "",
  q8_custody_setup: "",
  q9_cadence: "",
  q10_goal: "",
  q11_do_not_do: [],
  q12_commitment: "",
};

const EMPTY_GATE: GateFields = {
  name: "",
  email: "",
  telegram: "",
  source: "",
};

/* ── Constants ── */

const PAYMENT_LINK =
  process.env.NEXT_PUBLIC_RTP_DIAGNOSTIC_PAY_URL ||
  "https://buy.stripe.com/8x2bIU7GH0AFbAQ6qVd7q00";

const CAL_BOOK_URL =
  process.env.NEXT_PUBLIC_RTP_DIAGNOSTIC_CAL_URL ||
  "https://cal.com/kate-cooper/30min";

const TRADER_WALLET =
  process.env.NEXT_PUBLIC_RTP_TRADER_WALLET_PUBKEY ||
  "HDQ79fQ1YbL9CenS1DzfHizEWGrJdnmo99fgAWmdhuy5";

const EXPLORER_URL = `https://explorer.solana.com/address/${TRADER_WALLET}`;

/* ── Question Definitions ── */

interface QuestionOption {
  value: string;
  label: string;
  desc?: string;
}

interface Question {
  id: string;
  block: number;
  blockTitle: string;
  title: string;
  description: string;
  type: "radio" | "multi";
  options: QuestionOption[];
  helper?: string;
}

const QUESTIONS: Question[] = [
  /* ── Block 1: Trading Context ── */
  {
    id: "q1_venues",
    block: 1,
    blockTitle: "Trading Context",
    title: "Where do you currently trade?",
    description: "Select all that apply.",
    type: "multi",
    options: [
      { value: "cex", label: "Centralised exchanges only", desc: "Binance, Bybit, etc." },
      { value: "trad_broker", label: "Traditional broker", desc: "IG, Saxo, etc." },
      { value: "onchain", label: "On-chain perps / DEXs", desc: "Hyperliquid, Jupiter, Drift, etc." },
      { value: "mix", label: "Mix of CEX and on-chain", desc: "" },
    ],
  },
  {
    id: "q2_account_size",
    block: 1,
    blockTitle: "Trading Context",
    title: "What's your typical account size per venue?",
    description: "Picks the relevant capital band for engine sizing.",
    type: "radio",
    options: [
      { value: "<10k", label: "Under A$10k", desc: "" },
      { value: "10k-50k", label: "A$10k – A$50k", desc: "" },
      { value: "50k-250k", label: "A$50k – A$250k", desc: "" },
      { value: ">250k", label: "Above A$250k", desc: "" },
    ],
  },
  {
    id: "q3_activity",
    block: 1,
    blockTitle: "Trading Context",
    title: "How often do you actively manage positions?",
    description: "Gauges the complexity your engine needs to handle.",
    type: "radio",
    options: [
      { value: "daily", label: "Daily", desc: "" },
      { value: "several_week", label: "Several times a week", desc: "" },
      { value: "weekly", label: "Weekly", desc: "" },
      { value: "passive", label: "Rarely / mostly passive", desc: "" },
    ],
  },

  /* ── Block 2: Risk Envelope & Scars ── */
  {
    id: "q4_drawdown",
    block: 2,
    blockTitle: "Risk Envelope & Scars",
    title: "What would you consider an unacceptable drawdown?",
    description: "The hard floor your engine must respect — in percentage terms.",
    type: "radio",
    options: [
      { value: "<10%", label: "Under 10%", desc: "" },
      { value: "10-20%", label: "10% – 20%", desc: "" },
      { value: "20-35%", label: "20% – 35%", desc: "" },
      { value: ">35%", label: "Above 35%", desc: "" },
    ],
  },
  {
    id: "q5_pain_points",
    block: 2,
    blockTitle: "Risk Envelope & Scars",
    title: "Which of these have hurt you (or people you know) before?",
    description: "Select all that apply — I'll route around those failure modes.",
    type: "multi",
    options: [
      { value: "liquidations", label: "Liquidations on leverage", desc: "" },
      { value: "choppy", label: "Choppy markets grinding small PnL", desc: "" },
      { value: "funding_bleed", label: "Funding bleed on perps", desc: "" },
      { value: "venue_outages", label: "Venue outages / liquidity disappearing", desc: "" },
      { value: "scam_coins", label: "Illiquid / scam coins rugging", desc: "" },
      { value: "other", label: "Other (free text below)", desc: "" },
    ],
  },
  {
    id: "q6_risk_orientation",
    block: 2,
    blockTitle: "Risk Envelope & Scars",
    title: "On balance, what matters more to you?",
    description: "Shapes the engine's posture — defensive, aggressive, or balanced.",
    type: "radio",
    options: [
      { value: "avoid_downswings", label: "Avoiding big downswings", desc: "" },
      { value: "maximize_growth", label: "Maximising long-run growth", desc: "" },
      { value: "balanced", label: "Balanced", desc: "" },
    ],
  },

  /* ── Block 3: Custody & Operational ── */
  {
    id: "q7_custody_comfort",
    block: 3,
    blockTitle: "Custody & Operational",
    title: "How comfortable are you with self-custody?",
    description: "Wallets, seed phrases, hardware devices.",
    type: "radio",
    options: [
      { value: "not_at_all", label: "Not at all", desc: "" },
      { value: "somewhat", label: "Somewhat", desc: "" },
      { value: "comfortable", label: "Comfortable", desc: "" },
      { value: "very_comfortable", label: "Very comfortable / already using hardware wallets", desc: "" },
    ],
  },
  {
    id: "q8_custody_setup",
    block: 3,
    blockTitle: "Custody & Operational",
    title: "For this engine, how would you like funds to be held?",
    description: "Determines the custody architecture of your setup.",
    type: "radio",
    options: [
      { value: "own_wallet", label: "Only in my own wallet(s)", desc: "" },
      { value: "program_vaults", label: "On-chain program vaults with no operator keys", desc: "Zero-custody setup." },
      { value: "mix", label: "Mix, depending on strategy", desc: "" },
      { value: "not_sure", label: "Not sure yet — I'd like guidance", desc: "" },
    ],
  },
  {
    id: "q9_cadence",
    block: 3,
    blockTitle: "Custody & Operational",
    title: "How often would you like formal updates on the engine?",
    description: "Sets the default reporting rhythm.",
    type: "radio",
    options: [
      { value: "weekly", label: "Weekly recap", desc: "" },
      { value: "major_changes", label: "Only on major changes / events", desc: "" },
      { value: "monthly", label: "Monthly summary", desc: "" },
      { value: "live_dashboard", label: "I'd rather see a live dashboard", desc: "" },
    ],
  },

  /* ── Block 4: Intent & Constraints ── */
  {
    id: "q10_goal",
    block: 4,
    blockTitle: "Intent & Constraints",
    title: "What's the primary job you want this engine to do?",
    description: "Picks the engine archetype.",
    type: "radio",
    options: [
      { value: "compounding", label: "Grow my capital steadily over years", desc: "Compounding." },
      { value: "income", label: "Generate more regular cashflow / income-style PnL", desc: "" },
      { value: "directional", label: "Take higher-octane directional bets with strict risk caps", desc: "" },
      { value: "hedge", label: "Hedge an existing treasury / core holdings", desc: "" },
    ],
  },
  {
    id: "q11_do_not_do",
    block: 4,
    blockTitle: "Intent & Constraints",
    title: "What must this engine never do?",
    description: "Select all that apply — these become hard constraints.",
    type: "multi",
    options: [
      { value: "leverage_above", label: "Use leverage above a specific band", desc: "I'll ask for the band later." },
      { value: "illiquid", label: "Hold illiquid microcaps", desc: "" },
      { value: "overnight", label: "Run overnight positions", desc: "" },
      { value: "outside_solana", label: "Trade outside Solana ecosystem", desc: "" },
      { value: "other", label: "Other (free text below)", desc: "" },
    ],
  },
  {
    id: "q12_commitment",
    block: 4,
    blockTitle: "Intent & Constraints",
    title: "If the profile you see next looks aligned, how ready are you?",
    description: "No pressure — just helps me calibrate the next step.",
    type: "radio",
    options: [
      { value: "ready", label: "Ready to commit now", desc: "" },
      { value: "probably", label: "Probably — I just need minor clarification", desc: "" },
      { value: "not_yet", label: "Not yet — I need more context", desc: "" },
    ],
  },
];

const TOTAL_QUESTIONS = QUESTIONS.length;

/* ── Helpers ── */

function toggleMulti(arr: string[], val: string): string[] {
  return arr.includes(val) ? arr.filter((v) => v !== val) : [...arr, val];
}

function calBookHref(name?: string, email?: string): string {
  try {
    const url = new URL(CAL_BOOK_URL);
    const n = (name || "").trim();
    const e = (email || "").trim();
    if (n) url.searchParams.set("name", n);
    if (e) url.searchParams.set("email", e);
    url.searchParams.set("notes", "Resilience Blueprint · follow-up");
    return url.toString();
  } catch {
    return CAL_BOOK_URL;
  }
}

/* ── Main Page ── */

export default function BlueprintPage() {
  const [step, setStep] = useState<Step>("splash");
  const [questionIndex, setQuestionIndex] = useState(0);
  const [answers, setAnswers] = useState<BlueprintAnswers>(EMPTY_ANSWERS);
  const [gate, setGate] = useState<GateFields>(EMPTY_GATE);
  const [profile, setProfile] = useState<BlueprintProfile | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [askQuestion, setAskQuestion] = useState("");
  const [askQuestionSent, setAskQuestionSent] = useState(false);

  const [specimen, setSpecimen] = useState<SpecimenState | null>(null);

  const currentQ = QUESTIONS[questionIndex];
  const currentBlock = currentQ?.block;
  const blockTitle = currentQ?.blockTitle;

  // ── Specimen polling ──
  useEffect(() => {
    let alive = true;
    const load = async () => {
      try {
        const res = await fetch("/api/trader-status/", { cache: "no-store" });
        if (!res.ok) return;
        const data = (await res.json()) as SpecimenState;
        if (alive) setSpecimen(data);
      } catch {
        /* keep prior */
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

  // ── Navigation ──
  const goToQuestion = (idx: number) => {
    setQuestionIndex(idx);
    setStep("questions");
    if (typeof window !== "undefined") window.scrollTo({ top: 0, behavior: "smooth" });
  };

  const advanceQuestion = () => {
    if (questionIndex < TOTAL_QUESTIONS - 1) {
      setQuestionIndex((q) => q + 1);
      if (typeof window !== "undefined") window.scrollTo({ top: 0, behavior: "smooth" });
    } else {
      setStep("gate");
      if (typeof window !== "undefined") window.scrollTo({ top: 0, behavior: "smooth" });
    }
  };

  const goBack = () => {
    if (step === "gate") {
      setStep("questions");
      setQuestionIndex(TOTAL_QUESTIONS - 1);
      return;
    }
    if (step === "profile" || step === "results") {
      setStep("gate");
      return;
    }
    if (questionIndex > 0) {
      setQuestionIndex((q) => q - 1);
    } else {
      setStep("splash");
    }
  };

  // ── Answer handling ──
  const setRadio = (field: keyof BlueprintAnswers, value: string) => {
    setAnswers((prev) => ({ ...prev, [field]: value }));
    // Auto-advance on radio selection
    setTimeout(() => advanceQuestion(), 180);
  };

  const toggleMultiAnswer = (field: keyof BlueprintAnswers, value: string) => {
    setAnswers((prev) => {
      const current = prev[field] as string[];
      return { ...prev, [field]: toggleMulti(current, value) };
    });
  };

  const hasMultiSelection = (field: keyof BlueprintAnswers): boolean => {
    const arr = answers[field] as string[];
    return Array.isArray(arr) && arr.length > 0;
  };

  // ── Submit ──
  const submitBlueprint = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!gate.name.trim() || !gate.email.trim()) return;
    setIsSubmitting(true);
    setSubmitError(null);
    try {
      const res = await fetch("/api/diagnostic-intake/", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          kind: "blueprint_v1",
          name: gate.name,
          email: gate.email,
          telegram: gate.telegram,
          source: gate.source,
          answers,
        }),
      });
      if (!res.ok) {
        const err = await res.json().catch(() => ({ error: "submission failed" }));
        throw new Error(err.error || "submission failed");
      }
      const data = await res.json();
      setProfile(data.profile);
      setStep("profile");
      if (typeof window !== "undefined") window.scrollTo({ top: 0, behavior: "smooth" });
    } catch (err) {
      setSubmitError(
        err instanceof Error ? err.message : "Submission failed. Email hello@resilientprotocol.xyz instead."
      );
    } finally {
      setIsSubmitting(false);
    }
  };

  // ── Ask question handler ──
  const sendQuestion = async () => {
    if (!askQuestion.trim()) return;
    try {
      await fetch("/api/diagnostic-intake/", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          kind: "blueprint_v1",
          name: gate.name || "Blueprint visitor",
          email: gate.email || "no-reply@blueprint",
          telegram: gate.telegram,
          source: `pre-commit question: ${askQuestion}`,
          answers,
        }),
      });
      setAskQuestionSent(true);
    } catch {
      /* silently ignore — the question field is ancillary */
      setAskQuestionSent(true);
    }
  };

  /* ── RENDER ── */

  return (
    <div className="page">
      <Topbar activePage="compatibility" />

      {/* ════════ SPLASH ════════ */}
      {step === "splash" && (
        <section className="compat-shell compat-shell--wide">
          <header className="compat-hero">
            <div className="compat-hero-copy">
              <div className="sys2-sect-eyebrow">RESILIENCE BLUEPRINT</div>
              <h1 className="compat-title">
                Map your custody, risk envelope, and engine archetype — before
                anything touches capital
              </h1>
              <p className="compat-lede">
                A 12-question diagnostic that generates a personalised profile:
                on-chain readiness, suggested engine archetype, custody stance,
                and hard constraints. No templates, no shared edges — just an
                engineering brief built around your risk budget.
              </p>
              <div className="compat-hero-meta">
                <span className="sys2-status-pill watching">
                  <span className="sys2-status-dot" />
                  Systematic onboarding
                </span>
                <span className="compat-meta-note">12 questions · ~3 minutes</span>
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
                setQuestionIndex(0);
              }}
            >
              Start your Resilience Blueprint →
            </button>
            <span className="compat-meta-note">
              No account required. Your answers build a personalised profile.
            </span>
          </div>
        </section>
      )}

      {/* ════════ QUESTIONS ════════ */}
      {step === "questions" && currentQ && (
        <section className="compat-shell compat-shell--stage">
          {/* Progress bar */}
          <div className="compat-stepbar">
            <button type="button" onClick={goBack} className="compat-back">
              ← Back
            </button>
            <div className="compat-step-meta">
              <span className="compat-step-count">
                Block {currentBlock} of 4 · Question {questionIndex + 1} of {TOTAL_QUESTIONS}
              </span>
              <div className="compat-progress" aria-hidden>
                <div
                  className="compat-progress-fill"
                  style={{ width: `${((questionIndex + 1) / TOTAL_QUESTIONS) * 100}%` }}
                />
              </div>
            </div>
          </div>

          {/* Block label */}
          <div className="sys2-sect-eyebrow" style={{ marginBottom: "var(--space-md)" }}>
            {blockTitle}
          </div>

          {/* Question */}
          <div className="compat-q">
            <h2 className="compat-q-title">{currentQ.title}</h2>
            <p className="compat-q-desc">{currentQ.description}</p>

            {currentQ.helper && (
              <p className="blueprint-helper">{currentQ.helper}</p>
            )}

            <div className="compat-options" role={currentQ.type === "multi" ? "group" : "listbox"} aria-label={currentQ.title}>
              {currentQ.options.map((opt) => {
                const isRadio = currentQ.type === "radio";
                const aa = answers as unknown as Record<string, unknown>;
                const selected = isRadio
                  ? aa[currentQ.id] === opt.value
                  : (aa[currentQ.id] as string[]).includes(opt.value);

                return (
                  <button
                    key={opt.value}
                    type="button"
                    role={isRadio ? "option" : "checkbox"}
                    aria-selected={isRadio ? selected : undefined}
                    aria-checked={isRadio ? undefined : selected}
                    onClick={() => {
                      if (isRadio) {
                        setRadio(currentQ.id as keyof BlueprintAnswers, opt.value);
                      } else {
                        toggleMultiAnswer(currentQ.id as keyof BlueprintAnswers, opt.value);
                      }
                    }}
                    className={`compat-option${selected ? " is-selected" : ""}`}
                  >
                    <span className={`compat-option-radio${currentQ.type === "multi" ? " blueprint-multi-check" : ""}`} aria-hidden />
                    <span className="compat-option-copy">
                      <span className="compat-option-label">{opt.label}</span>
                      {opt.desc && <span className="compat-option-desc">{opt.desc}</span>}
                    </span>
                  </button>
                );
              })}
            </div>
          </div>

          {/* Multi-select continue button */}
          {currentQ.type === "multi" && (
            <div className="compat-actions" style={{ marginTop: "var(--space-lg)" }}>
              <button
                type="button"
                className="sys2-cta-primary"
                disabled={!hasMultiSelection(currentQ.id as keyof BlueprintAnswers)}
                onClick={advanceQuestion}
              >
                {hasMultiSelection(currentQ.id as keyof BlueprintAnswers)
                  ? "Continue →"
                  : "Select at least one"}
              </button>
              <button
                type="button"
                className="sys2-cta-tertiary"
                onClick={advanceQuestion}
              >
                Skip for now
              </button>
            </div>
          )}
        </section>
      )}

      {/* ════════ GATE ─═══════ */}
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
            <h2 className="compat-gate-title">Your Resilience Blueprint is ready</h2>
            <p className="compat-gate-lede">
              Enter your details to unlock your personalised profile — engine
              archetype, custody stance, risk envelope, and the next step.
              One blueprint, then silence unless you choose a path.
            </p>

            <form onSubmit={submitBlueprint} className="compat-gate-form">
              <div className="form-group">
                <label className="form-label" htmlFor="bp-name">
                  Your name
                </label>
                <input
                  id="bp-name"
                  className="form-input"
                  required
                  value={gate.name}
                  onChange={(e) => setGate((g) => ({ ...g, name: e.target.value }))}
                  placeholder="Enter your name"
                  autoComplete="name"
                />
              </div>
              <div className="form-group">
                <label className="form-label" htmlFor="bp-email">
                  Email
                </label>
                <input
                  id="bp-email"
                  className="form-input"
                  type="email"
                  required
                  value={gate.email}
                  onChange={(e) => setGate((g) => ({ ...g, email: e.target.value }))}
                  placeholder="name@company.com"
                  autoComplete="email"
                />
              </div>
              <div className="form-group">
                <label className="form-label" htmlFor="bp-telegram">
                  Telegram or Signal handle{" "}
                  <span style={{ fontWeight: 400, color: "var(--text-tertiary)" }}>(optional)</span>
                </label>
                <input
                  id="bp-telegram"
                  className="form-input"
                  value={gate.telegram}
                  onChange={(e) => setGate((g) => ({ ...g, telegram: e.target.value }))}
                  placeholder="@handle"
                />
              </div>
              <div className="form-group">
                <label className="form-label" htmlFor="bp-source">
                  How did you find Resilient?{" "}
                  <span style={{ fontWeight: 400, color: "var(--text-tertiary)" }}>(optional)</span>
                </label>
                <input
                  id="bp-source"
                  className="form-input"
                  value={gate.source}
                  onChange={(e) => setGate((g) => ({ ...g, source: e.target.value }))}
                  placeholder="e.g. Twitter, friend, Colosseum…"
                />
              </div>

              <button
                type="submit"
                className="sys2-cta-primary compat-full-cta"
                disabled={isSubmitting}
              >
                {isSubmitting
                  ? "Generating blueprint…"
                  : "Generate Resilience Blueprint →"}
              </button>

              {submitError && <div className="compat-error">{submitError}</div>}
            </form>
          </div>
        </section>
      )}

      {/* ════════ PROFILE ════════ */}
      {(step === "profile" || step === "results") && profile && (
        <>
          <section className="compat-shell compat-shell--wide">
            <header className="compat-hero">
              <div className="compat-hero-copy">
                <div className="sys2-sect-eyebrow">RESILIENCE BLUEPRINT</div>
                <h1 className="compat-title compat-title--result">
                  Your personalised engine profile
                </h1>
                <p className="compat-lede">
                  Based on your answers, here's the blueprint. It maps your
                  on-chain readiness, risk tolerance, and the engine archetype
                  that fits your constraints.
                </p>
              </div>
              <div className="compat-hero-side">
                <span className="sys2-status-pill watching">
                  <span className="sys2-status-dot" />
                  Cohort · limited slots
                </span>
              </div>
            </header>

            {/* Profile Card */}
            <div className="blueprint-profile-card">
              {/* Scores row */}
              <div className="blueprint-scores">
                <div className="blueprint-score">
                  <span className="blueprint-score-label">On-chain readiness</span>
                  <span className="blueprint-score-bar">
                    <span className="blueprint-score-fill" style={{ width: `${profile.onChainReadiness * 10}%` }} />
                  </span>
                  <span className="blueprint-score-val">{profile.onChainReadiness}/10</span>
                </div>
                <div className="blueprint-score">
                  <span className="blueprint-score-label">Risk tolerance</span>
                  <span className="blueprint-score-bar">
                    <span className="blueprint-score-fill" style={{ width: `${profile.riskTolerance * 10}%` }} />
                  </span>
                  <span className="blueprint-score-val">{profile.riskTolerance}/10</span>
                </div>
                <div className="blueprint-score">
                  <span className="blueprint-score-label">Complexity appetite</span>
                  <span className="blueprint-score-bar">
                    <span className="blueprint-score-fill" style={{ width: `${profile.complexityAppetite * 10}%` }} />
                  </span>
                  <span className="blueprint-score-val">{profile.complexityAppetite}/10</span>
                </div>
                <div className="blueprint-score">
                  <span className="blueprint-score-label">Commitment readiness</span>
                  <span className="blueprint-score-bar">
                    <span className="blueprint-score-fill" style={{ width: `${profile.commitmentReadiness * 10}%` }} />
                  </span>
                  <span className="blueprint-score-val">{profile.commitmentReadiness}/10</span>
                </div>
              </div>

              {/* On-chain readiness */}
              <div className="blueprint-section">
                <div className="blueprint-section-title">On-chain Readiness</div>
                <div className="blueprint-section-label">{profile.onChainLabel}</div>
                <div className="blueprint-section-body">{profile.onChainExplanation}</div>
              </div>

              {/* Risk envelope */}
              <div className="blueprint-section">
                <div className="blueprint-section-title">Risk Envelope</div>
                <div className="blueprint-section-body">{profile.riskSummary}</div>
              </div>

              {/* Engine archetype */}
              <div className="blueprint-section blueprint-section--highlight">
                <div className="blueprint-section-title">Suggested Engine Archetype</div>
                <div className="blueprint-section-label">{profile.archetype}</div>
                <div className="blueprint-section-body">{profile.archetypeDescription}</div>
              </div>

              {/* Custody stance */}
              <div className="blueprint-section">
                <div className="blueprint-section-title">Custody Stance</div>
                <div className="blueprint-section-body">{profile.custodyStance}</div>
              </div>

              {/* Commitment */}
              <div className="blueprint-section">
                <div className="blueprint-section-body" style={{ fontStyle: "italic", color: "var(--text-tertiary)" }}>
                  {profile.commitmentHint}
                </div>
              </div>
            </div>
          </section>

          {/* CTAs */}
          <section className="compat-shell compat-shell--wide">
            <div className="compat-results-grid">
              <div className="compat-results-main">
                <div className="compat-specimen">
                  <div className="compat-specimen-head">
                    <span className="validated-tag">LIVE SPECIMEN</span>
                    <span className="compat-specimen-title">
                      SOL/USDT Survivor 2.69 · {specimenOpen ? "live position open" : "waiting for alignment"}
                    </span>
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
              </div>

              <aside className="compat-fork">
                <div className="sys2-sect-eyebrow">BESPOKE STRATEGY BUILD</div>
                <h2 className="compat-fork-title">
                  If this blueprint feels right, secure your build slot
                </h2>
                <p className="compat-fork-lede">
                  I reserve a small number of build slots. Once you're in, I
                  convert this blueprint into a full engine spec and deployment
                  plan, plus up to four dedicated consultations.
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
                    Secure Bespoke Strategy Build – A$4,500
                  </a>
                  <a
                    href={calBookHref(gate.name, gate.email)}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="sys2-cta-secondary"
                  >
                    Book a 30-min fit call
                  </a>
                </div>

                {/* Secondary CTA: Ask question */}
                <div className="blueprint-question-cta">
                  <div className="blueprint-question-title">
                    Still deciding? Ask a question before committing.
                  </div>
                  {askQuestionSent ? (
                    <p className="compat-gate-lede" style={{ marginTop: "var(--space-sm)" }}>
                      Thanks — I'll reply by email. In the meantime, the build slot
                      is open whenever you're ready.
                    </p>
                  ) : (
                    <div style={{ display: "flex", gap: "var(--space-sm)", alignItems: "flex-end", flexWrap: "wrap" }}>
                      <textarea
                        className="form-input"
                        rows={2}
                        value={askQuestion}
                        onChange={(e) => setAskQuestion(e.target.value)}
                        placeholder="What would you like to know before you commit?"
                        style={{ flex: 1, minWidth: "200px" }}
                      />
                      <button
                        type="button"
                        className="sys2-cta-secondary"
                        onClick={sendQuestion}
                        disabled={!askQuestion.trim()}
                      >
                        Send
                      </button>
                    </div>
                  )}
                </div>

                <div className="compat-fork-foot">
                  <span>RTP-BLUEPRINT-V1</span>
                  <button
                    type="button"
                    className="compat-retake"
                    onClick={() => {
                      setStep("splash");
                      setQuestionIndex(0);
                      setAnswers(EMPTY_ANSWERS);
                      setGate(EMPTY_GATE);
                      setProfile(null);
                      setSubmitError(null);
                      setAskQuestion("");
                      setAskQuestionSent(false);
                    }}
                  >
                    Retake blueprint
                  </button>
                </div>
              </aside>
            </div>
          </section>
        </>
      )}
    </div>
  );
}
