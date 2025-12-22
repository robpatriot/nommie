/**
 * Utility functions for date and time formatting with locale support.
 */

/**
 * Formats a date/time string or Date object as a localized time string.
 *
 * @param date - Date string or Date object
 * @param locale - Locale string (e.g., 'en-GB', 'fr-FR')
 * @param options - Intl.DateTimeFormatOptions (optional)
 * @returns Formatted time string (e.g., "2:30 PM")
 */
export function formatTime(
  date: string | Date,
  locale: string,
  options: Intl.DateTimeFormatOptions = {
    hour: '2-digit',
    minute: '2-digit',
  }
): string {
  const dateObj = typeof date === 'string' ? new Date(date) : date
  return dateObj.toLocaleTimeString(locale, options)
}

/**
 * Formats a date/time string or Date object as a localized date string.
 *
 * @param date - Date string or Date object
 * @param locale - Locale string (e.g., 'en-GB', 'fr-FR')
 * @param options - Intl.DateTimeFormatOptions (optional)
 * @returns Formatted date string (e.g., "12/25/2023")
 */
export function formatDate(
  date: string | Date,
  locale: string,
  options: Intl.DateTimeFormatOptions = {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  }
): string {
  const dateObj = typeof date === 'string' ? new Date(date) : date
  return dateObj.toLocaleDateString(locale, options)
}

/**
 * Formats a date/time string or Date object as a localized date and time string.
 *
 * @param date - Date string or Date object
 * @param locale - Locale string (e.g., 'en-GB', 'fr-FR')
 * @param options - Intl.DateTimeFormatOptions (optional)
 * @returns Formatted date and time string (e.g., "12/25/2023, 2:30 PM")
 */
export function formatDateTime(
  date: string | Date,
  locale: string,
  options: Intl.DateTimeFormatOptions = {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  }
): string {
  const dateObj = typeof date === 'string' ? new Date(date) : date
  return dateObj.toLocaleString(locale, options)
}
