#!/usr/bin/env npx tsx
/**
 * scripts/check-railway-services.ts
 *
 * Programmatically verify all RTP Railway services.
 * Usage: npx tsx scripts/check-railway-services.ts
 *
 * Reads Railway workspace token from .secrets/railway-workspace-token (gitignored).
 * Output: red/green table of service status and last deploy time.
 */

const RAILWAY_TOKEN_PATH = ".secrets/railway-workspace-token";
const RAILWAY_GRAPHQL    = "https://backboard.railway.com/graphql/v2";
const PROJECT_ID         = "11004852-2ba7-46d9-aeb5-ab9558e965a0";
const ENV_ID             = "986bee12-1028-4016-aa42-ba0a174233b4";

// Known service IDs (from `railway service list`)
const SERVICES = [
  { name: "rtp-dashboard",        id: "f44e64aa-81d0-429d-b3e5-605d72ef2778",
    appUrl: "https://www.resilientprotocol.xyz" },
  { name: "rtp-devnet-loop",       id: "006a2ac8-d74b-4f1d-ac4e-8a044ebfb46d",
    appUrl: "https://rtp-devnet-loop-production.up.railway.app" },
  { name: "rtp-fee-crank",         id: "64861fa4-fc52-4615-8e43-3e97341c48c9",
    appUrl: "" },
  { name: "rtp-night-shift",       id: "0088cfc9-1310-4926-ab7e-1e3991028ed9",
    appUrl: "https://rtp-night-shift-production.up.railway.app" },
  { name: "rtp-promote-strategy",  id: "dcda209e-f95c-458c-be8d-f75ea730b761",
    appUrl: "" },
  { name: "rtp-swarm-ci",          id: "ca591cef-4b09-4797-b80d-fbbb04098ce4",
    appUrl: "https://rtp-swarm-ci-production.up.railway.app" },
];

interface SvcRow {
  name: string;
  id: string;
  status: string | null;
  lastDeployAt: string | null;
  appUrl: string;
  serviceUrl: string;
}

async function gql(token: string, q: string, vars?: Record<string, unknown>) {
  const r = await fetch(RAILWAY_GRAPHQL, {
    method: "POST",
    headers: { Authorization: `Bearer ${token}`, "Content-Type": "application/json" },
    body: JSON.stringify({ query: q, variables: vars }),
  });
  if (!r.ok) throw new Error(`HTTP ${r.status}`);
  return r.json();
}

async function statusFor(svc: typeof SERVICES[0], token: string): Promise<SvcRow> {
  const q = `query($sid: String!, $eid: String!) {
    serviceInstance(serviceId: $sid, environmentId: $eid) {
      latestDeployment { status createdAt }
    }
  }`;
  const d = await gql(token, q, { sid: svc.id, eid: ENV_ID }) as any;
  const dep = d?.data?.serviceInstance?.latestDeployment;
  return {
    name:        svc.name,
    id:          svc.id,
    status:      dep?.status   ?? null,
    lastDeployAt: dep?.createdAt ?? null,
    appUrl:      svc.appUrl,
    serviceUrl:  `https://railway.app/project/${PROJECT_ID}/service/${svc.id}`,
  };
}

const BOLD  = "\x1b[1m";
const DIM   = "\x1b[2m";
const RESET = "\x1b[0m";

function color(s: string | null) {
  if (!s)                            return "\x1b[31m";
  if (s === "SUCCESS" || s === "RUNNING" || s === "COMPLETED") return "\x1b[32m";
  if (s === "BUILDING" || s === "DEPLOYING")                    return "\x1b[33m";
  return "\x1b[31m";
}

async function main() {
  let token: string;
  try {
    const { readFileSync } = await import("fs");
    token = readFileSync(RAILWAY_TOKEN_PATH, "utf-8").trim();
  } catch {
    console.error(`${BOLD}check-railway-services${RESET}: token not found at ${RAILWAY_TOKEN_PATH}`);
    console.error("Generate at railway.com/account/tokens → store in .secrets/railway-workspace-token");
    process.exit(1);
  }

  const rows = await Promise.all(SERVICES.map((s) => statusFor(s, token)));

  console.log(`\n${BOLD}RTP Railway — ${rows.length} services${RESET}\n`);
  console.log(`  ${BOLD}Service                  Status           Last Deploy        App URL${RESET}`);
  console.log("  " + "─".repeat(92));

  let notGreen = 0;
  for (const r of rows) {
    const c    = color(r.status);
    const icon = !r.status ? "?" :
                 r.status === "SUCCESS" || r.status === "RUNNING" || r.status === "COMPLETED" ? "✓" :
                 r.status === "BUILDING" || r.status === "DEPLOYING" ? "⟳" : "✗";

    const name   = r.name.padEnd(24);
    const status = `${c}${icon} ${(r.status ?? "null").padEnd(17)}${RESET}`;
    const deploy = r.lastDeployAt
      ? new Date(r.lastDeployAt).toLocaleString("en-US", { month: "short", day: "numeric",
          hour: "2-digit", minute: "2-digit" }).padEnd(20)
      : DIM + "—".padEnd(20) + RESET;
    const app    = r.appUrl || DIM + "—".padEnd(50) + RESET;

    console.log(`  ${name} ${status} ${deploy} ${app}`);
    if (r.status !== "SUCCESS" && r.status !== "RUNNING" && r.status !== "COMPLETED") notGreen++;
  }

  const green = rows.length - notGreen;
  console.log("");
  if (notGreen === 0) {
    console.log(`${BOLD}Result: ${green}/${rows.length} services green ✓${RESET}\n`);
  } else {
    console.log(`${BOLD}Result: ${green}/${rows.length} green — ${notGreen} need attention${RESET}\n`);
    process.exit(1);
  }
}

main().catch((e) => { console.error(e); process.exit(1); });
