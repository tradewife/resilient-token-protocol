#!/usr/bin/env node
// Set / clear RTP_TRADER_MIN_ALIGNMENT_OVERRIDE and RTP_TRADER_SIGNAL_THRESHOLD_OVERRIDE
// on the Railway rtp-trader service. Loosening-only (the Rust trader silently
// ignores values tighter than the validated WFA config).
//
// Also manages two operational knobs (NOT loosening overrides):
//   RTP_TRADER_MIN_OPEN_COLLATERAL_LAMPORTS — fee-sane entry floor in
//     lamports (default 0.5 SOL). Fixed per-order costs (execution fee +
//     wrap ≈ 0.0012 SOL/RT) make sub-floor positions fee-negative.
//   RTP_GM_EXECUTION_FEE_LAMPORTS — keeper execution fee per GMTrade order
//     (venue floor 300k; probe ran 500k).
//
// Usage:
//   node scripts/railway-trader-override.mjs set --min-alignment 2 --signal-threshold 0.2
//   node scripts/railway-trader-override.mjs set --min-collateral-lamports 500000000 --execution-fee-lamports 400000
//   node scripts/railway-trader-override.mjs unset
//   node scripts/railway-trader-override.mjs show
//
// Reads RAILWAY_TOKEN (workspace token) from `.secrets/railway-workspace-token`
// or env. Does NOT trigger a deploy — call `scripts/railway-redeploy-trader.mjs`
// (or the GraphQL mutation in SESSION-CONTEXT.md) after.

import fs from 'node:fs';
import path from 'node:path';

const PROJECT_ID = '11004852-2ba7-46d9-aeb5-ab9558e965a0';
const ENVIRONMENT_ID = '986bee12-1028-4016-aa42-ba0a174233b4';
const SERVICE_ID = '40456d7a-5dfe-4112-8cf3-9a2ae5e3a910';
const GRAPHQL = 'https://backboard.railway.com/graphql/v2';

const OVERRIDE_KEYS = [
  'RTP_TRADER_MIN_ALIGNMENT_OVERRIDE',
  'RTP_TRADER_SIGNAL_THRESHOLD_OVERRIDE',
];
const OPERATIONAL_KEYS = [
  'RTP_TRADER_MIN_OPEN_COLLATERAL_LAMPORTS',
  'RTP_GM_EXECUTION_FEE_LAMPORTS',
];

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
    headers: {
      Authorization: `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ query, variables }),
  });
  const j = await r.json();
  if (j.errors) throw new Error(JSON.stringify(j.errors, null, 2));
  return j.data;
}

async function upsert(token, name, value) {
  return gql(token,
    `mutation($input:VariableUpsertInput!){variableUpsert(input:$input)}`,
    { input: {
        projectId: PROJECT_ID,
        environmentId: ENVIRONMENT_ID,
        serviceId: SERVICE_ID,
        name,
        value,
        skipDeploys: true,
    }});
}

async function listVars(token) {
  const data = await gql(token,
    `query($eid:String!,$sid:String!,$pid:String!){variablesForServiceDeployment(environmentId:$eid,serviceId:$sid,projectId:$pid)}`,
    { eid: ENVIRONMENT_ID, sid: SERVICE_ID, pid: PROJECT_ID });
  const obj = data?.variablesForServiceDeployment ?? {};
  return Object.entries(obj).map(([name, value]) => ({ name, value }));
}

function parseArgs(argv) {
  const out = {
    command: null,
    minAlignment: null,
    signalThreshold: null,
    minCollateralLamports: null,
    executionFeeLamports: null,
  };
  const [, , cmd, ...rest] = argv;
  out.command = cmd;
  for (let i = 0; i < rest.length; i++) {
    const k = rest[i];
    if (k === '--min-alignment') out.minAlignment = rest[++i];
    else if (k === '--signal-threshold') out.signalThreshold = rest[++i];
    else if (k === '--min-collateral-lamports') out.minCollateralLamports = rest[++i];
    else if (k === '--execution-fee-lamports') out.executionFeeLamports = rest[++i];
  }
  return out;
}

(async () => {
  const args = parseArgs(process.argv);
  if (!args.command) {
    console.error(
      'Usage: railway-trader-override.mjs <set|unset|show> [--min-alignment N] [--signal-threshold F] ' +
        '[--min-collateral-lamports N] [--execution-fee-lamports N]'
    );
    process.exit(1);
  }
  const token = loadToken();
  if (args.command === 'show') {
    const vars = await listVars(token);
    const tracked = vars.filter(v => [...OVERRIDE_KEYS, ...OPERATIONAL_KEYS].includes(v.name));
    if (!tracked.length) console.log('(no override/operational env vars set)');
    else for (const v of tracked) console.log(`${v.name}=${v.value}`);
    return;
  }
  if (args.command === 'unset') {
    for (const k of OVERRIDE_KEYS) {
      await upsert(token, k, '');
      console.log(`unset ${k}`);
    }
    console.log(
      'Done (loosening overrides only; operational keys RTP_TRADER_MIN_OPEN_COLLATERAL_LAMPORTS / ' +
        'RTP_GM_EXECUTION_FEE_LAMPORTS are left alone — set them explicitly to reset). ' +
        'Redeploy rtp-trader to apply.'
    );
    return;
  }
  if (args.command === 'set') {
    if (args.minAlignment !== null) {
      await upsert(token, 'RTP_TRADER_MIN_ALIGNMENT_OVERRIDE', String(args.minAlignment));
      console.log(`RTP_TRADER_MIN_ALIGNMENT_OVERRIDE=${args.minAlignment}`);
    }
    if (args.signalThreshold !== null) {
      await upsert(token, 'RTP_TRADER_SIGNAL_THRESHOLD_OVERRIDE', String(args.signalThreshold));
      console.log(`RTP_TRADER_SIGNAL_THRESHOLD_OVERRIDE=${args.signalThreshold}`);
    }
    if (args.minCollateralLamports !== null) {
      await upsert(token, 'RTP_TRADER_MIN_OPEN_COLLATERAL_LAMPORTS', String(args.minCollateralLamports));
      console.log(`RTP_TRADER_MIN_OPEN_COLLATERAL_LAMPORTS=${args.minCollateralLamports}`);
    }
    if (args.executionFeeLamports !== null) {
      await upsert(token, 'RTP_GM_EXECUTION_FEE_LAMPORTS', String(args.executionFeeLamports));
      console.log(`RTP_GM_EXECUTION_FEE_LAMPORTS=${args.executionFeeLamports}`);
    }
    console.log('Done. Redeploy rtp-trader to apply — defaults revert automatically when env vars unset.');
    return;
  }
  console.error(`Unknown command: ${args.command}`);
  process.exit(1);
})().catch(e => { console.error(e.message ?? e); process.exit(1); });
