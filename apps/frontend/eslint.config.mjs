// apps/frontend/eslint.config.mjs
// Flat ESLint config for the Next.js frontend (minimal + official Next presets)

import { FlatCompat } from '@eslint/eslintrc'
import eslintConfigPrettier from 'eslint-config-prettier/flat'

const compat = new FlatCompat({
  baseDirectory: import.meta.dirname,
})

export default [
  // Ignore typical build/output dirs
  {
    ignores: [
      '**/node_modules/**',
      '**/.next/**',
      '**/.turbo/**',
      '**/dist/**',
      '**/build/**',
      '**/coverage/**',
    ],
  },

  // ✅ This is what Next.js looks for — enables Next's rules (incl. core-web-vitals) + TypeScript support
  ...compat.config({
    extends: ['next/core-web-vitals', 'next/typescript'],
    settings: {
      // Helpful in monorepos if the Next app isn't at repo root
      next: { rootDir: '.' },
    },
  }),

  // Local, focused overrides
  {
    files: ['**/*.d.ts'],
    rules: {
      '@typescript-eslint/no-unused-vars': 'off',
    },
  },
  {
    files: ['next-env.d.ts'],
    rules: {
      '@typescript-eslint/triple-slash-reference': 'off',
    },
  },
  {
    files: ['test/**/*.{ts,tsx}'],
    rules: {
      '@typescript-eslint/no-explicit-any': 'off',
      '@typescript-eslint/no-unused-vars': [
        'warn',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],
    },
  },

  // Keep Prettier last to disable conflicting rules
  eslintConfigPrettier,
]
