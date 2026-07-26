#!/usr/bin/env node
// Set / clear RTP_TRADER_MIN_ALIGNMENT_OVERRIDE and RTP_TRADER_SIGNAL_THRESHOLD_OVERRIDE
// on the Railway rtp-trader service. Loosening-only (the Rust trader silently
// ignores values tighter than the validated WFA config).
//
// Usage:
//   node scripts/railway-trader-override.mjs set --min-alignment 2 --signal-threshold 0.2
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
  const out = { command: null, minAlignment: null, signalThreshold: null };
  const [, , cmd, ...rest] = argv;
  out.command = cmd;
  for (let i = 0; i < rest.length; i++) {
    const k = rest[i];
    if (k === '--min-alignment') out.minAlignment = rest[++i];
    else if (k === '--signal-threshold') out.signalThreshold = rest[++i];
  }
  return out;
}

(async () => {
  const args = parseArgs(process.argv);
  if (!args.command) {
    console.error('Usage: railway-trader-override.mjs <set|unset|show> [--min-alignment N] [--signal-threshold F]');
    process.exit(1);
  }
  const token = loadToken();
  if (args.command === 'show') {
    const vars = await listVars(token);
    const overrides = vars.filter(v => OVERRIDE_KEYS.includes(v.name));
    if (!overrides.length) console.log('(no override env vars set)');
    else for (const v of overrides) console.log(`${v.name}=${v.value}`);
    return;
  }
  if (args.command === 'unset') {
    for (const k of OVERRIDE_KEYS) {
      await upsert(token, k, '');
      console.log(`unset ${k}`);
    }
    console.log('Done. Redeploy rtp-trader to apply.');
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
    console.log('Done. Redeploy rtp-trader to apply — defaults revert automatically when env vars unset.');
    return;
  }
  console.error(`Unknown command: ${args.command}`);
  process.exit(1);
})().catch(e => { console.error(e.message ?? e); process.exit(1); });
