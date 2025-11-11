'use client'

import { useMemo } from 'react'
import { useTheme, type ThemeMode, type ResolvedTheme } from './theme-provider'

const LABELS: Record<'light' | 'dark' | 'system', string> = {
  light: 'Light',
  dark: 'Dark',
  system: 'System',
}

const ICONS: Record<'light' | 'dark' | 'system', string> = {
  light: '🌞',
  dark: '🌙',
  system: '🖥️',
}

const ORDER: Array<'system' | 'light' | 'dark'> = ['system', 'light', 'dark']

export function ThemeToggle({ className = '' }: { className?: string }) {
  const { theme, resolvedTheme, setTheme, hydrated } = useTheme()

  const nextTheme = useMemo(() => {
    const currentIndex = ORDER.indexOf(theme)
    return ORDER[(currentIndex + 1) % ORDER.length]
  }, [theme])

  const label = useMemo(() => {
    const root =
      typeof document !== 'undefined' ? document.documentElement : undefined

    const domTheme = root?.dataset.userTheme as ThemeMode | undefined
    const domResolved =
      root?.dataset.theme === 'dark' ? ('dark' as ResolvedTheme) : 'light'

    const effectiveTheme = hydrated
      ? theme
      : domTheme &&
          (domTheme === 'light' || domTheme === 'dark' || domTheme === 'system')
        ? domTheme
        : theme

    const effectiveResolved =
      hydrated || !domTheme || domTheme === 'system'
        ? resolvedTheme
        : domResolved

    if (effectiveTheme === 'system') {
      const resolvedLabel = LABELS[effectiveResolved] ?? LABELS.light
      return `Theme: ${LABELS.system} (${resolvedLabel})`
    }

    return `Theme: ${LABELS[effectiveTheme]}`
  }, [hydrated, theme, resolvedTheme])

  return (
    <button
      type="button"
      onClick={() => setTheme(nextTheme)}
      className={`inline-flex items-center gap-2 rounded-md border border-border bg-surface px-3 py-1.5 text-sm font-medium text-muted transition hover:border-primary/50 hover:bg-surface-strong hover:text-foreground ${className}`}
      title="Toggle theme (system → light → dark)"
    >
      <span role="img" aria-hidden suppressHydrationWarning>
        {ICONS[theme === 'system' ? resolvedTheme : theme]}
      </span>
      <span suppressHydrationWarning>{label}</span>
    </button>
  )
}
