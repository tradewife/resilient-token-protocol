"use client";

import React, { useMemo } from "react";
import {
  EChartsBarChart,
  type ChartConfig,
} from "../components/evilcharts/charts/echarts-bar-chart";
import { CHART_TOKENS, emeraldSeries } from "../lib/chartTheme";
import type { NightData } from "./nightTypes";

/**
 * Night Shift rigour, drawn: the top surviving candidates from the last
 * pipeline run (`/data/night.json`), ranked by survivor score. The
 * tallest column — the deployed Survivor — is highlighted automatically;
 * the x category carries the OOS stats so the EvilCharts tooltip reads
 * like a validation memo.
 */
export default function NightShiftChart({ night }: { night: NightData | null }) {
  const rows = useMemo(() => {
    if (!night?.top_candidates?.length) return [];
    return night.top_candidates
      .slice()
      .sort((a, b) => b.survivor_score - a.survivor_score)
      .map((c, i) => ({
        candidate: `C${i + 1} · Sharpe ${c.oos_sharpe.toFixed(2)} · DD ${c.oos_max_dd.toFixed(1)}%`,
        score: Number(c.survivor_score.toFixed(2)),
      }));
  }, [night]);

  if (rows.length === 0) return null;

  const config = {
    score: {
      label: "Survivor score",
      colors: emeraldSeries,
    },
  } satisfies ChartConfig;

  return (
    <div className="nightshift-chart-wrap">
      <div className="console-card-eyebrow">
        LAST NIGHT SHIFT · TOP {rows.length} SURVIVORS · RANKED BY SURVIVOR SCORE
      </div>
      <EChartsBarChart
        data={rows}
        config={config}
        xDataKey="candidate"
        className="nightshift-chart h-[240px] w-full"
        layout="vertical"
        barRadius={3}
        enableMaxValueHighlight
      >
        <EChartsBarChart.Grid />
        <EChartsBarChart.XAxis
          dataKey="candidate"
          tickFormatter={(value) => /^(C\d+)/.exec(value)?.[1] ?? value}
        />
        <EChartsBarChart.Tooltip />
        <EChartsBarChart.Bar dataKey="score" variant="gradient" glowing />
      </EChartsBarChart>
      <div className="nightshift-chart-caption" style={{ color: CHART_TOKENS.textTertiary }}>
        Hover a bar for its out-of-sample Sharpe and max drawdown. The lit
        column is the deployed engine.
      </div>
    </div>
  );
}
