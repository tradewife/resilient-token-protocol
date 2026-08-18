"use client";

import React, { useEffect, useState } from "react";
import {
  EChartsSankeyChart,
  type ChartConfig,
  type SankeyData,
} from "../components/evilcharts/charts/echarts-sankey-chart";
import { CHART_TOKENS } from "../lib/chartTheme";

/**
 * The offer, drawn: your mandate enters, the research wing manufactures,
 * the fixed gate suite decides, and only survivors reach your account.
 * Band widths carry the funnel truth — most manufactured configs never
 * ship. Animated column-cascade entrance via EvilCharts.
 *
 * Replaced the static axonometric SVG plate (RTP-ARCH-01).
 */

// Flow units are qualitative — the ratios tell the story (few survivors),
// mirroring the §3 pipeline copy (30,000 swept → validated survivors).
// Node names double as CSS custom-property keys (`--color-{key}-{n}`), so
// they must be identifier-safe — no spaces; the display text lives in
// each config `label`.
const FLOW_DATA: SankeyData = {
  nodes: [
    { name: "mandate" },
    { name: "research" },
    { name: "gates" },
    { name: "deployed" },
    { name: "rejected" },
  ],
  links: [
    { source: 0, target: 1, value: 100 },
    { source: 1, target: 2, value: 100 },
    { source: 2, target: 3, value: 16 },
    { source: 2, target: 4, value: 84 },
  ],
};

/** Label sets — the wide labels read on desktop; phones get one-word
    labels so the flow fits the viewport without scrolling or clipping. */
const FLOW_LABELS = {
  wide: {
    mandate: "Your mandate",
    research: "Research",
    gates: "Gates",
    deployed: "Deployed engine",
    rejected: "Does not ship",
  },
  narrow: {
    mandate: "Mandate",
    research: "Research",
    gates: "Gates",
    deployed: "Shipped",
    rejected: "Cut",
  },
} as const;

function flowConfig(labels: (typeof FLOW_LABELS)["wide"] | (typeof FLOW_LABELS)["narrow"]): ChartConfig {
  return {
    mandate: {
      label: labels.mandate,
      colors: {
        light: [CHART_TOKENS.textSecondary],
        dark: [CHART_TOKENS.textSecondary],
      },
    },
    research: {
      label: labels.research,
      colors: {
        light: [CHART_TOKENS.emeraldDim, CHART_TOKENS.emerald],
        dark: [CHART_TOKENS.emerald, CHART_TOKENS.emeraldBright],
      },
    },
    gates: {
      label: labels.gates,
      colors: {
        light: [CHART_TOKENS.emeraldDim],
        dark: [CHART_TOKENS.emeraldBright],
      },
    },
    deployed: {
      label: labels.deployed,
      colors: {
        light: [CHART_TOKENS.emerald],
        dark: [CHART_TOKENS.emeraldBright],
      },
    },
    rejected: {
      label: labels.rejected,
      colors: {
        light: [CHART_TOKENS.coralDim],
        dark: [CHART_TOKENS.coralMuted],
      },
    },
  };
}

export default function OfferFlow() {
  // The sankey is the one chart whose outside labels need real horizontal
  // room; below the phone breakpoint we swap to one-word labels and slimmer
  // nodes so the whole flow fits the viewport — no scroll, no clipping.
  const [narrow, setNarrow] = useState(false);
  useEffect(() => {
    const mq = window.matchMedia("(max-width: 810px)");
    const update = () => setNarrow(mq.matches);
    update();
    mq.addEventListener("change", update);
    return () => mq.removeEventListener("change", update);
  }, []);

  return (
    <figure className="offer-arch" aria-label="Pipeline flow: mandate through gates into a deployed engine">
      <EChartsSankeyChart
        className="offer-flow-chart h-[340px] w-full min-h-[280px]"
        data={FLOW_DATA}
        config={flowConfig(narrow ? FLOW_LABELS.narrow : FLOW_LABELS.wide)}
        nodeWidth={narrow ? 14 : 10}
        nodePadding={narrow ? 16 : 22}
        linkCurvature={0.5}
        align="justify"
      >
        <EChartsSankeyChart.Node isClickable radius={3}>
          {/* Outside labels hang into the next column on phones; inside
              labels center on each node so the flow never clips. */}
          <EChartsSankeyChart.NodeLabel position={narrow ? "inside" : "outside"} showValues={false} />
        </EChartsSankeyChart.Node>
        <EChartsSankeyChart.Link variant="gradient" />
        <EChartsSankeyChart.Tooltip />
      </EChartsSankeyChart>
      <figcaption className="offer-flow-caption">
        Only gate-passing survivors deploy. Width = volume of manufactured
        configs; most never clear the suite.
      </figcaption>
    </figure>
  );
}
