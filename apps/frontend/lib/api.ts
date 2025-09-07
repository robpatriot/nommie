import { auth } from '@/auth'

export class BackendApiError extends Error {
  constructor(
    message: string,
    public status: number,
    public code?: string,
    public traceId?: string
  ) {
    super(message)
    this.name = 'BackendApiError'
  }
}

export async function api<T = unknown>(
  input: RequestInfo | URL,
  init?: RequestInit
): Promise<T> {
  const res = await fetch(input, init)
  if (!res.ok) {
    throw new Error(`HTTP ${res.status} ${res.statusText}`)
  }
  const data: unknown = await res.json()
  return data as T
}

export async function fetchWithAuth(
  endpoint: string,
  options: RequestInit = {}
): Promise<Response> {
  const session = await auth()

  if (!session?.backendJwt) {
    throw new BackendApiError('No backend JWT available', 401, 'NO_JWT')
  }

  const baseUrl = process.env.NEXT_PUBLIC_BACKEND_BASE_URL
  if (!baseUrl) {
    throw new Error('NEXT_PUBLIC_BACKEND_BASE_URL not configured')
  }

  const url = `${baseUrl}${endpoint}`
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${session.backendJwt}`,
      ...options.headers,
    },
  })

  if (response.status === 401) {
    const traceId = response.headers.get('x-trace-id')
    throw new BackendApiError(
      'Unauthorized',
      401,
      'UNAUTHORIZED',
      traceId || undefined
    )
  }

  if (!response.ok) {
    const traceId = response.headers.get('x-trace-id')
    throw new BackendApiError(
      `Backend request failed: ${response.statusText}`,
      response.status,
      undefined,
      traceId || undefined
    )
  }

  return response
}

export async function getMe<T = unknown>(): Promise<T> {
  const response = await fetchWithAuth('/api/private/me')
  return response.json()
}
