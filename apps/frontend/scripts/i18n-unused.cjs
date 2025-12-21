const fs = require('node:fs')
const path = require('node:path')

const ROOT = path.resolve(process.cwd())
const MESSAGES_DIR = path.join(ROOT, 'messages')

const DEFAULT_LOCALE = 'en-GB'
const LOCALES = ['en-GB', 'fr-FR', 'de-DE', 'es-ES', 'it-IT']
const NAMESPACES = [
  'common',
  'nav',
  'errors',
  'toasts',
  'settings',
  'lobby',
  'game',
]

const SOURCE_DIRS = ['app', 'components', 'hooks', 'lib', 'utils', 'test']

const DYNAMIC_PREFIXES = [
  'settings.appearance.options.',
  'settings.language.options.',
  'errors.codes.',
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

function escapeRegExp(s) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function main() {
  for (const locale of LOCALES) {
    const localeDir = path.join(MESSAGES_DIR, locale)
    if (!fs.existsSync(localeDir)) {
      throw new Error(
        `Missing locale directory: ${path.relative(ROOT, localeDir)}`
      )
    }
  }

  const keys = loadAllKeys(DEFAULT_LOCALE)

  const files = SOURCE_DIRS.flatMap((d) => {
    const dir = path.join(ROOT, d)
    return fs.existsSync(dir) ? listFilesRecursive(dir) : []
  })

  const corpus = files.map((f) => fs.readFileSync(f, 'utf8')).join('\n')

  // Extract all namespace patterns from the codebase
  const namespaceMatches = [
    ...corpus.matchAll(
      /(useTranslations|getTranslations)\s*\(\s*["'`]([^"'`]+)["'`]/g
    ),
  ]
  const usedNamespaces = new Set(
    namespaceMatches.map((m) => m[2]).filter((ns) => ns.length > 0)
  )

  const unused = []
  for (const key of keys) {
    const isCoveredByDynamicPrefix = DYNAMIC_PREFIXES.some((p) =>
      key.startsWith(p)
    )
    if (isCoveredByDynamicPrefix) continue

    // Check if key is used as full string literal
    const fullKeyPattern = new RegExp(`["'\`]${escapeRegExp(key)}["'\`]`, 'g')
    if (fullKeyPattern.test(corpus)) {
      continue // Key is used as full string
    }

    // Check if key is used with namespace prefix
    // e.g., key = "common.home.hero.kicker"
    // Check for: useTranslations('common') + t('home.hero.kicker')
    // or: useTranslations('common.home') + t('hero.kicker')
    // or: useTranslations('common.home.hero') + t('kicker')
    const keyParts = key.split('.')
    let found = false

    for (let i = 0; i < keyParts.length - 1; i++) {
      const namespace = keyParts.slice(0, i + 1).join('.')
      const keySuffix = keyParts.slice(i + 1).join('.')

      if (usedNamespaces.has(namespace)) {
        // Check if the key suffix is used in the code (as string literal)
        const keySuffixPattern = new RegExp(
          `["'\`]${escapeRegExp(keySuffix)}["'\`]`,
          'g'
        )
        if (keySuffixPattern.test(corpus)) {
          found = true
          break
        }

        // Check if the key suffix is used in template literals
        // e.g., t(`gameStates.${state}`) where namespace is 'lobby.gameList'
        // and keySuffix is 'gameStates.LOBBY'
        // We need to check if any parent of keySuffix is in a template literal
        const suffixParts = keySuffix.split('.')
        for (let j = suffixParts.length - 1; j > 0; j--) {
          const suffixParent = suffixParts.slice(0, j).join('.')
          const templatePattern = new RegExp(
            '`[^`]*' + escapeRegExp(suffixParent) + '[^`]*\\$\\{[^}]+\\}[^`]*`',
            'g'
          )
          if (templatePattern.test(corpus)) {
            found = true
            break
          }
        }
        if (found) break
      }
    }

    if (!found) {
      unused.push(key)
    }
  }

  unused.sort()

  if (unused.length) {
    console.error(`[i18n] Unused keys (${unused.length}):`)
    for (const k of unused) console.error(`- ${k}`)
    process.exit(1)
  }

  console.log('[i18n] OK: no unused keys detected')
}

main()
