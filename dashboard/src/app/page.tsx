"use client";

import React, { useEffect, useState, useCallback, useMemo } from "react";
import { useWallet, useConnection } from "@solana/wallet-adapter-react";
import { PublicKey, LAMPORTS_PER_SOL } from "@solana/web3.js";
import Link from "next/link";
import Topbar from "./Topbar";
import { fetchTreasuryState } from "../lib/sdk";

/* ── Constants ── */

const TREASURY_AUTHORITY = "********************************************";
const TREASURY_PDA = "6PYPAnwiMoZvzphAWEu3EsNz3PpwjJ6YcZabj34qVQ4Z";
const DEVNET_RPC = "https://api.devnet.solana.com";
const MAINNET_RPC = "https://api.mainnet-beta.solana.com";

const MAINNET_TXS = [
  { label: "Open · CPI invoke_signed", tx: "2bLg1FuJ6iqwYq6SKi5EcZQWszarDZhS68bCbGTRLKMwhYqsU7G57fTtG4G6GFx3ZKN15qhb85zy28pGJvSdrnG3", note: "99,214 CU", kind: "open" },
  { label: "Close · SOL returned", tx: "dFqkoP2wX2meR8Mv8CngujJJUNBYuv5peCyzRYFPBvpN3uqCqXqRCy4TPyw5JbAZhumCaJdGaJoQvJrJGJzxfHF", note: "settled mainnet", kind: "close" },
  { label: "Open · REST autonomous", tx: "YtGKq46wEgeUqoWouV5LXvv6mAxb5dCYmRHy622i7UtP5UoXsKZJtqscJf9fWLjzjZwCZhGw7r4EMgKV3wU2CBg", note: "score = 0.400", kind: "open" },
  { label: "Close · REST autonomous", tx: "56PLUQAPGqtAcvRUgJBreMrubAETZkpFCoyHzkwt3jCGCwZYHeonbxcJp244ZipeHuNBAwAX6r1wWkcR9LFcdmM6", note: "settled mainnet", kind: "close" },
];

const TYPE_COLORS: Record<string, string> = {
  trend: "var(--emerald)", mean_reversion: "var(--coral)", volatility: "#a78bfa",
  carry: "#f59e0b", risk_premium: "var(--emerald)", mr: "var(--coral)", vol: "#a78bfa",
};

const REGIME_LABELS: Record<string, string> = {
  trending: "Trending", ranging: "Ranging", both: "All Regimes",
};

const LOOP_NODES = [
  { key: "research", label: "Research", desc: "30K configs · 9-fold WFA" },
  { key: "bridge",   label: "Bridge",   desc: "Python → Rust JSON" },
  { key: "evolve",   label: "Evolve",   desc: "LLM proposes mutations" },
  { key: "gates",    label: "Gates",    desc: "Bounds + delta check" },
  { key: "execute",  label: "Execute",  desc: "Treasury PDA invoke_signed" },
  { key: "feedback", label: "Feedback", desc: "PnL → next cycle" },
] as const;

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
    entry_score: number; size_usd: number;
  } | null;
  trade_history: Array<{
    entry_price: number; exit_price: number; entry_time: number;
    exit_time: number; pnl_pct: number; exit_reason: string; size_usd: number;
  }>;
  candle_count: number;
  last_poll: string;
  total_pnl_sol: number;
  total_trades: number;
}

interface LivenessData {
  programId: string;
  live: boolean;
  executable: boolean;
  slot: number | null;
}

interface NightData {
  num_folds: number;
  runtime_seconds: number;
  top_candidates: Array<{
    symbol: string; survivor_score: number; oos_sharpe: number;
    oos_consistency: number; oos_max_dd: number;
    overfitting_score: number; fragility: number;
  }>;
}

interface StrategyCard {
  id: string; name: string; type: string;
  regime: string; priority: number; decay_risk: string;
}

interface DeadEnd {
  title: string; date: string; root_cause: string;
}

/* ── SVG Components ── */

