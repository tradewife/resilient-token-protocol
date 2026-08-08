import { NextResponse } from "next/server";
import { promises as fs } from "fs";
import path from "path";

/**
 * Mandate Diagnostic intake endpoint.
 *
 * POST /api/diagnostic-intake — receives a mandate form submission.
 * Persists to a JSONL file (dashboard container fs) AND emits one
 * structured log line per submission so Railway deploy logs keep a
 * durable record. Container fs is ephemeral; the log line is the
 * backup of record until Stripe Payment Links front the flow.
 *
 * GET /api/diagnostic-intake — owner-facing list (guarded by a shared
 * secret header to keep client mandates out of public responses).
 */

const STORE_DIR = process.env.RTP_INTAKE_DIR || "/tmp/rtp-intake";
const STORE_FILE = path.join(STORE_DIR, "mandate-submissions.jsonl");
const ACCESS_SECRET = process.env.RTP_INTAKE_SECRET || null;

interface IntakePayload {
  name: string;
  email: string;
  capitalBand: string;
  objective: string;
  horizon: string;
  hardTarget: string;
  maxDrawdown: string;
  riskBudget: string;
  constraints: string;
  lossTolerance: string;
  venues: string;
  custody: string;
  reporting: string;
  cadence: string;
  existingStyles: string;
  regimes: string;
  otherContext: string;
  delivery: string;
  contact: string;
  deadline: string;
}

function sanitize(s: unknown): string {
  return typeof s === "string" ? s.slice(0, 2000).replace(/\r?\n/g, " ") : "";
}

export async function POST(request: Request) {
  let body: Partial<IntakePayload>;
  try {
    body = await request.json();
  } catch {
    return NextResponse.json({ error: "invalid JSON" }, { status: 400 });
  }

  const name = sanitize(body.name).trim();
  const email = sanitize(body.email).trim();
  if (!name || !email || !email.includes("@")) {
    return NextResponse.json({ error: "name and valid email required" }, { status: 400 });
  }

  const record = {
    received_at: new Date().toISOString(),
    name,
    email,
    capital_band: sanitize(body.capitalBand),
    objective: sanitize(body.objective),
    horizon: sanitize(body.horizon),
    hard_target: sanitize(body.hardTarget),
    max_drawdown: sanitize(body.maxDrawdown),
    risk_budget: sanitize(body.riskBudget),
    constraints: sanitize(body.constraints),
    loss_tolerance: sanitize(body.lossTolerance),
    venues: sanitize(body.venues),
    custody: sanitize(body.custody),
    reporting: sanitize(body.reporting),
    cadence: sanitize(body.cadence),
    existing_styles: sanitize(body.existingStyles),
    regimes: sanitize(body.regimes),
    other_context: sanitize(body.otherContext),
    delivery: sanitize(body.delivery),
    contact: sanitize(body.contact),
    deadline: sanitize(body.deadline),
  };

  const line = JSON.stringify(record);

  // Durable-in-logs record (Railway deploy logs)
  console.log(`[DIAGNOSTIC-INTAKE] ${line}`);

  // Best-effort file persistence (ephemeral container fs)
  try {
    await fs.mkdir(STORE_DIR, { recursive: true });
    await fs.appendFile(STORE_FILE, line + "\n", "utf8");
  } catch (err) {
    console.error(`[DIAGNOSTIC-INTAKE] file write failed: ${err}`);
  }

  return NextResponse.json({ ok: true });
}

export async function GET(request: Request) {
  // Fail closed: mandates contain sensitive client data. The list endpoint
  // requires RTP_INTAKE_SECRET to be configured and presented.
  if (!ACCESS_SECRET) {
    return NextResponse.json({ error: "not configured" }, { status: 403 });
  }
  const auth = request.headers.get("authorization") || "";
  if (auth !== `Bearer ${ACCESS_SECRET}`) {
    return NextResponse.json({ error: "unauthorized" }, { status: 401 });
  }
  try {
    const raw = await fs.readFile(STORE_FILE, "utf8");
    const submissions = raw
      .trim()
      .split("\n")
      .filter(Boolean)
      .map((l) => {
        try {
          return JSON.parse(l);
        } catch {
          return null;
        }
      })
      .filter(Boolean);
    return NextResponse.json({ count: submissions.length, submissions });
  } catch {
    return NextResponse.json({ count: 0, submissions: [] });
  }
}
