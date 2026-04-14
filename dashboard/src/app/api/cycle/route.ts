import { NextResponse } from "next/server";
import fs from "fs";
import path from "path";

export async function GET() {
  try {
    const cyclePath = path.resolve(
      process.cwd(),
      "..",
      "..",
      "data",
      "devnet-cycles",
      "latest",
      "cycle.json"
    );
    const raw = fs.readFileSync(cyclePath, "utf-8");
    const cycle = JSON.parse(raw);

    const mutations_accepted: Array<{ param: string; value: number; rationale: string }> =
      cycle.mutations_accepted ?? [];
    const mutations_rejected: Array<{ param: string; value: number; rationale: string }> =
      cycle.mutations_rejected ?? [];
    const params_used: Record<string, number> = cycle.params_used ?? {};
    const params_next: Record<string, number> = cycle.params_next ?? {};

    const diffs: Array<{ param: string; from: number; to: number }> = [];
    for (const k of Object.keys(params_used)) {
      const from = params_used[k];
      const to = params_next[k] ?? from;
      if (String(from) !== String(to)) {
        diffs.push({ param: k, from, to });
      }
    }

    return NextResponse.json({
      cycle_id: cycle.cycle_id ?? null,
      params_used,
      params_next,
      mutations_accepted,
      mutations_rejected,
      diffs,
      used_llm: cycle.used_llm ?? false,
      model_label: cycle.model_label ?? "?",
      memory_file_count: (cycle.memory_files ?? []).length,
      timestamp: cycle.cycle_id,
    });
  } catch {
    return NextResponse.json({ error: "cycle.json not found" }, { status: 404 });
  }
}
