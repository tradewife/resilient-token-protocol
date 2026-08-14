# Security

This file covers the application/infrastructure security posture for the
**Resilient Token Protocol** dashboard (`dashboard/`) and live trader
(`rtp/swarm/`). On-chain program security (the Anchor `rtp-treasury`
program) is documented separately in `archive/COLOSSEUM-AUDIT.md`.

## Reporting a vulnerability

Contact **katejcooper.atelier@gmail.com** directly. Do not open a public
issue or PR describing the vulnerability before it is fixed. Include:

- What you found and how to reproduce it
- Which service/endpoint is affected (dashboard vs trader vs program)
- Suggested fix, if you have one

We acknowledge reports within 48h and aim to ship a fix with the next
deploy. No bug bounty is offered at this time.

## Trust model

- **Permissionless inbound** — anyone can pull fees into the treasury PDA
  or trigger the deterministic redistribution check on-chain.
- **Authority-gated outbound + metric writes** — the Anchor program enforces
  this on-chain; the PDA owns funds and no private key exists for treasury.
- **Trading wallet** (`HDQ79fQ1YbL9CenS1DzfHizEWGrJdnmo99fgAWmdhuy5`) is
  funded and pays gas; there is no human key for treasury/trading.

## Operator secrets

| Variable | Where | Gates | Fail behavior |
|---|---|---|---|
| `RTP_OPERATOR_API_SECRET` | `rtp-dashboard` + `rtp-trader` (same value) | `POST /api/clear-position` → trader `/clear-position` | **Fails closed**: 503 if unset, 401 if wrong |
| `RTP_INTAKE_SECRET` | `rtp-dashboard` | `GET /api/diagnostic-intake` (lead list) | **Fails closed**: 403 if unset, 401 if wrong |
| `RESEND_API_KEY` | `rtp-dashboard` | Lead notification email | Notification skipped if unset |
| Stripe keys | `rtp-dashboard` | Payment links (Stripe-hosted checkout) | n/a — card data never touches RTP |

Local copies of secrets live in `.secrets/` (gitignored, mode 0600 where
relevant). Never commit secret values to the repo.

## Operational security invariants

1. **Operator endpoints fail closed.** `RTP_OPERATOR_API_SECRET` must be set
   identically on `rtp-dashboard` and `rtp-trader`, or `/api/clear-position`
   is unavailable (503). Forgetting to set it is safer than public access.
2. **No public state mutation.** `GET /api/clear-position` returns 405;
   only authenticated `POST` clears trader position state.
3. **No PII in deploy logs.** `[DIAGNOSTIC-INTAKE]` log lines contain
   lead ID, kind, timestamp, and notified status — never name, email, or
   payload. SQLite (volume) + Resend are the durable paths.
4. **Intake is rate-limited.** 5 POSTs per IP per 10 minutes, 32KB body
   cap, strict email validation, honeypot field. Constants live in
   `dashboard/src/app/api/diagnostic-intake/route.ts`.
5. **Security headers are global.** HSTS, `X-Frame-Options: DENY`,
   `X-Content-Type-Options: nosniff`, Referrer-Policy, Permissions-Policy,
   and CSP are set for all routes in `dashboard/next.config.ts`. Do not
   weaken `frame-ancestors 'none'` or `object-src 'none'`.
6. **Trader internal HTTP is auth-gated.** The trader's status server
   (port 8080, Railway private networking) only clears position state on
   `POST /clear-position` with the operator secret; the deprecated `/clear`
   alias returns 410.
7. **CORS is allow-listed.** Public API routes
   (`/api/trader-status`, `/api/mainnet-balance`) echo
   `Access-Control-Allow-Origin` only when the caller's origin is on the
   dashboard allow-list in `dashboard/src/lib/cors.ts`; all other origins
   get no CORS header and the browser blocks them. Never reintroduce
   `Access-Control-Allow-Origin: *`. To grant a new origin, add it to
   `ALLOWED_ORIGINS`, never reopen a wildcard.

## Rotation

Generate and install a new shared operator secret:

```bash
node scripts/railway-operator-secret.mjs --rotate
node scripts/railway-redeploy-trader.mjs   # trader picks it up at startup
# Redeploy rtp-dashboard (Railway UI or push to main)
```

The installer writes the value to both services and saves a local copy to
`.secrets/rtp-operator-api-secret` (gitignored). Verify with:

```bash
node scripts/railway-operator-secret.mjs --show
```
