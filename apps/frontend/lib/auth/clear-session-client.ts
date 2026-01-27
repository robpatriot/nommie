// Client-side session clearing utilities
// This file is safe to import from client components

import { BACKEND_JWT_COOKIE_NAME } from '@/lib/auth/backend-jwt-cookie'

let isRedirecting = false

export function clearBackendSessionClient(): void {
  document.cookie = `${BACKEND_JWT_COOKIE_NAME}=; expires=Thu, 01 Jan 1970 00:00:00 UTC; path=/;`
}

export function redirectToHomeClient(): void {
  if (isRedirecting) return
  isRedirecting = true
  window.location.href = '/api/auth/signout-session-stale'
}
