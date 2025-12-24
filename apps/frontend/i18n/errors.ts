export const KNOWN_ERROR_CODES = [
  // Auth
  'UNAUTHORIZED',
  'UNAUTHORIZED_MISSING_BEARER',
  'UNAUTHORIZED_INVALID_JWT',
  'UNAUTHORIZED_EXPIRED_JWT',
  'FORBIDDEN',
  'FORBIDDEN_USER_NOT_FOUND',
  'EMAIL_NOT_ALLOWED',
  'MISSING_BACKEND_JWT',
  'BACKEND_STARTING',
  'NOT_A_MEMBER',
  'INSUFFICIENT_ROLE',

  // Validation
  'INVALID_GAME_ID',
  'INVALID_EMAIL',
  'INVALID_GOOGLE_SUB',
  'INVALID_SEAT',
  'INVALID_BID',
  'MUST_FOLLOW_SUIT',
  'CARD_NOT_IN_HAND',
  'OUT_OF_TURN',
  'PHASE_MISMATCH',
  'PARSE_CARD',
  'INVALID_TRUMP_CONVERSION',
  'VALIDATION_ERROR',
  'BAD_REQUEST',
  'INVALID_HEADER',
  'PRECONDITION_REQUIRED',

  // Not found
  'GAME_NOT_FOUND',
  'USER_NOT_FOUND',
  'PLAYER_NOT_FOUND',
  'NOT_FOUND',

  // Conflict
  'GOOGLE_SUB_MISMATCH',
  'SEAT_TAKEN',
  'UNIQUE_EMAIL',
  'OPTIMISTIC_LOCK',
  'CONFLICT',

  // System
  'DB_ERROR',
  'DB_UNAVAILABLE',
  'DB_POOL_EXHAUSTED',
  'DB_TIMEOUT',
  'LOCK_TIMEOUT_ACQUIRE',
  'LOCK_TIMEOUT_BODY',
  'MIGRATION_CANCELLED',
  'MIGRATION_FAILED',
  'POSTCHECK_MISMATCH',
  'SQLITE_LOCK_ERROR',
  'UNIQUE_VIOLATION',
  'FK_VIOLATION',
  'CHECK_VIOLATION',
  'RECORD_NOT_FOUND',
  'INTERNAL',
  'INTERNAL_ERROR',
  'CONFIG_ERROR',
  'DATA_CORRUPTION',

  // Client-side / generic
  'UNKNOWN_ERROR',
] as const

export type KnownErrorCode = (typeof KNOWN_ERROR_CODES)[number]

export function isKnownErrorCode(value: string): value is KnownErrorCode {
  return (KNOWN_ERROR_CODES as readonly string[]).includes(value)
}

export function errorCodeToMessageKey(code: string | undefined | null): string {
  if (!code) {
    return 'errors.codes.UNKNOWN_ERROR'
  }

  if (isKnownErrorCode(code)) {
    return `errors.codes.${code}`
  }

  if (process.env.NODE_ENV !== 'production') {
    console.warn(
      `Unknown error code '${code}', falling back to errors.codes.UNKNOWN_ERROR`
    )
  }

  return 'errors.codes.UNKNOWN_ERROR'
}
