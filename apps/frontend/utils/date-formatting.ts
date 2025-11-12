/**
 * Utility functions for date and time formatting.
 */

/**
 * Formats a date/time string or Date object as a localized time string.
 *
 * @param date - Date string or Date object
 * @param options - Intl.DateTimeFormatOptions (optional)
 * @returns Formatted time string (e.g., "2:30 PM")
 */
export function formatTime(
  date: string | Date,
  options: Intl.DateTimeFormatOptions = {
    hour: '2-digit',
    minute: '2-digit',
  }
): string {
  const dateObj = typeof date === 'string' ? new Date(date) : date
  return dateObj.toLocaleTimeString([], options)
}

/**
 * Formats a date/time string or Date object as a localized date string.
 *
 * @param date - Date string or Date object
 * @param options - Intl.DateTimeFormatOptions (optional)
 * @returns Formatted date string (e.g., "12/25/2023")
 */
export function formatDate(
  date: string | Date,
  options: Intl.DateTimeFormatOptions = {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  }
): string {
  const dateObj = typeof date === 'string' ? new Date(date) : date
  return dateObj.toLocaleDateString([], options)
}

/**
 * Formats a date/time string or Date object as a localized date and time string.
 *
 * @param date - Date string or Date object
 * @param options - Intl.DateTimeFormatOptions (optional)
 * @returns Formatted date and time string (e.g., "12/25/2023, 2:30 PM")
 */
export function formatDateTime(
  date: string | Date,
  options: Intl.DateTimeFormatOptions = {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  }
): string {
  const dateObj = typeof date === 'string' ? new Date(date) : date
  return dateObj.toLocaleString([], options)
}

/**
 * Formats a date/time string or Date object as a relative time string.
 * (e.g., "2 minutes ago", "3 hours ago", "yesterday")
 *
 * @param date - Date string or Date object
 * @returns Relative time string
 */
export function formatRelativeTime(date: string | Date): string {
  const dateObj = typeof date === 'string' ? new Date(date) : date
  const now = new Date()
  const diffMs = now.getTime() - dateObj.getTime()
  const diffSeconds = Math.floor(diffMs / 1000)
  const diffMinutes = Math.floor(diffSeconds / 60)
  const diffHours = Math.floor(diffMinutes / 60)
  const diffDays = Math.floor(diffHours / 24)

  if (diffSeconds < 60) {
    return 'just now'
  }
  if (diffMinutes < 60) {
    return `${diffMinutes} minute${diffMinutes !== 1 ? 's' : ''} ago`
  }
  if (diffHours < 24) {
    return `${diffHours} hour${diffHours !== 1 ? 's' : ''} ago`
  }
  if (diffDays === 1) {
    return 'yesterday'
  }
  if (diffDays < 7) {
    return `${diffDays} days ago`
  }
  return formatDate(dateObj)
}
