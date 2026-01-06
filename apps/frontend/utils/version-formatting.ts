/**
 * Utility functions for formatting version numbers.
 */

/**
 * Formats a version string by removing trailing zeros and associated dots.
 *
 * Examples:
 * - "1.0.0" -> "1"
 * - "1.2.0" -> "1.2"
 * - "1.2.3" -> "1.2.3"
 * - "2.0" -> "2"
 * - "1.0" -> "1"
 *
 * @param version - Version string (e.g., "1.0.0")
 * @returns Formatted version string with trailing zeros removed
 */
export function formatVersion(version: string): string {
  if (!version || version.trim() === '') {
    return version
  }

  // Split by dots
  const parts = version.split('.')

  // Find the last non-zero numeric part
  let lastNonZeroIndex = parts.length - 1
  while (lastNonZeroIndex >= 0) {
    const part = parts[lastNonZeroIndex]
    const num = parseInt(part, 10)
    // If it's a valid number and not zero, stop
    if (!isNaN(num) && num !== 0) {
      break
    }
    // If it's not a valid number (e.g., "alpha", "beta"), keep it
    if (isNaN(num)) {
      break
    }
    // If it's zero, continue looking backwards
    lastNonZeroIndex--
  }

  // If all parts were zeros, return "0"
  if (lastNonZeroIndex < 0) {
    return '0'
  }

  // Return the version string up to the last non-zero part
  return parts.slice(0, lastNonZeroIndex + 1).join('.')
}
