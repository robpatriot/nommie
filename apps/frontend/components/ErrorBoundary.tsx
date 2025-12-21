'use client'

import { Component, type ReactNode } from 'react'
import { logError } from '@/lib/logging/error-logger'
import { BackendApiError } from '@/lib/errors'

interface ErrorBoundaryProps {
  children: ReactNode
  fallback?: (error: Error, reset: () => void) => ReactNode
  onError?: (error: Error, errorInfo: { componentStack: string }) => void
  translations?: {
    title: string
    fallbackMessage: string
    tryAgain: string
    reloadPage: string
    devDetails: string
    details?: {
      show: string
      hide: string
      details: string
      statusLabel: string
      codeLabel: string
      traceIdLabel: string
    }
  }
}

interface ErrorBoundaryState {
  hasError: boolean
  error: Error | null
  expanded: boolean
}

/**
 * Error Boundary component to catch and handle React errors in component trees.
 * Prevents the entire app from crashing when an error occurs.
 */
class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props)
    this.state = {
      hasError: false,
      error: null,
      expanded: false,
    }
  }

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return {
      hasError: true,
      error,
      expanded: false,
    }
  }

  componentDidCatch(error: Error, errorInfo: { componentStack: string }) {
    // Log error for debugging
    logError('ErrorBoundary caught an error', error, {
      componentStack: errorInfo.componentStack,
    })

    // Call optional error handler
    this.props.onError?.(error, errorInfo)
  }

  handleReset = () => {
    this.setState({
      hasError: false,
      error: null,
      expanded: false,
    })
  }

  handleToggleExpanded = () => {
    this.setState((prevState) => ({
      expanded: !prevState.expanded,
    }))
  }

  render() {
    if (this.state.hasError && this.state.error) {
      if (this.props.fallback) {
        return this.props.fallback(this.state.error, this.handleReset)
      }

      const t = this.props.translations ?? {
        title: 'Something went wrong',
        fallbackMessage: 'An unexpected error occurred',
        tryAgain: 'Try again',
        reloadPage: 'Reload page',
        devDetails: 'Error details (dev only)',
        details: {
          show: 'Show',
          hide: 'Hide',
          details: 'details',
          statusLabel: 'Status',
          codeLabel: 'Code',
          traceIdLabel: 'Trace ID',
        },
      }

      const error = this.state.error
      const isBackendError = error instanceof BackendApiError
      const hasTraceId = isBackendError && error.traceId

      // Default fallback UI
      return (
        <div className="flex min-h-screen items-center justify-center bg-background p-4">
          <div className="max-w-md rounded-lg border border-danger/40 bg-danger/10 p-6 text-center">
            <h2 className="mb-2 text-lg font-semibold text-danger-foreground">
              {t.title}
            </h2>
            <p className="mb-4 text-sm text-muted-foreground">
              {error.message || t.fallbackMessage}
            </p>
            {isBackendError && (
              <div className="mb-4">
                {this.state.expanded && (
                  <div className="space-y-1 text-xs text-danger/80 text-left">
                    <p>
                      <span className="font-semibold">
                        {t.details?.statusLabel}:
                      </span>{' '}
                      {error.status}
                    </p>
                    {error.code && (
                      <p>
                        <span className="font-semibold">
                          {t.details?.codeLabel}:
                        </span>{' '}
                        {error.code}
                      </p>
                    )}
                    {hasTraceId && (
                      <p className="font-mono text-xs break-all">
                        <span className="font-semibold">
                          {t.details?.traceIdLabel}:
                        </span>{' '}
                        {error.traceId}
                      </p>
                    )}
                  </div>
                )}
                {hasTraceId && (
                  <button
                    onClick={this.handleToggleExpanded}
                    className="mt-1 text-xs text-danger/90 hover:text-danger underline"
                  >
                    {this.state.expanded
                      ? `${t.details?.hide} ${t.details?.details}`
                      : `${t.details?.show} ${t.details?.details}`}
                  </button>
                )}
              </div>
            )}
            <div className="flex gap-2 justify-center">
              <button
                onClick={this.handleReset}
                className="rounded bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
              >
                {t.tryAgain}
              </button>
              <button
                onClick={() => window.location.reload()}
                className="rounded bg-surface px-4 py-2 text-sm font-medium text-foreground transition-colors hover:bg-surface-strong"
              >
                {t.reloadPage}
              </button>
            </div>
            {process.env.NODE_ENV === 'development' && (
              <details className="mt-4 text-left">
                <summary className="cursor-pointer text-xs text-muted-foreground">
                  {t.devDetails}
                </summary>
                <pre className="mt-2 overflow-auto rounded bg-surface p-2 text-xs">
                  {error.stack}
                </pre>
              </details>
            )}
          </div>
        </div>
      )
    }

    return this.props.children
  }
}

export default ErrorBoundary
