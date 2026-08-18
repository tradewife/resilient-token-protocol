import { createMDX } from "fumadocs-mdx/next";
import type { NextConfig } from "next";
import path from "path";

const securityHeaders = [
  {
    key: "Strict-Transport-Security",
    value: "max-age=31536000; includeSubDomains; preload",
  },
  {
    key: "X-Frame-Options",
    value: "DENY",
  },
  {
    key: "X-Content-Type-Options",
    value: "nosniff",
  },
  {
    key: "Referrer-Policy",
    value: "strict-origin-when-cross-origin",
  },
  {
    key: "Permissions-Policy",
    value: "camera=(), microphone=(), geolocation=(), payment=()",
  },
  {
    key: "Content-Security-Policy",
    value: [
      "default-src 'self'",
      "base-uri 'self'",
      "object-src 'none'",
      "frame-ancestors 'none'",
      "form-action 'self'",
      "img-src 'self' data: blob: https://explorer.solana.com",
      "font-src 'self' data:",
      "style-src 'self' 'unsafe-inline'",
      "script-src 'self' 'unsafe-inline' 'unsafe-eval'",
      "connect-src 'self' https://api.devnet.solana.com https://api.mainnet-beta.solana.com https://flashapi.trade",
      "upgrade-insecure-requests",
    ].join("; "),
  },
];

const nextConfig: NextConfig = {
  output: "standalone",
  trailingSlash: true,
  // Dev-only: Next 16 blocks cross-origin dev chunk loads by default;
  // allow the loopback alias so the site hydrates from either host.
  allowedDevOrigins: ["127.0.0.1"],
  images: {
    unoptimized: true,
  },
  // node:sqlite is a Node built-in (experimental in Node 22+). Keep it
  // external so the standalone server can require it at runtime.
  serverExternalPackages: ["node:sqlite"],
  transpilePackages: ["@coral-xyz/anchor", "@solana/spl-token", "@resilient-protocol/sdk"],
  async redirects() {
    return [
      {
        source: "/diagnostic",
        destination: "/compatibility",
        permanent: true,
      },
      {
        source: "/diagnostic/",
        destination: "/compatibility/",
        permanent: true,
      },
    ];
  },
  async headers() {
    return [
      {
        source: "/:path*",
        headers: securityHeaders,
      },
    ];
  },
  turbopack: {
    root: path.resolve(__dirname),
  },
};

const withMDX = createMDX();

export default withMDX(nextConfig);
