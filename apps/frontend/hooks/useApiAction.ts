'use client'

import { useCallback } from 'react'
import { BackendApiError } from '@/lib/errors'
import type { ToastMessage } from '@/components/Toast'

type ActionResult<T = void> =
  | { kind: 'ok'; data?: T }
  | {
      kind: 'error'
      message: string
      status: number
      code?: string
      traceId?: string
    }

type UseApiActionOptions = {
  showToast: (
    message: string,
    type: ToastMessage['type'],
    error?: BackendApiError
  ) => void
  onSuccess?: () => void | Promise<void>
  successMessage?: string
  errorMessage?: string
}

/**
 * Hook to handle API actions with consistent error handling, toast notifications, and traceId logging.
 *
 * @param options - Configuration options
 * @returns A function that wraps an async action with error handling
 */
export function useApiAction(options: UseApiActionOptions) {
  const { showToast, onSuccess, successMessage, errorMessage } = options

  return useCallback(
    async <T = void>(
      action: () => Promise<ActionResult<T>>,
      options?: {
        successMessage?: string
        errorMessage?: string
        onSuccess?: () => void | Promise<void>
      }
    ): Promise<T | null> => {
      try {
        const result = await action()

        if (result.kind === 'error') {
          const actionError = new BackendApiError(
            result.message || errorMessage || 'Action failed',
            result.status,
            result.code,
            result.traceId
          )

          // Prefer the actual error message from the backend, fallback to provided error message
          const finalErrorMessage =
            actionError.message ||
            options?.errorMessage ||
            errorMessage ||
            'Action failed'
          showToast(finalErrorMessage, 'error', actionError)

          if (process.env.NODE_ENV === 'development' && actionError.traceId) {
            console.error('API action error traceId:', actionError.traceId)
          }

          return null
        }

        const finalSuccessMessage = options?.successMessage || successMessage
        if (finalSuccessMessage) {
          showToast(finalSuccessMessage, 'success')
        }

        const finalOnSuccess = options?.onSuccess || onSuccess
        if (finalOnSuccess) {
          await finalOnSuccess()
        }

        return (result.data ?? null) as T | null
      } catch (err) {
        const message =
          err instanceof Error
            ? err.message
            : errorMessage || 'Unable to complete action'
        const wrappedError =
          err instanceof BackendApiError
            ? err
            : new BackendApiError(message, 500, 'UNKNOWN_ERROR')

        showToast(wrappedError.message, 'error', wrappedError)

        if (process.env.NODE_ENV === 'development' && wrappedError.traceId) {
          console.error('API action error traceId:', wrappedError.traceId)
        }

        return null
      }
    },
    [showToast, onSuccess, successMessage, errorMessage]
  )
}
