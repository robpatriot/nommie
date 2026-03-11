import { describe, expect, it, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '../../../utils'
import userEvent from '@testing-library/user-event'
import AdminUsersClient from '@/app/admin/users/AdminUsersClient'
import {
  searchAdminUsersAction,
  revokeAdminAction,
} from '@/app/actions/admin-user-actions'

const mockShowToast = vi.fn()
const mockHideToast = vi.fn()

vi.mock('@/hooks/useToast', () => ({
  useToast: () => ({
    toasts: [],
    showToast: mockShowToast,
    hideToast: mockHideToast,
  }),
}))

vi.mock('@/app/actions/admin-user-actions', () => ({
  searchAdminUsersAction: vi.fn(),
  grantAdminAction: vi.fn(),
  revokeAdminAction: vi.fn(),
}))

vi.mock('next/link', () => ({
  __esModule: true,
  default: ({
    children,
    ...props
  }: {
    children: React.ReactNode
    href: string
  }) => <a {...props}>{children}</a>,
}))

describe('AdminUsersClient', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockShowToast.mockClear()
    vi.mocked(searchAdminUsersAction).mockResolvedValue({
      kind: 'ok',
      data: { items: [], next_cursor: null },
    })
  })

  it('renders search UI and placeholder', () => {
    render(<AdminUsersClient currentUserId={null} />)

    expect(
      screen.getByRole('searchbox', { name: /search by email or username/i })
    ).toBeInTheDocument()
    expect(
      screen.getByPlaceholderText(/search by email or username/i)
    ).toBeInTheDocument()
  })

  it('renders search results when data is returned', async () => {
    const user = userEvent.setup()
    vi.mocked(searchAdminUsersAction).mockResolvedValue({
      kind: 'ok',
      data: {
        items: [
          {
            id: 1,
            display_name: 'Alice',
            email: 'alice@example.com',
            role: 'user',
          },
        ],
        next_cursor: null,
      },
    })

    render(<AdminUsersClient currentUserId={null} />)

    const input = screen.getByRole('searchbox', {
      name: /search by email or username/i,
    })
    await user.type(input, 'alice')

    await waitFor(
      () => {
        expect(searchAdminUsersAction).toHaveBeenCalledWith(
          expect.objectContaining({ q: 'alice' })
        )
      },
      { timeout: 2000 }
    )

    await waitFor(
      () => {
        expect(screen.getByText('Alice')).toBeInTheDocument()
        expect(screen.getByText('alice@example.com')).toBeInTheDocument()
      },
      { timeout: 2000 }
    )
  })

  it('disables revoke button for current user row', async () => {
    const user = userEvent.setup()
    vi.mocked(searchAdminUsersAction).mockResolvedValue({
      kind: 'ok',
      data: {
        items: [
          {
            id: 42,
            display_name: 'Me',
            email: 'me@example.com',
            role: 'admin',
          },
        ],
        next_cursor: null,
      },
    })

    render(<AdminUsersClient currentUserId={42} />)

    const input = screen.getByRole('searchbox', {
      name: /search by email or username/i,
    })
    await user.type(input, 'me')

    await waitFor(
      () => {
        expect(screen.getByText('Me')).toBeInTheDocument()
      },
      { timeout: 2000 }
    )

    const revokeButton = screen.getByRole('button', {
      name: /revoke admin/i,
    })
    expect(revokeButton).toBeDisabled()
  })

  it('shows LAST_ADMIN_PROTECTION message on revoke error', async () => {
    const user = userEvent.setup()
    vi.mocked(searchAdminUsersAction).mockResolvedValue({
      kind: 'ok',
      data: {
        items: [
          {
            id: 2,
            display_name: 'Other',
            email: 'other@example.com',
            role: 'admin',
          },
        ],
        next_cursor: null,
      },
    })
    vi.mocked(revokeAdminAction).mockResolvedValue({
      kind: 'error',
      message: 'Cannot revoke',
      status: 409,
      code: 'LAST_ADMIN_PROTECTION',
    })

    render(<AdminUsersClient currentUserId={1} />)

    const input = screen.getByRole('searchbox', {
      name: /search by email or username/i,
    })
    await user.type(input, 'other')

    await waitFor(
      () => {
        expect(screen.getByText('Other')).toBeInTheDocument()
      },
      { timeout: 2000 }
    )

    const revokeButton = screen.getByRole('button', { name: /revoke admin/i })
    vi.stubGlobal('confirm', () => true)
    await user.click(revokeButton)

    await waitFor(
      () => {
        expect(mockShowToast).toHaveBeenCalledWith(
          'Cannot revoke the last admin.',
          'error'
        )
      },
      { timeout: 2000 }
    )
  })

  it('shows CANNOT_REVOKE_OWN_ADMIN message on revoke error', async () => {
    const user = userEvent.setup()
    vi.mocked(searchAdminUsersAction).mockResolvedValue({
      kind: 'ok',
      data: {
        items: [
          {
            id: 99,
            display_name: 'Target',
            email: 't@example.com',
            role: 'admin',
          },
        ],
        next_cursor: null,
      },
    })
    vi.mocked(revokeAdminAction).mockResolvedValue({
      kind: 'error',
      message: 'Cannot revoke',
      status: 409,
      code: 'CANNOT_REVOKE_OWN_ADMIN',
    })

    render(<AdminUsersClient currentUserId={1} />)

    const input = screen.getByRole('searchbox', {
      name: /search by email or username/i,
    })
    await user.type(input, 'target')

    await waitFor(
      () => {
        expect(screen.getByText('Target')).toBeInTheDocument()
      },
      { timeout: 2000 }
    )

    const revokeBtn = screen.getByRole('button', { name: /revoke admin/i })
    vi.stubGlobal('confirm', () => true)
    await user.click(revokeBtn)

    await waitFor(
      () => {
        expect(mockShowToast).toHaveBeenCalledWith(
          'You cannot revoke your own admin role.',
          'error'
        )
      },
      { timeout: 2000 }
    )
  })
})
