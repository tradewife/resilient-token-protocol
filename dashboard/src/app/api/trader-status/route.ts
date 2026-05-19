import { NextResponse } from "next/server";

const TRADER_INTERNAL_URL =
  process.env.RAILWAY_SERVICE_RTP_TRADER_URL ||
  process.env.TRADER_STATUS_URL ||
  null;

// Railway private networking routes to the container's EXPOSEd port via the
// private domain on port 80. The RAILWAY_SERVICE_*_URL env var already
// includes the scheme and hostname — do NOT append the container port.
function getTraderUrl(): string | null {
  if (TRADER_INTERNAL_URL) {
    const base = TRADER_INTERNAL_URL.replace(/\/+$/, "");
    return `${base}/state`;
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

export async function GET() {
  // Try live trader first
  const traderUrl = getTraderUrl();
  if (traderUrl) {
    try {
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 3000);
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
            "Access-Control-Allow-Origin": "*",
          },
        });
      }
    } catch {
      // Trader unreachable — fall through to static
    }
  }

  // Fallback: static baked file
  const staticData = await getStaticFallback();
  if (staticData) {
    return NextResponse.json(staticData, {
      headers: {
        "Cache-Control": "public, s-maxage=30",
        "Access-Control-Allow-Origin": "*",
        "X-Data-Source": "static-fallback",
      },
    });
  }

  return NextResponse.json(
    { error: "Trader state not available" },
    { status: 503 }
  );
}
