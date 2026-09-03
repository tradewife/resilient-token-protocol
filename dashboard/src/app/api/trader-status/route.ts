import { NextResponse } from "next/server";
import { corsHeaders } from "@/lib/cors";

const TRADER_INTERNAL_URL =
  process.env.RAILWAY_SERVICE_RTP_TRADER_URL ||
  process.env.TRADER_STATUS_URL ||
  null;

const TRADER_PORT = process.env.RTP_TRADER_HTTP_PORT || "8080";

// Full URL to a /state JSON document. Used for local `next dev` against the
// live trader (or the production dashboard proxy) without Railway private DNS.
const TRADER_STATE_URL = process.env.RTP_TRADER_STATE_URL || null;

// Railway private networking: http://<service>.railway.internal:<container-port>
// RAILWAY_SERVICE_*_URL provides scheme + hostname, container port must be appended.
function getTraderUrl(): string | null {
  if (TRADER_STATE_URL) {
    return TRADER_STATE_URL;
  }
  if (TRADER_INTERNAL_URL) {
    const base = TRADER_INTERNAL_URL.replace(/\/+$/, "");
    return `${base}:${TRADER_PORT}/state`;
  }
  return null;
}

// Fallback: read the baked static file if trader URL is not available
async function getStaticFallback(): Promise<object | null> {
  try {
    const fs = await import("fs");
    const path = await import("path");
    const filePath = path.join(process.cwd(), "public", "data", "trader-state.json");
    const content = fs.readFileSync(filePath, "utf-8");
    return JSON.parse(content);
  } catch {
    return null;
  }
}

export async function GET(request: Request) {
  // Try live trader first
  const traderUrl = getTraderUrl();
  if (traderUrl) {
    try {
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 5000);
      const res = await fetch(traderUrl, {
        signal: controller.signal,
        headers: { Accept: "application/json" },
        cache: "no-store",
      });
      clearTimeout(timeout);
      if (res.ok) {
        const data = await res.json();
        return NextResponse.json(data, {
          headers: {
            "Cache-Control": "public, s-maxage=10, stale-while-revalidate=30",
            ...corsHeaders(request),
          },
        });
      }
      // Non-OK response from trader — include diagnostic info in fallback headers
      const staticData = await getStaticFallback();
      return NextResponse.json(staticData || { error: "Trader returned non-OK" }, {
        status: 200,
        headers: {
          "Cache-Control": "public, s-maxage=30",
          ...corsHeaders(request),
          "X-Data-Source": "static-fallback",
          "X-Trader-Url": traderUrl,
          "X-Trader-Status": res.status.toString(),
        },
      });
    } catch (err) {
      // Trader unreachable — include diagnostic info in fallback headers
      const errMsg = err instanceof Error ? err.message : String(err);
      const staticData = await getStaticFallback();
      return NextResponse.json(staticData || { error: "Trader unreachable" }, {
        status: 200,
        headers: {
          "Cache-Control": "public, s-maxage=30",
          ...corsHeaders(request),
          "X-Data-Source": "static-fallback",
          "X-Trader-Url": traderUrl,
          "X-Trader-Error": errMsg.slice(0, 200),
        },
      });
    }
  }

  // Fallback: static baked file
  const staticData = await getStaticFallback();
  if (staticData) {
    return NextResponse.json(staticData, {
      headers: {
        "Cache-Control": "public, s-maxage=30",
        ...corsHeaders(request),
        "X-Data-Source": "static-fallback",
        "X-Trader-Url": "not-configured",
      },
    });
  }

  return NextResponse.json(
    { error: "Trader state not available" },
    { status: 503 }
  );
}
