# RTP Runbooks

Operational incident-response procedures for the RTP services. Keep these
current as infrastructure changes; a stale runbook is worse than none.

Services are deployed on Railway (project `resilient-token-protocol`,
production environment). Deployment is automatic on every push to `main`.

## Diagnostics first

Before acting, gather signals. Record the UTC timestamp of the event.

- **Service status**: `node scripts/check-railway-services.ts` (from repo root)
- **Logs**: `node scripts/railway-logs.mjs --service <svc> --last 200`
  - Services: `rtp-dashboard`, `rtp-trader`, `rtp-devnet-loop`,
    `rtp-night-shift`, `rtp-fee-crank`, `rtp-promote-strategy`
- **Recent deploy/build failures**: Railway dashboard -> service -> Deployments
  (failed builds surface as alert emails).

## Triage matrix

| Symptom | Likely cause | First action |
|---|---|---|
| `/health` 503 on `rtp-trader` | consecutive errors (`>=5`) or stale heartbeat (`>30 min`) | read trader logs; check `[POLL]`/`[OVERRIDE]` lines |
| Dashboard build failing | dep/codegate: see CLAUDE.md intentional pins + peer conflict | `cd dashboard && npm ci --legacy-peer-deps && npm run build` locally |
| Trader not using validated config | `RTP_TRADER_*_OVERRIDE` or missing config file | confirm startup param line + `[OVERRIDE]` warnings in logs |
| Night Shift results missing for a day | cron only writes back the newest 2 days; repo connection dropped | reconnect repo (Settings -> Connect Repo) |
| Custom domain missing | an earlier `railway up` wiped registrations | re-add via `customDomainCreate` GraphQL; never `railway up` |

## Recovery

- **Rollback a bad dashboard/trader deploy**: Railway dashboard -> current
  version -> select previous successful deployment -> Promote (Redeploy).
  There is no in-repo one-click rollback yet; this is the documented manual path.
- **Apply trader override** (loosening-only, e.g. relax confluence):
  ```
  node scripts/railway-trader-override.mjs set --min-alignment 2 --signal-threshold 0.2
  node scripts/railway-redeploy-trader.mjs
  ```
  Tighter-than-validated values are silently ignored by design.
- **Rotate operator secret** (only if leaked):
  `node scripts/railway-operator-secret.mjs --rotate`, then redeploy both
  `rtp-dashboard` and `rtp-trader` with matching `RTP_OPERATOR_API_SECRET`.

## Post-incident

1. Capture the failing log window and the exact service/commit/UTC time.
2. Update this file or note the fix in `research/dead_ends.md`.
3. If it changes operational assumptions, record a lesson in `CLAUDE.md`
   Trader Operational Notes before the next agent touches that code path.

Security posture, CORS allow-lists, and on-chain invariants live in
`SECURITY.md` and `CLAUDE.md` — never bypass those in a hotfix.
