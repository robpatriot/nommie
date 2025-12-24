import { redirect } from 'next/navigation'
import { BackendApiError } from '@/lib/api'

/**
 * Handle an email-allowlist failure in a server component.
 *
 * If the error indicates the user's email is not allowed, this will:
 * - Redirect to a route handler that signs out the user and shows the access denied message
 *
 * Note: We can't call signOut() directly from a Server Component, so we redirect
 * to a Route Handler that can modify cookies.
 */
export async function handleAllowlistError(error: unknown) {
  if (
    error instanceof BackendApiError &&
    error.status === 403 &&
    error.code === 'EMAIL_NOT_ALLOWED'
  ) {
    // Redirect to route handler that signs out and redirects to home with access denied message
    redirect('/api/auth/signout-access-denied')
  }
}

/**
 * Handle a stale session error in a server component.
 *
 * If the error indicates the user's session is stale (user not found in database),
 * this will:
 * - Redirect to a route handler that signs out the user and redirects to home
 *
 * Note: We can't call signOut() directly from a Server Component, so we redirect
 * to a Route Handler that can modify cookies.
 */
export async function handleStaleSessionError(error: unknown) {
  if (
    error instanceof BackendApiError &&
    error.status === 401 &&
    error.code === 'FORBIDDEN_USER_NOT_FOUND'
  ) {
    // Redirect to route handler that signs out and redirects to home
    redirect('/api/auth/signout-session-stale')
  }
}
