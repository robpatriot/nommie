# Frontend Theme System

## Document Scope

This note documents how the shared Tailwind theme tokens are defined,
consumed, and extended. UI layout sequencing and broader UX priorities are
tracked separately in `frontend-ui-roadmap.md`.

This document describes how the centralized Tailwind theme is wired up in the
Next.js frontend and how to extend it responsibly.

## Overview

- **Source of truth**: semantic CSS variables live in
  `apps/frontend/app/globals.css`. They define tokens such as
  `--color-bg`, `--color-surface`, and `--color-primary`, with both light and
  dark values (plus `.theme-light` / `.theme-dark` overrides).
- **Tailwind mapping**: `apps/frontend/tailwind.config.js` maps those variables
  to Tailwind color names (for example `bg-background`, `text-muted`,
  `border-border`, `bg-primary`, `bg-success`, `bg-warning`, etc.). This keeps
  utility usage consistent and makes palettes easy to swap.
- **Runtime theme control**: `ThemeProvider` (`apps/frontend/components/theme-provider.tsx`)
  manages the user’s preference. It:
  - reads `localStorage` for a stored choice (`light`, `dark`, or `system`);
  - listens to `prefers-color-scheme`;
  - applies `data-theme` and `dark` class to the root element;
  - exposes `useTheme()` so components can react to `theme` / `resolvedTheme`.
- **Initial render**: `app/layout.tsx` injects a small script before hydration to
  apply the correct theme immediately and wraps the app in `ThemeProvider`.
- **Toggle UI**: `ThemeToggle` (`apps/frontend/components/theme-toggle.tsx`) cycles
  between `system → light → dark` and persists the selection.

## Using Theme Tokens

When building UI components:

- Prefer semantic utilities such as `bg-background`, `bg-surface`,
  `bg-surface-strong`, `text-foreground`, `text-muted`, `border-border`,
  `ring`, `bg-primary`, `bg-accent`, `bg-success`, `bg-warning`, `bg-danger`.
- Avoid hard-coded Tailwind palette colors (`bg-slate-900`, `text-gray-500`,
  etc.) unless you are introducing a brand-new semantic token.
- For elevated surfaces use the provided `shadow-elevated`.
- If you need additional states (e.g., `info`, `neutral`), add matching CSS
  variables in `globals.css` and register them in `tailwind.config.js`.

## Extending the Palette

1. Define light/dark values inside `:root` and `.theme-dark` blocks in
   `globals.css`. Stick to RGB triplets so they work with Tailwind alpha
   suffixes (`/10`, `/90`, etc.).
2. Update `tailwind.config.js` to expose the new token under `theme.extend.colors`
   (and, if necessary, the related `backgroundColor`, `textColor`, `borderColor`,
   `ringColor`, or `boxShadow` entries).
3. Reference the new semantic utility in components (`bg-info`, `text-info`,
   `border-info`, etc.).
4. Document the addition here to keep the palette discoverable.

## Programmatic Theme Access

```tsx
import { useTheme } from '@/components/theme-provider'

function Example() {
  const { theme, resolvedTheme, setTheme } = useTheme()
  // theme: 'light' | 'dark' | 'system'
  // resolvedTheme: 'light' | 'dark'
  // setTheme('dark') forces dark mode until changed again
}
```

If you need custom controls, build them on top of `useTheme()` rather than
re-implementing preference storage or DOM updates.

## Testing & QA Checklist

- Verify light/dark modes in browsers that support `prefers-color-scheme`.
- Ensure `ThemeToggle` cycles through the three states and the selection
  persists across reloads.
- Confirm overriding the theme does not break when system preference changes.
- When adding new components, audit for stray palette colors (`gray-*`,
  `slate-*`, etc.) and swap them to semantic tokens.

