import { describe, it, expect, vi } from 'vitest'
import { drainResponseBody } from './drain-response'

describe('drainResponseBody', () => {
  it('reads the body when present', async () => {
    const response = new Response('ok')
    const spy = vi.spyOn(response, 'arrayBuffer')

    await drainResponseBody(response)

    expect(spy).toHaveBeenCalledTimes(1)
  })

  it('no-ops when body is already used', async () => {
    const response = new Response('ok')
    await response.arrayBuffer()
    const spy = vi.spyOn(response, 'arrayBuffer')

    await drainResponseBody(response)

    expect(spy).not.toHaveBeenCalled()
  })

  it('no-ops when body is absent', async () => {
    const response = new Response(null)
    const spy = vi.spyOn(response, 'arrayBuffer')

    await drainResponseBody(response)

    expect(spy).not.toHaveBeenCalled()
  })

  it('does not throw if reading fails', async () => {
    const response = new Response('ok')
    const error = new Error('read failed')
    const readSpy = vi
      .spyOn(response, 'arrayBuffer')
      .mockRejectedValueOnce(error)
    const cancelSpy = vi.spyOn(response.body!, 'cancel')

    await expect(drainResponseBody(response)).resolves.toBeUndefined()

    expect(readSpy).toHaveBeenCalledTimes(1)
    expect(cancelSpy).toHaveBeenCalledTimes(1)
  })
})
