#!/usr/bin/env node
// Install the RTP_OPERATOR_API_SECRET on the Railway rtp-dashboard and
// rtp-trader services. This shared secret gates the dashboard's
// /api/clear-position route AND the trader's internal /clear-position
// endpoint. Both services need the same value.
//
// Usage:
//   node scripts/railway-operator-secret.mjs                  # generate + install
//   node scripts/railway-operator-secret.mjs --secret <value> # install a chosen value
//   node scripts/railway-operator-secret.mjs --rotate         # generate a new value
//   node scripts/railway-operator-secret.mjs --show           # show currently stored value
//   node scripts/railway-operator-secret.mjs --unset          # remove from both services
//
// Reads RAILWAY_TOKEN (workspace token) from `.secrets/railway-workspace-token`
// or env. Does NOT trigger a deploy — call scripts/railway-redeploy-trader.mjs
// and redeploy rtp-dashboard separately after.

import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";

const PROJECT_ID = "11004852-2ba7-46d9-aeb5-ab9558e965a0";
const ENVIRONMENT_ID = "986bee12-1028-4016-aa42-ba0a174233b4";
const DASHBOARD_SERVICE_ID = "f44e64aa-81d0-429d-b3e5-605d72ef2778";
const TRADER_SERVICE_ID = "40456d7a-5dfe-4112-8cf3-9a2ae5e3a910";
const GRAPHQL = "https://backboard.railway.com/graphql/v2";
const VAR_NAME = "RTP_OPERATOR_API_SECRET";
const SECRET_FILE = path.resolve(process.cwd(), ".secrets/rtp-operator-api-secret");

const TARGETS = [
  { name: "rtp-dashboard", id: DASHBOARD_SERVICE_ID },
  { name: "rtp-trader",    id: TRADER_SERVICE_ID },
];

function loadToken() {
  if (process.env.RAILWAY_TOKEN) return process.env.RAILWAY_TOKEN.trim();
  const p = path.resolve(process.cwd(), ".secrets/railway-workspace-token");
  if (fs.existsSync(p)) return fs.readFileSync(p, "utf8").trim();
  console.error(
    "Railway token not found. Set RAILWAY_TOKEN or create .secrets/railway-workspace-token"
  );
  process.exit(1);
}

async function gql(token, query, variables) {
  const r = await fetch(GRAPHQL, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ query, variables }),
  });
  const j = await r.json();
  if (j.errors) throw new Error(JSON.stringify(j.errors, null, 2));
  return j.data;
}

async function upsertVar(token, serviceId, name, value) {
  return gql(
    token,
    `mutation($input:VariableUpsertInput!) { variableUpsert(input: $input) }`,
    {
      input: {
        projectId: PROJECT_ID,
        environmentId: ENVIRONMENT_ID,
        serviceId,
        name,
        value,
        skipDeploys: true,
      },
    }
  );
}

async function deleteVar(token, serviceId, name) {
  return gql(
    token,
    `mutation($input:VariableDeleteInput!) { variableDelete(input: $input) }`,
    {
      input: {
        projectId: PROJECT_ID,
        environmentId: ENVIRONMENT_ID,
        serviceId,
        name,
      },
    }
  );
}

async function readVar(token, serviceId, name) {
  const data = await gql(
    token,
    `query($eid:String!,$sid:String!,$pid:String!){
       variablesForServiceDeployment(environmentId:$eid,serviceId:$sid,projectId:$pid)
     }`,
    { eid: ENVIRONMENT_ID, sid: serviceId, pid: PROJECT_ID }
  );
  const obj = data?.variablesForServiceDeployment ?? {};
  return obj[name] ?? null;
}

function generateSecret() {
  return crypto.randomBytes(36).toString("base64url");
}

function saveLocal(secret) {
  fs.mkdirSync(path.dirname(SECRET_FILE), { recursive: true });
  fs.writeFileSync(
    SECRET_FILE,
    `# Local copy of RTP_OPERATOR_API_SECRET for ${new Date().toISOString()}\n${secret}\n`,
    { mode: 0o600 }
  );
  console.log(`Saved local copy to ${SECRET_FILE} (gitignored)`);
}

function parseArgs(argv) {
  const out = { command: null, secret: null };
  for (let i = 2; i < argv.length; i++) {
    const k = argv[i];
    if (k === "--secret") out.secret = argv[++i];
    else if (k === "--rotate" || k === "--show" || k === "--unset") {
      out.command = k.replace(/^--/, "");
    }
  }
  return out;
}

(async () => {
  const args = parseArgs(process.argv);
  const token = loadToken();

  if (args.command === "show") {
    for (const t of TARGETS) {
      const v = await readVar(token, t.id, VAR_NAME);
      console.log(
        `${t.name.padEnd(16)} ${VAR_NAME}=${v === null ? "(unset)" : maskSecret(v)}`
      );
    }
    return;
  }

  if (args.command === "unset") {
    for (const t of TARGETS) {
      await deleteVar(token, t.id, VAR_NAME);
      console.log(`unset ${VAR_NAME} on ${t.name}`);
    }
    console.log(
      "Done. Redeploy rtp-dashboard and rtp-trader for the unset to take effect."
    );
    return;
  }

  let secret = args.secret;
  if (!secret) {
    secret = generateSecret();
    console.log(
      `Generated new secret (${secret.length} chars). Store it safely — it will not be shown in full again.`
    );
  }
  if (secret.length < 24) {
    console.error("Refusing to install a secret shorter than 24 characters.");
    process.exit(1);
  }

  for (const t of TARGETS) {
    await upsertVar(token, t.id, VAR_NAME, secret);
    console.log(`set ${VAR_NAME} on ${t.name}`);
  }
  saveLocal(secret);

  console.log("\nNext steps:");
  console.log("  1. Redeploy rtp-dashboard so it picks up the new env at startup.");
  console.log("  2. Run: node scripts/railway-redeploy-trader.mjs");
  console.log("  3. Verify: curl -i -X POST https://rtp-dashboard-production.up.railway.app/api/clear-position/  -> 401");
})();

function maskSecret(s) {
  if (s.length <= 8) return "****";
  return `${s.slice(0, 4)}…${s.slice(-4)} (${s.length} chars)`;
}
