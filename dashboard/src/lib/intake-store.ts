import fs from "fs";
import path from "path";
import { DatabaseSync } from "node:sqlite";

/**
 * Small durable lead store for Compatibility Check + mandate intake.
 * Backed by SQLite on a Railway volume (default /data/intake.sqlite).
 *
 * Falls back to a local path under /tmp when the volume is unavailable
 * so local `next dev` still works.
 */

export type IntakeKind = "compatibility_v5" | "mandate_intake" | "blueprint_v1";

export interface IntakeRecord {
  id: number;
  received_at: string;
  kind: IntakeKind;
  name: string;
  email: string;
  payload: Record<string, string>;
}

const DEFAULT_DB_PATH = process.env.RTP_INTAKE_DB_PATH || "/data/intake.sqlite";

let dbSingleton: DatabaseSync | null = null;
let resolvedPath: string | null = null;

function resolveDbPath(): string {
  if (resolvedPath) return resolvedPath;

  const preferred = DEFAULT_DB_PATH;
  const preferredDir = path.dirname(preferred);

  try {
    fs.mkdirSync(preferredDir, { recursive: true });
    // Prove we can write (volume present / permissions OK).
    const probe = path.join(preferredDir, ".rtp-write-probe");
    fs.writeFileSync(probe, "ok");
    fs.unlinkSync(probe);
    resolvedPath = preferred;
    return resolvedPath;
  } catch {
    const fallback = path.join("/tmp/rtp-intake", "intake.sqlite");
    fs.mkdirSync(path.dirname(fallback), { recursive: true });
    resolvedPath = fallback;
    console.warn(
      `[INTAKE-STORE] cannot write ${preferred}; falling back to ${fallback}`
    );
    return resolvedPath;
  }
}

function getDb(): DatabaseSync {
  if (dbSingleton) return dbSingleton;

  const dbPath = resolveDbPath();
  const db = new DatabaseSync(dbPath);
  db.exec(`
    PRAGMA journal_mode = WAL;
    PRAGMA synchronous = NORMAL;
    CREATE TABLE IF NOT EXISTS intake_leads (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      received_at TEXT NOT NULL,
      kind TEXT NOT NULL,
      name TEXT NOT NULL,
      email TEXT NOT NULL,
      payload_json TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_intake_leads_received_at
      ON intake_leads(received_at);
    CREATE INDEX IF NOT EXISTS idx_intake_leads_email
      ON intake_leads(email);
    CREATE INDEX IF NOT EXISTS idx_intake_leads_kind
      ON intake_leads(kind);
  `);
  dbSingleton = db;
  console.log(`[INTAKE-STORE] open ${dbPath}`);
  return db;
}

export function insertLead(input: {
  kind: IntakeKind;
  name: string;
  email: string;
  payload: Record<string, string>;
}): IntakeRecord {
  const db = getDb();
  const received_at = new Date().toISOString();
  const payload_json = JSON.stringify(input.payload ?? {});

  const result = db
    .prepare(
      `INSERT INTO intake_leads (received_at, kind, name, email, payload_json)
       VALUES (?, ?, ?, ?, ?)`
    )
    .run(received_at, input.kind, input.name, input.email, payload_json);

  const id = Number(result.lastInsertRowid);
  return {
    id,
    received_at,
    kind: input.kind,
    name: input.name,
    email: input.email,
    payload: input.payload,
  };
}

export function listLeads(limit = 200): IntakeRecord[] {
  const db = getDb();
  const rows = db
    .prepare(
      `SELECT id, received_at, kind, name, email, payload_json
       FROM intake_leads
       ORDER BY id DESC
       LIMIT ?`
    )
    .all(Math.max(1, Math.min(limit, 1000))) as Array<{
    id: number;
    received_at: string;
    kind: string;
    name: string;
    email: string;
    payload_json: string;
  }>;

  return rows.map((r) => {
    let payload: Record<string, string> = {};
    try {
      payload = JSON.parse(r.payload_json) as Record<string, string>;
    } catch {
      payload = {};
    }
    return {
      id: r.id,
      received_at: r.received_at,
      kind: (["compatibility_v5", "mandate_intake", "blueprint_v1"].includes(r.kind) ? r.kind : "mandate_intake") as IntakeKind,
      name: r.name,
      email: r.email,
      payload,
    };
  });
}

export function leadCount(): number {
  const db = getDb();
  const row = db.prepare(`SELECT COUNT(*) AS c FROM intake_leads`).get() as {
    c: number;
  };
  return Number(row?.c ?? 0);
}

export function dbPathInUse(): string {
  return resolveDbPath();
}
