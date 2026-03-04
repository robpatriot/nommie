/**
 * Best-effort helper to fully consume or cancel a fetch `Response` body so that
 * connections can be cleanly reused and DevTools/network tooling do not show
 * hanging "content download" phases.
 *
 * Draining is intentionally conservative and never throws: failures are ignored
 * because the primary goal is connection cleanup rather than data access.
 */
export async function drainResponseBody(response: Response): Promise<void> {
  // If the body was already consumed or is unavailable, there is nothing to do.
  if (response.bodyUsed || !response.body) {
    return
  }

  try {
    // Use arrayBuffer to consume the stream without incurring text decoding
    // overhead or assuming a particular content encoding.
    await response.arrayBuffer()
  } catch {
    // If draining via read fails (e.g. AbortError), fall back to cancelling the
    // underlying stream when supported. Errors from cancellation are ignored.
    try {
      await response.body.cancel()
    } catch {
      // ignore
    }
  }
}
