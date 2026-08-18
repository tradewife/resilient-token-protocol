"use client";

import React from "react";
import {
  EChartsPieChart,
  type ChartConfig,
} from "../components/evilcharts/charts/echarts-pie-chart";
import { CHART_TOKENS } from "../lib/chartTheme";

/**
 * The 70 / 20 / 10 redistribution split, drawn as an animated donut.
 * Emerald carries the value-return flows; coral marks the ecosystem fund.
 */

// Keys double as CSS custom-property names (`--color-{key}-{n}`) — no
// spaces; the display text lives in each config `label`.
const SPLIT_DATA = [
  { name: "holders", share: 70 },
  { name: "dev", share: 20 },
  { name: "ecosystem", share: 10 },
];

const SPLIT_CONFIG = {
  holders: {
    label: "Holders · 70%",
    colors: {
      light: [CHART_TOKENS.emeraldDim],
      dark: [CHART_TOKENS.emeraldBright],
    },
  },
  dev: {
    label: "Project Dev · 20%",
    colors: {
      light: [CHART_TOKENS.emeraldDim],
      dark: [CHART_TOKENS.emerald],
    },
  },
  ecosystem: {
    label: "Ecosystem · 10%",
    colors: {
      light: [CHART_TOKENS.coralDim],
      dark: [CHART_TOKENS.coralMuted],
    },
  },
} satisfies ChartConfig;

export default function RedistributionDonut() {
  return (
    <div className="docs-redistribution-donut">
      <EChartsPieChart
        data={SPLIT_DATA}
        config={SPLIT_CONFIG}
        dataKey="share"
        nameKey="name"
        className="h-[260px] w-full"
      >
        <EChartsPieChart.Pie
          variant="gradient"
          innerRadius="58%"
          outerRadius="85%"
          cornerRadius={4}
          paddingAngle={2}
          isClickable
        />
        <EChartsPieChart.Legend align="center" verticalAlign="bottom" />
        <EChartsPieChart.Tooltip />
      </EChartsPieChart>
    </div>
  );
}
