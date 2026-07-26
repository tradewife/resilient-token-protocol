#!/usr/bin/env node
// Pull recent deployment logs from Railway rtp-trader (or any service) for live
// operator debugging. Default: rtp-trader logs from the latest deployment.
//
// Usage:
//   node scripts/railway-logs.mjs                              # last 60 lines from rtp-trader
//   node scripts/railway-logs.mjs --service rtp-dashboard     # different service
//   node scripts/railway-logs.mjs --last 200 --filter OVERRIDE # filter to override events
//   node scripts/railway-logs.mjs --from aec992bc-...         # specific deployment id

import fs from 'node:fs';
import path from 'node:path';

const PROJECT_ID = '11004852-2ba7-46d9-aeb5-ab9558e965a0';
const ENVIRONMENT_ID = '986bee12-1028-4016-aa42-ba0a174233b4';
const GRAPHQL = 'https://backboard.railway.com/graphql/v2';

const SERVICE_IDS = {
  'rtp-trader':        '40456d7a-5dfe-4112-8cf3-9a2ae5e3a910',
  'rtp-dashboard':     'f44e64aa-81d0-429d-b3e5-605d72ef2778',
};

function loadToken() {
  if (process.env.RAILWAY_TOKEN) return process.env.RAILWAY_TOKEN.trim();
  const p = path.resolve(process.cwd(), '.secrets/railway-workspace-token');
  if (fs.existsSync(p)) return fs.readFileSync(p, 'utf8').trim();
  console.error('Railway token not found. Set RAILWAY_TOKEN or create .secrets/railway-workspace-token');
  process.exit(1);
}

async function gql(token, query, variables) {
  const r = await fetch(GRAPHQL, {
    method: 'POST',
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({ query, variables }),
  });
  const j = await r.json();
  if (j.errors) throw new Error(JSON.stringify(j.errors, null, 2));
  return j.data;
}

function parseArgs(argv) {
  const out = { service: 'rtp-trader', last: 60, filter: null, from: null };
  for (let i = 2; i < argv.length; i++) {
    const k = argv[i];
    if (k === '--service') out.service = argv[++i];
    else if (k === '--last') out.last = Number(argv[++i]);
    else if (k === '--filter') out.filter = argv[++i];
    else if (k === '--from') out.from = argv[++i];
  }
  return out;
}

(async () => {
  const args = parseArgs(process.argv);
  const sid = SERVICE_IDS[args.service];
  if (!sid) {
    console.error(`Unknown service: ${args.service}. Known: ${Object.keys(SERVICE_IDS).join(', ')}`);
    process.exit(1);
  }
  const token = loadToken();

  let deploymentId = args.from;
  if (!deploymentId) {
    const deps = await gql(token,
      `query($input:DeploymentListInput!){deployments(input:$input,first:1){edges{node{id status createdAt}}}}`,
      { input: { projectId: PROJECT_ID, serviceId: sid } });
    const node = deps?.deployments?.edges?.[0]?.node;
    if (!node) { console.error('No deployments found'); process.exit(1); }
    deploymentId = node.id;
    console.error(`# ${args.service} :: deployment ${node.id} (${node.status}) @ ${node.createdAt}`);
  }

  const logs = await gql(token,
    `query($d:String!){deploymentLogs(deploymentId:$d){message timestamp}}`,
    { d: deploymentId });
  const strip = (s) => s.replace(/\x1b\[[0-9;]*m/g, '').trim();
  let rows = logs.deploymentLogs || [];
  if (args.filter) rows = rows.filter(l => strip(l.message).includes(args.filter));
  rows = rows.slice(-args.last);
  for (const l of rows) {
    const ts = l.timestamp?.slice(0, 19) ?? '?';
    console.log(`[${ts}] ${strip(l.message).slice(0, 320)}`);
  }
})().catch(e => { console.error(e.message ?? e); process.exit(1); });
