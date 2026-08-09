import { createMDX } from "fumadocs-mdx/next";
import type { NextConfig } from "next";
import path from "path";

const nextConfig: NextConfig = {
  output: "standalone",
  trailingSlash: true,
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
  turbopack: {
    root: path.resolve(__dirname),
  },
};

const withMDX = createMDX();

export default withMDX(nextConfig);
