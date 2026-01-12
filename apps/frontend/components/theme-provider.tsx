// apps/frontend/components/theme-provider.tsx
'use client'

import {
  createContext,
  startTransition,
  useCallback,
  useContext,
  useEffect,
  useMemo,
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

type ThemeContextValue = {
  colourScheme: ColourScheme
  resolvedColourScheme: ResolvedColourScheme
  setColourScheme: (mode: ColourScheme) => Promise<SimpleActionResult>
  themeName: ThemeName
  setThemeName: (name: ThemeName) => Promise<SimpleActionResult>
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
    if (value === null) {
      window.localStorage.removeItem(key)
    } else {
      window.localStorage.setItem(key, value)
    }
  } catch {
    // ignore storage write errors
  }
}

type ThemeProviderProps = {
  children: React.ReactNode
  initialColourScheme?: ColourScheme
  initialResolved?: ResolvedColourScheme
  initialThemeName?: ThemeName
}

type ApplySource = 'hydrate' | 'user' | 'storage' | 'system'

export function ThemeProvider({
  children,
  initialColourScheme = 'system',
  initialResolved = 'light',
  initialThemeName = 'standard',
}: ThemeProviderProps) {
  const systemPreference = readSystemPreference()

  const [colourScheme, setColourSchemeState] =
    useState<ColourScheme>(initialColourScheme)
  const [systemColourScheme, setSystemColourScheme] =
    useState<ResolvedColourScheme>(systemPreference)
  const [resolvedColourScheme, setResolvedColourScheme] =
    useState<ResolvedColourScheme>(
      initialColourScheme === 'system'
        ? (initialResolved ?? systemPreference)
        : (initialColourScheme as ResolvedColourScheme)
    )
  const [themeName, setThemeNameState] = useState<ThemeName>(initialThemeName)
  const [hydrated, setHydrated] = useState(false)

  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [saveCount, setSaveCount] = useState(0)
  const isSaving = saveCount > 0

  const clearError = useCallback(() => setErrorMessage(null), [])

  const computeResolved = useCallback(
    (mode: ColourScheme, sys: ResolvedColourScheme): ResolvedColourScheme =>
      mode === 'system' ? sys : (mode as ResolvedColourScheme),
    []
  )

  const applyToDom = useCallback(
    (mode: ColourScheme, resolved: ResolvedColourScheme, name: ThemeName) => {
      if (typeof document === 'undefined') return
      const root = document.documentElement

      root.dataset.colourScheme = mode
      root.dataset.themeName = name

      root.classList.toggle('dark', resolved === 'dark')
      root.style.colorScheme = resolved
    },
    []
  )

  const applyLocal = useCallback(
    (
      next: { colourScheme?: ColourScheme; themeName?: ThemeName },
      source: ApplySource
    ) => {
      const nextMode =
        next.colourScheme != null && isValidColourScheme(next.colourScheme)
          ? next.colourScheme
          : colourScheme

      const nextName =
        next.themeName != null && isValidThemeName(next.themeName)
          ? next.themeName
          : themeName

      const nextResolved = computeResolved(nextMode, systemColourScheme)

      startTransition(() => {
        setColourSchemeState(nextMode)
        setThemeNameState(nextName)
        setResolvedColourScheme(nextResolved)
      })

      applyToDom(nextMode, nextResolved, nextName)

      // Write-through cache + cross-tab propagation:
      // Persist preference values (including "system"), not resolved.
      safeWriteLocalStorage(COLOUR_SCHEME_STORAGE_KEY, nextMode)
      safeWriteLocalStorage(THEME_NAME_STORAGE_KEY, nextName)

      void source
      return { nextMode, nextName, nextResolved }
    },
    [applyToDom, colourScheme, computeResolved, systemColourScheme, themeName]
  )

  const persistToBackend = useCallback(
    async (payload: { colour_scheme?: ColourScheme; theme?: ThemeName }) => {
      setSaveCount((n) => n + 1)
      try {
        const result = await updateUserOptionsAction(payload)
        if (result.kind === 'error') {
          setErrorMessage(result.message)
        } else {
          setErrorMessage(null)
        }
        return result
      } finally {
        setSaveCount((n) => Math.max(0, n - 1))
      }
    },
    []
  )

  // Initial hydration alignment:
  // DOM is authoritative (server + boot script already set attrs/class/style).
  // Adopt DOM values and seed localStorage from them.
  useEffect(() => {
    if (typeof document === 'undefined') return

    const domMode = getDomColourScheme()
    const domName = getDomThemeName()

    const nextMode: ColourScheme = domMode ?? initialColourScheme ?? 'system'
    const nextName: ThemeName = domName ?? initialThemeName ?? 'standard'

    // Trust DOM's current dark class to avoid mismatch
    const domResolved: ResolvedColourScheme =
      document.documentElement.classList.contains('dark') ? 'dark' : 'light'

    const nextResolved: ResolvedColourScheme =
      nextMode === 'system' ? domResolved : (nextMode as ResolvedColourScheme)

    startTransition(() => {
      setColourSchemeState(nextMode)
      setThemeNameState(nextName)
      setResolvedColourScheme(nextResolved)
      setHydrated(true)
    })

    applyToDom(nextMode, nextResolved, nextName)

    // Seed localStorage cache (baseline for cross-tab)
    safeWriteLocalStorage(COLOUR_SCHEME_STORAGE_KEY, nextMode)
    safeWriteLocalStorage(THEME_NAME_STORAGE_KEY, nextName)
  }, [applyToDom, initialColourScheme, initialThemeName])

  // Track OS scheme changes: if user preference is "system", update resolved + DOM.
  useEffect(() => {
    if (typeof window === 'undefined') return

    const media = window.matchMedia('(prefers-color-scheme: dark)')

    const updateFromMedia = (matches: boolean) => {
      const nextSys: ResolvedColourScheme = matches ? 'dark' : 'light'

      startTransition(() => {
        setSystemColourScheme(nextSys)
      })

      // Only adjust visuals if user preference is system
      if (getDomColourScheme() === 'system') {
        const nextResolved = nextSys
        startTransition(() => {
          setResolvedColourScheme(nextResolved)
        })
        applyToDom('system', nextResolved, getDomThemeName() ?? themeName)
      }
    }

    updateFromMedia(media.matches)

    const handleChange = (event: MediaQueryListEvent) => {
      updateFromMedia(event.matches)
    }

    media.addEventListener('change', handleChange)
    return () => media.removeEventListener('change', handleChange)
  }, [applyToDom, themeName])

  // Cross-tab sync: react to localStorage changes from other tabs.
  // Apply locally, but do NOT persist to backend (to avoid storms).
  useEffect(() => {
    if (typeof window === 'undefined') return

    const handleStorage = (event: StorageEvent) => {
      if (event.storageArea !== window.localStorage) return

      if (event.key === COLOUR_SCHEME_STORAGE_KEY) {
        const next = event.newValue
        const nextMode: ColourScheme =
          next == null ? 'system' : isValidColourScheme(next) ? next : 'system'
        applyLocal({ colourScheme: nextMode }, 'storage')
        return
      }

      if (event.key === THEME_NAME_STORAGE_KEY) {
        const next = event.newValue
        if (next != null && isValidThemeName(next)) {
          applyLocal({ themeName: next }, 'storage')
        }
      }
    }

    window.addEventListener('storage', handleStorage)
    return () => window.removeEventListener('storage', handleStorage)
  }, [applyLocal])

  const setColourScheme = useCallback(
    async (mode: ColourScheme) => {
      clearError()
      applyLocal({ colourScheme: mode }, 'user')
      return persistToBackend({ colour_scheme: mode })
    },
    [applyLocal, clearError, persistToBackend]
  )

  const setThemeName = useCallback(
    async (name: ThemeName) => {
      clearError()
      applyLocal({ themeName: name }, 'user')
      return persistToBackend({ theme: name })
    },
    [applyLocal, clearError, persistToBackend]
  )

  const contextValue = useMemo<ThemeContextValue>(
    () => ({
      colourScheme,
      resolvedColourScheme,
      setColourScheme,
      themeName,
      setThemeName,
      hydrated,
      isSaving,
      errorMessage,
      clearError,
    }),
    [
      colourScheme,
      resolvedColourScheme,
      setColourScheme,
      themeName,
      setThemeName,
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
  if (!ctx) {
    throw new Error('useTheme must be used within a ThemeProvider')
  }
  return ctx
}
