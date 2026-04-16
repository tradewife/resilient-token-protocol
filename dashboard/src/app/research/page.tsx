"use client";

import React, { useEffect, useState } from "react";
import Link from "next/link";

interface NightData {
  _date: string;
  _report: string;
  run_at: string;
  runtime_seconds: number;
  num_folds: number;
  symbols: string[];
  market_state: Record<string, {
    current_adx: number;
    current_regime: string;
    volatility_percentile: number;
    recent_30d_return_pct: number;
    adx_trend: string;
  }>;
  production_baseline: Record<string, {
    survivor_score: number;
    oos_sharpe: number;
    oos_consistency: number;
    params: Record<string, number>;
  }>;
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
}

export default function ResearchPage() {
  const [data, setData] = useState<NightData | null>(null);

  useEffect(() => {
    fetch("/data/night.json")
      .then((r) => r.json())
      .then((d) => { if (!d.error) setData(d); })
      .catch(() => {});
  }, []);

  const runtimeMin = data ? Math.round(data.runtime_seconds / 60) : 0;
  const topCandidate = data?.top_candidates?.[0];

  // Deduplicate top candidates by unique survivor_score (they share the same config family)
  const uniqueCandidates = data?.top_candidates
    ? data.top_candidates.filter((c, i, arr) =>
        arr.findIndex((x) => x.survivor_score === c.survivor_score && x.symbol === c.symbol) === i
      ).slice(0, 5)
    : [];

  return (
    <div className="page">
      {/* ── Top bar ────────────────────────────────────────── */}
      <header className="topbar">
        <div className="brand">
          <img className="brand-icon" src="/icon.svg" alt="RTP" />
          <Link href="/" className="brand-name" style={{ textDecoration: "none", color: "inherit" }}>
            RESILIENT TOKEN PROTOCOL
          </Link>
        </div>
        <div className="topbar-actions">
          <Link href="/docs" className="btn-connect" style={{ textDecoration: "none", fontSize: "0.8125rem", padding: "6px 14px" }}>
            Docs
          </Link>
          <Link href="/launch" className="btn-connect" style={{ textDecoration: "none", fontSize: "0.8125rem", padding: "6px 14px" }}>
            Platform Demo
          </Link>
          <Link href="/research" className="btn-connect" style={{ textDecoration: "none", fontSize: "0.8125rem", padding: "6px 14px" }}>
            Research
          </Link>
          <Link href="/" className="btn-connect" style={{ textDecoration: "none", fontSize: "0.8125rem", padding: "6px 14px" }}>
            Dashboard
          </Link>
        </div>
      </header>

      {!data ? (
        <section className="launch-hero">
          <h1 className="launch-title">Night Shift Research</h1>
          <p className="launch-subtitle">Loading latest results...</p>
        </section>
      ) : (
        <>
          {/* ── Hero ──────────────────────────────────────────── */}
          <section className="launch-hero">
            <h1 className="launch-title">Night Shift Research</h1>
            <p className="launch-subtitle">
              {data.symbols.length} symbols &middot; {data.num_folds}-fold walk-forward &middot; {runtimeMin} min runtime &middot; {data._date}
            </p>
          </section>

          {/* ── Market state ─────────────────────────────────── */}
          <section className="research-section">
            <h2 className="section-title">Market State</h2>
            <div className="research-grid">
              {Object.entries(data.market_state).map(([sym, m]) => (
                <div className="info-card" key={sym}>
                  <span className="info-label">{sym}</span>
                  <div className="market-row">
                    <span className="market-tag">{m.current_regime}</span>
                    <span className="market-tag">{m.adx_trend}</span>
                  </div>
                  <div className="market-detail">
                    <span>ADX {m.current_adx}</span>
                    <span>Vol {m.volatility_percentile}%</span>
                    <span className={m.recent_30d_return_pct >= 0 ? "positive" : "negative"}>
                      {m.recent_30d_return_pct >= 0 ? "+" : ""}{m.recent_30d_return_pct}%
                    </span>
                  </div>
                </div>
              ))}
            </div>
          </section>

          {/* ── Production baseline ──────────────────────────── */}
          <section className="research-section">
            <h2 className="section-title">Production Baseline</h2>
            <table className="research-table">
              <thead>
                <tr>
                  <th>Symbol</th>
                  <th>OOS Sharpe</th>
                  <th>Consistency</th>
                  <th>Survivor</th>
                </tr>
              </thead>
              <tbody>
                {Object.entries(data.production_baseline).map(([sym, b]) => (
                  <tr key={sym}>
                    <td className="sym">{sym}</td>
                    <td>{b.oos_sharpe.toFixed(2)}</td>
                    <td>{(b.oos_consistency * 100).toFixed(0)}%</td>
                    <td>{b.survivor_score.toFixed(2)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </section>

          {/* ── Top candidate highlight ─────────────────────── */}
          {topCandidate && (
            <section className="research-section highlight-section">
              <div className="highlight-badge">STRONG RECOMMEND</div>
              <h2 className="section-title">
                Top Candidate: {topCandidate.symbol} (Survivor {topCandidate.survivor_score.toFixed(2)})
              </h2>
              <div className="highlight-grid">
                <div className="info-card highlight-card">
                  <span className="info-label">OOS Sharpe</span>
                  <span className="highlight-value">{topCandidate.oos_sharpe.toFixed(2)}</span>
                </div>
                <div className="info-card highlight-card">
                  <span className="info-label">Consistency</span>
                  <span className="highlight-value">{(topCandidate.oos_consistency * 100).toFixed(0)}%</span>
                </div>
                <div className="info-card highlight-card">
                  <span className="info-label">Max DD</span>
                  <span className="highlight-value">{topCandidate.oos_max_dd.toFixed(1)}%</span>
                </div>
                <div className="info-card highlight-card">
                  <span className="info-label">Overfitting</span>
                  <span className="highlight-value">{topCandidate.overfitting_score.toFixed(2)}</span>
                </div>
                <div className="info-card highlight-card">
                  <span className="info-label">Fragility</span>
                  <span className="highlight-value">{topCandidate.fragility.toFixed(2)}</span>
                </div>
                <div className="info-card highlight-card">
                  <span className="info-label">Trades/Fold</span>
                  <span className="highlight-value">{topCandidate.oos_avg_trades_per_fold.toFixed(0)}</span>
                </div>
              </div>

              <div className="param-block">
                <span className="info-label">Strategy Config</span>
                <pre className="param-pre"><code>{JSON.stringify(topCandidate.params, null, 2)}</code></pre>
              </div>

              {(() => {
                const base = data.production_baseline[topCandidate.symbol];
                if (!base) return null;
                return (
                  <div className="delta-block">
                    <span className="info-label">vs Production Baseline</span>
                    <div className="delta-grid">
                      {Object.keys(topCandidate.params).map((k) => {
                        const from = base.params[k];
                        const to = topCandidate.params[k];
                        if (from === to) return null;
                        return (
                          <div className="delta-item" key={k}>
                            <span className="delta-param">{k}</span>
                            <span className="delta-values">
                              <span className="delta-from">{from}</span>
                              <span className="delta-arrow">&rarr;</span>
                              <span className="delta-to">{to}</span>
                            </span>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                );
              })()}
            </section>
          )}

          {/* ── Other candidates ─────────────────────────────── */}
          {uniqueCandidates.length > 1 && (
            <section className="research-section">
              <h2 className="section-title">Other Validated Candidates</h2>
              <table className="research-table">
                <thead>
                  <tr>
                    <th>#</th>
                    <th>Symbol</th>
                    <th>Survivor</th>
                    <th>OOS Sharpe</th>
                    <th>Consistency</th>
                    <th>Overfitting</th>
                    <th>Trades/Fold</th>
                  </tr>
                </thead>
                <tbody>
                  {uniqueCandidates.map((c, i) => (
                    <tr key={i}>
                      <td>{i + 1}</td>
                      <td className="sym">{c.symbol}</td>
                      <td>{c.survivor_score.toFixed(2)}</td>
                      <td>{c.oos_sharpe.toFixed(2)}</td>
                      <td>{(c.oos_consistency * 100).toFixed(0)}%</td>
                      <td>{c.overfitting_score.toFixed(2)}</td>
                      <td>{c.oos_avg_trades_per_fold.toFixed(0)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </section>
          )}

          {/* ── Full report ──────────────────────────────────── */}
          {data._report && (
            <section className="research-section">
              <h2 className="section-title">Full Report</h2>
              <div className="report-block">
                <pre className="report-pre">{data._report}</pre>
              </div>
            </section>
          )}
        </>
      )}

      {/* ── Footer ──────────────────────────────────────────── */}
      <footer className="vitals">
        <div className="vital">
          <span className="vital-value">{data?.symbols.length ?? "—"} symbols</span>
          <span className="vital-label">Symbols</span>
        </div>
        <div className="vital">
          <span className="vital-value">{data?.num_folds ?? "—"} folds</span>
          <span className="vital-label">Walk-Forward</span>
        </div>
        <div className="vital">
          <span className="vital-value">{runtimeMin} min</span>
          <span className="vital-label">Runtime</span>
        </div>
        <Link href="/" className="vital-link">Back to Dashboard</Link>
      </footer>
    </div>
  );
}
