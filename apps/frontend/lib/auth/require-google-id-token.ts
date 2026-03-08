/**
 * Require a valid Google ID token from the account object during sign-in.
 *
 * We rely on Google's OpenID Connect flow to return an ID token.
 * If account.id_token is missing during sign-in, fail immediately rather
 * than falling back to unverified profile fields.
 *
 * Call only when account.provider === 'google'.
 *
 * @throws If id_token is missing or invalid
 * @returns The id_token string when valid
 */
export async function requireGoogleIdToken(
  account: { provider?: string; id_token?: string } | null
): Promise<string> {
  const idToken = account?.id_token
  if (!idToken || typeof idToken !== 'string') {
    const { logError } = await import('@/lib/logging/error-logger')
    logError(
      'Google id_token missing at sign-in',
      new Error('id_token required'),
      {
        action: 'initialLogin',
      }
    )
    throw new Error('Google sign-in failed: missing id_token')
  }

  return idToken
}
