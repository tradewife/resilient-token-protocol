import { NextResponse } from "next/server";
import {
  dbPathInUse,
  insertLead,
  leadCount,
  listLeads,
  type IntakeKind,
} from "@/lib/intake-store";
import { notifyConfigSummary, notifyLead } from "@/lib/intake-notify";

/**
 * Mandate / Compatibility intake endpoint.
 *
 * POST /api/diagnostic-intake — receives either:
 *   - kind: "compatibility_v5"  — scorecard funnel lead (5 Qs + email)
 *   - kind: "mandate_intake"     — full paid advisory terms form
 *   - (omitted / legacy)        — treated as mandate_intake
 *
 * Persistence:
 *   1. SQLite on Railway volume (RTP_INTAKE_DB_PATH, default /data/intake.sqlite)
 *   2. Structured log line [DIAGNOSTIC-INTAKE] for Railway deploy logs
 *   3. Email notify to katejcooper.atelier@gmail.com via Resend (RESEND_API_KEY)
 *
 * GET /api/diagnostic-intake — owner-facing list (Bearer RTP_INTAKE_SECRET).
 */

const ACCESS_SECRET = process.env.RTP_INTAKE_SECRET || null;

function sanitize(s: unknown): string {
  return typeof s === "string" ? s.slice(0, 2000).replace(/\r?\n/g, " ") : "";
}

export async function POST(request: Request) {
  let body: Record<string, unknown>;
  try {
    body = (await request.json()) as Record<string, unknown>;
  } catch {
    return NextResponse.json({ error: "invalid JSON" }, { status: 400 });
  }

  const name = sanitize(body.name).trim();
  const email = sanitize(body.email).trim();
  if (!name || !email || !email.includes("@")) {
    return NextResponse.json(
      { error: "name and valid email required" },
      { status: 400 }
    );
  }

  const kindRaw = sanitize(body.kind).trim().toLowerCase();
  const kind: IntakeKind =
    kindRaw === "compatibility_v5" ||
    kindRaw === "compat_v5" ||
    kindRaw === "scorecard"
      ? "compatibility_v5"
      : "mandate_intake";

  // Flatten known fields into a stable payload map (snake_case keys).
  const payload: Record<string, string> = {
    current_situation: sanitize(body.currentSituation),
    desired_outcome: sanitize(body.desiredOutcome),
    patience_mindset: sanitize(body.patienceMindset),
    expected_horizon: sanitize(body.expectedHorizon),
    solution_model: sanitize(body.solutionModel),
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

  let lead;
  try {
    lead = insertLead({ kind, name, email, payload });
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.error(`[DIAGNOSTIC-INTAKE] sqlite insert failed: ${msg}`);
    // Still emit the log line so the lead is not totally lost.
    console.log(
      `[DIAGNOSTIC-INTAKE] ${JSON.stringify({
        received_at: new Date().toISOString(),
        kind,
        name,
        email,
        ...payload,
        persist_error: msg,
      })}`
    );
    return NextResponse.json(
      { error: "failed to persist lead" },
      { status: 500 }
    );
  }

  // Durable-in-logs backup (Railway deploy logs)
  console.log(
    `[DIAGNOSTIC-INTAKE] ${JSON.stringify({
      id: lead.id,
      received_at: lead.received_at,
      kind: lead.kind,
      name: lead.name,
      email: lead.email,
      ...lead.payload,
      db: dbPathInUse(),
    })}`
  );

  // Email Kate — best-effort; never fails the request after durable write.
  const notify = await notifyLead(lead);

  return NextResponse.json({
    ok: true,
    kind: lead.kind,
    id: lead.id,
    notified: notify.ok,
  });
}

export async function GET(request: Request) {
  // Fail closed: leads contain personal data.
  if (!ACCESS_SECRET) {
    return NextResponse.json({ error: "not configured" }, { status: 403 });
  }
  const auth = request.headers.get("authorization") || "";
  if (auth !== `Bearer ${ACCESS_SECRET}`) {
    return NextResponse.json({ error: "unauthorized" }, { status: 401 });
  }

  try {
    const url = new URL(request.url);
    const limit = Number(url.searchParams.get("limit") || "200");
    const submissions = listLeads(limit);
    const notify = notifyConfigSummary();
    return NextResponse.json({
      count: leadCount(),
      db: dbPathInUse(),
      notify,
      submissions,
    });
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.error(`[DIAGNOSTIC-INTAKE] list failed: ${msg}`);
    return NextResponse.json({ error: "list failed", detail: msg }, { status: 500 });
  }
}
