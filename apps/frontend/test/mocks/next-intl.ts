import { vi } from 'vitest'
import { readFileSync } from 'fs'
import { join } from 'path'
import { fileURLToPath } from 'url'

// Resolve path to apps/frontend directory
// From test/mocks/next-intl.ts, go up to apps/frontend
const __filename = fileURLToPath(import.meta.url)
const testMocksDir = join(__filename, '..')
const testDir = join(testMocksDir, '..')
const frontendDir = join(testDir, '..')

// Load actual translation files
function loadMessages(namespace: string): Record<string, unknown> {
  try {
    const filePath = join(frontendDir, 'messages', 'en-GB', `${namespace}.json`)
    const content = readFileSync(filePath, 'utf-8')
    return JSON.parse(content)
  } catch (error) {
    // Log error in development to help debug
    if (process.env.NODE_ENV !== 'production') {
      console.warn(`Failed to load messages for namespace ${namespace}:`, error)
    }
    return {}
  }
}

// Flatten nested object to dot-notation keys
function flattenMessages(
  obj: Record<string, unknown>,
  prefix = ''
): Record<string, string> {
  const result: Record<string, string> = {}
  for (const [key, value] of Object.entries(obj)) {
    const fullKey = prefix ? `${prefix}.${key}` : key
    if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
      Object.assign(
        result,
        flattenMessages(value as Record<string, unknown>, fullKey)
      )
    } else if (typeof value === 'string') {
      result[fullKey] = value
    }
  }
  return result
}

// Load all namespaces
const namespaces = [
  'common',
  'nav',
  'settings',
  'errors',
  'toasts',
  'lobby',
  'game',
]
const allMessages: Record<string, string> = {}

for (const ns of namespaces) {
  const messages = loadMessages(ns)
  const flattened = flattenMessages(messages, ns)
  Object.assign(allMessages, flattened)
}

// Mock translation function that looks up actual translations
function createMockT(namespace?: string) {
  return (key: string, values?: Record<string, unknown>) => {
    // Build full key: if namespace is provided, prepend it to the key
    // e.g., useTranslations('game.gameRoom') + t('ready.title') = 'game.gameRoom.ready.title'
    const fullKey = namespace ? `${namespace}.${key}` : key

    // Try to find the translation, fall back to key if not found
    let result = allMessages[fullKey] || allMessages[key] || key

    // Format with values if provided (simple placeholder replacement)
    if (values) {
      for (const [k, v] of Object.entries(values)) {
        // Handle simple {key} replacements
        result = result.replace(new RegExp(`\\{${k}\\}`, 'g'), String(v))
        // Note: ICU MessageFormat pluralization would need a proper library
        // For now, tests will see the pattern if pluralization is used
      }
    }
    return result
  }
}

// Mock locale
let mockLocale = 'en-GB'

export const mockUseTranslations = (namespace?: string) => {
  return createMockT(namespace)
}

export const mockUseLocale = () => {
  return mockLocale
}

export const mockSetLocale = (locale: string) => {
  mockLocale = locale
}

// Mock the next-intl module
vi.mock('next-intl', () => ({
  useTranslations: (namespace?: string) => mockUseTranslations(namespace),
  useLocale: () => mockUseLocale(),
  NextIntlClientProvider: ({ children }: { children: React.ReactNode }) => {
    return children
  },
}))

// Mock next-intl/server
vi.mock('next-intl/server', () => ({
  getTranslations: async (namespace?: string) => {
    return createMockT(namespace)
  },
}))
