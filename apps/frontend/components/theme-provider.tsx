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

export type ThemeMode = 'light' | 'dark' | 'system'
export type ResolvedTheme = 'light' | 'dark'

const STORAGE_KEY = 'nommie.theme'
const COOKIE_KEY = 'nommie_theme'

type ThemeContextValue = {
  theme: ThemeMode
  resolvedTheme: ResolvedTheme
  setTheme: (mode: ThemeMode) => void
  hydrated: boolean
}

const ThemeContext = createContext<ThemeContextValue | undefined>(undefined)

const isThemeMode = (value: unknown): value is ThemeMode =>
  typeof value === 'string' &&
  (value === 'light' || value === 'dark' || value === 'system')

const getDomUserTheme = (): ThemeMode | undefined => {
  if (typeof document === 'undefined') {
    return undefined
  }
  const attr = document.documentElement.dataset.userTheme
  if (attr === 'light' || attr === 'dark' || attr === 'system') {
    return attr
  }
  return undefined
}

const readStoredTheme = (): ThemeMode => {
  if (typeof window === 'undefined') {
    return 'system'
  }
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY)
    if (isThemeMode(stored)) {
      return stored
    }
  } catch {
    // ignore storage access errors
  }
  return 'system'
}

const readSystemPreference = (): ResolvedTheme => {
  if (typeof window === 'undefined') {
    return 'light'
  }
  try {
    return window.matchMedia('(prefers-color-scheme: dark)').matches
      ? 'dark'
      : 'light'
  } catch {
    return 'light'
  }
}

type InitialThemeState = {
  theme: ThemeMode
  systemTheme: ResolvedTheme
  resolvedTheme: ResolvedTheme
}

type ThemeProviderProps = {
  children: React.ReactNode
  initialTheme?: ThemeMode
  initialResolved?: ResolvedTheme
}

export function ThemeProvider({
  children,
  initialTheme = 'system',
  initialResolved = 'light',
}: ThemeProviderProps) {
  // Compute initial values directly instead of using refs (React 19 doesn't allow ref access during render)
  const systemPreference = readSystemPreference()
  const initialThemeState: InitialThemeState = {
    theme: initialTheme,
    systemTheme: systemPreference,
    resolvedTheme:
      initialTheme === 'system'
        ? (initialResolved ?? systemPreference)
        : (initialTheme as ResolvedTheme),
  }

  const [theme, setThemeState] = useState<ThemeMode>(initialThemeState.theme)
  const [systemTheme, setSystemTheme] = useState<ResolvedTheme>(
    initialThemeState.systemTheme
  )
  const [resolvedTheme, setResolvedTheme] = useState<ResolvedTheme>(
    initialThemeState.resolvedTheme
  )
  const [hydrated, setHydrated] = useState(false)

  const applyTheme = useCallback(
    (mode: ThemeMode, nextResolved: ResolvedTheme) => {
      if (typeof document === 'undefined') return
      const root = document.documentElement
      root.dataset.theme = nextResolved
      root.dataset.userTheme = mode
      root.classList.toggle('dark', nextResolved === 'dark')
      root.style.colorScheme = nextResolved
    },
    []
  )

  useEffect(() => {
    if (typeof document === 'undefined') {
      return
    }

    const domTheme = getDomUserTheme()
    const stored = readStoredTheme()
    const nextTheme = domTheme ?? stored
    const domResolved =
      document.documentElement.dataset.theme === 'dark' ? 'dark' : 'light'

    const nextResolved =
      nextTheme === 'system' ? domResolved : (nextTheme as ResolvedTheme)

    startTransition(() => {
      setThemeState(nextTheme)
      setResolvedTheme(nextResolved)
      setHydrated(true)
    })
    applyTheme(nextTheme, nextResolved)
  }, [applyTheme])

  useEffect(() => {
    if (typeof window === 'undefined') {
      return
    }

    const media = window.matchMedia('(prefers-color-scheme: dark)')
    const handleChange = (event: MediaQueryListEvent) => {
      setSystemTheme(event.matches ? 'dark' : 'light')
    }

    startTransition(() => {
      setSystemTheme(media.matches ? 'dark' : 'light')
    })
    media.addEventListener('change', handleChange)

    return () => media.removeEventListener('change', handleChange)
  }, [])

  useEffect(() => {
    if (typeof window === 'undefined') {
      return
    }

    const handleStorage = (event: StorageEvent) => {
      if (event.key !== STORAGE_KEY) {
        return
      }

      const next = event.newValue
      if (isThemeMode(next)) {
        setThemeState(next)
      } else if (next === null) {
        setThemeState('system')
      }
    }

    window.addEventListener('storage', handleStorage)
    return () => window.removeEventListener('storage', handleStorage)
  }, [])

  useEffect(() => {
    const nextResolved =
      theme === 'system' ? systemTheme : (theme as ResolvedTheme)
    startTransition(() => {
      setResolvedTheme(nextResolved)
    })
    applyTheme(theme, nextResolved)

    if (typeof window !== 'undefined') {
      try {
        const cookieValue =
          theme === 'system' ? `system:${nextResolved}` : theme
        const maxAge = 60 * 60 * 24 * 365 // 1 year
        document.cookie = `${COOKIE_KEY}=${cookieValue}; path=/; max-age=${maxAge}; samesite=lax`
      } catch {
        // ignore cookie write errors
      }
    }
  }, [theme, systemTheme, applyTheme])

  const setTheme = useCallback((mode: ThemeMode) => {
    setThemeState(mode)
    if (typeof window === 'undefined') {
      return
    }
    try {
      if (mode === 'system') {
        window.localStorage.removeItem(STORAGE_KEY)
      } else {
        window.localStorage.setItem(STORAGE_KEY, mode)
      }
    } catch {
      // ignore storage write errors
    }
  }, [])

  const contextValue = useMemo<ThemeContextValue>(
    () => ({
      theme,
      resolvedTheme,
      setTheme,
      hydrated,
    }),
    [theme, resolvedTheme, setTheme, hydrated]
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
