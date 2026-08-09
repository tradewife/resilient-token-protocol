import type { IntakeRecord } from "./intake-store";

/**
 * Notify Kate on every intake lead.
 *
 * Providers (first configured wins):
 *   1. Resend HTTP API  — RESEND_API_KEY (+ optional RTP_NOTIFY_FROM)
 *   2. SMTP via nodemailer-less raw fetch is not available; we use
 *      Resend-compatible shape only to keep zero native deps.
 *
 * Always returns a status object; never throws to the request path.
 */

const NOTIFY_TO =
  process.env.RTP_NOTIFY_EMAIL || "katejcooper.atelier@gmail.com";
const NOTIFY_FROM =
  process.env.RTP_NOTIFY_FROM || "RTP Intake <onboarding@resend.dev>";
const RESEND_API_KEY = process.env.RESEND_API_KEY || "";

function subjectFor(lead: IntakeRecord): string {
  if (lead.kind === "compatibility_v5") {
    const path =
      lead.payload.solution_model === "advisory"
        ? "Build"
        : lead.payload.solution_model === "explore" ||
            lead.payload.solution_model === "developer"
          ? "Exploring"
          : "Scorecard";
    return `[RTP Lead] Compatibility · ${path} · ${lead.name}`;
  }
  return `[RTP Lead] Mandate intake · ${lead.name}`;
}

function bodyText(lead: IntakeRecord): string {
  const lines: string[] = [
    `New RTP intake lead #${lead.id}`,
    `Received: ${lead.received_at}`,
    `Kind:     ${lead.kind}`,
    `Name:     ${lead.name}`,
    `Email:    ${lead.email}`,
    "",
    "Payload:",
  ];
  const keys = Object.keys(lead.payload).sort();
  if (keys.length === 0) {
    lines.push("  (empty)");
  } else {
    for (const k of keys) {
      const v = lead.payload[k];
      if (!v) continue;
      lines.push(`  ${k}: ${v}`);
    }
  }
  lines.push("", "— RTP dashboard intake");
  return lines.join("\n");
}

function bodyHtml(lead: IntakeRecord): string {
  const rows = Object.keys(lead.payload)
    .sort()
    .filter((k) => lead.payload[k])
    .map(
      (k) =>
        `<tr><td style="padding:4px 12px 4px 0;color:#6b7280;vertical-align:top;white-space:nowrap">${escapeHtml(
          k
        )}</td><td style="padding:4px 0;color:#111827">${escapeHtml(
          lead.payload[k]
        )}</td></tr>`
    )
    .join("");

  return `<!doctype html>
<html><body style="font-family:ui-sans-serif,system-ui,sans-serif;line-height:1.5;color:#111827">
  <h2 style="margin:0 0 8px;font-size:18px">New RTP intake lead #${lead.id}</h2>
  <p style="margin:0 0 16px;color:#6b7280;font-size:13px">${escapeHtml(
    lead.received_at
  )} · ${escapeHtml(lead.kind)}</p>
  <p style="margin:0 0 4px"><strong>${escapeHtml(lead.name)}</strong></p>
  <p style="margin:0 0 16px"><a href="mailto:${escapeHtml(
    lead.email
  )}">${escapeHtml(lead.email)}</a></p>
  <table style="border-collapse:collapse;font-size:14px">${rows}</table>
  <p style="margin:24px 0 0;color:#9ca3af;font-size:12px">RTP dashboard intake</p>
</body></html>`;
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

export type NotifyResult =
  | { ok: true; provider: "resend"; id?: string }
  | { ok: false; provider: "none" | "resend"; error: string };

export async function notifyLead(lead: IntakeRecord): Promise<NotifyResult> {
  if (!RESEND_API_KEY) {
    console.warn(
      `[INTAKE-NOTIFY] skipped email for lead #${lead.id} — set RESEND_API_KEY to enable`
    );
    return {
      ok: false,
      provider: "none",
      error: "RESEND_API_KEY not configured",
    };
  }

  try {
    const res = await fetch("https://api.resend.com/emails", {
      method: "POST",
      headers: {
        Authorization: `Bearer ${RESEND_API_KEY}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        from: NOTIFY_FROM,
        to: [NOTIFY_TO],
        reply_to: lead.email,
        subject: subjectFor(lead),
        text: bodyText(lead),
        html: bodyHtml(lead),
      }),
    });

    const data = (await res.json().catch(() => ({}))) as {
      id?: string;
      message?: string;
      name?: string;
    };

    if (!res.ok) {
      const err =
        data.message || data.name || `Resend HTTP ${res.status}`;
      console.error(`[INTAKE-NOTIFY] resend failed lead #${lead.id}: ${err}`);
      return { ok: false, provider: "resend", error: err };
    }

    console.log(
      `[INTAKE-NOTIFY] emailed ${NOTIFY_TO} lead #${lead.id} id=${data.id || "?"}`
    );
    return { ok: true, provider: "resend", id: data.id };
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.error(`[INTAKE-NOTIFY] exception lead #${lead.id}: ${msg}`);
    return { ok: false, provider: "resend", error: msg };
  }
}

export function notifyConfigSummary(): {
  to: string;
  from: string;
  resendConfigured: boolean;
} {
  return {
    to: NOTIFY_TO,
    from: NOTIFY_FROM,
    resendConfigured: Boolean(RESEND_API_KEY),
  };
}
