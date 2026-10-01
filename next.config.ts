import type { NextConfig } from "next";
import { resolve } from "path";

const nextConfig: NextConfig = {
  reactStrictMode: true,
  output: "export",
  turbopack: {
    root: resolve(__dirname),
  },
};

export default nextConfig;