import { NextResponse } from "next/server";
import { corsHeaders } from "@/lib/cors";

// TRADER_WALLET_PUBKEY comes from `dashboard/.env.local` (or the Railway
// dashboard service env), with the prior published address as a fallback for
// dev builds. Production must set this env var to the active rtp-trader
// wallet — the trader persists its wallet string in trader-state.json, but
// `trader-state.json` is not the source of truth for balance lookups here
// because the volume mount can drift if the keypair is rotated without a
// state-file rotation. Setting an explicit env var avoids that drift.
const TRADER_WALLET = process.env.RTP_TRADER_WALLET_PUBKEY ||
  "Driyi8Sw2622yCefU34zrjBsQynrDoGD31tBecXrEF6R";
const MAINNET_RPC = "https://api.mainnet-beta.solana.com";

export async function GET(request: Request) {
  try {
    const { searchParams } = new URL(request.url);
    const wallet = searchParams.get("wallet") || TRADER_WALLET;
    const res = await fetch(MAINNET_RPC, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "getBalance",
        params: [wallet],
      }),
      cache: "no-store",
    });
    const json = await res.json();
    const lamports: number = json?.result?.value ?? 0;
    return NextResponse.json(
      { lamports, sol: lamports / 1e9, wallet },
      {
        headers: {
          "Cache-Control": "public, s-maxage=10, stale-while-revalidate=30",
          ...corsHeaders(request),
        },
      },
    );
  } catch {
    return NextResponse.json({ lamports: 0, sol: 0 }, { status: 503 });
  }
}
