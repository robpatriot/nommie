import React from 'react'
import { describe, it, expect, vi } from 'vitest'
import { render, screen, userEvent } from '../test/utils'
import AuthControl from './AuthControl'

describe('AuthControl', () => {
  it('renders sign in button when unauthenticated', () => {
    const mockOnLogin = vi.fn()
    
    render(<AuthControl state="unauthenticated" onLogin={mockOnLogin} />)
    
    const signInButton = screen.getByRole('button', { name: /sign in/i })
    expect(signInButton).toBeInTheDocument()
    expect(signInButton).not.toBeDisabled()
    
    // Verify the button has proper accessibility attributes
    expect(signInButton).toHaveAttribute('aria-label', 'Sign in')
  })

  it('renders sign out button when authenticated', () => {
    const mockOnLogout = vi.fn()
    
    render(<AuthControl state="authenticated" onLogout={mockOnLogout} />)
    
    const signOutButton = screen.getByRole('button', { name: /sign out/i })
    expect(signOutButton).toBeInTheDocument()
    expect(signOutButton).not.toBeDisabled()
    
    // Verify the button has proper accessibility attributes
    expect(signOutButton).toHaveAttribute('aria-label', 'Sign out')
  })

  it('renders loading button when loading', () => {
    render(<AuthControl state="loading" />)
    
    const loadingButton = screen.getByRole('button', { name: /loading authentication/i })
    expect(loadingButton).toBeInTheDocument()
    expect(loadingButton).toBeDisabled()
    expect(loadingButton).toHaveTextContent('Loading...')
    
    // Verify the button has proper accessibility attributes
    expect(loadingButton).toHaveAttribute('aria-label', 'Loading authentication')
  })

  it('calls onLogin when sign in button is clicked', async () => {
    const user = userEvent.setup()
    const mockOnLogin = vi.fn()
    
    render(<AuthControl state="unauthenticated" onLogin={mockOnLogin} />)
    
    const signInButton = screen.getByRole('button', { name: /sign in/i })
    await user.click(signInButton)
    
    expect(mockOnLogin).toHaveBeenCalledTimes(1)
  })

  it('calls onLogout when sign out button is clicked', async () => {
    const user = userEvent.setup()
    const mockOnLogout = vi.fn()
    
    render(<AuthControl state="authenticated" onLogout={mockOnLogout} />)
    
    const signOutButton = screen.getByRole('button', { name: /sign out/i })
    await user.click(signOutButton)
    
    expect(mockOnLogout).toHaveBeenCalledTimes(1)
  })

  it('does not call handlers when loading', async () => {
    const user = userEvent.setup()
    const mockOnLogin = vi.fn()
    const mockOnLogout = vi.fn()
    
    render(<AuthControl state="loading" onLogin={mockOnLogin} onLogout={mockOnLogout} />)
    
    const loadingButton = screen.getByRole('button', { name: /loading authentication/i })
    
    // Button should be disabled, so click should not trigger handlers
    expect(loadingButton).toBeDisabled()
    
    // Even if we try to click, it shouldn't call handlers (though userEvent respects disabled state)
    await user.click(loadingButton)
    
    expect(mockOnLogin).not.toHaveBeenCalled()
    expect(mockOnLogout).not.toHaveBeenCalled()
  })

  it('has proper accessibility structure with status role', () => {
    const mockOnLogin = vi.fn()
    
    render(<AuthControl state="unauthenticated" onLogin={mockOnLogin} />)
    
    const statusElement = screen.getByRole('status', { name: /authentication status/i })
    expect(statusElement).toBeInTheDocument()
    
    const button = screen.getByRole('button')
    expect(button).toBeInTheDocument()
    expect(statusElement).toContainElement(button)
  })
})
