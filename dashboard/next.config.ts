import type { NextConfig } from "next";
import path from "path";

const nextConfig: NextConfig = {
  output: "export",
  trailingSlash: true,
  images: {
    unoptimized: true,
  },
  transpilePackages: ["@coral-xyz/anchor", "@solana/spl-token", "@resilient-protocol/sdk"],
  turbopack: {
    root: path.resolve(__dirname),
  },
};

export default nextConfig;
