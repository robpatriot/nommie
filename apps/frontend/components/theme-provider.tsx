// apps/frontend/components/theme-provider.tsx
'use client'

import {
  createContext,
  startTransition,
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from 'react'
import type { SimpleActionResult } from '@/lib/api/action-helpers'
import { updateUserOptionsAction } from '@/app/actions/settings-actions'

export type ColourScheme = 'light' | 'dark' | 'system'
export type ResolvedColourScheme = 'light' | 'dark'
export type ThemeName = 'standard' | 'high_roller' | 'oldtime'

export const COLOUR_SCHEME_STORAGE_KEY = 'nommie.colour_scheme'
export const THEME_NAME_STORAGE_KEY = 'nommie.theme_name'

export const COLOUR_SCHEMES = [
  'light',
  'dark',
  'system',
] as const satisfies readonly ColourScheme[]

export const THEME_NAMES = [
  'standard',
  'high_roller',
  'oldtime',
] as const satisfies readonly ThemeName[]

const isOneOf = <T extends readonly string[]>(
  values: T,
  value: unknown
): value is T[number] =>
  typeof value === 'string' && (values as readonly string[]).includes(value)

export const isValidColourScheme = (value: unknown): value is ColourScheme =>
  isOneOf(COLOUR_SCHEMES, value)

export const isValidThemeName = (value: unknown): value is ThemeName =>
  isOneOf(THEME_NAMES, value)

type ApplyPreferencesPatch = Partial<{
  themeName: ThemeName
  colourScheme: ColourScheme
}>

type ApplyPreferencesOptions = {
  // Persist to backend DB?
  persistBackend?: boolean
  // Write localStorage baseline?
  persistStorage?: boolean
}

type ThemeContextValue = {
  themeName: ThemeName
  colourScheme: ColourScheme
  resolvedColourScheme: ResolvedColourScheme

  applyPreferences: (
    patch: ApplyPreferencesPatch,
    options?: ApplyPreferencesOptions
  ) => Promise<SimpleActionResult>

  hydrated: boolean
  isSaving: boolean
  errorMessage: string | null
  clearError: () => void
}

const ThemeContext = createContext<ThemeContextValue | undefined>(undefined)

const getDomColourScheme = (): ColourScheme | undefined => {
  if (typeof document === 'undefined') return undefined
  const attr = document.documentElement.dataset.colourScheme
  return isValidColourScheme(attr) ? attr : undefined
}

const getDomThemeName = (): ThemeName | undefined => {
  if (typeof document === 'undefined') return undefined
  const attr = document.documentElement.dataset.themeName
  return isValidThemeName(attr) ? attr : undefined
}

const readSystemPreference = (): ResolvedColourScheme => {
  if (typeof window === 'undefined') return 'light'
  try {
    return window.matchMedia('(prefers-color-scheme: dark)').matches
      ? 'dark'
      : 'light'
  } catch {
    return 'light'
  }
}

const safeWriteLocalStorage = (key: string, value: string | null) => {
  if (typeof window === 'undefined') return
  try {
    const current = window.localStorage.getItem(key)
    if (value === null) {
      if (current !== null) window.localStorage.removeItem(key)
      return
    }
    if (current !== value) window.localStorage.setItem(key, value)
  } catch {
    // ignore
  }
}

type ThemeProviderProps = {
  children: React.ReactNode
  initialColourScheme?: ColourScheme
  initialThemeName?: ThemeName

  // tell client when logout/login has occurred
  isAuthenticated: boolean
}

export function ThemeProvider({
  children,
  initialColourScheme = 'system',
  initialThemeName = 'standard',
  isAuthenticated,
}: ThemeProviderProps) {
  const [themeName, setThemeName] = useState<ThemeName>(initialThemeName)
  const [colourScheme, setColourScheme] =
    useState<ColourScheme>(initialColourScheme)
  // We set this to an initial value but it's immediately over written from the DOM
  const [resolvedColourScheme, setResolvedColourScheme] =
    useState<ResolvedColourScheme>(
      initialColourScheme === 'dark' ? 'dark' : 'light'
    )

  const [hydrated, setHydrated] = useState(false)

  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [saveCount, setSaveCount] = useState(0)
  const isSaving = saveCount > 0

  const clearError = useCallback(() => setErrorMessage(null), [])

  const applyThemeNameToDom = useCallback((name: ThemeName) => {
    if (typeof document === 'undefined') return
    const root = document.documentElement
    if (root.dataset.themeName !== name) {
      root.dataset.themeName = name
    }
  }, [])

  const applyColourSchemeToDom = useCallback(
    (mode: ColourScheme, resolved: ResolvedColourScheme) => {
      if (typeof document === 'undefined') return
      const root = document.documentElement

      if (root.dataset.colourScheme !== mode) {
        root.dataset.colourScheme = mode
      }

      const shouldBeDark = resolved === 'dark'
      if (root.classList.contains('dark') !== shouldBeDark) {
        root.classList.toggle('dark', shouldBeDark)
      }

      if (root.style.colorScheme !== resolved) {
        root.style.colorScheme = resolved
      }
    },
    []
  )

  const persistToBackend = useCallback(
    async (payload: { colour_scheme?: ColourScheme; theme?: ThemeName }) => {
      setSaveCount((n) => n + 1)
      try {
        const result = await updateUserOptionsAction(payload)
        if (result.kind === 'error') setErrorMessage(result.message)
        else setErrorMessage(null)
        return result
      } finally {
        setSaveCount((n) => Math.max(0, n - 1))
      }
    },
    []
  )

  const applyPreferences = useCallback(
    async (patch: ApplyPreferencesPatch, options?: ApplyPreferencesOptions) => {
      const persistBackend = options?.persistBackend ?? false
      const persistStorage = options?.persistStorage ?? false

      // Validate patch
      if (patch.themeName != null && !isValidThemeName(patch.themeName)) {
        return { kind: 'ok' } as SimpleActionResult
      }
      if (
        patch.colourScheme != null &&
        !isValidColourScheme(patch.colourScheme)
      ) {
        return { kind: 'ok' } as SimpleActionResult
      }

      // Nothing to do
      if (patch.themeName == null && patch.colourScheme == null) {
        return { kind: 'ok' } as SimpleActionResult
      }

      const nextThemeName = patch.themeName ?? themeName
      const nextColourScheme = patch.colourScheme ?? colourScheme

      // Resolve only if colourScheme is being changed (patch-based behaviour)
      let nextResolved = resolvedColourScheme
      if (patch.colourScheme != null) {
        nextResolved =
          nextColourScheme === 'system'
            ? readSystemPreference()
            : nextColourScheme
      }

      const themeChanged = nextThemeName !== themeName
      const schemeChanged = nextColourScheme !== colourScheme
      const resolvedChanged = nextResolved !== resolvedColourScheme

      // Commit state
      startTransition(() => {
        if (themeChanged) setThemeName(nextThemeName)
        if (schemeChanged) setColourScheme(nextColourScheme)
        if (resolvedChanged) setResolvedColourScheme(nextResolved)
      })

      // Patch DOM writes (only affected fields)
      if (themeChanged) {
        applyThemeNameToDom(nextThemeName)
      }
      if (schemeChanged || resolvedChanged) {
        applyColourSchemeToDom(nextColourScheme, nextResolved)
      }

      // Storage baseline writes (never on storage-sourced applies)
      if (persistStorage) {
        if (themeChanged)
          safeWriteLocalStorage(THEME_NAME_STORAGE_KEY, nextThemeName)
        if (schemeChanged)
          safeWriteLocalStorage(COLOUR_SCHEME_STORAGE_KEY, nextColourScheme)
      }

      // Backend writes (only for explicit user actions)
      if (persistBackend) {
        const payload: { colour_scheme?: ColourScheme; theme?: ThemeName } = {}
        if (themeChanged) payload.theme = nextThemeName
        if (schemeChanged) payload.colour_scheme = nextColourScheme

        if (payload.theme != null || payload.colour_scheme != null) {
          return persistToBackend(payload)
        }
      }

      return { kind: 'ok' } as SimpleActionResult
    },
    [
      themeName,
      colourScheme,
      resolvedColourScheme,
      applyThemeNameToDom,
      applyColourSchemeToDom,
      persistToBackend,
    ]
  )

  // Seed once from DOM (authoritative at hydration time).
  // This is intentionally NOT routed through applyPreferences.
  useEffect(() => {
    if (hydrated) return
    if (typeof document === 'undefined') return

    const domMode: ColourScheme =
      getDomColourScheme() ?? initialColourScheme ?? 'system'
    const domName: ThemeName =
      getDomThemeName() ?? initialThemeName ?? 'standard'
    const domResolved: ResolvedColourScheme =
      document.documentElement.classList.contains('dark') ? 'dark' : 'light'

    startTransition(() => {
      setThemeName(domName)
      setColourScheme(domMode)
      setResolvedColourScheme(domMode === 'system' ? domResolved : domMode)
      setHydrated(true)
    })

    // Seed baseline storage from DOM once
    safeWriteLocalStorage(THEME_NAME_STORAGE_KEY, domName)
    safeWriteLocalStorage(COLOUR_SCHEME_STORAGE_KEY, domMode)
  }, [hydrated, initialColourScheme, initialThemeName])

  // OS scheme changes: incremental only (no preference changes, no storage, no backend)
  useEffect(() => {
    if (typeof window === 'undefined') return

    const media = window.matchMedia('(prefers-color-scheme: dark)')

    const updateFromMedia = (matches: boolean) => {
      if (colourScheme !== 'system') return

      const nextResolved: ResolvedColourScheme = matches ? 'dark' : 'light'

      startTransition(() => {
        setResolvedColourScheme(nextResolved)
      })

      applyColourSchemeToDom('system', nextResolved)
    }

    updateFromMedia(media.matches)

    const handleChange = (event: MediaQueryListEvent) =>
      updateFromMedia(event.matches)

    media.addEventListener('change', handleChange)
    return () => media.removeEventListener('change', handleChange)
  }, [colourScheme, applyColourSchemeToDom])

  // Cross-tab sync: input-only, no echo
  useEffect(() => {
    if (typeof window === 'undefined') return

    const handleStorage = (event: StorageEvent) => {
      if (event.storageArea !== window.localStorage) return

      if (event.key === COLOUR_SCHEME_STORAGE_KEY) {
        const next = event.newValue
        const nextMode: ColourScheme =
          next == null ? 'system' : isValidColourScheme(next) ? next : 'system'

        void applyPreferences(
          { colourScheme: nextMode },
          { persistBackend: false, persistStorage: false }
        )
        return
      }

      if (event.key === THEME_NAME_STORAGE_KEY) {
        const next = event.newValue
        if (next != null && isValidThemeName(next)) {
          void applyPreferences(
            { themeName: next },
            { persistBackend: false, persistStorage: false }
          )
        }
      }
    }

    window.addEventListener('storage', handleStorage)
    return () => window.removeEventListener('storage', handleStorage)
  }, [applyPreferences])

  // SPA logout fix: when auth transitions to unauthenticated, reassert public defaults locally.
  // We run in layout effect to minimize chance of a flash (we can revisit later if needed).
  const prevAuthRef = useRef<boolean>(isAuthenticated)
  useLayoutEffect(() => {
    const prev = prevAuthRef.current
    prevAuthRef.current = isAuthenticated

    if (!hydrated) return

    // only on transition true -> false
    if (prev && !isAuthenticated) {
      // Local only: do not persist backend or storage.
      void applyPreferences(
        { themeName: 'standard', colourScheme: 'system' },
        { persistBackend: false, persistStorage: false }
      )
    }
  }, [hydrated, isAuthenticated, applyPreferences])

  const contextValue = useMemo<ThemeContextValue>(
    () => ({
      themeName,
      colourScheme,
      resolvedColourScheme,
      applyPreferences,
      hydrated,
      isSaving,
      errorMessage,
      clearError,
    }),
    [
      themeName,
      colourScheme,
      resolvedColourScheme,
      applyPreferences,
      hydrated,
      isSaving,
      errorMessage,
      clearError,
    ]
  )

  return (
    <ThemeContext.Provider value={contextValue}>
      {children}
    </ThemeContext.Provider>
  )
}

export function useTheme() {
  const ctx = useContext(ThemeContext)
  if (!ctx) throw new Error('useTheme must be used within a ThemeProvider')
  return ctx
}
