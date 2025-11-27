/** @type {import('next').NextConfig} */
const nextConfig = {
  // App Router is stable in Next.js 15, no need for experimental flag
  env: {
    NEXT_PUBLIC_BACKEND_BASE_URL: process.env.BACKEND_BASE_URL,
  },
  output: 'standalone',
}

module.exports = nextConfig
