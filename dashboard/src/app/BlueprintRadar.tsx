"use client";

import React, { useMemo } from "react";
import {
  EChartsRadarChart,
  type ChartConfig,
} from "../components/evilcharts/charts/echarts-radar-chart";
import { emeraldSeries } from "../lib/chartTheme";

interface BlueprintRadarProps {
  onChainReadiness: number;
  riskTolerance: number;
  complexityAppetite: number;
  commitmentReadiness: number;
}

/**
 * The personalised blueprint profile drawn as a radar — one polygon over
 * the four scored axes, growing out of the centre on reveal. Replaces the
 * flat score bars in the profile modal.
 */
export default function BlueprintRadar(props: BlueprintRadarProps) {
  // One row per radar spoke; the "you" series holds every axis value.
  const data = useMemo(
    () => [
      { axis: "On-chain readiness", you: props.onChainReadiness },
      { axis: "Risk tolerance", you: props.riskTolerance },
      { axis: "Complexity appetite", you: props.complexityAppetite },
      { axis: "Commitment", you: props.commitmentReadiness },
    ],
    [props.onChainReadiness, props.riskTolerance, props.complexityAppetite, props.commitmentReadiness]
  );

  const config = {
    you: {
      label: "Your blueprint",
      colors: emeraldSeries,
    },
  } satisfies ChartConfig;

  return (
    <div className="blueprint-radar">
      <EChartsRadarChart
        data={data}
        config={config}
        className="h-[280px] w-full"
      >
        <EChartsRadarChart.PolarGrid gridType="polygon" />
        <EChartsRadarChart.PolarAngleAxis dataKey="axis" />
        <EChartsRadarChart.Tooltip />
        <EChartsRadarChart.Radar dataKey="you" variant="filled" fillOpacity={0.25}>
          <EChartsRadarChart.Dot variant="colored-border" />
          <EChartsRadarChart.ActiveDot variant="default" />
        </EChartsRadarChart.Radar>
      </EChartsRadarChart>
    </div>
  );
}
