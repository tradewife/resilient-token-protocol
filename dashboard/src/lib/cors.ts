/**
 * Allow-list CORS helper for RTP public API routes.
 *
 * Browsers block cross-origin reads unless the responding server sets
 * `Access-Control-Allow-Origin`. Returning `*` would let *any* website read
 * the response. Instead we only echo the caller's origin back when it is on
 * our allow-list; for any other origin the header is omitted and the browser
 * enforces the block. Same-origin requests never need CORS at all.
 *
 * Add the dashboard domain anywhere it runs (dev + Railway production).
 */
const ALLOWED_ORIGINS = new Set<string>([
  "http://localhost:3000", // local `next dev`
  "https://resilientprotocol.xyz",
  "https://www.resilientprotocol.xyz",
  "https://rtp-dashboard-production.up.railway.app", // direct Railway endpoint
]);

export function corsHeaders(request: Request): Record<string, string> {
  const origin = request.headers.get("origin");
  if (origin && ALLOWED_ORIGINS.has(origin)) {
    return {
      "Access-Control-Allow-Origin": origin,
      "Vary": "Origin",
    };
  }
  return {};
}
