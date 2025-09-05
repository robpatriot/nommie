// @ts-check
import eslint from '@eslint/js';
import tseslint from 'typescript-eslint';
import reactPlugin from 'eslint-plugin-react';
import reactHooks from 'eslint-plugin-react-hooks';
import nextPlugin from '@next/eslint-plugin-next';
import globals from 'globals';
import eslintConfigPrettier from 'eslint-config-prettier/flat'; // 👈 disables ESLint rules that conflict with Prettier

/**
 * Nommie ESLint 9 flat config (Next.js + React + TypeScript)
 * - No compat layers, no .eslintrc*, no .eslintignore.
 * - Restricts linting to TS/TSX files only.
 * - `eslint-config-prettier` last to prevent ESLint/Prettier flip-flop.
 */
export default [
  // 0) Global ignores
  {
    ignores: [
      '**/node_modules/**',
      '**/.next/**',
      '**/.turbo/**',
      '**/dist/**',
      '**/build/**',
      '**/coverage/**',
      'apps/backend/**', // Rust backend not linted by ESLint
    ],
  },

  // 1) Base JS recommended (TS/TSX only)
  {
    files: ['**/*.ts', '**/*.tsx'],
    ...eslint.configs.recommended,
  },

  // 2) React (flat) + JSX runtime (TSX only)
  {
    files: ['**/*.tsx'],
    ...reactPlugin.configs.flat.recommended,
    ...reactPlugin.configs.flat['jsx-runtime'],
  },

  // 3) TypeScript (untyped) recommended (fast, TS/TSX only)
  ...tseslint.configs.recommended.map(cfg => ({
    ...cfg,
    files: ['**/*.ts', '**/*.tsx'],
  })),

  // 4) Next.js + React Hooks (frontend + packages, TS/TSX only)
  {
    files: ['apps/frontend/**/*.{ts,tsx}', 'packages/**/*.{ts,tsx}'],
    plugins: {
      '@next/next': nextPlugin,
      'react-hooks': reactHooks,
    },
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
      ecmaVersion: 'latest',
      sourceType: 'module',
    },
    rules: {
      ...nextPlugin.configs.recommended.rules,
      ...(nextPlugin.configs['core-web-vitals']?.rules ?? {}),
      ...reactHooks.configs.recommended.rules,
    },
  },

  // 5) (Optional) Typed rules — enable after base passes cleanly
  // {
  //   files: ['**/*.ts', '**/*.tsx'],
  //   ...tseslint.configs.recommendedTypeChecked,
  //   ...tseslint.configs.stylisticTypeChecked,
  //   languageOptions: {
  //     parserOptions: {
  //       project: true,
  //       tsconfigRootDir: import.meta.dirname,
  //     },
  //   },
  // },

  // 6) Prettier — must be last
  eslintConfigPrettier,
];

