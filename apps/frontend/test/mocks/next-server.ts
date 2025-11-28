import { vi } from 'vitest'

// Mock next/server to prevent errors when next-auth tries to import it
vi.mock('next/server', () => ({
  NextResponse: class NextResponse {
    static json = vi.fn((body, init) => ({
      json: () => Promise.resolve(body),
      status: init?.status ?? 200,
      headers: new Headers(init?.headers),
    }))
    static redirect = vi.fn((url) => ({
      status: 307,
      headers: new Headers({ Location: String(url) }),
    }))
  },
  headers: vi.fn(() => new Headers()),
  cookies: vi.fn(() => ({
    get: vi.fn(),
    set: vi.fn(),
    delete: vi.fn(),
    has: vi.fn(),
    getAll: vi.fn(() => []),
  })),
}))
