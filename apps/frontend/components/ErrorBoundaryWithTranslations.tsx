'use client'

import { useTranslations } from 'next-intl'
import ErrorBoundary from './ErrorBoundary'
import type { ReactNode } from 'react'

interface ErrorBoundaryWithTranslationsProps {
  children: ReactNode
  fallback?: (error: Error, reset: () => void) => ReactNode
  onError?: (error: Error, errorInfo: { componentStack: string }) => void
}

export default function ErrorBoundaryWithTranslations({
  children,
  fallback,
  onError,
}: ErrorBoundaryWithTranslationsProps) {
  const t = useTranslations('errors.boundary')

  return (
    <ErrorBoundary
      fallback={fallback}
      onError={onError}
      translations={{
        title: t('title'),
        fallbackMessage: t('fallbackMessage'),
        tryAgain: t('tryAgain'),
        reloadPage: t('reloadPage'),
        devDetails: t('devDetails'),
      }}
    >
      {children}
    </ErrorBoundary>
  )
}
