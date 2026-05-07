"use client";

import React, { useEffect, useState, useCallback } from "react";
import Link from "next/link";
import Topbar from "../Topbar";

/* ──────────────────────── Types ──────────────────────── */

interface CycleData {
  cycle_id: string | null;
  params_used: Record<string, number>;
  params_next: Record<string, number>;
  mutations_accepted: Array<{ param: string; value: number; rationale: string }>;
  mutations_rejected: Array<{ param: string; value: number; rationale: string }>;
  diffs: Array<{ param: string; from: number; to: number }>;
  used_llm: boolean;
  model_label: string;
  memory_file_count: number;
  raw_llm_response?: string | null;
  timestamp: string | null;
}

interface TraderState {
  wallet: string;
  open_position: {
    entry_price: number;
    entry_time: number;
    peak_price: number;
    entry_score: number;
    size_usd: number;
  } | null;
  trade_history: Array<{
    entry_price: number;
    exit_price: number;
    entry_time: number;
    exit_time: number;
    pnl_pct: number;
    exit_reason: string;
    size_usd: number;
  }>;
  candle_count: number;
  last_poll: string;
  total_pnl_sol: number;
  total_trades: number;
}

interface NightData {
  _date: string;
  top_candidates: Array<{
    symbol: string;
    params: Record<string, number>;
    survivor_score: number;
    oos_sharpe: number;
    oos_consistency: number;
    oos_max_dd: number;
    overfitting_score: number;
    fragility: number;
    oos_avg_trades_per_fold: number;
    rejected: boolean;
  }>;
  num_folds: number;
  symbols: string[];
  runtime_seconds: number;
}

interface StrategyCard {
  id: string;
  name: string;
  type: string;
  regime: string;
  priority: number;
  decay_risk: string;
  entry: string;
  exit: string;
}

interface DeadEnd {
  title: string;
  date: string;
  root_cause: string;
  verdict: string;
  test_result: string;
}

/* ──────────────────────── Constants ──────────────────────── */

const MAINNET_TXS = [
  { label: "CPI Open (invoke_signed)", tx: "2bLg1FuJ6iqwYq6SKi5EcZQWszarDZhS68bCbGTRLKMwhYqsU7G57fTtG4G6GFx3ZKN15qhb85zy28pGJvSdrnG3", note: "99,214 CU" },
  { label: "CPI Close (SOL returned)", tx: "dFqkoP2wX2meR8Mv8CngujJJUNBYuv5peCyzRYFPBvpN3uqCqXqRCy4TPyw5JbAZhumCaJdGaJoQvJrJGJzxfHF", note: "confirmed" },
  { label: "REST Open (autonomous)", tx: "YtGKq46wEgeUqoWouV5LXvv6mAxb5dCYmRHy622i7UtP5UoXsKZJtqscJf9fWLjzjZwCZhGw7r4EMgKV3wU2CBg", note: "score=0.400" },
  { label: "REST Close (SOL returned)", tx: "56PLUQAPGqtAcvRUgJBreMrubAETZkpFCoyHzkwt3jCGCwZYHeonbxcJp244ZipeHuNBAwAX6r1wWkcR9LFcdmM6", note: "confirmed" },
];

const TYPE_COLORS: Record<string, string> = {
  trend: "var(--emerald)",
  mean_reversion: "var(--coral)",
  volatility: "#a78bfa",
  carry: "#f59e0b",
  risk_premium: "var(--emerald)",
  mr: "var(--coral)",
  vol: "#a78bfa",
};

const REGIME_LABELS: Record<string, string> = {
  trending: "Trending",
  ranging: "Ranging",
  both: "All Regimes",
};

/* ──────────────────────── Component ──────────────────────── */

