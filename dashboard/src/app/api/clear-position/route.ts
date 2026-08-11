import { NextResponse } from "next/server";

const TRADER_INTERNAL_URL =
  process.env.RAILWAY_SERVICE_RTP_TRADER_URL ||
  process.env.TRADER_STATUS_URL ||
  null;

const TRADER_PORT = process.env.RTP_TRADER_HTTP_PORT || "8080";
const OPERATOR_SECRET = process.env.RTP_OPERATOR_API_SECRET || null;

function getTraderUrl(path: string): string | null {
  if (TRADER_INTERNAL_URL) {
    const base = TRADER_INTERNAL_URL.replace(/\/+$/, "");
    return `${base}:${TRADER_PORT}${path}`;
  }
  return null;
}

function bearerToken(request: Request): string | null {
  const auth = request.headers.get("authorization") || "";
  if (auth.toLowerCase().startsWith("bearer ")) {
    return auth.slice("bearer ".length).trim();
  }
  return request.headers.get("x-rtp-operator-secret");
}

function unauthorized(message = "unauthorized") {
  return NextResponse.json({ error: message }, { status: 401 });
}

export async function POST(request: Request) {
  if (!OPERATOR_SECRET) {
    return NextResponse.json(
      { error: "operator API secret not configured" },
      { status: 503 }
    );
  }

  if (bearerToken(request) !== OPERATOR_SECRET) {
    return unauthorized();
  }

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
      headers: {
        Authorization: `Bearer ${OPERATOR_SECRET}`,
        "X-RTP-Operator-Secret": OPERATOR_SECRET,
      },
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

export async function GET() {
  return NextResponse.json(
    { error: "method not allowed" },
    { status: 405, headers: { Allow: "POST" } }
  );
}
