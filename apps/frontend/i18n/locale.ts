export const LOCALE_COOKIE_NAME = 'nommie_locale'

export const SUPPORTED_LOCALES = [
  'en-GB',
  'fr-FR',
  'de-DE',
  'es-ES',
  'it-IT',
] as const

export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number]

export const DEFAULT_LOCALE: SupportedLocale = 'en-GB'

const SUPPORTED_LANGUAGES = ['en', 'fr', 'de', 'es', 'it'] as const

type SupportedLanguage = (typeof SUPPORTED_LANGUAGES)[number]

export type LocaleSource = 'cookie' | 'accept-language' | 'default'

export function isSupportedLocale(value: string): value is SupportedLocale {
  return (SUPPORTED_LOCALES as readonly string[]).includes(value)
}

function isSupportedLanguage(value: string): value is SupportedLanguage {
  return (SUPPORTED_LANGUAGES as readonly string[]).includes(value)
}

export function languageFallback(locale: string): SupportedLocale | null {
  const base = locale.split('-')[0]
  if (!base || !isSupportedLanguage(base)) {
    return null
  }

  switch (base) {
    case 'en':
      return 'en-GB'
    case 'fr':
      return 'fr-FR'
    case 'de':
      return 'de-DE'
    case 'es':
      return 'es-ES'
    case 'it':
      return 'it-IT'
  }
}

export function parseAcceptLanguage(headerValue: string | null): string[] {
  if (!headerValue) return []

  return headerValue
    .split(',')
    .map((part) => part.trim())
    .map((part) => {
      const [tag, ...params] = part.split(';').map((p) => p.trim())
      const qParam = params.find((p) => p.startsWith('q='))
      const q = qParam ? Number(qParam.slice(2)) : 1
      return { tag, q: Number.isFinite(q) ? q : 0 }
    })
    .filter((x) => x.tag.length > 0 && x.q > 0)
    .sort((a, b) => b.q - a.q)
    .map((x) => x.tag)
}

export function resolveLocale(input: {
  cookieLocale?: string | null
  acceptLanguageHeader?: string | null
}): { locale: SupportedLocale; source: LocaleSource } {
  const cookieLocale = input.cookieLocale?.trim()
  if (cookieLocale && isSupportedLocale(cookieLocale)) {
    return { locale: cookieLocale, source: 'cookie' }
  }

  const candidates = parseAcceptLanguage(input.acceptLanguageHeader ?? null)
  for (const candidate of candidates) {
    if (isSupportedLocale(candidate)) {
      return { locale: candidate, source: 'accept-language' }
    }

    const fallback = languageFallback(candidate)
    if (fallback) {
      return { locale: fallback, source: 'accept-language' }
    }
  }

  return { locale: DEFAULT_LOCALE, source: 'default' }
}
