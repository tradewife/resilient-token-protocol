import { NextResponse } from "next/server";
import { corsHeaders } from "@/lib/cors";

async function binanceSpot(): Promise<number | null> {
  const res = await fetch(
    "https://api.binance.com/api/v3/ticker/price?symbol=SOLUSDT",
    { cache: "no-store" },
  );
  if (!res.ok) return null;
  const json = (await res.json()) as { price?: string };
  const n = Number(json.price);
  return Number.isFinite(n) && n > 0 ? n : null;
}

async function coingeckoSpot(): Promise<number | null> {
  const res = await fetch(
    "https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd",
    { cache: "no-store" },
  );
  if (!res.ok) return null;
  const json = (await res.json()) as { solana?: { usd?: number } };
  const n = Number(json.solana?.usd);
  return Number.isFinite(n) && n > 0 ? n : null;
}

export async function GET(request: Request) {
  try {
    const price = (await binanceSpot()) ?? (await coingeckoSpot());
    if (price == null) {
      return NextResponse.json({ price: null }, { status: 503 });
    }
    return NextResponse.json(
      { price, symbol: "SOLUSDT" },
      {
        headers: {
          "Cache-Control": "public, s-maxage=15, stale-while-revalidate=30",
          ...corsHeaders(request),
        },
      },
    );
  } catch {
    return NextResponse.json({ price: null }, { status: 503 });
  }
}
