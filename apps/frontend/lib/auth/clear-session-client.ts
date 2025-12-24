// Client-side session clearing utilities
// This file is safe to import from client components

let isRedirecting = false

/**
 * Clear backend session (cookie) in client context.
 * Uses document.cookie since we can't use Next.js cookies() in client components.
 */
export function clearBackendSessionClient(): void {
  // Delete the cookie by setting it to expire in the past
  const cookieName = 'backend_jwt'
  document.cookie = `${cookieName}=; expires=Thu, 01 Jan 1970 00:00:00 UTC; path=/;`
}

/**
 * Redirect to sign out route handler which will clear NextAuth session
 * and redirect to home page. Use this when the backend session is stale.
 * Guards against multiple simultaneous redirects.
 */
export function redirectToHomeClient(): void {
  if (isRedirecting) {
    return
  }
  isRedirecting = true
  window.location.href = '/api/auth/signout-session-stale'
}
