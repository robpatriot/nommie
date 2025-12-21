import type { SupportedLocale } from './locale'

export type Namespace =
  | 'common'
  | 'nav'
  | 'settings'
  | 'errors'
  | 'toasts'
  | 'lobby'
  | 'game'

export async function loadNamespace(
  locale: SupportedLocale,
  namespace: Namespace
): Promise<Record<string, unknown>> {
  const mod = await import(`../messages/${locale}/${namespace}.json`)
  return (mod as { default: Record<string, unknown> }).default
}

export async function loadMessages(
  locale: SupportedLocale,
  namespaces: Namespace[]
): Promise<Record<string, unknown>> {
  const entries = await Promise.all(
    namespaces.map(async (ns) => {
      return [ns, await loadNamespace(locale, ns)] as const
    })
  )

  return Object.fromEntries(entries)
}
