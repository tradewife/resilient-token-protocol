import { NextResponse } from "next/server";
import {
  dbPathInUse,
  insertLead,
  leadCount,
  listLeads,
  type IntakeKind,
} from "@/lib/intake-store";
import { notifyConfigSummary, notifyLead } from "@/lib/intake-notify";
import { computeBlueprintProfile } from "@/lib/blueprint-scoring";
import type { BlueprintAnswers } from "@/lib/blueprint-scoring";

/**
 * Mandate / Compatibility / Blueprint intake endpoint.
 *
 * POST /api/diagnostic-intake — receives either:
 *   - kind: "compatibility_v5"  — scorecard funnel lead (5 Qs + email)
 *   - kind: "mandate_intake"     — full paid advisory terms form
 *   - kind: "blueprint_v1"       — Resilience Blueprint (12 Qs + scores)
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
const MAX_BODY_BYTES = 32_768;
const RATE_LIMIT_WINDOW_MS = 10 * 60 * 1000;
const RATE_LIMIT_MAX = 5;
const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

type RateLimitBucket = {
  count: number;
  resetAt: number;
};

const rateLimitBuckets = new Map<string, RateLimitBucket>();

function sanitize(s: unknown, max = 2000): string {
  return typeof s === "string" ? s.slice(0, max).replace(/\r?\n/g, " ") : "";
}

function clientIp(request: Request): string {
  const forwarded = request.headers.get("x-forwarded-for");
  if (forwarded) return forwarded.split(",")[0]?.trim() || "unknown";
  return (
    request.headers.get("cf-connecting-ip") ||
    request.headers.get("x-real-ip") ||
    "unknown"
  );
}

function checkRateLimit(request: Request): { ok: true } | { ok: false; retryAfter: number } {
  const now = Date.now();
  const key = clientIp(request);
  const current = rateLimitBuckets.get(key);
  if (!current || current.resetAt <= now) {
    rateLimitBuckets.set(key, {
      count: 1,
      resetAt: now + RATE_LIMIT_WINDOW_MS,
    });
    return { ok: true };
  }

  if (current.count >= RATE_LIMIT_MAX) {
    return {
      ok: false,
      retryAfter: Math.ceil((current.resetAt - now) / 1000),
    };
  }

  current.count += 1;
  return { ok: true };
}

function validEmail(email: string): boolean {
  return email.length <= 254 && EMAIL_RE.test(email);
}

export async function POST(request: Request) {
  const rateLimit = checkRateLimit(request);
  if (!rateLimit.ok) {
    return NextResponse.json(
      { error: "too many submissions" },
      {
        status: 429,
        headers: { "Retry-After": rateLimit.retryAfter.toString() },
      }
    );
  }

  let rawBody: string;
  try {
    rawBody = await request.text();
  } catch {
    return NextResponse.json({ error: "invalid request body" }, { status: 400 });
  }

  if (rawBody.length > MAX_BODY_BYTES) {
    return NextResponse.json({ error: "request body too large" }, { status: 413 });
  }

  let body: Record<string, unknown>;
  try {
    body = JSON.parse(rawBody) as Record<string, unknown>;
  } catch {
    return NextResponse.json({ error: "invalid JSON" }, { status: 400 });
  }

  // Basic honeypot. Real forms do not send this field.
  if (sanitize(body.website, 200).trim()) {
    return NextResponse.json({ ok: true });
  }

  const name = sanitize(body.name, 120).trim();
  const email = sanitize(body.email, 254).trim().toLowerCase();
  if (!name || !email || !validEmail(email)) {
    return NextResponse.json(
      { error: "name and valid email required" },
      { status: 400 }
    );
  }

  const kindRaw = sanitize(body.kind).trim().toLowerCase();
  const kind: IntakeKind =
    kindRaw === "blueprint_v1" || kindRaw === "blueprint"
      ? "blueprint_v1"
      : kindRaw === "compatibility_v5" ||
        kindRaw === "compat_v5" ||
        kindRaw === "scorecard"
        ? "compatibility_v5"
        : "mandate_intake";

  // ── Blueprint v1: compute profile + persist ──
  if (kind === "blueprint_v1") {
    const answers = body.answers as Record<string, unknown> | undefined;
    if (!answers || typeof answers !== "object") {
      return NextResponse.json(
        { error: "answers object required for blueprint" },
        { status: 400 }
      );
    }

    let profile;
    try {
      profile = computeBlueprintProfile(answers as unknown as BlueprintAnswers);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error(`[DIAGNOSTIC-INTAKE] blueprint scoring failed: ${msg}`);
      return NextResponse.json(
        { error: "failed to compute blueprint profile" },
        { status: 500 }
      );
    }

    // Flatten answers + scores into the payload map for persistence.
    const a = answers as Record<string, unknown>;
    const payload: Record<string, string> = {
      q1_venues: sanitize(a.q1_venues),
      q2_account_size: sanitize(a.q2_account_size),
      q3_activity: sanitize(a.q3_activity),
      q4_drawdown: sanitize(a.q4_drawdown),
      q5_pain_points: sanitize(a.q5_pain_points),
      q6_risk_orientation: sanitize(a.q6_risk_orientation),
      q7_custody_comfort: sanitize(a.q7_custody_comfort),
      q8_custody_setup: sanitize(a.q8_custody_setup),
      q9_cadence: sanitize(a.q9_cadence),
      q10_goal: sanitize(a.q10_goal),
      q11_do_not_do: sanitize(a.q11_do_not_do),
      q12_commitment: sanitize(a.q12_commitment),
      telegram: sanitize(body.telegram, 120),
      source: sanitize(body.source, 200),
      on_chain_readiness: String(profile.onChainReadiness),
      risk_tolerance: String(profile.riskTolerance),
      complexity_appetite: String(profile.complexityAppetite),
      commitment_readiness: String(profile.commitmentReadiness),
      archetype: profile.archetype,
      on_chain_label: profile.onChainLabel,
    };

    let lead;
    try {
      lead = insertLead({ kind, name, email, payload });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error(`[DIAGNOSTIC-INTAKE] sqlite insert failed: ${msg}`);
      console.log(
        `[DIAGNOSTIC-INTAKE] ${JSON.stringify({
          received_at: new Date().toISOString(),
          kind,
          payload_keys: Object.keys(payload).filter((key) => payload[key]),
          persist_error: msg,
        })}`
      );
      return NextResponse.json(
        { error: "failed to persist lead" },
        { status: 500 }
      );
    }

    const notify = await notifyLead(lead);
    console.log(
      `[DIAGNOSTIC-INTAKE] ${JSON.stringify({
        id: lead.id,
        received_at: lead.received_at,
        kind: lead.kind,
        db: dbPathInUse(),
        notified: notify.ok,
        scores: {
          onChain: profile.onChainReadiness,
          risk: profile.riskTolerance,
          complexity: profile.complexityAppetite,
          commitment: profile.commitmentReadiness,
        },
      })}`
    );

    return NextResponse.json({
      ok: true,
      kind: lead.kind,
      id: lead.id,
      notified: notify.ok,
      profile,
    });
  }

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
    // Do not mirror PII into deploy logs. The client can retry or email directly.
    console.log(
      `[DIAGNOSTIC-INTAKE] ${JSON.stringify({
        received_at: new Date().toISOString(),
        kind,
        payload_keys: Object.keys(payload).filter((key) => payload[key]),
        persist_error: msg,
      })}`
    );
    return NextResponse.json(
      { error: "failed to persist lead" },
      { status: 500 }
    );
  }

  // Durable store is SQLite. Keep deploy logs free of client PII.
  const notify = await notifyLead(lead);
  console.log(
    `[DIAGNOSTIC-INTAKE] ${JSON.stringify({
      id: lead.id,
      received_at: lead.received_at,
      kind: lead.kind,
      db: dbPathInUse(),
      notified: notify.ok,
    })}`
  );

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
