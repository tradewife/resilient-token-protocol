#!/usr/bin/env node
// Trigger a redeploy of the Railway rtp-trader service. Use after changing
// env vars via railway-trader-override.mjs so the new env takes effect at
// process start (env vars are read once on startup).
//
// Usage:
//   node scripts/railway-redeploy-trader.mjs

import fs from 'node:fs';
import path from 'node:path';

const SERVICE_ID = '40456d7a-5dfe-4112-8cf3-9a2ae5e3a910';
const ENVIRONMENT_ID = '986bee12-1028-4016-aa42-ba0a174233b4';
const GRAPHQL = 'https://backboard.railway.com/graphql/v2';

function loadToken() {
  if (process.env.RAILWAY_TOKEN) return process.env.RAILWAY_TOKEN.trim();
  const p = path.resolve(process.cwd(), '.secrets/railway-workspace-token');
  if (fs.existsSync(p)) return fs.readFileSync(p, 'utf8').trim();
  console.error('Railway token not found.');
  process.exit(1);
}

(async () => {
  const token = loadToken();
  const r = await fetch(GRAPHQL, {
    method: 'POST',
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({
      query: `mutation($sid:String!,$eid:String!){serviceInstanceDeployV2(serviceId:$sid,environmentId:$eid)}`,
      variables: { sid: SERVICE_ID, eid: ENVIRONMENT_ID },
    }),
  });
  const j = await r.json();
  if (j.errors) {
    console.error(JSON.stringify(j.errors, null, 2));
    process.exit(1);
  }
  console.log(`Redeploy started: ${j.data.serviceInstanceDeployV2}`);
})();
