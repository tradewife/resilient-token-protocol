"use client";

import React, { useMemo } from "react";
import {
  EChartsAreaChart,
  type ChartConfig,
} from "../components/evilcharts/charts/echarts-area-chart";
import { CHART_TOKENS, emeraldSeries, coralSeries } from "../lib/chartTheme";
import {
  formatPnlPct,
  summarizeTradePnl,
  type ClosedTradeLike,
} from "../lib/tradePnl";

/** Trader history rows carry the exit reason for the tooltip. */
type ChartTrade = ClosedTradeLike & { exit_reason?: string };

/**
 * Equity curve chart — the animated replacement for the hand-rolled
 * PnlSparkline SVG. One point per closed trade; the y value is the
 * COMPOUNDED equity return (per-trade net % applied at the real capital
 * exposure: 20% of wallet × 9× leverage), net of measured GMTrade fees.
 * The x value carries the exit reason + exit price so the EvilCharts
 * tooltip (which renders the raw axis value) reads like a trade memo,
 * while the axis tick formatter keeps the printed labels short ("T3").
 */
export default function PnlChart({ trades }: { trades: ChartTrade[] }) {
  const { series, isProfit } = useMemo(() => {
    const { cumulativeEquity } = summarizeTradePnl(trades);
    // cumulativeEquity starts at 0; one point per closed trade after it.
    const rows = cumulativeEquity.map((pnl, i) => {
      const t = i === 0 ? null : trades[i - 1];
      const trade =
        t == null
          ? "Start"
          : `T${i} · ${t.exit_reason ?? "exit"} · closed $${t.exit_price.toFixed(2)}`;
      return { trade, pnl: Number(pnl.toFixed(3)) };
    });
    const last = cumulativeEquity[cumulativeEquity.length - 1];
    return { series: rows, isProfit: last >= 0 };
  }, [trades]);

  const config = useMemo(
    () =>
      ({
        pnl: {
          label: "Equity (%)",
          colors: isProfit ? emeraldSeries : coralSeries,
        },
      }) satisfies ChartConfig,
    [isProfit]
  );

  return (
    <EChartsAreaChart
      data={series}
      config={config}
      xDataKey="trade"
      className="pnl-chart h-[220px] w-full"
      curveType="smooth"
      animationType="left-to-right"
      chartOptions={{
        // Zero reference line — the sparkline's dashed baseline, kept.
        yAxis: {
          splitLine: {
            lineStyle: { color: CHART_TOKENS.textTertiary, opacity: 0.25 },
          },
        },
      }}
    >
      <EChartsAreaChart.Grid />
      <EChartsAreaChart.XAxis
        dataKey="trade"
        tickFormatter={(value) => {
          // "T3 · TrailingStop · closed $96.44" → "T3" on the axis.
          const m = /^(T\d+|Start)/.exec(value);
          return m ? m[1] : value;
        }}
      />
      <EChartsAreaChart.YAxis
        tickFormatter={(value) => formatPnlPct(value, 1)}
      />
      <EChartsAreaChart.Tooltip variant="default" cursor />
      <EChartsAreaChart.Area dataKey="pnl" variant="gradient" strokeWidth={1.5}>
        <EChartsAreaChart.Dot variant="default" />
        <EChartsAreaChart.ActiveDot variant="colored-border" />
      </EChartsAreaChart.Area>
    </EChartsAreaChart>
  );
}
