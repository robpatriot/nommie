const fs = require('node:fs')
const path = require('node:path')

const ROOT = path.resolve(process.cwd())
const MESSAGES_DIR = path.join(ROOT, 'messages')

const DEFAULT_LOCALE = 'en-GB'
const NAMESPACES = [
  'common',
  'nav',
  'errors',
  'toasts',
  'settings',
  'lobby',
  'game',
]

// Where to look for translation usage
const SOURCE_DIRS = ['app', 'components', 'hooks', 'lib', 'utils', 'test']

// Some keys are intentionally built dynamically; we skip strict existence checks
const DYNAMIC_PREFIXES = [
  'errors.codes.',
  'toasts.',
  'game.gameRoom.phase.',
  'game.gameRoom.trump.',
  'game.gameRoom.orientation.',
  'game.gameRoom.seat.badge.',
  'game.cards.suit.',
]

function readJson(filePath) {
  const raw = fs.readFileSync(filePath, 'utf8')
  return JSON.parse(raw)
}

function isPlainObject(value) {
  return value != null && typeof value === 'object' && !Array.isArray(value)
}

function flatten(obj, prefix = '') {
  const out = new Set()

  const walk = (value, p) => {
    if (isPlainObject(value)) {
      for (const [k, v] of Object.entries(value)) {
        walk(v, p ? `${p}.${k}` : k)
      }
      return
    }

    if (Array.isArray(value)) {
      throw new Error(`Arrays are not allowed in messages at '${p}'`)
    }

    if (typeof value !== 'string') {
      throw new Error(`Non-string message value at '${p}' (${typeof value})`)
    }

    out.add(p)
  }

  walk(obj, prefix)
  return out
}

function loadAllKeys(locale) {
  const merged = new Set()
  for (const ns of NAMESPACES) {
    const filePath = path.join(MESSAGES_DIR, locale, `${ns}.json`)
    if (!fs.existsSync(filePath)) {
      throw new Error(`Missing messages file: ${path.relative(ROOT, filePath)}`)
    }
    const json = readJson(filePath)
    const flat = flatten(json, ns)
    for (const key of flat) {
      merged.add(key)
    }
  }
  return merged
}

function listFilesRecursive(dir) {
  const out = []
  const entries = fs.readdirSync(dir, { withFileTypes: true })
  for (const entry of entries) {
    const full = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      if (entry.name === 'node_modules' || entry.name === '.next') continue
      out.push(...listFilesRecursive(full))
    } else if (entry.isFile()) {
      if (
        full.endsWith('.ts') ||
        full.endsWith('.tsx') ||
        full.endsWith('.js') ||
        full.endsWith('.jsx')
      ) {
        out.push(full)
      }
    }
  }
  return out
}

function main() {
  // Load declared keys from default locale
  const knownKeys = loadAllKeys(DEFAULT_LOCALE)

  // Collect source files
  const files = SOURCE_DIRS.flatMap((d) => {
    const dir = path.join(ROOT, d)
    return fs.existsSync(dir) ? listFilesRecursive(dir) : []
  })

  const usedKeys = new Set()

  const literalPattern =
    /t(?:Game|Errors|Lobby|Common|Settings)?\(\s*['"`]([^'"`]+)['"`]\s*\)/g

  for (const file of files) {
    const src = fs.readFileSync(file, 'utf8')

    // 1) Direct calls like tGame('game.gameRoom.foo') or t('game.gameRoom.foo')
    let match
    while ((match = literalPattern.exec(src)) !== null) {
      const rawKey = match[1]
      if (!rawKey) continue
      if (rawKey.includes('${')) continue // skip template literals
      if (!rawKey.includes('.')) continue // skip bare keys we can't namespace
      // Track all literal keys used in translation calls, regardless of whether they exist
      // This allows us to detect missing keys that are used but not yet defined
      usedKeys.add(rawKey)
    }

    // 2) useTranslations('game') + t('foo.bar') → assume 'game.foo.bar'
    // This is a simple regex, not a full AST walk, but matches our common pattern.
    const nsMatches = [
      ...src.matchAll(/useTranslations\(\s*['"`]([^'"`]+)['"`]\s*\)/g),
    ]
    const namespaces = nsMatches.map((m) => m[1])
    if (namespaces.length === 0) continue

    // For each namespace found, look for t('<key>') calls in the same file
    const shortCallPattern = /t\(\s*['"`]([^'"`]+)['"`]\s*\)/g
    let m2
    while ((m2 = shortCallPattern.exec(src)) !== null) {
      const suffix = m2[1]
      if (!suffix) continue
      if (suffix.includes('${')) continue

      // If suffix already looks fully-qualified (starts with ns.), keep as-is
      if (NAMESPACES.some((ns) => suffix.startsWith(`${ns}.`))) {
        usedKeys.add(suffix)
      } else {
        for (const ns of namespaces) {
          if (!NAMESPACES.includes(ns)) continue
          if (!suffix.includes('.')) continue
          const candidate = `${ns}.${suffix}`
          // Track all literal keys used in translation calls, regardless of whether they exist
          // This allows us to detect missing keys that are used but not yet defined
          usedKeys.add(candidate)
        }
      }
    }
  }

  // Filter used keys that should have concrete entries in en-GB
  const missing = []
  for (const key of usedKeys) {
    if (!key) continue
    if (!NAMESPACES.some((ns) => key.startsWith(`${ns}.`))) continue
    if (DYNAMIC_PREFIXES.some((p) => key.startsWith(p))) continue

    // Skip if the key already exists
    if (knownKeys.has(key)) continue

    // Check if a longer path exists that would make this a false positive
    // For example, if "lobby.gameList.fields.players" exists, don't report "lobby.fields.players" as missing
    let isFalsePositive = false
    for (const existingKey of knownKeys) {
      // If an existing key is a longer path that contains this key's suffix, it's likely a false positive
      // e.g., "lobby.gameList.fields.players" contains "fields.players" which matches "lobby.fields.players"
      const keyParts = key.split('.')
      const existingParts = existingKey.split('.')

      if (existingParts.length > keyParts.length) {
        // Check if the suffix of the key matches the suffix of the existing key
        const keySuffix = keyParts.slice(1).join('.') // Remove namespace prefix
        const existingSuffix = existingParts.slice(1).join('.') // Remove namespace prefix

        if (
          existingSuffix === keySuffix ||
          existingSuffix.endsWith('.' + keySuffix)
        ) {
          isFalsePositive = true
          break
        }
      }
    }

    if (!isFalsePositive) {
      missing.push(key)
    }
  }

  if (missing.length) {
    missing.sort()
    console.error(
      `[i18n] Missing en-GB translations for used keys (${missing.length}):`
    )
    for (const key of missing) {
      console.error(`- ${key}`)
    }
    process.exit(1)
  }

  console.log('[i18n] OK: all used translation keys exist in en-GB')
}

main()
