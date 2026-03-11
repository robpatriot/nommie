import { describe, expect, it, vi, beforeEach } from 'vitest'
import { redirect } from 'next/navigation'
import { auth } from '@/auth'
import { getMe } from '@/lib/api/user-me'

// Mock modules before importing the layout
vi.mock('next/navigation', async (importOriginal) => {
  const actual = (await importOriginal()) as Record<string, unknown>
  return {
    ...actual,
    redirect: vi.fn(() => {
      throw new Error('NEXT_REDIRECT')
    }),
  }
})

vi.mock('@/auth', () => ({
  auth: vi.fn(),
}))

vi.mock('@/lib/api/user-me', () => ({
  getMe: vi.fn(),
}))

describe('AdminLayout', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('redirects when session is null', async () => {
    vi.mocked(auth).mockResolvedValue(null as never)

    const AdminLayout = (await import('@/app/admin/layout')).default

    await expect(
      AdminLayout({ children: <div data-testid="child">x</div> })
    ).rejects.toThrow('NEXT_REDIRECT')

    expect(redirect).toHaveBeenCalledWith('/')
  })

  it('redirects when me is null', async () => {
    vi.mocked(auth).mockResolvedValue({ user: { email: 'a@b.com' } } as never)
    vi.mocked(getMe).mockResolvedValue(null)

    const AdminLayout = (await import('@/app/admin/layout')).default

    await expect(
      AdminLayout({ children: <div data-testid="child">x</div> })
    ).rejects.toThrow('NEXT_REDIRECT')

    expect(redirect).toHaveBeenCalledWith('/')
  })

  it('redirects when me.role is not admin', async () => {
    vi.mocked(auth).mockResolvedValue({ user: { email: 'a@b.com' } } as never)
    vi.mocked(getMe).mockResolvedValue({ id: 1, role: 'user' } as never)

    const AdminLayout = (await import('@/app/admin/layout')).default

    await expect(
      AdminLayout({ children: <div data-testid="child">x</div> })
    ).rejects.toThrow('NEXT_REDIRECT')

    expect(redirect).toHaveBeenCalledWith('/')
  })

  it('renders children when session and admin role are present', async () => {
    vi.mocked(auth).mockResolvedValue({ user: { email: 'a@b.com' } } as never)
    vi.mocked(getMe).mockResolvedValue({ id: 1, role: 'admin' } as never)

    const AdminLayout = (await import('@/app/admin/layout')).default

    const result = await AdminLayout({
      children: <div data-testid="child">Admin content</div>,
    })

    expect(result).toBeDefined()
    expect(redirect).not.toHaveBeenCalled()
  })
})
