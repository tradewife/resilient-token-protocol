import { NextResponse } from "next/server";
import fs from "fs";
import path from "path";

function walkDir(dir: string): string[] {
  const results: string[] = [];
  if (!fs.existsSync(dir)) return results;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      results.push(...walkDir(full));
    } else if (entry.name !== ".gitkeep") {
      results.push(full);
    }
  }
  return results;
}

export async function GET() {
  try {
    const memRoot = path.resolve(process.cwd(), "..", "..", "data", "swarm-memory");
    const files = walkDir(memRoot);

    let latestFile = "";
    let latestTs = 0;
    for (const f of files) {
      try {
        const stat = fs.statSync(f);
        if (stat.mtimeMs > latestTs) {
          latestTs = stat.mtimeMs;
          latestFile = path.basename(f);
        }
      } catch { /* skip */ }
    }

    const tiers = ["core", "overview", "project", "working"] as const;
    const breakdown: Record<string, number> = {};
    for (const tier of tiers) {
      const tierDir = path.join(memRoot, tier);
      breakdown[tier] = fs.existsSync(tierDir)
        ? walkDir(tierDir).length
        : 0;
    }

    return NextResponse.json({
      fileCount: files.length,
      latestFile: latestFile || null,
      latestTimestamp: latestTs
        ? new Date(latestTs).toISOString()
        : null,
      breakdown,
    });
  } catch {
    return NextResponse.json({ error: "swarm-memory not found" }, { status: 404 });
  }
}
