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

export type ColourScheme = 'light' | 'dark' | 'system'
export type ResolvedColourScheme = 'light' | 'dark'
export type ThemeName = 'standard' | 'high_roller' | 'oldtime'

const COLOUR_SCHEME_STORAGE_KEY = 'nommie.colour_scheme'
const THEME_NAME_STORAGE_KEY = 'nommie.theme_name'

const COLOUR_SCHEME_COOKIE_KEY = 'nommie_colour_scheme'

type ThemeContextValue = {
  colourScheme: ColourScheme
  resolvedColourScheme: ResolvedColourScheme
  setColourScheme: (mode: ColourScheme) => void
  themeName: ThemeName
  setThemeName: (name: ThemeName) => void
  hydrated: boolean
}

const ThemeContext = createContext<ThemeContextValue | undefined>(undefined)

const isColourScheme = (value: unknown): value is ColourScheme =>
  typeof value === 'string' &&
  (value === 'light' || value === 'dark' || value === 'system')

const isThemeName = (value: unknown): value is ThemeName =>
  typeof value === 'string' &&
  (value === 'standard' || value === 'high_roller' || value === 'oldtime')

const getDomUserColourScheme = (): ColourScheme | undefined => {
  if (typeof document === 'undefined') return undefined
  const attr = document.documentElement.dataset.colourScheme
  return isColourScheme(attr) ? attr : undefined
}

const getDomThemeName = (): ThemeName | undefined => {
  if (typeof document === 'undefined') return undefined
  const attr = document.documentElement.dataset.themeName
  return isThemeName(attr) ? attr : undefined
}

const readStoredColourScheme = (): ColourScheme => {
  if (typeof window === 'undefined') return 'system'
  try {
    const stored = window.localStorage.getItem(COLOUR_SCHEME_STORAGE_KEY)
    if (isColourScheme(stored)) return stored
  } catch {
    // ignore storage access errors
  }
  return 'system'
}

