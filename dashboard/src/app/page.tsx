"use client";

import React, { useEffect, useState, useMemo } from "react";
import Link from "next/link";
import Topbar from "./Topbar";
import OfferFlow from "./OfferFlow";
import PnlChart from "./PnlChart";
import NightShiftChart from "./NightShiftChart";
import type { NightData } from "./nightTypes";
import { inferTradeSide, tradeSideCssClass } from "../lib/tradeSide";
import {
  formatPnlPct,
  netTradePnlPct,
  summarizeTradePnl,
} from "../lib/tradePnl";

/* ── Constants ── */

// Captured once at module load so render stays pure (React compiler rule).
const PAGE_LOAD_TS = Date.now();

const MAINNET_TXS = [
  { label: "Open · CPI invoke_signed", tx: "2bLg1FuJ6iqwYq6SKi5EcZQWszarDZhS68bCbGTRLKMwhYqsU7G57fTtG4G6GFx3ZKN15qhb85zy28pGJvSdrnG3", note: "99,214 CU", kind: "open" },
  { label: "Close · SOL returned", tx: "dFqkoP2wX2meR8Mv8CngujJJUNBYuv5peCyzRYFPBvpN3uqCqXqRCy4TPyw5JbAZhumCaJdGaJoQvJrJGJzxfHF", note: "settled mainnet", kind: "close" },
  { label: "Open · REST autonomous", tx: "YtGKq46wEgeUqoWouV5LXvv6mAxb5dCYmRHy622i7UtP5UoXsKZJtqscJf9fWLjzjZwCZhGw7r4EMgKV3wU2CBg", note: "score = 0.400", kind: "open" },
  { label: "Close · REST autonomous", tx: "56PLUQAPGqtAcvRUgJBreMrubAETZkpFCoyHzkwt3jCGCwZYHeonbxcJp244ZipeHuNBAwAX6r1wWkcR9LFcdmM6", note: "settled mainnet", kind: "close" },
];

/* ── Interfaces ── */

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
  timestamp: string | null;
  error?: string;
}

interface TraderState {
  wallet: string;
  open_position: {
    entry_price: number; entry_time: number; peak_price: number;
    entry_score: number; size_usd: number; side: string;
  } | null;
  trade_history: Array<{
    entry_price: number; exit_price: number; entry_time: number;
    exit_time: number; pnl_pct: number; exit_reason: string; size_usd: number; side?: string;
  }>;
  candle_count: number;
  last_poll: string;
  total_pnl_sol: number;
  total_trades: number;
  /** Live StrategyParams from /state — drives the entry-rule copy. */
  active_config?: { signal_threshold?: number };
}

/* NightData shape lives in nightTypes.ts (shared with NightShiftChart). */

/* ── Invariant data ── */



/* ── Main Page ── */

