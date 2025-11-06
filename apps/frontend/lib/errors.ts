// Client-safe error classes (can be imported from client components)

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
