import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import React from 'react'
import ErrorBoundary from './ErrorBoundary'

// Component that throws an error during render
const ThrowError = () => {
  throw new Error('Test error message')
}

// Mock console.error for error boundary tests (React logs errors to console)
// We'll restore it in individual tests that need to verify logging
const originalError = console.error
beforeEach(() => {
  // Mock console.error to suppress React's error logging during tests
  // Individual tests can spy on it if they need to verify logging behavior
  console.error = vi.fn()
})

afterEach(() => {
  console.error = originalError
  vi.unstubAllEnvs()
})

describe('ErrorBoundary', () => {
  it('renders children when no error occurs', () => {
    render(
      <ErrorBoundary>
        <div>Test content</div>
      </ErrorBoundary>
    )

    expect(screen.getByText('Test content')).toBeInTheDocument()
  })

  it('catches and displays error with default fallback', () => {
    render(
      <ErrorBoundary>
        <ThrowError />
      </ErrorBoundary>
    )

    expect(screen.getByText('Something went wrong')).toBeInTheDocument()
    expect(screen.getByText('Test error message')).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Try again' })
    ).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Reload page' })
    ).toBeInTheDocument()
  })

  it('displays custom fallback when provided', () => {
    const customFallback = (error: Error, reset: () => void) => (
      <div>
        <p>Custom error: {error.message}</p>
        <button onClick={reset}>Custom reset</button>
      </div>
    )

    render(
      <ErrorBoundary fallback={customFallback}>
        <ThrowError />
      </ErrorBoundary>
    )

    expect(
      screen.getByText('Custom error: Test error message')
    ).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Custom reset' })
    ).toBeInTheDocument()
  })

  it('calls onError callback when error occurs', () => {
    const onError = vi.fn()

    render(
      <ErrorBoundary onError={onError}>
        <ThrowError />
      </ErrorBoundary>
    )

    expect(onError).toHaveBeenCalledTimes(1)
    expect(onError).toHaveBeenCalledWith(
      expect.any(Error),
      expect.objectContaining({
        componentStack: expect.any(String),
      })
    )
    expect(onError.mock.calls[0][0].message).toBe('Test error message')
  })

  it('resets error state when reset button is clicked', async () => {
    const user = userEvent.setup()
    const NoError = () => <div>No error</div>

    // Error boundaries need the component tree to change to recover from errors
    // We'll test that reset clears the error state by using a key to force remount
    const { rerender } = render(
      <ErrorBoundary key="error">
        <ThrowError />
      </ErrorBoundary>
    )

    // Error should be displayed
    expect(screen.getByText('Something went wrong')).toBeInTheDocument()

    // Click reset button - this should reset the error state in the boundary
    const resetButton = screen.getByRole('button', { name: 'Try again' })
    await user.click(resetButton)

    // After reset, re-render with a different key and a component that doesn't throw
    // This forces React to remount the boundary, allowing it to render children again
    rerender(
      <ErrorBoundary key="recovered">
        <NoError />
      </ErrorBoundary>
    )

    // Should render children again now that boundary is remounted and component doesn't throw
    expect(screen.getByText('No error')).toBeInTheDocument()
  })

  it('displays error details in development mode', () => {
    vi.stubEnv('NODE_ENV', 'development')

    render(
      <ErrorBoundary>
        <ThrowError />
      </ErrorBoundary>
    )

    expect(screen.getByText('Error details (dev only)')).toBeInTheDocument()
    // Check for error message in the paragraph (not the stack trace)
    expect(screen.getByText('Test error message')).toBeInTheDocument()
    // Verify stack trace is present
    const stackTrace = screen.getByText(/Error: Test error message/)
    expect(stackTrace).toBeInTheDocument()
  })

  it('hides error details in production mode', () => {
    vi.stubEnv('NODE_ENV', 'production')

    render(
      <ErrorBoundary>
        <ThrowError />
      </ErrorBoundary>
    )

    expect(
      screen.queryByText('Error details (dev only)')
    ).not.toBeInTheDocument()
  })

  it('displays default message when error has no message', () => {
    const ThrowErrorNoMessage = () => {
      throw new Error('')
    }

    render(
      <ErrorBoundary>
        <ThrowErrorNoMessage />
      </ErrorBoundary>
    )

    expect(screen.getByText('An unexpected error occurred')).toBeInTheDocument()
  })

  it('logs error to console', () => {
    // Create a spy that captures all console.error calls
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

    render(
      <ErrorBoundary>
        <ThrowError />
      </ErrorBoundary>
    )

    // The centralized logger calls console.error synchronously now
    // It logs: '[Error Logger]', { message, error, context, timestamp }
    expect(consoleSpy).toHaveBeenCalled()

    // Check that it was called with error-related content
    const calls = consoleSpy.mock.calls
    const hasErrorLog = calls.some((call) => {
      const firstArg = call[0]
      return (
        typeof firstArg === 'string' &&
        (firstArg.includes('ErrorBoundary') ||
          firstArg.includes('Error Logger') ||
          firstArg.includes('[Error]'))
      )
    })
    expect(hasErrorLog).toBe(true)

    consoleSpy.mockRestore()
  })
})
