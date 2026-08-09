declare module "node:sqlite" {
  export interface StatementSyncResult {
    lastInsertRowid: number | bigint;
    changes: number;
  }

  export interface StatementSync {
    run(...params: unknown[]): StatementSyncResult;
    get(...params: unknown[]): unknown;
    all(...params: unknown[]): unknown[];
  }

  export class DatabaseSync {
    constructor(path: string, options?: { open?: boolean; readOnly?: boolean });
    exec(sql: string): void;
    prepare(sql: string): StatementSync;
    close(): void;
  }
}
