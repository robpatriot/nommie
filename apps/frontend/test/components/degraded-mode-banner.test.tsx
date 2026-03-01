import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import DegradedModeBanner from '@/components/DegradedModeBanner'
import { useBackendReadiness } from '@/lib/providers/backend-readiness-provider'

vi.mock('@/lib/providers/backend-readiness-provider', () => ({
  useBackendReadiness: vi.fn(),
}))

describe('DegradedModeBanner', () => {
  it('renders children when isReady is true', () => {
    vi.mocked(useBackendReadiness).mockReturnValue({
      isReady: true,
      reportFailure: vi.fn(),
      reportSuccess: vi.fn(),
      triggerRecovery: vi.fn(),
    })

    render(
      <DegradedModeBanner>
        <div data-testid="child">Target Content</div>
      </DegradedModeBanner>
    )

    expect(screen.getByTestId('child')).toBeDefined()
    expect(screen.queryByText(/getting things ready/i)).toBeNull()
  })

  it('renders the banner when isReady is false', () => {
    vi.mocked(useBackendReadiness).mockReturnValue({
      isReady: false,
      reportFailure: vi.fn(),
      reportSuccess: vi.fn(),
      triggerRecovery: vi.fn(),
    })

    render(
      <DegradedModeBanner>
        <div data-testid="child">Target Content</div>
      </DegradedModeBanner>
    )

    expect(screen.queryByTestId('child')).toBeNull()
    expect(screen.getByText(/getting things ready/i)).toBeDefined()
    expect(screen.getByText(/starting up/i)).toBeDefined()
  })
})
