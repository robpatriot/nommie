import { render as rtlRender, RenderOptions } from '@testing-library/react'
import { ReactElement } from 'react'

// AllProviders wrapper - currently just renders children, ready for providers later
const AllProviders = ({ children }: { children: React.ReactNode }) => {
  return <>{children}</>
}

// Custom render function that wraps RTL's render
const render = (
  ui: ReactElement,
  options?: Omit<RenderOptions, 'wrapper'>
) => rtlRender(ui, { wrapper: AllProviders, ...options })

// Re-export commonly used testing utilities
export * from '@testing-library/react'
export { render, screen } from '@testing-library/react'
export { default as userEvent } from '@testing-library/user-event'
