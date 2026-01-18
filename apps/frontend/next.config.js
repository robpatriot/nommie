const createNextIntlPlugin = require('next-intl/plugin')

const withNextIntl = createNextIntlPlugin('./i18n/request.ts')

const canonicalBackendBase =
  process.env.BACKEND_BASE_URL || process.env.NEXT_PUBLIC_BACKEND_BASE_URL

/** @type {import('next').NextConfig} */
const nextConfig = {
  // App Router is stable in Next.js 15, no need for experimental flag
  
  // Optimize bundle size and reduce unused JavaScript
  compiler: {
    // Remove console.log in production
    removeConsole: process.env.NODE_ENV === 'production' ? {
      exclude: ['error', 'warn'],
    } : false,
  },
  
  // Optimize images
  images: {
    formats: ['image/avif', 'image/webp'],
  },
  
  env: {
    NEXT_PUBLIC_BACKEND_BASE_URL: canonicalBackendBase,
    NEXT_PUBLIC_BACKEND_WS_URL:
      process.env.BACKEND_WS_URL ||
      process.env.NEXT_PUBLIC_BACKEND_WS_URL ||
      (canonicalBackendBase
        ? canonicalBackendBase.replace(/^http/, 'ws')
        : undefined),
  },
  output: 'standalone',
  async headers() {
    // Build Content-Security-Policy
    // Allows 'unsafe-inline' for Next.js hydration and theme scripts
    // while restricting external sources to only what's needed
    
    // Get backend URLs for CSP connect-src directive
  const backendBaseUrl = process.env.BACKEND_BASE_URL || process.env.NEXT_PUBLIC_BACKEND_BASE_URL
  const backendWsUrl = process.env.BACKEND_WS_URL || process.env.NEXT_PUBLIC_BACKEND_WS_URL
      
    // Build connect-src directive with backend URLs
    const connectSrc = [
      "'self'",
      'https://accounts.google.com', // Google OAuth
    ]

    if (backendBaseUrl) {
      // Add HTTP/HTTPS backend URL
      try {
        const url = new URL(backendBaseUrl)
        connectSrc.push(`${url.protocol}//${url.host}`)
      } catch {
        // Invalid URL, skip
      }
    }
    
    if (backendWsUrl) {
      // Add WebSocket backend URL
      try {
        const url = new URL(backendWsUrl)
        connectSrc.push(`${url.protocol}//${url.host}`)
      } catch {
        // Invalid URL, skip
      }
    } else if (backendBaseUrl) {
      // Derive WebSocket URL from HTTP URL if not explicitly set
      try {
        const url = new URL(backendBaseUrl)
        const wsProtocol = url.protocol === 'https:' ? 'wss:' : 'ws:'
        connectSrc.push(`${wsProtocol}//${url.host}`)
      } catch {
        // Invalid URL, skip
      }
    }
    
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
      // Connect (fetch/API/WebSocket): allow self + Google OAuth + backend URLs
      `connect-src ${connectSrc.join(' ')}`,
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

module.exports = withNextIntl(nextConfig)