function ClosedLoopSVG({ hoveredIdx, onHover, liveLabels }: { hoveredIdx: number | null; onHover: (i: number | null) => void; liveLabels: string[] }) {
  const cx = 320, cy = 320, R = 215, nodeR = 60;
  const positions = LOOP_NODES.map((_, i) => {
    const a = -Math.PI / 2 + (i * 2 * Math.PI) / 6;
    return { x: cx + R * Math.cos(a), y: cy + R * Math.sin(a) };
  });

  return (
    <svg viewBox="0 0 640 640" className="loop-svg" role="img" aria-label="Closed loop diagram">
      <defs>
        <radialGradient id="centerGlow" cx="50%" cy="50%" r="50%">
          <stop offset="0%" stopColor="var(--emerald)" stopOpacity="0.18" />
          <stop offset="100%" stopColor="var(--emerald)" stopOpacity="0" />
        </radialGradient>
        <marker id="arrowHead" viewBox="0 0 10 10" refX="6" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
          <path d="M 0 0 L 10 5 L 0 10 z" fill="var(--emerald-dim)" />
        </marker>
      </defs>
      <circle cx={cx} cy={cy} r={R - 12} fill="url(#centerGlow)" />
      <circle cx={cx} cy={cy} r={R} fill="none" stroke="var(--border)" strokeWidth="1" strokeDasharray="2 8" />
      {positions.map((p, i) => {
        const next = positions[(i + 1) % 6];
        return (
          <path key={`arc-${i}`} d={`M ${p.x} ${p.y} A ${R} ${R} 0 0 1 ${next.x} ${next.y}`}
            fill="none" stroke="var(--emerald-dim)" strokeWidth="1.5" opacity="0.6" />
        );
      })}
      <g transform={`translate(${cx}, ${cy})`}>
        <circle r="78" fill="var(--surface-0)" stroke="var(--emerald-dim)" />
        <circle r="78" fill="none" stroke="var(--emerald)" strokeWidth="0.5" opacity="0.4" />
        <text textAnchor="middle" y="-12" className="loop-center-eyebrow">GOVERNED BY</text>
        <text textAnchor="middle" y="10" className="loop-center-title">SOULCONTRACT</text>
        <text textAnchor="middle" y="30" className="loop-center-sub">16 invariants · enforced</text>
      </g>
      {LOOP_NODES.map((node, i) => {
        const p = positions[i];
        const active = hoveredIdx === i;
        return (
          <g key={node.key} transform={`translate(${p.x}, ${p.y})`}
            onMouseEnter={() => onHover(i)} onMouseLeave={() => onHover(null)}
            style={{ cursor: "pointer" }}>
            <circle r={nodeR + 6} fill="none" stroke="var(--emerald)" strokeWidth="1.5"
              opacity={active ? 0.5 : 0} style={{ transition: "opacity 0.3s" }} />
            <circle r={nodeR} fill={active ? "oklch(18% 0.04 160)" : "var(--surface-0)"}
              stroke={active ? "var(--emerald)" : "var(--border)"}
              strokeWidth={active ? 2 : 1} style={{ transition: "all 0.25s" }} />
            <text textAnchor="middle" y="-10" className="loop-node-eyebrow">{String(i + 1).padStart(2, "0")}</text>
            <text textAnchor="middle" y="10" className="loop-node-title">{node.label}</text>
            <text textAnchor="middle" y="28" className="loop-node-sub">{liveLabels[i] ?? node.desc}</text>
          </g>
        );
      })}
    </svg>
  );
}

function PnlSparkline({ trades }: { trades: TraderState["trade_history"] }) {
  const W = 720, H = 180, PAD_X = 14, PAD_Y = 22;
  const series = useMemo(() => {
    if (!trades || trades.length === 0) return [] as number[];
    const out: number[] = [0];
    let acc = 0;
    for (const t of trades) { acc += t.pnl_pct; out.push(acc); }
    return out;
  }, [trades]);

  if (series.length < 2) {
    return (
      <div className="sparkline-empty">
        <div className="sparkline-empty-glyph">⏷</div>
        <div className="sparkline-empty-title">Awaiting first closed trade</div>
        <div className="sparkline-empty-sub">
          The trader is watching SOL/USDT live. Cumulative PnL appears here the moment the first position closes.
        </div>
      </div>
    );
  }

  const min = Math.min(...series, 0), max = Math.max(...series, 0);
  const range = max - min || 1;
  const xy = series.map((v, i) => {
    const x = PAD_X + (i / (series.length - 1)) * (W - 2 * PAD_X);
    const y = H - PAD_Y - ((v - min) / range) * (H - 2 * PAD_Y);
    return [x, y] as const;
  });
  const path = xy.map(([x, y], i) => `${i ? "L" : "M"} ${x.toFixed(1)} ${y.toFixed(1)}`).join(" ");
  const last = xy[xy.length - 1];
  const area = `${path} L ${last[0]} ${H - PAD_Y} L ${PAD_X} ${H - PAD_Y} Z`;
  const final = series[series.length - 1];
  const color = final >= 0 ? "var(--emerald)" : "var(--coral)";
  const zeroY = H - PAD_Y - ((0 - min) / range) * (H - 2 * PAD_Y);

  return (
    <svg viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" className="sparkline-svg">
      <line x1={PAD_X} x2={W - PAD_X} y1={zeroY} y2={zeroY} stroke="var(--border)" strokeDasharray="2 6" />
      <path d={area} fill={color} fillOpacity="0.10" />
      <path d={path} fill="none" stroke={color} strokeWidth="2" strokeLinejoin="round" strokeLinecap="round" />
      {xy.map(([x, y], i) => (
        <circle key={i} cx={x} cy={y} r={i === xy.length - 1 ? 3.5 : 2} fill={color} />
      ))}
      <text x={last[0] - 6} y={last[1] - 8} className="sparkline-final" fill={color} textAnchor="end">
        {final >= 0 ? "+" : ""}{final.toFixed(2)}%
      </text>
    </svg>
  );
}

/* ── Main Page ── */