export default function SystemPage() {
  const [cycle, setCycle] = useState<CycleData | null>(null);
  const [trader, setTrader] = useState<TraderState | null>(null);
  const [night, setNight] = useState<NightData | null>(null);
  const [strategies, setStrategies] = useState<StrategyCard[]>([]);
  const [deadEnds, setDeadEnds] = useState<DeadEnd[]>([]);
  const [activeLoopNode, setActiveLoopNode] = useState(0);

  // Fetch all data sources
  const fetchCycle = useCallback(async () => {
    try {
      const r = await fetch("/data/cycle.json");
      if (r.ok) { const d = await r.json(); if (!d.error) setCycle(d); }
    } catch {}
  }, []);

  useEffect(() => { fetchCycle(); }, [fetchCycle]);

  useEffect(() => {
    let alive = true;
    const poll = async () => {
      try {
        const r = await fetch("/api/trader-status/");
        if (r.ok) { const d: TraderState = await r.json(); if (d.wallet && alive) setTrader(d); }
      } catch {}
    };
    poll();
    const id = setInterval(poll, 15000);
    return () => { alive = false; clearInterval(id); };
  }, []);

  useEffect(() => {
    (async () => {
      try {
        const r = await fetch("/data/night.json");
        if (r.ok) { const d = await r.json(); if (!d.error) setNight(d); }
      } catch {}
    })();
  }, []);

  useEffect(() => {
    (async () => {
      try {
        const r = await fetch("/data/strategy-library.json");
        if (r.ok) setStrategies(await r.json());
      } catch {}
      try {
        const r = await fetch("/data/dead-ends.json");
        if (r.ok) setDeadEnds(await r.json());
      } catch {}
    })();
  }, []);

  // Animate loop nodes
  useEffect(() => {
    const id = setInterval(() => setActiveLoopNode((n) => (n + 1) % 6), 2000);
    return () => clearInterval(id);
  }, []);

  const runtimeMin = night ? Math.round(night.runtime_seconds / 60) : 0;
  const bestCandidate = night?.top_candidates?.[0];

  const lastTraderPnl = trader?.trade_history?.slice(-1)[0];
  const pnlDeltaLabel = lastTraderPnl
    ? `${lastTraderPnl.pnl_pct >= 0 ? "+" : ""}${lastTraderPnl.pnl_pct.toFixed(2)}% (${lastTraderPnl.exit_reason})`
    : "No closed trades yet";

  return (
    <div className="page">
      <Topbar activePage="research" />

      {/* ── Section 1: The Closed Loop ── */}
      <section className="sys-hero">
        <div className="sys-hero-label">SELF-EVOLVING AUTONOMOUS INTELLIGENCE</div>
        <h1 className="sys-hero-title">The Closed Loop</h1>
        <p className="sys-hero-sub">
          Research validates. The daemon evolves. The trader executes. Performance feeds back.
          No human in the loop. Every mutation is gated. Every trade is on-chain.
        </p>

        <div className="loop-container">
          {[
            { label: "Research Engine", sub: "30K configs · 9-fold WFA", idx: 0 },
            { label: "Bridge", sub: "Python → Rust (typed JSON)", idx: 1 },
            { label: "LLM Evolve", sub: cycle ? (cycle.used_llm ? `model: ${cycle.model_label}` : "deterministic") : "loading...", idx: 2 },
            { label: "Mutation Gates", sub: cycle ? `${cycle.mutations_accepted.length} accepted · ${cycle.mutations_rejected.length} rejected` : "loading...", idx: 3 },
            { label: "Live Trader", sub: trader ? (trader.open_position ? "POSITION OPEN" : "FLAT — Watching") : "connecting...", idx: 4 },
            { label: "PnL Feedback", sub: pnlDeltaLabel, idx: 5 },
          ].map((node) => (
            <div key={node.idx} className={`loop-node ${activeLoopNode === node.idx ? "active" : ""}`}>
              <div className="loop-node-dot" />
              <div className="loop-node-label">{node.label}</div>
              <div className="loop-node-sub">{node.sub}</div>
            </div>
          ))}
          {/* Arrows between nodes */}
          {[0,1,2,3,4].map((i) => (
            <div key={`arrow-${i}`} className="loop-arrow">→</div>
          ))}
        </div>
        <div className="loop-feedback-arrow">↑ PnL feeds back to Research ↑</div>
      </section>

      {/* ── Section 2: Live Evolution Feed ── */}
      <section className="sys-section">
        <h2 className="sys-section-title">Evolution Feed</h2>
        <div className="sys-section-sub">Latest daemon cycle — the system&apos;s reasoning chain</div>

        {!cycle ? (
          <div className="sys-empty">Waiting for cycle data...</div>
        ) : (
          <div className="evolution-feed">
            {/* LLM Proposal */}
            <div className="feed-card">
              <div className="feed-card-badge" style={{ background: cycle.used_llm ? "var(--emerald)" : "var(--text-muted)" }}>
                {cycle.used_llm ? "LLM" : "FALLBACK"}
              </div>
              <div className="feed-card-body">
                <div className="feed-card-title">
                  Strategy Mutation Proposal
                  <span className="feed-card-meta">{cycle.model_label} · {cycle.cycle_id || "—"}</span>
                </div>
                <div className="feed-card-detail">
                  {cycle.mutations_accepted.length + cycle.mutations_rejected.length} mutations proposed.
                  Prompt included {trader ? `real PnL (${trader.total_pnl_sol.toFixed(4)} SOL, ${trader.total_trades} trades)` : "no live data yet"}.
                </div>
                {/* Show raw LLM response if available */}
                {cycle.raw_llm_response && (
                  <details className="feed-details">
                    <summary className="feed-details-toggle">Raw LLM response</summary>
                    <pre className="feed-pre">{cycle.raw_llm_response.slice(0, 800)}{cycle.raw_llm_response.length > 800 ? "..." : ""}</pre>
                  </details>
                )}
              </div>
            </div>

            {/* Bounds Gate */}
            <div className="feed-card">
              <div className="feed-card-badge gate-pass">GATE 1</div>
              <div className="feed-card-body">
                <div className="feed-card-title">Soulcontract Bounds Check</div>
                <div className="mutation-list">
                  {[...cycle.mutations_accepted, ...cycle.mutations_rejected].map((m, i) => {
                    const accepted = cycle.mutations_accepted.some(a => a.param === m.param && a.value === m.value);
                    return (
                      <div key={i} className={`mutation-item ${accepted ? "pass" : "fail"}`}>
                        <span className="mutation-param">{m.param}</span>
                        <span className="mutation-val">{m.value}</span>
                        <span className="mutation-status">{accepted ? "PASS" : "OUT OF BOUNDS"}</span>
                      </div>
                    );
                  })}
                </div>
              </div>
            </div>

            {/* Delta Gate */}
            <div className="feed-card">
              <div className="feed-card-badge gate-pass">GATE 2</div>
              <div className="feed-card-body">
                <div className="feed-card-title">Delta Check (max 20% change)</div>
                <div className="feed-card-detail">
                  {cycle.diffs.length > 0 ? (
                    <div className="mutation-list">
                      {cycle.diffs.map((d, i) => (
                        <div key={i} className="mutation-item pass">
                          <span className="mutation-param">{d.param}</span>
                          <span className="mutation-val">{d.from} → {d.to}</span>
                          <span className="mutation-status">APPLIED</span>
                        </div>
                      ))}
                    </div>
                  ) : (
                    "No parameter changes this cycle — system is stable."
                  )}
                </div>
              </div>
            </div>

            {/* Trader Feedback */}
            {trader && trader.trade_history.length > 0 && (
              <div className="feed-card">
                <div className="feed-card-badge" style={{ background: "var(--coral)" }}>LIVE</div>
                <div className="feed-card-body">
                  <div className="feed-card-title">Trader Feedback</div>
                  <div className="feed-card-detail">
                    Last {Math.min(3, trader.trade_history.length)} trades:
                    <div className="mutation-list" style={{ marginTop: 8 }}>
                      {trader.trade_history.slice(-3).reverse().map((t, i) => (
                        <div key={i} className={`mutation-item ${t.pnl_pct >= 0 ? "pass" : "fail"}`}>
                          <span className="mutation-param">{t.exit_reason}</span>
                          <span className="mutation-val" style={{ color: t.pnl_pct >= 0 ? "var(--emerald)" : "var(--coral)" }}>
                            {t.pnl_pct >= 0 ? "+" : ""}{t.pnl_pct.toFixed(2)}%
                          </span>
                          <span className="mutation-status">
                            ${(t.size_usd || 0).toFixed(0)}
                          </span>
                        </div>
                      ))}
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>
        )}
      </section>

      {/* ── Section 3: Research Pipeline ── */}
      <section className="sys-section">
        <h2 className="sys-section-title">Research Pipeline</h2>
        <div className="sys-section-sub">Institutional-grade rigor — every strategy earns its way to production</div>

        <div className="pipeline-grid">
          {[
            {
              num: "1",
              title: "Grid Search",
              metric: "30,000",
              unit: "parameter combinations per symbol per night",
              detail: "Exhaustive sweep across signal thresholds, TP/SL multipliers, trailing stops, hold times, and alignment windows.",
              color: "var(--emerald)",
            },
            {
              num: "2",
              title: "Walk-Forward Validation",
              metric: night?.num_folds ?? 9,
              unit: "expanding-window folds (36-day OOS each)",
              detail: "Each candidate tested on 9 independent out-of-sample windows. No look-ahead bias. Median OOS Sharpe wins — not mean.",
              color: "var(--emerald)",
            },
            {
              num: "3",
              title: "Darwinian Evolution",
              metric: "5",
              unit: "generations × 50 population = 250 refined candidates",
              detail: "Top survivors mutate and compete. Repeat for 5 generations. Fragility is a penalty, not rejection: survivor *= 1/(1+fragility).",
              color: "var(--coral)",
            },
            {
              num: "4",
              title: "Overfitting Detection",
              metric: "3",
              unit: "independent checks — IS/OOS gap, fold consistency, parameter fragility",
              detail: `Best candidate: overfitting_score=${bestCandidate?.overfitting_score?.toFixed(2) ?? "—"}, fragility=${bestCandidate?.fragility?.toFixed(2) ?? "—"}, consistency=${bestCandidate ? (bestCandidate.oos_consistency * 100).toFixed(0) + "%" : "—"}.`,
              color: bestCandidate && bestCandidate.overfitting_score < 0.5 ? "var(--emerald)" : "var(--coral)",
            },
            {
              num: "5",
              title: "Full-Sim Validation",
              metric: "0.1%",
              unit: "fees + 10bps slippage + max 20% position + compounding",
              detail: "Top candidates re-run through the full simulator with realistic execution costs. Fast sim vs full sim calibrated weekly.",
              color: "var(--emerald)",
            },
          ].map((step) => (
            <div key={step.num} className="pipeline-card">
              <div className="pipeline-num" style={{ color: step.color }}>{step.num}</div>
              <div className="pipeline-content">
                <div className="pipeline-title">{step.title}</div>
                <div className="pipeline-metric">
                  <span style={{ color: step.color, fontFamily: "var(--font-display)", fontSize: "1.25rem" }}>{step.metric}</span>
                  <span style={{ fontSize: "0.75rem", color: "var(--text-secondary)", marginLeft: 8 }}>{step.unit}</span>
                </div>
                <div className="pipeline-detail">{step.detail}</div>
              </div>
            </div>
          ))}
        </div>

        {/* Validated result highlight */}
        {bestCandidate && (
          <div className="highlight-box" style={{ marginTop: "var(--space-xl)" }}>
            <div className="highlight-badge-inline">VALIDATED — {bestCandidate.symbol}</div>
            <div className="highlight-metrics">
              {[
                { label: "Calmar Ratio", value: "44.89" },
                { label: "9x Return", value: "+554%" },
                { label: "Max DD", value: "12.3%" },
                { label: "Consistency", value: "100%" },
                { label: "Liquidations", value: "0" },
                { label: "Candidates Tested", value: "16,228" },
              ].map((m, i) => (
                <div key={i} className="highlight-metric">
                  <span className="highlight-metric-value">{m.value}</span>
                  <span className="highlight-metric-label">{m.label}</span>
                </div>
              ))}
            </div>
          </div>
        )}
      </section>

      {/* ── Section 4: Strategy Intelligence ── */}
      <section className="sys-section">
        <h2 className="sys-section-title">Strategy Intelligence</h2>
        <div className="sys-section-sub">15 strategies catalogued. 9 failures remembered. LLM selects what to explore next.</div>

        <div className="strategy-grid-3col">
          {/* Active Strategy */}
          <div className="strategy-panel active-panel">
            <div className="panel-header">
              <span className="panel-badge live-badge">LIVE</span>
              Active Strategy
            </div>
            <div className="active-strategy-name">SOL/USDT Survivor 2.69</div>
            <div className="active-strategy-type">Multi-timeframe trend following · 9x leverage</div>
            <div className="active-params">
              {[
                { k: "signal_threshold", v: "0.25" },
                { k: "tp_atr", v: "5.0" },
                { k: "sl_atr", v: "2.7" },
                { k: "trail_atr", v: "0.14" },
                { k: "min_alignment", v: "3" },
                { k: "max_hold", v: "36h" },
              ].map((p) => (
                <span key={p.k} className="param-chip">{p.k}={p.v}</span>
              ))}
            </div>
            <div className="active-status">
              {trader ? (
                <>
                  <span className="status-dot live" />
                  {trader.open_position ? "Position open on mainnet" : `Watching · ${trader.candle_count} candles · ${trader.total_trades} trades`}
                </>
              ) : "Connecting to trader..."}
            </div>
          </div>

          {/* Strategy Explorer */}
          <div className="strategy-panel">
            <div className="panel-header">
              <span className="panel-badge explore-badge">15 STRATEGIES</span>
              Strategy Library
            </div>
            <div className="strategy-cards">
              {strategies.map((s) => (
                <div key={s.id} className={`strategy-card priority-${s.priority}`}>
                  <div className="strategy-card-header">
                    <span className="strategy-id">{s.id}</span>
                    <span className="strategy-name">{s.name}</span>
                  </div>
                  <div className="strategy-card-meta">
                    <span className="strategy-type-badge" style={{ color: TYPE_COLORS[s.type] || "var(--text-muted)" }}>
                      {s.type.replace("risk_premium", "momentum")}
                    </span>
                    <span className="strategy-regime">{REGIME_LABELS[s.regime] || s.regime}</span>
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Dead Ends */}
          <div className="strategy-panel dead-ends-panel">
            <div className="panel-header">
              <span className="panel-badge dead-badge">{deadEnds.length} DEAD ENDS</span>
              Failure Memory
            </div>
            <div className="dead-ends-list">
              {deadEnds.map((de, i) => (
                <div key={i} className="dead-end-item">
                  <div className="dead-end-title">{de.title}</div>
                  <div className="dead-end-cause">
                    <span className="cause-badge">{de.root_cause}</span>
                    {de.date !== "unknown" && <span className="dead-end-date">{de.date}</span>}
                  </div>
                </div>
              ))}
            </div>
            <div className="dead-end-footer">
              The system checks this log before every exploration run. Failures are never repeated.
            </div>
          </div>
        </div>
      </section>

      {/* ── Section 5: Architecture ── */}
      <section className="sys-section">
        <h2 className="sys-section-title">Architecture</h2>
        <div className="sys-section-sub">Three layers. One invariant: agents propose, constraints dispose.</div>

        <div className="arch-stack">
          {/* On-chain layer */}
          <div className="arch-layer onchain">
            <div className="arch-layer-label">ON-CHAIN · SOLANA / ANCHOR</div>
            <div className="arch-layer-content">
              <div className="arch-box">
                <span className="arch-box-title">Treasury PDA</span>
                <span className="arch-box-detail">Fees → yield → 70/20/10 redistribute · Flash Trade CPI via invoke_signed</span>
              </div>
              <div className="arch-box">
                <span className="arch-box-title">18 Constitutional Invariants</span>
                <span className="arch-box-detail">PDA ownership · per-token isolation · 20% position cap · phase transitions irreversible · emergency freeze</span>
              </div>
              <div className="arch-box">
                <span className="arch-box-title">Strategy Lifecycle</span>
                <span className="arch-box-detail">Register → update metrics → hard stops (10% DD, 5 losses) → soft decay (3 strikes) → retire</span>
              </div>
            </div>
          </div>

          {/* Arrow */}
          <div className="arch-arrow">▲ invoke_signed (no human key) ▲</div>

          {/* Swarm layer */}
          <div className="arch-layer swarm">
            <div className="arch-layer-label">SWARM RUNTIME · RUST</div>
            <div className="arch-layer-content">
              <div className="arch-wings">
                {[
                  { name: "Trading", desc: "Flash Trade CPI · REST API · PnL tracking", accent: true },
                  { name: "Evolve", desc: "LLM proposer · mutation gates · delta check", accent: true },
                  { name: "Security", desc: "Threat detection · rate limiting" },
                  { name: "Knowledge", desc: "Persistent JSON store · cross-wing queries" },
                  { name: "Audit", desc: "3-agent tribunal · Byzantine consensus" },
                  { name: "Futureproof", desc: "Deprecation monitoring · heartbeat" },
                ].map((w) => (
                  <div key={w.name} className={`arch-wing ${w.accent ? "accent" : ""}`}>
                    <span className="arch-wing-name">{w.name}</span>
                    <span className="arch-wing-desc">{w.desc}</span>
                  </div>
                ))}
              </div>
              <div className="arch-coordinator">
                <span className="arch-coord-label">Coordinator</span>
                <span className="arch-coord-detail">Soulguard enforces SOULCONTRACT.md on every message · 330 tests passing</span>
              </div>
            </div>
          </div>

          {/* Arrow */}
          <div className="arch-arrow">▲ bridge.rs (typed JSON) ▲</div>

          {/* Research layer */}
          <div className="arch-layer research">
            <div className="arch-layer-label">RESEARCH LAYER · PYTHON</div>
            <div className="arch-layer-content">
              <div className="arch-box">
                <span className="arch-box-title">Night Shift</span>
                <span className="arch-box-detail">30K configs → 9-fold WFA → Darwinian → Monte Carlo + CPCV robustness</span>
              </div>
              <div className="arch-box">
                <span className="arch-box-title">LLM Strategy Selector</span>
                <span className="arch-box-detail">Reads library + dead ends → picks 3 most promising strategies for exploration</span>
              </div>
              <div className="arch-box">
                <span className="arch-box-title">5 Strategy Plugins</span>
                <span className="arch-box-detail">S02 BB Breakout · S04 RSI Exhaustion · S06 Vol Squeeze · S10 Momentum · S13 ADX Trend</span>
              </div>
            </div>
          </div>
        </div>

        {/* Railway status */}
        <div className="railway-status">
          <span className="railway-label">7 Railway services</span>
          {["rtp-trader", "rtp-dashboard", "rtp-devnet-loop", "rtp-night-shift", "rtp-swarm-ci", "rtp-fee-crank", "rtp-promote-strategy"].map((svc) => (
            <span key={svc} className="railway-pill">
              <span className="railway-dot" />
              {svc.replace("rtp-", "")}
            </span>
          ))}
        </div>
      </section>

      {/* ── Section 6: On-Chain Proof ── */}
      <section className="sys-section">
        <h2 className="sys-section-title">On-Chain Proof</h2>
        <div className="sys-section-sub">Real mainnet transactions. Not testnet. Not simulation.</div>

        <div className="proof-grid">
          {MAINNET_TXS.map((tx, i) => (
            <a key={i} href={`https://explorer.solana.com/tx/${tx.tx}`}
              target="_blank" rel="noopener noreferrer"
              className="proof-card"
            >
              <div className="proof-card-header">
                <span className="proof-type">{tx.label}</span>
                <span className="proof-link-icon">↗</span>
              </div>
              <div className="proof-tx">{tx.tx.slice(0, 12)}...{tx.tx.slice(-8)}</div>
              <div className="proof-note">{tx.note}</div>
            </a>
          ))}
        </div>

        <div className="proof-extras">
          <a href="https://explorer.solana.com/tx/4RVehmPVpnFYHrsF6N64RjVh7mszRzKF9DQVHd8TUqBHwrnyDYavf3TnDYJC4b5PrJWVSubZkNuyVkF1oJzk71RT?cluster=devnet"
            target="_blank" rel="noopener noreferrer" className="proof-extra-link">
            Redistribution TX (devnet) ↗
          </a>
          <a href="https://explorer.solana.com/address/8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB?cluster=devnet"
            target="_blank" rel="noopener noreferrer" className="proof-extra-link">
            Program (devnet) ↗
          </a>
          <a href="https://github.com/tradewife/resilient-token-protocol"
            target="_blank" rel="noopener noreferrer" className="proof-extra-link">
            Source on GitHub ↗
          </a>
        </div>
      </section>

      {/* Footer */}
      <footer className="vitals">
        <div className="vital">
          <span className="vital-value">330 tests</span>
          <span className="vital-label">Rust + Anchor</span>
        </div>
        <div className="vital">
          <span className="vital-value">18 invariants</span>
          <span className="vital-label">Constitutional</span>
        </div>
        <div className="vital">
          <span className="vital-value">6 wings</span>
          <span className="vital-label">Swarm Runtime</span>
        </div>
        <div className="vital">
          <span className="vital-value">1 function call</span>
          <span className="vital-label">SDK Integration</span>
        </div>
        <Link href="/" className="vital-link">Back to Dashboard</Link>
      </footer>
    </div>
  );
}
