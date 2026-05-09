import { NextResponse } from "next/server";

const TRADER_WALLET = "Driyi8Sw2622yCefU34zrjBsQynrDoGD31tBecXrEF6R";
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
        },
      },
    );
  } catch {
    return NextResponse.json({ lamports: 0, sol: 0 }, { status: 503 });
  }
}
