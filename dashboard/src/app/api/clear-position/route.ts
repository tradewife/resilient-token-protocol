import { NextResponse } from "next/server";

const TRADER_INTERNAL_URL =
  process.env.RAILWAY_SERVICE_RTP_TRADER_URL ||
  process.env.TRADER_STATUS_URL ||
  null;

const TRADER_PORT = process.env.RTP_TRADER_HTTP_PORT || "8080";

function getTraderUrl(path: string): string | null {
  if (TRADER_INTERNAL_URL) {
    const base = TRADER_INTERNAL_URL.replace(/\/+$/, "");
    return `${base}:${TRADER_PORT}${path}`;
  }
  return null;
}

export async function POST() {
  const url = getTraderUrl("/clear-position");
  if (!url) {
    return NextResponse.json(
      { error: "Trader URL not configured" },
      { status: 500 }
    );
  }

  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 5000);
    const res = await fetch(url, {
      method: "POST",
      signal: controller.signal,
    });
    clearTimeout(timeout);

    if (res.ok) {
      return NextResponse.json({ status: "ok", message: "Position cleared" });
    }
    return NextResponse.json(
      { error: `Trader returned ${res.status}` },
      { status: 502 }
    );
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return NextResponse.json({ error: msg }, { status: 502 });
  }
}

// Also support GET for convenience
export async function GET() {
  return POST();
}
