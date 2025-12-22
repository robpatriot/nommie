/**
 * Utility functions for number formatting with locale support.
 */

/**
 * Formats a number with locale-aware formatting.
 *
 * @param value - Number to format
 * @param locale - Locale string (e.g., 'en-GB', 'fr-FR')
 * @param options - Intl.NumberFormatOptions (optional)
 * @returns Formatted number string
 */
export function formatNumber(
  value: number,
  locale: string,
  options?: Intl.NumberFormatOptions
): string {
  return new Intl.NumberFormat(locale, options).format(value)
}

/**
 * Formats a number with a fixed number of decimal places.
 *
 * @param value - Number to format
 * @param locale - Locale string (e.g., 'en-GB', 'fr-FR')
 * @param decimals - Number of decimal places (default: 2)
 * @returns Formatted number string
 */
export function formatNumberFixed(
  value: number,
  locale: string,
  decimals: number = 2
): string {
  return new Intl.NumberFormat(locale, {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  }).format(value)
}

/**
 * Formats bytes as a human-readable string (B, KB, MB, GB, etc.).
 *
 * @param bytes - Number of bytes
 * @param locale - Locale string (e.g., 'en-GB', 'fr-FR')
 * @returns Formatted string (e.g., "1.5 MB")
 */
export function formatBytes(bytes: number, locale: string): string {
  if (bytes === 0) {
    return formatNumber(0, locale) + ' B'
  }

  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  const value = bytes / Math.pow(k, i)

  return formatNumberFixed(value, locale, 2) + ' ' + sizes[i]
}

/**
 * Formats a duration in milliseconds as a human-readable string.
 *
 * @param ms - Duration in milliseconds
 * @param locale - Locale string (e.g., 'en-GB', 'fr-FR')
 * @returns Formatted string (e.g., "1.5 s" or "150 ms")
 */
export function formatDuration(ms: number, locale: string): string {
  if (ms < 1000) {
    return formatNumberFixed(ms, locale, 2) + ' ms'
  }
  const seconds = ms / 1000
  return formatNumberFixed(seconds, locale, 2) + ' s'
}