const readStoredThemeName = (): ThemeName => {
  if (typeof window === 'undefined') return 'standard'
  try {
    const stored = window.localStorage.getItem(THEME_NAME_STORAGE_KEY)
    if (isThemeName(stored)) return stored
  } catch {
    // ignore storage access errors
  }
  return 'standard'
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

type ThemeProviderProps = {
  children: React.ReactNode
  initialColourScheme?: ColourScheme
  initialResolved?: ResolvedColourScheme
  initialThemeName?: ThemeName
}

export function ThemeProvider({
  children,
  initialColourScheme = 'system',
  initialResolved = 'light',
  initialThemeName = 'standard',
}: ThemeProviderProps) {
  // Compute initial values directly instead of using refs (React 19 doesn't allow ref access during render)
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

  const applyTheme = useCallback(
    (
      mode: ColourScheme,
      nextResolved: ResolvedColourScheme,
      name: ThemeName
    ) => {
      if (typeof document === 'undefined') return
      const root = document.documentElement

      // New DOM contract: keep everything as "colourScheme" + "themeName"
      root.dataset.colourScheme = mode
      root.dataset.themeName = name

      root.classList.toggle('dark', nextResolved === 'dark')
      root.style.colorScheme = nextResolved
    },
    []
  )

  // Initial hydration alignment:
  // Prefer DOM (set by the inline boot script) if present, else fall back to localStorage,
  // else use server-provided initial props.
  useEffect(() => {
    if (typeof document === 'undefined') return

    const domMode = getDomUserColourScheme()
    const storedMode = readStoredColourScheme()
    const nextMode = domMode ?? storedMode ?? initialColourScheme

    const domName = getDomThemeName()
    const storedName = readStoredThemeName()

    // If server provided a non-default themeName, prefer it.
    const shouldPreferServerThemeName = initialThemeName !== 'standard'
    const nextName = shouldPreferServerThemeName
      ? initialThemeName
      : (domName ?? storedName ?? 'standard')

    // Determine resolved scheme:
    // - If the boot script already toggled 'dark', trust DOM (avoids mismatch)
    // - Otherwise fall back to the computed system preference
    const domResolved: ResolvedColourScheme =
      document.documentElement.classList.contains('dark') ? 'dark' : 'light'

    const nextResolved: ResolvedColourScheme =
      nextMode === 'system' ? domResolved : (nextMode as ResolvedColourScheme)

    startTransition(() => {
      setColourSchemeState(nextMode)
      setResolvedColourScheme(nextResolved)
      setThemeNameState(nextName)
      setHydrated(true)
    })

    applyTheme(nextMode, nextResolved, nextName)

    // If server has a preferred theme name, persist it for future loads
    if (typeof window !== 'undefined') {
      try {
        const storedRaw = window.localStorage.getItem(THEME_NAME_STORAGE_KEY)
        if (
          shouldPreferServerThemeName &&
          (storedRaw === null || storedRaw !== initialThemeName)
        ) {
          window.localStorage.setItem(THEME_NAME_STORAGE_KEY, initialThemeName)
        }
      } catch {
        // ignore storage write errors
      }
    }
  }, [applyTheme, initialColourScheme, initialThemeName])

  // Track system scheme changes
  useEffect(() => {
    if (typeof window === 'undefined') return

    const media = window.matchMedia('(prefers-color-scheme: dark)')
    const handleChange = (event: MediaQueryListEvent) => {
      setSystemColourScheme(event.matches ? 'dark' : 'light')
    }

    startTransition(() => {
      setSystemColourScheme(media.matches ? 'dark' : 'light')
    })

    media.addEventListener('change', handleChange)
    return () => media.removeEventListener('change', handleChange)
  }, [])

  // Cross-tab sync
  useEffect(() => {
    if (typeof window === 'undefined') return

    const handleStorage = (event: StorageEvent) => {
      if (event.key === COLOUR_SCHEME_STORAGE_KEY) {
        const next = event.newValue
        if (isColourScheme(next)) {
          setColourSchemeState(next)
        } else if (next === null) {
          setColourSchemeState('system')
        }
        return
      }

      if (event.key === THEME_NAME_STORAGE_KEY) {
        const next = event.newValue
        if (isThemeName(next)) {
          setThemeNameState(next)
        }
      }
    }

    window.addEventListener('storage', handleStorage)
    return () => window.removeEventListener('storage', handleStorage)
  }, [])

  // Apply any changes to DOM + cookie (cookie is Step 3, but keeping current behaviour)
  useEffect(() => {
    const nextResolved: ResolvedColourScheme =
      colourScheme === 'system'
        ? systemColourScheme
        : (colourScheme as ResolvedColourScheme)

    startTransition(() => {
      setResolvedColourScheme(nextResolved)
    })

    applyTheme(colourScheme, nextResolved, themeName)

    if (typeof window !== 'undefined') {
      try {
        const cookieValue =
          colourScheme === 'system' ? `system:${nextResolved}` : colourScheme
        const maxAge = 60 * 60 * 24 * 365 // 1 year
        document.cookie = `${COLOUR_SCHEME_COOKIE_KEY}=${cookieValue}; path=/; max-age=${maxAge}; samesite=lax`
      } catch {
        // ignore cookie write errors
      }
    }
  }, [colourScheme, systemColourScheme, themeName, applyTheme])

  const setColourScheme = useCallback((mode: ColourScheme) => {
    setColourSchemeState(mode)
    if (typeof window === 'undefined') return

    try {
      if (mode === 'system') {
        window.localStorage.removeItem(COLOUR_SCHEME_STORAGE_KEY)
      } else {
        window.localStorage.setItem(COLOUR_SCHEME_STORAGE_KEY, mode)
      }
    } catch {
      // ignore storage write errors
    }
  }, [])

  const setThemeName = useCallback((name: ThemeName) => {
    setThemeNameState(name)
    if (typeof window === 'undefined') return

    try {
      window.localStorage.setItem(THEME_NAME_STORAGE_KEY, name)
    } catch {
      // ignore storage write errors
    }
  }, [])

  const contextValue = useMemo<ThemeContextValue>(
    () => ({
      colourScheme,
      resolvedColourScheme,
      setColourScheme,
      themeName,
      setThemeName,
      hydrated,
    }),
    [
      colourScheme,
      resolvedColourScheme,
      setColourScheme,
      themeName,
      setThemeName,
      hydrated,
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
