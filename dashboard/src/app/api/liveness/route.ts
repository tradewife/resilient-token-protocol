import { NextResponse } from "next/server";

const PROGRAM_ID = "4LvsHbe9LLwgogcDbH7ieTsGcWZctjYFZkzZwaHDM8Ad";

export async function GET() {
  try {
    const res = await fetch("https://api.devnet.solana.com", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "getAccountInfo",
        params: [PROGRAM_ID, { encoding: "base64" }],
      }),
    });
    const json = await res.json();
    const value = json?.result?.value;
    return NextResponse.json({
      programId: PROGRAM_ID,
      live: value !== null && value !== undefined,
      executable: value?.executable ?? false,
      slot: json?.result?.context?.slot ?? null,
    });
  } catch {
    return NextResponse.json({
      programId: PROGRAM_ID,
      live: false,
      error: "RPC unreachable",
    }, { status: 502 });
  }
}
