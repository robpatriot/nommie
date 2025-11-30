/** @type {import('next').NextConfig} */
const nextConfig = {
  // App Router is stable in Next.js 15, no need for experimental flag
  env: {
    NEXT_PUBLIC_BACKEND_BASE_URL: process.env.BACKEND_BASE_URL,
  },
  output: 'standalone',
  async headers() {
    // Build Content-Security-Policy
    // Using balanced approach: allows 'unsafe-inline' for Next.js compatibility
    // while restricting external sources to only what's needed
    const csp = [
      // Default: only allow same-origin
      "default-src 'self'",
      // Scripts: allow self + unsafe-inline (needed for Next.js hydration and theme script)
      "script-src 'self' 'unsafe-inline' 'unsafe-eval'", // unsafe-eval needed for Next.js in dev
      // Styles: allow self + unsafe-inline (Tailwind CSS uses inline styles)
      // Google Fonts domain included for next/font/google fallback (Next.js self-hosts in prod)
      "style-src 'self' 'unsafe-inline' https://fonts.googleapis.com",
      // Images: allow self + data URIs (for inline images)
      "img-src 'self' data:",
      // Fonts: allow self + Google Fonts (Next.js self-hosts, but include for fallback)
      "font-src 'self' https://fonts.gstatic.com data:",
      // Connect (fetch/API): allow self + Google OAuth
      "connect-src 'self' https://accounts.google.com",
      // Frames: allow Google OAuth popup
      "frame-src 'self' https://accounts.google.com",
      // Other resources
      "object-src 'none'",
      "base-uri 'self'",
      "form-action 'self'",
      "frame-ancestors 'none'",
      "upgrade-insecure-requests",
    ].join('; ')

    return [
      {
        // Apply to all routes
        source: '/:path*',
        headers: [
          {
            key: 'Content-Security-Policy',
            value: csp,
          },
          {
            key: 'X-Content-Type-Options',
            value: 'nosniff',
          },
          {
            key: 'X-Frame-Options',
            value: 'DENY',
          },
          {
            key: 'Strict-Transport-Security',
            value: 'max-age=31536000; includeSubDomains; preload',
          },
          {
            key: 'Referrer-Policy',
            value: 'strict-origin-when-cross-origin',
          },
          {
            key: 'Permissions-Policy',
            value: 'geolocation=(), microphone=(), camera=(), payment=(), usb=(), magnetometer=(), gyroscope=(), accelerometer=()',
          },
          {
            key: 'X-XSS-Protection',
            value: '1; mode=block',
          },
        ],
      },
    ]
  },
}

module.exports = nextConfig