export default function Home() {
  const [treasurySol, setTreasurySol] = useState<number | null>(null);
  const [cycle, setCycle] = useState<CycleData | null>(null);
  const [trader, setTrader] = useState<TraderState | null>(null);
  const [night, setNight] = useState<NightData | null>(null);

  // ── Trader wallet balance (mainnet) ──
  // The yield engine runs on mainnet. The devnet demo PDA only holds rent-exempt minimum.
  // Show the mainnet trader wallet balance as the real "Treasury SOL" figure.
  // Uses server-side API route to avoid CORS issues with public Solana RPC.
  //
  // Wallet source priority (highest first):
  //   1. `trader.wallet` from the live /api/trader-status response — the
  //      actual pubkey the trader is signing with right now.
  //   2. `process.env.NEXT_PUBLIC_RTP_TRADER_WALLET_PUBKEY` — pinned
  //      override for builds where trader-state is unavailable.
  //   3. Legacy default (kept so dev builds without env still render — the
  //      value is the pubkey published in earlier shipping docs).
  const LEGACY_TRADER_WALLET = "Driyi8Sw2622yCefU34zrjBsQynrDoGD31tBecXrEF6R";
  const TRADER_WALLET =
    (trader?.wallet && trader.wallet.length > 0 ? trader.wallet : null) ||
    process.env.NEXT_PUBLIC_RTP_TRADER_WALLET_PUBKEY ||
    LEGACY_TRADER_WALLET;
  const TRADER_WALLET_SHORT =
    TRADER_WALLET.length >= 8
      ? `${TRADER_WALLET.slice(0, 4)}...${TRADER_WALLET.slice(-4)}`
      : TRADER_WALLET;
  useEffect(() => {
    let alive = true;
    const poll = async () => {
      try {
        const res = await fetch("/api/mainnet-balance");
        const json = await res.json();
        if (alive) setTreasurySol(json.sol ?? 0);
      } catch { /* retry on next poll */ }
    };
    poll();
    const id = setInterval(poll, 15_000);
    return () => { alive = false; clearInterval(id); };
  }, []);

  // ── Fetch cycle data ──
  useEffect(() => {
    (async () => {
      try {
        const res = await fetch("/data/cycle.json");
        if (res.ok) { const data: CycleData = await res.json(); if (!data.error) setCycle(data); }
      } catch {}
    })();
  }, []);

  // ── Fetch trader state ──
  useEffect(() => {
    let alive = true;
    const poll = async () => {
      try {
        // trailingSlash: true in next.config — hit the canonical path to avoid a 308
        const res = await fetch("/api/trader-status/");
        if (res.ok) { const data: TraderState = await res.json(); if (data.wallet && alive) setTrader(data); }
      } catch {}
    };
    poll();
    const id = setInterval(poll, 15_000);
    return () => { alive = false; clearInterval(id); };
  }, []);

  // ── Fetch night data ──
  useEffect(() => {
    (async () => {
      try { const r = await fetch("/data/night.json"); if (r.ok) { const d = await r.json(); if (!d.error) setNight(d); } } catch {}
    })();
  }, []);

  /* ── Derived ── */

  const pnl = useMemo(
    () => summarizeTradePnl(trader?.trade_history),
    [trader]
  );

  // Headline = compounded equity return: per-trade net % applied at the
  // real capital exposure (20% of wallet × 9×). See tradePnl.ts.
  const totalPnlPct = pnl.totalEquityPct;
  const winRate = pnl.winRatePct;

  // Date.now() at module load keeps render pure (React compiler rule);
  // days-running only needs per-page-load granularity.
  const daysRunning = useMemo(() => {
    const deployedAt = new Date("2026-05-12T04:20:00Z").getTime();
    return Math.max(0, Math.ceil((PAGE_LOAD_TS - deployedAt) / 86400000));
  }, []);

  const traderStatus =
    trader == null ? "connecting" :
    trader.open_position ? "in_position" : "watching";

  /* ── Render ── */

  return (
    <div className="page">
      <Topbar activePage="dashboard" />

      {/* ════════ HERO ════════ */}
      <section className="hero" style={{ marginBottom: 0 }}>
        <div className="hero-image-wrap">
          <img src="/bg-flower.jpg" alt="Ethereal flower in emerald and coral: the organic intelligence that drives RTP" />
        </div>

        <div className="hero-content">
          <div className="hero-copy">
            <span className="hero-label">BESPOKE TREASURY INFRASTRUCTURE · CRYPTO-NATIVE CAPITAL · SELF-CUSTODY</span>
            <h1 className="hero-title">
              A trading engine built for one account: yours
            </h1>
            <p className="hero-tagline" style={{
              fontSize: "1.25rem",
              fontWeight: 400,
              color: "var(--text-primary)",
              letterSpacing: "-0.01em",
              margin: "0.5rem 0 0.75rem",
              lineHeight: 1.4,
            }}>
              Shared strategies get crowded.{" "}<span className="hero-tagline-break">Yours is cut to your measurements.</span>
            </p>
            <p className="hero-subtitle">
              Tell us your terms — risk budget, drawdown limit, horizon. We engineer a distinct
              strategy around them, price it at live venue fees measured on-chain, and put it
              through a fixed gate suite before anything runs on your capital. You keep the keys
              and the kill switch. We stay loyal to no one but you.
            </p>
          </div>

          <div className="sys2-hero-cta-row" style={{ marginBottom: "var(--space-lg)" }}>
            <Link href="/compatibility" className="sys2-cta-primary">
              Start Compatibility Check →
            </Link>
          </div>
        </div>

        {/* Vitals strip — full width */}
        <div className="sys2-vitals" style={{ gridColumn: "1 / -1" }}>
          <div className="sys2-vital">
            <div className="sys2-vital-label">Cumulative PnL</div>
            <div className={`sys2-vital-value ${totalPnlPct >= 0 ? "pos" : "neg"}`}>
              {formatPnlPct(totalPnlPct)}
            </div>
            <div className="sys2-vital-sub">
              {pnl.tradeCount} closed · equity compounded · net of measured GMTrade fees
            </div>
          </div>
          <div className="sys2-vital">
            <div className="sys2-vital-label">Days Active</div>
            <div className="sys2-vital-value">{daysRunning}</div>
            <div className="sys2-vital-sub">Since May 12 · Railway</div>
          </div>
          <div className="sys2-vital">
            <div className="sys2-vital-label">Treasury SOL</div>
            <div className="sys2-vital-value">
              {treasurySol !== null && treasurySol > 0 ? treasurySol.toFixed(4) : "—"}
            </div>
            <div className="sys2-vital-sub">{treasurySol !== null && treasurySol > 0 ? `Mainnet · ${TRADER_WALLET_SHORT}` : "Mainnet"}</div>
          </div>
          <div className="sys2-vital">
            <div className="sys2-vital-label">Mainnet TXs</div>
            <div className="sys2-vital-value">4</div>
            <div className="sys2-vital-sub">CPI + REST proofs</div>
          </div>
          <div className="sys2-vital">
            <div className="sys2-vital-label">Test coverage</div>
            <div className="sys2-vital-value">423<span className="sys2-vital-unit">+12</span></div>
            <div className="sys2-vital-sub">Rust unit + integration</div>
          </div>
          <div className="sys2-vital">
            <div className="sys2-vital-label">Calmar (validated)</div>
            <div className="sys2-vital-value">44.89</div>
            <div className="sys2-vital-sub">{"SOL Survivor 2.69 · "}<span className="vital-sub-line">{"9x lev"}</span></div>
          </div>
        </div>
      </section>

      {/* ════════ §1 PROVEN ON MAINNET ════════ */}
      <section className="sys2-section" id="live">
        <header className="sys2-sect-head">
          <div>
            <div className="sys2-sect-eyebrow">§1 · verifiable mainnet performance</div>
            <h2 className="sys2-sect-title">Trust is built on public state you can verify</h2>
            <p className="sys2-sect-lede">
              Beta testing with skin in the game — real capital on mainnet since May 12, 2026,
              every position verifiable on Solana Explorer. A Rust agent executes validated
              strategies on Solana perps, signed autonomously. When the original venue (Flash
              Trade) announced its wind-down in August 2026, the pipeline measured GMTrade&apos;s
              live on-chain costs and re-validated the engine: 10/10 gates. Migration underway.
        </p>
          </div>
          <div className="sys2-sect-side">
            <span className={`sys2-status-pill ${traderStatus}`}>
              <span className="sys2-status-dot" />
              {traderStatus === "in_position" ? "Position open on mainnet" :
               traderStatus === "watching" ? "Flat · venue migration to GMTrade" : "Connecting…"}
            </span>
          </div>
        </header>

        <div className="console-grid">
          <div className="console-card">
            <div className="console-card-eyebrow">CURRENT POSITION</div>
            {trader?.open_position ? (
              <>
                <div className="console-big">
                  SOL/USDT · {trader.open_position.side?.toUpperCase() ?? "LONG"} · 9×
                </div>
                <div className="console-row">
                  <span>Entry</span><span className="mono">${trader.open_position.entry_price.toFixed(4)}</span>
                </div>
                <div className="console-row">
                  <span>Peak</span><span className="mono">${trader.open_position.peak_price.toFixed(4)}</span>
                </div>
                <div className="console-row">
                  <span>Size</span><span className="mono">${trader.open_position.size_usd.toFixed(2)}</span>
                </div>
                <div className="console-row">
                  <span>Entry score</span><span className="mono">{trader.open_position.entry_score.toFixed(3)}</span>
                </div>
              </>
            ) : (
              <>
                <div className="console-big console-muted">Flat</div>
                <div className="console-empty-text">
                  Survivor 2.69 enters LONG when score &gt; {(trader?.active_config?.signal_threshold ?? 0.3)} or SHORT when score &lt; {-(trader?.active_config?.signal_threshold ?? 0.3)}, with 2+ aligned timeframes. 20% capital, 9× leverage. Stop-loss 2.5× ATR, take-profit 6.0× ATR, trailing 1.0× ATR.
                </div>
              </>
            )}
            <div className="console-foot">
              Last poll · <span className="mono">{trader?.last_poll?.slice(11, 19) ?? "—"}</span>
            </div>
          </div>

          <div className="console-card chart-card">
            <div className="console-card-eyebrow">EQUITY CURVE · NET OF FEES · 20% CAPITAL × 9× · ALL CLOSED TRADES</div>
            {(trader?.trade_history?.length ?? 0) >= 1 ? (
              <PnlChart trades={trader?.trade_history ?? []} />
            ) : (
              <div className="sparkline-empty">
                <div className="sparkline-empty-glyph">⏷</div>
                <div className="sparkline-empty-title">Awaiting first closed trade</div>
                <div className="sparkline-empty-sub">
                  The trader is watching SOL/USDT live. Cumulative PnL appears here the moment the first position closes.
                </div>
              </div>
            )}
            <div className="chart-stats">
              <div className="chart-stat">
                <span className="chart-stat-val">{trader?.total_trades ?? 0}</span>
                <span className="chart-stat-lab">trades</span>
              </div>
              <div className="chart-stat">
                <span className={`chart-stat-val ${totalPnlPct >= 0 ? "pos" : "neg"}`}>
                  {formatPnlPct(totalPnlPct)}
                </span>
                <span className="chart-stat-lab">equity compounded</span>
              </div>
              <div className="chart-stat">
                <span className="chart-stat-val">{winRate == null ? "—" : `${winRate.toFixed(0)}%`}</span>
                <span className="chart-stat-lab">win rate</span>
              </div>
              <div className="chart-stat">
                <span className="chart-stat-val mono">{trader?.total_pnl_sol?.toFixed(4) ?? "0.0000"}</span>
                <span className="chart-stat-lab">SOL realized</span>
              </div>
            </div>
          </div>
        </div>

        {/* Trade tape */}
        {(trader?.trade_history?.length ?? 0) > 0 && (
          <div className="trade-tape">
            <div className="trade-tape-head">
              <span>RECENT TAPE</span>
              <span className="mono trade-tape-sub">last {Math.min(8, trader?.trade_history?.length ?? 0)} trades</span>
            </div>
            <div className="trade-tape-rows">
              {[...(trader?.trade_history ?? [])].slice(-8).reverse().map((t, i) => {
                const net = netTradePnlPct(t);
                const side = inferTradeSide(t);
                return (
                <div key={i} className={`trade-row ${net >= 0 ? "pos" : "neg"}`}>
                  <span className="mono">SOL/USDT</span>
                  <span className="mono dim">${t.entry_price.toFixed(2)} → ${t.exit_price.toFixed(2)}</span>
                  <span className="trade-reason">{t.exit_reason}</span>
                  <span className={`trade-side ${tradeSideCssClass(side)}`}>{side.toUpperCase()}</span>
                  <span className={`mono trade-pnl ${net >= 0 ? "pos" : "neg"}`}>
                    {formatPnlPct(net)}
                  </span>
                </div>
              );
              })}
            </div>
          </div>
        )}

        {/* Mainnet proof — hidden on mobile (cards too heavy) */}
        <div className="hide-mobile" style={{ marginTop: "var(--space-2xl)" }}>
          <header className="sys2-sect-head" style={{ marginBottom: "var(--space-lg)" }}>
            <div>
              <div className="sys2-sect-eyebrow">ON-CHAIN PROOF</div>
              <h2 style={{ fontFamily: "var(--font-display)", fontSize: "clamp(1.125rem, 2vw, 1.5rem)", fontWeight: 500, color: "var(--text-primary)", letterSpacing: "-0.01em", margin: 0 }}>
                Real mainnet transactions, not testnet
              </h2>
            </div>
          </header>
          <div className="proof2-grid">
            {MAINNET_TXS.map((tx) => (
              <a key={tx.tx} href={`https://explorer.solana.com/tx/${tx.tx}`}
                target="_blank" rel="noopener noreferrer" className={`proof2-card kind-${tx.kind}`}>
                <div className="proof2-head">
                  <span className="proof2-kind">{tx.kind === "open" ? "OPEN" : "CLOSE"}</span>
                  <span className="proof2-link">↗</span>
                </div>
                <div className="proof2-label">{tx.label}</div>
                <div className="proof2-tx mono">{tx.tx.slice(0, 10)}…{tx.tx.slice(-8)}</div>
                <div className="proof2-note">{tx.note}</div>
              </a>
            ))}
          </div>
          <div className="proof2-extras">
            <a href="https://explorer.solana.com/tx/4RVehmPVpnFYHrsF6N64RjVh7mszRzKF9DQVHd8TUqBHwrnyDYavf3TnDYJC4b5PrJWVSubZkNuyVkF1oJzk71RT?cluster=devnet"
              target="_blank" rel="noopener noreferrer" className="proof2-extra">Devnet redistribution TX ↗</a>
            <a href="https://explorer.solana.com/address/8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB?cluster=devnet"
              target="_blank" rel="noopener noreferrer" className="proof2-extra">Treasury program (devnet) ↗</a>
            <a href="https://explorer.solana.com/tx/2bLg1FuJ6iqwYq6SKi5EcZQWszarDZhS68bCbGTRLKMwhYqsU7G57fTtG4G6GFx3ZKN15qhb85zy28pGJvSdrnG3"
              target="_blank" rel="noopener noreferrer" className="proof2-extra">Mainnet CPI proof ↗</a>
          </div>
        </div>
      </section>

      {/* ════════ §2 THE OFFER ════════ */}
      <section className="sys2-section" id="offer" style={{ marginTop: "var(--space-4xl)" }}>
        <div className="offer-layout">
          <div className="offer-copy">
            <div className="sys2-sect-eyebrow">§2 · the offer</div>
            <h2 className="sys2-sect-title offer-title">
              An autonomous system engineered to your terms
            </h2>
            <div className="offer-lede">
              <p>
                Not another indicator.
                <br />
                Not a shared vault or staking protocol.
              </p>
              <p>
                We take your exact constraints — capital size, maximum drawdown, horizon,
                the assets you want to accumulate — and engineer a distinct strategy for
                one destination: your account. No shared edges. What you receive, nobody
                else runs.
              </p>
              <p>
                Every engine is priced against live on-chain fees and must clear a fixed
                gate suite (out-of-sample performance, fold consistency, drawdown, zero
                liquidations). If it can’t clear them at real costs, it does not ship.
              </p>
              <p>
                You keep custody and the kill switch. The treasury is controlled by code,
                not by a person. We never hold your funds and we take no cut of your
                trades. When we recommend a venue, the measurement is yours to inspect.
              </p>
              <p>
                High-touch by design. We deliberately run only a few engagements at a
                time.
              </p>
            </div>
            <div className="offer-actions">
              <Link href="/compatibility" className="sys2-cta-primary">
                Start Compatibility Check →
              </Link>
            </div>
          </div>
          <div className="offer-visual">
            <OfferFlow />
          </div>
        </div>
      </section>

      {/* ════════ §3 SELF-IMPROVING RESEARCH ENGINE ════════ */}
      <section className="sys2-section" id="pipeline" style={{ marginTop: "var(--space-4xl)" }}>
        <header className="sys2-sect-head">
          <div>
            <div className="sys2-sect-eyebrow">§3 · the engine room</div>
            <h2 className="sys2-sect-title">Exhaustive search. Validated survivors</h2>
            <p className="sys2-sect-lede">
              We do not guess strategy parameters. Every night, our offline validation pipeline —
              The Night Shift — runs thousands of computational iterations to ensure only the
              most robust systems are deployed. An LLM consults a library of 15 strategies and a
              log of remembered failures before every exploration run.
        </p>
          </div>
        </header>

        <ol className="pipe2-steps">
          {[
            { n: "01", t: "Grid Search", m: "30,000", u: "configs swept per symbol per night", d: "Exhaustive sweep across signal threshold, TP/SL multipliers, trailing stop, hold time, alignment." },
            { n: "02", t: "Walk-Forward Validation", m: `${night?.num_folds ?? 9}`, u: "expanding-window folds · 36 days OOS each", d: "No look-ahead. Median OOS Sharpe wins, not mean. Each candidate tested on 9 independent windows." },
            { n: "03", t: "Darwinian Evolution", m: "5×50", u: "generations × population = 250 refined survivors", d: "Top survivors mutate and compete. Fragility is a penalty, not rejection: survivor *= 1/(1+fragility)." },
            { n: "04", t: "Overfitting Detection", m: "3", u: "independent checks: IS/OOS gap, fold consistency, fragility", d: "Monte Carlo drawdown over 10K paths + Combinatorial Purged CV with PBO. Anything fragile is dropped." },
            { n: "05", t: "Full-Sim Validation", m: "0.1%", u: "fees + 10 bps slippage + 20% position cap + compounding", d: "Top candidates re-run through the production simulator. Fast vs full sim calibrated weekly." },
          ].map((s, i) => (
            <li key={s.n} className="pipe2-step">
              <div className="pipe2-num">{s.n}</div>
              <div className="pipe2-body">
                <div className="pipe2-title">{s.t}</div>
                <div className="pipe2-metric">
                  <span className="pipe2-big">{s.m}</span>
                  <span className="pipe2-unit">{s.u}</span>
                </div>
                <div className="pipe2-desc">{s.d}</div>
              </div>
              {i < 4 && <div className="pipe2-tick">▾</div>}
            </li>
          ))}
        </ol>

        {/* Validated stats + last night's survivors, one row */}
        <div className="engine-proof-row">
          <div className="console-card validated-card">
            <div className="console-card-eyebrow">
              Walk-forward validated · Survivor 2.69 · 9× · OOS, not live PnL
            </div>
            <div className="validated-grid">
              {[
                { v: "44.89", l: "Calmar Ratio" }, { v: "+554%", l: "9× return" },
                { v: "12.3%", l: "Max drawdown" }, { v: "100%", l: "Fold consistency" },
                { v: "0", l: "Liquidations" }, { v: "16,228", l: "Candidates tested" },
              ].map((m) => (
                <div key={m.l} className="validated-cell">
                  <span className="validated-val">{m.v}</span>
                  <span className="validated-lab">{m.l}</span>
                </div>
              ))}
            </div>
          </div>

          {/* Night Shift survivors — the pipeline's output, ranked */}
          <div className="console-card chart-card">
            <NightShiftChart night={night} />
          </div>
        </div>
      </section>

      {/* ════════ §5 ENGAGE ════════ */}
      <section className="sys2-section sys2-cta-section" id="integrate">
        <div className="cta2-card">
          <div className="cta2-content">
            <div className="sys2-sect-eyebrow">§4 · high-touch engagement & pricing</div>
            <h2 className="cta2-title">Bespoke Strategy Build · A$4,500</h2>
            <p className="cta2-lede">
              One-time. No ongoing asset management fees or performance cuts. You pay once
              for a paper-validated strategy mapped to your terms — report, config, and up to
              four implementation consultations included.
        </p>
            <div className="cta2-actions">
              <Link href="/compatibility" className="sys2-cta-primary">
                Start Compatibility Check →
              </Link>
            </div>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="vitals">
        <div className="vital">
          <span className="vital-value">{TRADER_WALLET_SHORT}</span>
          <span className="vital-label">Trader Wallet (Mainnet)</span>
        </div>
        <div className="vital">
          <a className="vital-link" href="https://resilientprotocol.xyz" target="_blank" rel="noopener noreferrer">resilientprotocol.xyz ↗</a>
          <span className="vital-label">Dashboard</span>
        </div>
        <div className="vital">
          <a className="vital-link" href={`https://explorer.solana.com/address/${TRADER_WALLET}`}
            target="_blank" rel="noopener noreferrer">Solana Explorer ↗</a>
          <span className="vital-label">Trader Wallet (Mainnet)</span>
        </div>
      </footer>
    </div>
  );
}