export default function Home() {
  const { publicKey, connected } = useWallet();
  const { connection } = useConnection();

  const [treasurySol, setTreasurySol] = useState<number | null>(null);
  const [walletSol, setWalletSol] = useState<number | null>(null);
  const [cycle, setCycle] = useState<CycleData | null>(null);
  const [liveness, setLiveness] = useState<LivenessData | null>(null);
  const [trader, setTrader] = useState<TraderState | null>(null);
  const [yieldReceived, setYieldReceived] = useState<number | null>(null);
  const [yieldLoading, setYieldLoading] = useState(false);
  const [isFrozen, setIsFrozen] = useState(false);
  const [night, setNight] = useState<NightData | null>(null);
  const [strategies, setStrategies] = useState<StrategyCard[]>([]);
  const [deadEnds, setDeadEnds] = useState<DeadEnd[]>([]);
  const [hoveredIdx, setHoveredIdx] = useState<number | null>(null);

  // ── Treasury PDA balance (devnet) ──
  useEffect(() => {
    let alive = true;
    const poll = async () => {
      try {
        const lamports = await connection.getBalance(new PublicKey(TREASURY_PDA));
        if (alive) setTreasurySol(lamports / LAMPORTS_PER_SOL);
      } catch { /* retry on next poll */ }
    };
    poll();
    const id = setInterval(poll, 10_000);
    return () => { alive = false; clearInterval(id); };
  }, [connection]);

  // ── Connected wallet balance (mainnet) ──
  useEffect(() => {
    if (!publicKey) return;
    let alive = true;
    const poll = async () => {
      try {
        const res = await fetch(MAINNET_RPC, {
          method: "POST", headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ jsonrpc: "2.0", id: 1, method: "getBalance", params: [publicKey.toBase58()] }),
        });
        const json = await res.json();
        const lamports: number = json?.result?.value ?? 0;
        if (alive) setWalletSol(lamports / LAMPORTS_PER_SOL);
      } catch { /* retry */ }
    };
    poll();
    const id = setInterval(poll, 15_000);
    return () => { alive = false; clearInterval(id); };
  }, [publicKey]);

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
        const res = await fetch("/api/trader-status");
        if (res.ok) { const data: TraderState = await res.json(); if (data.wallet && alive) setTrader(data); }
      } catch {}
    };
    poll();
    const id = setInterval(poll, 15_000);
    return () => { alive = false; clearInterval(id); };
  }, []);

  // ── Fetch night/strategies/deadends ──
  useEffect(() => {
    (async () => {
      try { const r = await fetch("/data/night.json"); if (r.ok) { const d = await r.json(); if (!d.error) setNight(d); } } catch {}
      try { const r = await fetch("/data/strategy-library.json"); if (r.ok) setStrategies(await r.json()); } catch {}
      try { const r = await fetch("/data/dead-ends.json"); if (r.ok) setDeadEnds(await r.json()); } catch {}
    })();
  }, []);

  // ── Program liveness ──
  useEffect(() => {
    let alive = true;
    const check = async () => {
      try {
        const res = await fetch("https://api.devnet.solana.com", {
          method: "POST", headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ jsonrpc: "2.0", id: 1, method: "getAccountInfo",
            params: ["8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB", { encoding: "base64" }] }),
        });
        const json = await res.json();
        const value = json?.result?.value;
        if (alive) setLiveness({
          programId: "8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB",
          live: value !== null && value !== undefined,
          executable: value?.executable ?? false,
          slot: json?.result?.context?.slot ?? null,
        });
      } catch {}
    };
    check();
    const id = setInterval(check, 30_000);
    return () => { alive = false; clearInterval(id); };
  }, []);

  // ── Treasury frozen state ──
  useEffect(() => {
    let alive = true;
    const check = async () => {
      try {
        const devnetConn = new (await import("@solana/web3.js")).Connection(DEVNET_RPC, "confirmed");
        const state = await fetchTreasuryState(devnetConn, TREASURY_AUTHORITY);
        if (alive) setIsFrozen(state.isFrozen);
      } catch {}
    };
    check();
    const id = setInterval(check, 30_000);
    return () => { alive = false; clearInterval(id); };
  }, []);

  // ── Yield received ──
  useEffect(() => {
    if (!publicKey) { setYieldReceived(null); return; }
    let cancelled = false;
    setYieldLoading(true);
    (async () => {
      try {
        const devnetConn = new (await import("@solana/web3.js")).Connection(DEVNET_RPC, "confirmed");
        const treasuryPubkey = new PublicKey(TREASURY_PDA);
        const walletStr = publicKey.toBase58();
        const signatures = await devnetConn.getSignaturesForAddress(treasuryPubkey, { limit: 20 });
        let totalYieldLamports = 0;
        for (const sigInfo of signatures) {
          if (cancelled) break;
          try {
            const tx = await devnetConn.getTransaction(sigInfo.signature, { maxSupportedTransactionVersion: 0 });
            if (!tx || !tx.meta) continue;
            const { preBalances, postBalances } = tx.meta;
            const accountKeys = tx.transaction.message.staticAccountKeys
              ? tx.transaction.message.staticAccountKeys
              : (tx.transaction.message as { accountKeys: PublicKey[] }).accountKeys;
            for (let i = 0; i < accountKeys.length; i++) {
              const key = accountKeys[i] instanceof PublicKey ? (accountKeys[i] as PublicKey).toBase58() : String(accountKeys[i]);
              if (key === walletStr) {
                const delta = (postBalances[i] ?? 0) - (preBalances[i] ?? 0);
                if (delta > 0) totalYieldLamports += delta;
              }
            }
          } catch {}
        }
        if (!cancelled) { setYieldReceived(totalYieldLamports / LAMPORTS_PER_SOL); setYieldLoading(false); }
      } catch { if (!cancelled) { setYieldReceived(null); setYieldLoading(false); } }
    })();
    return () => { cancelled = true; };
  }, [publicKey]);

  /* ── Derived ── */

  const totalPnlPct = useMemo(
    () => trader?.trade_history?.reduce((a, t) => a + t.pnl_pct, 0) ?? 0, [trader]
  );
  const winRate = useMemo(() => {
    if (!trader || trader.trade_history.length === 0) return null;
    return (trader.trade_history.filter((t) => t.pnl_pct > 0).length / trader.trade_history.length) * 100;
  }, [trader]);

  const lastTrade = trader?.trade_history?.slice(-1)[0];
  const traderStatus =
    trader == null ? "connecting" :
    trader.open_position ? "in_position" : "watching";

  const liveLabels = [
    night ? `${night.num_folds}-fold WFA` : "30K configs",
    "typed JSON",
    cycle ? (cycle.used_llm ? cycle.model_label : "deterministic") : "—",
    cycle ? `${cycle.mutations_accepted.length}/${cycle.mutations_accepted.length + cycle.mutations_rejected.length} pass` : "—",
    trader ? (trader.open_position ? "POSITION OPEN" : `${trader.candle_count} candles`) : "—",
    lastTrade ? `${lastTrade.pnl_pct >= 0 ? "+" : ""}${lastTrade.pnl_pct.toFixed(2)}%` : "first trade pending",
  ];

  /* ── Render ── */

  return (
    <div className="page">
      <Topbar activePage="dashboard" />

      {/* Emergency freeze banner */}
      {isFrozen && (
        <div style={{
          background: "#dc2626", color: "#fff", padding: "10px 24px",
          textAlign: "center", fontSize: "0.875rem", fontWeight: 600,
          letterSpacing: "0.04em",
        }}>
          TREASURY FROZEN — All operations halted by authority. Unfreeze requires 2-of-3 multisig approval + 24h time lock.
        </div>
      )}

      {/* ════════ HERO ════════ */}
      <section className="hero" style={{ marginBottom: 0 }}>
        <div className="hero-image-wrap">
          <img src="/bg-flower.jpg" alt="Ethereal flower in emerald and coral — the organic intelligence that drives RTP" />
        </div>

        <div className="hero-content">
          <div className="hero-copy">
            <span className="hero-label">SOLANA-NATIVE · AUTONOMOUS YIELD · SELF-FUNDING</span>
            <h1 className="hero-title">
              Every token gets a
              <br />
              program-enforced treasury
            </h1>
            <p className="hero-subtitle">
              Token projects route trading fees to RTP → the swarm generates yield via on-chain perps → yield flows back to holders. 70/20/10 split, enforced on-chain. No RTP token. Pure infrastructure.
            </p>
          </div>

          <div className="sys2-hero-cta-row" style={{ marginBottom: "var(--space-lg)" }}>
            <Link href="/launch" className="sys2-cta-primary">
              Launch a token <span className="cta-badge-devnet">DEVNET</span>
            </Link>
            <Link href="/docs" className="sys2-cta-secondary">Read the docs →</Link>
          </div>

          {connected && publicKey && (yieldLoading || (yieldReceived !== null && yieldReceived > 0)) && (
            <div className="hero-yield">
              {yieldLoading ? (
                <span className="yield-text">Scanning treasury transactions...</span>
              ) : (
                <span className="yield-text">
                  You have received <strong>{yieldReceived?.toFixed(4)} SOL</strong> from RTP
                </span>
              )}
            </div>
          )}
        </div>

        {/* Vitals strip — full width */}
        <div className="sys2-vitals" style={{ gridColumn: "1 / -1" }}>
          <div className="sys2-vital">
            <div className="sys2-vital-label">Cumulative PnL</div>
            <div className={`sys2-vital-value ${totalPnlPct >= 0 ? "pos" : "neg"}`}>
              {totalPnlPct >= 0 ? "+" : ""}{totalPnlPct.toFixed(2)}%
            </div>
            <div className="sys2-vital-sub">{trader?.total_trades ?? 0} closed trades</div>
          </div>
          <div className="sys2-vital">
            <div className="sys2-vital-label">Treasury SOL</div>
            <div className="sys2-vital-value">
              {treasurySol !== null && treasurySol > 0 ? treasurySol.toFixed(4) : "—"}
            </div>
            <div className="sys2-vital-sub">{treasurySol !== null && treasurySol > 0 ? "6PYPAn...Q4Z" : "Devnet"}</div>
          </div>
          <div className="sys2-vital">
            <div className="sys2-vital-label">Mainnet TXs</div>
            <div className="sys2-vital-value">4</div>
            <div className="sys2-vital-sub">CPI + REST proofs</div>
          </div>
          <div className="sys2-vital">
            <div className="sys2-vital-label">Test coverage</div>
            <div className="sys2-vital-value">325<span className="sys2-vital-unit">+5</span></div>
            <div className="sys2-vital-sub">Rust unit + integration</div>
          </div>
          <div className="sys2-vital">
            <div className="sys2-vital-label">Calmar (validated)</div>
            <div className="sys2-vital-value">44.89</div>
            <div className="sys2-vital-sub"><a href="#pipeline" title="View research pedigree ↓" style={{ color: "inherit", borderBottom: "1px dotted var(--border)", textDecoration: "none" }}>SOL Survivor 2.69</a> · 9× lev</div>
          </div>
        </div>
      </section>

      {/* ════════ §1 LIVE TRADER CONSOLE ════════ */}
      <section className="sys2-section" id="live">
        <header className="sys2-sect-head">
          <div>
            <div className="sys2-sect-eyebrow">§1 · live console</div>
            <h2 className="sys2-sect-title">No human in the loop</h2>
          </div>
          <div className="sys2-sect-side">
            <span className={`sys2-status-pill ${traderStatus}`}>
              <span className="sys2-status-dot" />
              {traderStatus === "in_position" ? "Position open on mainnet" :
               traderStatus === "watching" ? "Flat — watching the tape" : "Connecting…"}
            </span>
          </div>
        </header>

        <div className="console-grid">
          <div className="console-card">
            <div className="console-card-eyebrow">CURRENT POSITION</div>
            {trader?.open_position ? (
              <>
                <div className="console-big">
                  SOL/USDT · LONG · 9×
                  <a href="#pipeline" style={{ display: "inline-block", fontSize: "0.625rem", marginLeft: "16px", color: "var(--text-tertiary)", borderBottom: "1px dotted var(--border)", textDecoration: "none", verticalAlign: "middle", transform: "translateY(-4px)" }} title="View research pedigree ↓">Survivor 2.69 ↓</a>
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
                  <a href="#pipeline" title="View research pedigree ↓" style={{ color: "var(--text-primary)", borderBottom: "1px dotted var(--border)", textDecoration: "none" }}>Survivor 2.69</a> enters when score &gt; 0.25 with 3+ bullish timeframes. The next valid signal
                  triggers a 9× SOL LONG of 20% capital. Stop-loss 2.7× ATR, take-profit 5.0× ATR, trailing 0.14× ATR.
                </div>
              </>
            )}
            <div className="console-foot">
              Last poll · <span className="mono">{trader?.last_poll?.slice(11, 19) ?? "—"}</span>
            </div>
          </div>

          <div className="console-card chart-card">
            <div className="console-card-eyebrow">CUMULATIVE PNL · ALL CLOSED TRADES</div>
            <PnlSparkline trades={trader?.trade_history ?? []} />
            <div className="chart-stats">
              <div className="chart-stat">
                <span className="chart-stat-val">{trader?.total_trades ?? 0}</span>
                <span className="chart-stat-lab">trades</span>
              </div>
              <div className="chart-stat">
                <span className={`chart-stat-val ${totalPnlPct >= 0 ? "pos" : "neg"}`}>
                  {totalPnlPct >= 0 ? "+" : ""}{totalPnlPct.toFixed(2)}%
                </span>
                <span className="chart-stat-lab">cumulative</span>
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
        <div className="trade-tape">
          <div className="trade-tape-head">
            <span>RECENT TAPE</span>
            <span className="mono trade-tape-sub">last {Math.min(8, trader?.trade_history?.length ?? 0)} trades</span>
          </div>
          {(trader?.trade_history?.length ?? 0) === 0 ? (
            <div className="trade-tape-empty">No tape yet. The first close will print here in real time.</div>
          ) : (
            <div className="trade-tape-rows">
              {[...(trader?.trade_history ?? [])].slice(-8).reverse().map((t, i) => (
                <div key={i} className={`trade-row ${t.pnl_pct >= 0 ? "pos" : "neg"}`}>
                  <span className="mono">SOL/USDT</span>
                  <span className="mono dim">${t.entry_price.toFixed(2)} → ${t.exit_price.toFixed(2)}</span>
                  <span className="trade-reason">{t.exit_reason}</span>
                  <span className="mono">${t.size_usd.toFixed(0)}</span>
                  <span className={`mono trade-pnl ${t.pnl_pct >= 0 ? "pos" : "neg"}`}>
                    {t.pnl_pct >= 0 ? "+" : ""}{t.pnl_pct.toFixed(2)}%
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
      </section>

      {/* ════════ §2 CLOSED LOOP ════════ */}
      <section className="sys2-section" id="loop">
        <header className="sys2-sect-head">
          <div>
            <div className="sys2-sect-eyebrow">§2 · the closed loop</div>
            <h2 className="sys2-sect-title">Self-correcting in six steps</h2>
            <p className="sys2-sect-lede">
              Research validates. The bridge marshals. The LLM proposes mutations. The gates reject anything
              outside the soulcontract bounds. The Treasury PDA executes on Solana. PnL feeds back into the
              next research cycle. The loop runs every six hours, indefinitely.
            </p>
          </div>
        </header>
        <div className="loop-stage">
          <ClosedLoopSVG hoveredIdx={hoveredIdx} onHover={setHoveredIdx} liveLabels={liveLabels} />
        </div>
        <div className="loop-legend">
          {LOOP_NODES.map((n, i) => (
            <div key={n.key} className={`loop-legend-item ${hoveredIdx === i ? "active" : ""}`}
              onMouseEnter={() => setHoveredIdx(i)} onMouseLeave={() => setHoveredIdx(null)}
              style={{ cursor: "pointer" }}>
              <span className="loop-legend-num">{String(i + 1).padStart(2, "0")}</span>
              <div>
                <div className="loop-legend-label">{n.label}</div>
                <div className="loop-legend-sub">{liveLabels[i]}</div>
              </div>
            </div>
          ))}
        </div>
      </section>

      {/* ════════ §3 ARCHITECTURE STACK ════════ */}
      <section className="sys2-section" id="architecture">
        <header className="sys2-sect-head">
          <div>
            <div className="sys2-sect-eyebrow">§3 · architecture</div>
            <h2 className="sys2-sect-title">Three layers, one invariant</h2>
            <p className="sys2-sect-lede">
              Agents propose. Constraints dispose. Every layer can fail open without bringing the others down,
              because the on-chain program is the only authority that can move funds.
            </p>
          </div>
        </header>

        <div className="arch2-stack">
          <div className="arch2-layer arch2-research">
            <div className="arch2-layer-side">
              <div className="arch2-layer-tag">PYTHON</div>
              <div className="arch2-layer-name">Research Layer</div>
              <div className="arch2-layer-sub">Where strategies earn the right to be executed</div>
            </div>
            <div className="arch2-layer-cells">
              <div className="arch2-cell">
                <div className="arch2-cell-title">Night Shift</div>
                <div className="arch2-cell-sub">30K configs · 9-fold WFA · Darwinian evolution · Monte Carlo + CPCV robustness</div>
              </div>
              <div className="arch2-cell">
                <div className="arch2-cell-title">Strategy Selector</div>
                <div className="arch2-cell-sub">LLM reads library + dead ends · picks 3 most promising candidates per cycle</div>
              </div>
              <div className="arch2-cell">
                <div className="arch2-cell-title">5 Strategy Plugins</div>
                <div className="arch2-cell-sub">S02 Breakout · S04 RSI Exhaustion · S06 Vol Squeeze · S10 Momentum · S13 ADX</div>
              </div>
            </div>
          </div>

          <div className="arch2-bridge">
            <span className="arch2-bridge-line" />
            <span className="arch2-bridge-label">bridge.rs · typed JSON · ExecutePermit payload</span>
            <span className="arch2-bridge-line" />
          </div>

          <div className="arch2-layer arch2-swarm">
            <div className="arch2-layer-side">
              <div className="arch2-layer-tag">RUST</div>
              <div className="arch2-layer-name">Swarm Runtime</div>
              <div className="arch2-layer-sub">Six wings under one Coordinator</div>
            </div>
            <div className="arch2-layer-cells wings">
              {[
                { name: "Trading", desc: "Flash Trade CPI · REST · PnL", live: true },
                { name: "Evolve", desc: "LLM proposer · gates · rollback", live: true },
                { name: "Audit", desc: "3-agent tribunal · consensus" },
                { name: "Security", desc: "Threats · rate limits · alerts" },
                { name: "Knowledge", desc: "JSON store · cross-wing graph" },
                { name: "Futureproof", desc: "Deprecation · heartbeat" },
              ].map((w) => (
                <div key={w.name} className={`arch2-wing ${w.live ? "live" : ""}`}>
                  <div className="arch2-wing-head">
                    <span className="arch2-wing-name">{w.name}</span>
                    {w.live && <span className="arch2-wing-pulse" />}
                  </div>
                  <div className="arch2-wing-desc">{w.desc}</div>
                </div>
              ))}
            </div>
            <div className="arch2-coord">
              <span className="arch2-coord-tag">COORDINATOR</span>
              Soulguard parses <code className="inline-code">SOULCONTRACT.md</code> and validates every message · 325 unit + 5 integration tests
            </div>
          </div>

          <div className="arch2-bridge">
            <span className="arch2-bridge-line" />
            <span className="arch2-bridge-label">invoke_signed · Treasury PDA seeds · no human key</span>
            <span className="arch2-bridge-line" />
          </div>

          <div className="arch2-layer arch2-onchain">
            <div className="arch2-layer-side">
              <div className="arch2-layer-tag">SOLANA · ANCHOR</div>
              <div className="arch2-layer-name">On-Chain Program</div>
              <div className="arch2-layer-sub">The only authority that can move funds</div>
            </div>
            <div className="arch2-layer-cells">
              <div className="arch2-cell">
                <div className="arch2-cell-title">Treasury PDA</div>
                <div className="arch2-cell-sub">Per-mint isolation · receives SOL fees · signs Flash Trade CPI · 70/20/10 redistribute</div>
              </div>
              <div className="arch2-cell">
                <div className="arch2-cell-title">Constitutional Invariants</div>
                <div className="arch2-cell-sub">PDA ownership · 20% position cap · phase irreversible · emergency freeze · zero-address rejection</div>
              </div>
              <div className="arch2-cell">
                <div className="arch2-cell-title">Strategy Lifecycle</div>
                <div className="arch2-cell-sub">Register → Live → hard stops (10% DD, 5 losses) → soft decay (3 strikes) → retire</div>
              </div>
            </div>
          </div>
        </div>

        <div className="rail-strip">
          <span className="rail-label">7 Railway services · all green</span>
          <div className="rail-pills">
            {["rtp-trader", "rtp-dashboard", "rtp-devnet-loop", "rtp-night-shift", "rtp-swarm-ci", "rtp-fee-crank", "rtp-promote-strategy"].map((svc) => (
              <span key={svc} className="rail-pill">
                <span className="rail-dot" />
                {svc.replace("rtp-", "")}
              </span>
            ))}
          </div>
        </div>
      </section>

      {/* ════════ §4 RESEARCH PIPELINE ════════ */}
      <section className="sys2-section" id="pipeline" style={{ marginTop: "var(--space-4xl)" }}>
        <header className="sys2-sect-head">
          <div>
            <div className="sys2-sect-eyebrow">§4 · research pipeline</div>
            <h2 className="sys2-sect-title">How a strategy earns the right to trade real SOL</h2>
            <p className="sys2-sect-lede">
              Every config goes through five gates before a single lamport moves. None of these are heuristics —
              they are codified in <code className="inline-code">research/promotion_criteria.py</code>.
            </p>
          </div>
        </header>

        <ol className="pipe2-steps">
          {[
            { n: "01", t: "Grid Search", m: "30,000", u: "configs swept per symbol per night", d: "Exhaustive sweep across signal threshold, TP/SL multipliers, trailing stop, hold time, alignment." },
            { n: "02", t: "Walk-Forward Validation", m: `${night?.num_folds ?? 9}`, u: "expanding-window folds · 36 days OOS each", d: "No look-ahead. Median OOS Sharpe wins, not mean. Each candidate tested on 9 independent windows." },
            { n: "03", t: "Darwinian Evolution", m: "5×50", u: "generations × population = 250 refined survivors", d: "Top survivors mutate and compete. Fragility is a penalty, not rejection: survivor *= 1/(1+fragility)." },
            { n: "04", t: "Overfitting Detection", m: "3", u: "independent checks — IS/OOS gap, fold consistency, fragility", d: "Monte Carlo drawdown over 10K paths + Combinatorial Purged CV with PBO. Anything fragile is dropped." },
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

        <div className="validated-card">
          <div className="validated-head">
            <span className="validated-tag">VALIDATED · LIVE ON MAINNET</span>
            <span className="validated-title">SOL/USDT Survivor 2.69 · 9× leverage</span>
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
      </section>

      {/* ════════ §5 STRATEGY INTELLIGENCE ════════ */}
      <section className="sys2-section" id="strategies">
        <header className="sys2-sect-head">
          <div>
            <div className="sys2-sect-eyebrow">§5 · strategy intelligence</div>
            <h2 className="sys2-sect-title">15 strategies catalogued · {deadEnds.length || 9} failures remembered</h2>
            <p className="sys2-sect-lede">
              The LLM consults the library and the dead-ends log before every exploration run. Failures are
              never repeated; the system learns by remembering what does not work.
            </p>
          </div>
        </header>

        <div className="intel-grid">
          <article className="intel-panel intel-active">
            <header className="intel-panel-head">
              <span className="intel-pill live">LIVE</span>
              <span className="intel-panel-title">Active Strategy</span>
            </header>
            <div className="intel-active-name">SOL/USDT · Survivor 2.69</div>
            <div className="intel-active-type">Multi-timeframe trend following · 9× leverage</div>
            <div className="intel-chips">
              {[["signal_threshold","0.25"],["tp_atr","5.0"],["sl_atr","2.7"],["trail_atr","0.14"],["min_alignment","3"],["max_hold","36h"]].map(([k, v]) => (
                <span key={k} className="intel-chip"><span className="dim">{k}</span>={v}</span>
              ))}
            </div>
            <div className="intel-active-status">
              <span className={`status-dot ${trader ? "live" : ""}`} />
              {trader ? trader.open_position ? "Position open on mainnet" : `Watching · ${trader.candle_count} candles · ${trader.total_trades} trades` : "Connecting to trader…"}
            </div>
          </article>

          <article className="intel-panel">
            <header className="intel-panel-head">
              <span className="intel-pill explore">{strategies.length || 15} STRATEGIES</span>
              <span className="intel-panel-title">Strategy Library</span>
            </header>
            <div className="intel-list">
              {strategies.map((s) => (
                <div key={s.id} className={`intel-row priority-${s.priority}`}>
                  <span className="intel-id mono">{s.id}</span>
                  <span className="intel-name">{s.name}</span>
                  <span className="intel-type" style={{ color: TYPE_COLORS[s.type] || "var(--text-muted)" }}>
                    {s.type.replace("risk_premium", "momentum").replace("_", " ")}
                  </span>
                  <span className="intel-regime mono">{REGIME_LABELS[s.regime] || s.regime}</span>
                </div>
              ))}
            </div>
          </article>

          <article className="intel-panel intel-dead">
            <header className="intel-panel-head">
              <span className="intel-pill dead">{deadEnds.length || 9} DEAD ENDS</span>
              <span className="intel-panel-title">Failure Memory</span>
            </header>
            <div className="intel-list">
              {deadEnds.map((d, i) => (
                <div key={i} className="intel-dead-row">
                  <div className="intel-dead-title">{d.title}</div>
                  <div className="intel-dead-meta">
                    <span className="intel-dead-cause">{d.root_cause}</span>
                    {d.date !== "unknown" && <span className="intel-dead-date mono">{d.date}</span>}
                  </div>
                </div>
              ))}
            </div>
            <div className="intel-dead-foot">Read before every exploration run · failures never repeated</div>
          </article>
        </div>
      </section>

      {/* ════════ §6 ON-CHAIN PROOF ════════ */}
      <section className="sys2-section" id="proof">
        <header className="sys2-sect-head">
          <div>
            <div className="sys2-sect-eyebrow">§6 · on-chain proof</div>
            <h2 className="sys2-sect-title">Real mainnet transactions, not testnet</h2>
            <p className="sys2-sect-lede">
              The Treasury PDA opens and closes Flash Trade positions on Solana mainnet via
              <code className="inline-code">invoke_signed</code>. No human keypair is involved in trading.
              Click any transaction to verify on Explorer.
            </p>
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
          <a href="https://github.com/tradewife/resilient-token-protocol"
            target="_blank" rel="noopener noreferrer" className="proof2-extra">Source on GitHub ↗</a>
        </div>
      </section>

      {/* ════════ §7 INTEGRATE CTA ════════ */}
      <section className="sys2-section sys2-cta-section" id="integrate">
        <div className="cta2-card">
          <div className="cta2-content">
            <div className="sys2-sect-eyebrow">§7 · integrate</div>
            <h2 className="cta2-title">One function call. A program-enforced treasury for any token.</h2>
            <p className="cta2-lede">
              No RTP token. No custody. No new wallet. The SDK registers a Token-2022 mint with its own
              Treasury PDA in a single call. Trading fees flow in. Yield flows out 70/20/10. The program is
              the only thing that can sign — by design.
            </p>
            <pre className="cta2-code"><code>{`import { registerWithRTP } from "@resilient-protocol/sdk";

const result = await registerWithRTP(connection, payer, {
  authority: payer.publicKey,
});

// result.treasuryPDA → program-owned, no human can sign for it`}</code></pre>
            <div className="cta2-actions">
              <Link href="/launch" className="sys2-cta-primary">
                Launch a token <span className="cta-badge-devnet">DEVNET</span>
              </Link>
              <Link href="/docs" className="sys2-cta-secondary">Read the docs →</Link>
            </div>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="vitals">
        <div className="vital">
          <span className="vital-value">
            8rt6yi...2RB{" "}
            {liveness && (
              <span className={`liveness-badge ${liveness.live ? "live" : "down"}`}>
                {liveness.live ? "● Live" : "● Recheck"}
              </span>
            )}
          </span>
          <span className="vital-label">Program ID (Devnet)</span>
        </div>
        <div className="vital">
          <span className="vital-value">6PYPAn...Q4Z</span>
          <span className="vital-label">Treasury PDA</span>
        </div>
        <div className="vital">
          <span className="vital-value">70 / 20 / 10</span>
          <span className="vital-label">Redistribution Split</span>
        </div>
        <div className="vital">
          <span className="vital-value">Sustenance</span>
          <span className="vital-label">Current Phase</span>
        </div>
        <div className="vital">
          <a className="vital-link" href="https://explorer.solana.com/tx/4RVehmPVpnFYHrsF6N64RjVh7mszRzKF9DQVHd8TUqBHwrnyDYavf3TnDYJC4b5PrJWVSubZkNuyVkF1oJzk71RT?cluster=devnet"
            target="_blank" rel="noopener noreferrer">On-Chain Proof ↗</a>
          <span className="vital-label">On-Chain Proof</span>
        </div>
        <div className="vital">
          <a className="vital-link" href="https://github.com/tradewife/resilient-token-protocol"
            target="_blank" rel="noopener noreferrer">Source on GitHub ↗</a>
          <span className="vital-label">Repository</span>
        </div>
        <div className="vital">
          <a className="vital-link" href={`https://explorer.solana.com/address/${TREASURY_PDA}?cluster=devnet`}
            target="_blank" rel="noopener noreferrer">Solana Explorer ↗</a>
          <span className="vital-label">Treasury</span>
        </div>
      </footer>
    </div>
  );
}
