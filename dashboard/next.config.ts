import { createMDX } from "fumadocs-mdx/next";
import type { NextConfig } from "next";
import path from "path";

const nextConfig: NextConfig = {
  output: "standalone",
  trailingSlash: true,
  images: {
    unoptimized: true,
  },
  transpilePackages: ["@coral-xyz/anchor", "@solana/spl-token", "@resilient-protocol/sdk"],
  turbopack: {
    root: path.resolve(__dirname),
  },
};

const withMDX = createMDX();

export default withMDX(nextConfig);
