# Frontend Theme System

## Document Scope

This note documents how the shared Tailwind theme tokens are defined,
consumed, and extended. UI layout sequencing and broader UX priorities are
tracked separately in `../dev-roadmap.md`.

This document describes how the centralized Tailwind theme is wired up in the
Next.js frontend and how to extend it responsibly.

## Overview

- **Source of truth**: semantic CSS variables live in
  `apps/frontend/app/globals.css`. Tailwind v4 uses an `@theme` block for default
  values (e.g. `--color-background`, `--color-card`, `--color-primary`). Dark mode
  overrides live in `.dark`; named themes (standard, high_roller, oldtime) use
  `[data-theme-name='...']` selectors.
- **Tailwind mapping**: Tailwind v4 generates utilities from `@theme` variables
  automatically (e.g. `bg-background`, `text-muted`, `border-border`, `bg-primary`,
  `bg-success`, `bg-warning`, `bg-destructive`). No explicit config mapping is
  needed for colours; `tailwind.config.js` extends only fonts.
- **Runtime control**: `ThemeProvider` (`apps/frontend/components/theme-provider.tsx`)
  manages theme name and colour scheme. It:
  - reads `localStorage` for stored choices (`theme_name`, `colour_scheme`);
  - listens to `prefers-color-scheme` when scheme is `system`;
  - applies `data-theme-name`, `data-colour-scheme`, and `dark` class on the root;
  - exposes `useTheme()` so components can read `themeName`, `colourScheme`,
    `resolvedColourScheme` and call `applyPreferences()`.
- **Initial render**: `app/layout.tsx` injects a boot script before hydration to
  apply the correct theme immediately and wraps the app in `ThemeProvider`.
- **User controls**: `ColourSchemeSelector` (light/dark/system) and `ThemeSelector`
  (standard/high_roller/oldtime) both call `applyPreferences()` with
  `persistBackend: true`, syncing to the backend and localStorage.

## Using Theme Tokens

When building UI components:

- Prefer semantic utilities such as `bg-background`, `bg-card`, `bg-muted`,
  `text-foreground`, `text-muted`, `border-border`, `ring`, `bg-primary`,
  `bg-accent`, `bg-success`, `bg-warning`, `bg-destructive`.
- Avoid hard-coded Tailwind palette colors (`bg-slate-900`, `text-gray-500`,
  etc.) unless you are introducing a brand-new semantic token.
- For elevated surfaces use the provided `shadow-elevated`.
- If you need additional states (e.g., `info`, `neutral`), add matching CSS
  variables in the `@theme` block or theme overrides in `globals.css`.

## Extending the Palette

1. Define the token in the `@theme` block in `globals.css` for light/default
   values. Add dark overrides in `.dark` (and `[data-theme-name='...'].dark` for
   named themes). Use hex values; Tailwind v4 supports alpha suffixes (`/10`, `/90`).
2. Tailwind v4 auto-generates utilities from `@theme` variables; no config changes
   are needed for standard colour tokens.
3. Reference the new semantic utility in components (`bg-info`, `text-info`,
   `border-info`, etc.).
4. Document the addition here to keep the palette discoverable.

## Programmatic Theme Access

```tsx
import { useTheme } from '@/components/theme-provider'

function Example() {
  const { themeName, colourScheme, resolvedColourScheme, applyPreferences } = useTheme()
  // themeName: 'standard' | 'high_roller' | 'oldtime'
  // colourScheme: 'light' | 'dark' | 'system'
  // resolvedColourScheme: 'light' | 'dark'
  // applyPreferences({ colourScheme: 'dark' }, { persistBackend: true, persistStorage: true })
}
```

If you need custom controls, build them on top of `useTheme()` and
`applyPreferences()` rather than re-implementing preference storage or DOM updates.

## Testing & QA Checklist

- Verify light/dark modes in browsers that support `prefers-color-scheme`.
- Ensure `ColourSchemeSelector` and `ThemeSelector` update all options and
  selections persist across reloads.
- Confirm `system` mode tracks OS preference; explicit light/dark overrides
  remain stable when system preference changes.
- When adding new components, audit for stray palette colors (`gray-*`,
  `slate-*`, etc.) and swap them to semantic tokens.

