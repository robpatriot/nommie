/** @type {import('next').NextConfig} */
const nextConfig = {
  // App Router is stable in Next.js 15, no need for experimental flag
  env: {
    NEXT_PUBLIC_BACKEND_BASE_URL: process.env.BACKEND_BASE_URL,
    NEXT_PUBLIC_BACKEND_WS_URL:
      process.env.BACKEND_WS_URL ||
      (process.env.BACKEND_BASE_URL
        ? process.env.BACKEND_BASE_URL.replace(/^http/, 'ws')
        : undefined),
  },
  output: 'standalone',
}

module.exports = nextConfig
