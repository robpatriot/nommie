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

// Keys that are allowed to be identical to the default locale
// These are cases where the translation is legitimately the same (e.g., abbreviations, proper nouns)
const ALLOWED_IDENTICAL_KEYS = [
  'game.gameRoom.hand.suitAbbrev.H', // Hearts abbreviation is "H" in both English and German
  'game.cards.rank.A', // Ace abbreviation is "A" in all languages (Ace/As/Ass/Asso all start with A)
  'game.cards.rank.K', // King abbreviation is "K" in both English and German (King/König both start with K)
  'game.cards.rank.J', // Jack abbreviation is "J" in both English and Spanish (Jack/Jota both start with J)
  'common.home.aside.bidValue', // Bid value is a number, same in all languages
  'common.home.aside.centerCardRank', // Card rank "A" is universal
  'common.home.aside.centerCardSuit', // Card suit symbol "♠" is universal
  'common.home.aside.trumpSuitIcon', // Card suit symbol "♣" is universal
  'common.home.aside.playerWest', // "West" is the same in English and German (cardinal direction)
]

function readJson(filePath) {
  const raw = fs.readFileSync(filePath, 'utf8')
  return JSON.parse(raw)
}

function isPlainObject(value) {
  return value != null && typeof value === 'object' && !Array.isArray(value)
}

function flatten(obj, prefix = '') {
  const out = new Map()

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

    out.set(p, value)
  }

  walk(obj, prefix)
  return out
}

function loadLocaleNamespace(locale, ns) {
  const filePath = path.join(MESSAGES_DIR, locale, `${ns}.json`)
  if (!fs.existsSync(filePath)) {
    throw new Error(`Missing messages file: ${path.relative(ROOT, filePath)}`)
  }
  return readJson(filePath)
}

function loadAll(locale) {
  const merged = new Map()
  for (const ns of NAMESPACES) {
    const json = loadLocaleNamespace(locale, ns)
    const flat = flatten(json, ns)
    for (const [k, v] of flat.entries()) {
      if (merged.has(k)) {
        throw new Error(`Duplicate key '${k}' for locale ${locale}`)
      }
      merged.set(k, v)
    }
  }
  return merged
}

function loadKnownErrorCodes() {
  const errorsTsPath = path.join(ROOT, 'i18n', 'errors.ts')
  if (!fs.existsSync(errorsTsPath)) {
    throw new Error(
      `Missing i18n error code source file: ${path.relative(ROOT, errorsTsPath)}`
    )
  }
  const src = fs.readFileSync(errorsTsPath, 'utf8')
  const marker = 'KNOWN_ERROR_CODES = ['
  const start = src.indexOf(marker)
  if (start === -1) {
    throw new Error('Could not find KNOWN_ERROR_CODES array in i18n/errors.ts')
  }
  const afterMarker = src.slice(start + marker.length)
  const endIndex = afterMarker.indexOf('] as const')
  if (endIndex === -1) {
    throw new Error(
      'Could not find end of KNOWN_ERROR_CODES array in i18n/errors.ts'
    )
  }
  const arrayText = afterMarker.slice(0, endIndex)
  const codes = []
  for (const line of arrayText.split('\n')) {
    const match = line.match(/'([A-Z0-9_]+)'/)
    if (match) {
      codes.push(match[1])
    }
  }
  return codes
}

function diffKeys(a, b) {
  const missing = []
  for (const key of a.keys()) {
    if (!b.has(key)) missing.push(key)
  }
  const extra = []
  for (const key of b.keys()) {
    if (!a.has(key)) extra.push(key)
  }
  missing.sort()
  extra.sort()
  return { missing, extra }
}

function main() {
  const base = loadAll(DEFAULT_LOCALE)
  let failed = false

  // Ensure that all KNOWN_ERROR_CODES have corresponding errors.codes.<CODE> entries
  // and that there are no extra codes in the messages that are not in the registry.
  const knownCodes = loadKnownErrorCodes()
  const messageCodes = []
  for (const key of base.keys()) {
    if (key.startsWith('errors.codes.')) {
      messageCodes.push(key.slice('errors.codes.'.length))
    }
  }
  const missingCodeMessages = knownCodes.filter(
    (code) => !messageCodes.includes(code)
  )
  const extraCodeMessages = messageCodes.filter(
    (code) => !knownCodes.includes(code)
  )
  if (missingCodeMessages.length || extraCodeMessages.length) {
    failed = true
    console.error('\n[i18n] Error code/message mismatch')
    if (missingCodeMessages.length) {
      console.error(
        `- Missing messages for error codes (${missingCodeMessages.length}):`
      )
      for (const code of missingCodeMessages) {
        console.error(`  - errors.codes.${code}`)
      }
    }
    if (extraCodeMessages.length) {
      console.error(
        `- Extra error code messages not in KNOWN_ERROR_CODES (${extraCodeMessages.length}):`
      )
      for (const code of extraCodeMessages) {
        console.error(`  - errors.codes.${code}`)
      }
    }
  }

  for (const locale of LOCALES) {
    const current = loadAll(locale)
    const { missing, extra } = diffKeys(base, current)

    if (missing.length || extra.length) {
      failed = true
      console.error(`\n[i18n] Key mismatch for locale ${locale}`)
      if (missing.length) {
        console.error(`- Missing (${missing.length}):`)
        for (const k of missing) console.error(`  - ${k}`)
      }
      if (extra.length) {
        console.error(`- Extra (${extra.length}):`)
        for (const k of extra) console.error(`  - ${k}`)
      }
    }

    if (locale !== DEFAULT_LOCALE) {
      const untranslated = []
      for (const [k, v] of current.entries()) {
        const baseValue = base.get(k)
        if (baseValue === undefined) continue
        if (v === baseValue && !ALLOWED_IDENTICAL_KEYS.includes(k)) {
          untranslated.push(k)
        }
      }
      untranslated.sort()
      if (untranslated.length) {
        failed = true
        console.error(
          `\n[i18n] Untranslated values for locale ${locale} (must differ from ${DEFAULT_LOCALE})`
        )
        for (const k of untranslated) {
          console.error(`- ${k}`)
        }
      }
    }
  }

  if (failed) {
    process.exit(1)
  }

  console.log('[i18n] OK: keys aligned and translations differ from en-GB')
}

main()
