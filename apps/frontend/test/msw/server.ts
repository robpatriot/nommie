import { setupServer } from 'msw/node'

// Create MSW server instance with no handlers (empty setup)
export const server = setupServer()

// Default export for convenience
export default server
