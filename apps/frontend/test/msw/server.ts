import { setupServer } from 'msw/node'
import { http, HttpResponse } from 'msw'

export const handlers = [
  http.get('/api/ws-token', () => {
    return HttpResponse.json({ token: 'mock-ws-token' })
  }),
]

export const server = setupServer(...handlers)

export default server
