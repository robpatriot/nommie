export default function LobbyLoading() {
  return (
    <div className="min-h-screen bg-background py-12">
      <div className="mx-auto max-w-4xl px-4 sm:px-6 lg:px-8">
        {/* Header skeleton */}
        <div className="mb-6">
          <div className="rounded-lg border border-border bg-muted shadow-elevated p-6">
            <div className="mb-4 flex items-center justify-between">
              <div className="h-9 w-48 animate-pulse rounded bg-card" />
              <div className="flex items-center gap-3">
                <div className="h-9 w-28 animate-pulse rounded bg-card" />
                <div className="h-9 w-24 animate-pulse rounded bg-card" />
              </div>
            </div>
            <div className="mb-4 h-10 w-48 animate-pulse rounded bg-card" />
          </div>
        </div>

        {/* Game lists skeleton */}
        <div className="space-y-6">
          {/* Joinable games skeleton */}
          <div className="mb-8">
            <div className="mb-4 flex items-center justify-between gap-3">
              <div className="h-7 w-40 animate-pulse rounded bg-card" />
              <div className="h-9 w-32 animate-pulse rounded bg-card" />
            </div>
            <div className="rounded-lg border border-border bg-card p-6">
              <div className="space-y-3">
                {[1, 2, 3].map((i) => (
                  <div
                    key={i}
                    className="flex items-center justify-between gap-4 rounded border border-border bg-muted p-4"
                  >
                    <div className="flex-1 space-y-2">
                      <div className="h-5 w-48 animate-pulse rounded bg-card" />
                      <div className="h-4 w-32 animate-pulse rounded bg-card" />
                    </div>
                    <div className="h-9 w-20 animate-pulse rounded bg-card" />
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* In-progress games skeleton */}
          <div className="mb-8">
            <div className="mb-4 flex items-center justify-between gap-3">
              <div className="h-7 w-40 animate-pulse rounded bg-card" />
              <div className="h-9 w-32 animate-pulse rounded bg-card" />
            </div>
            <div className="rounded-lg border border-border bg-card p-6">
              <div className="space-y-3">
                {[1, 2].map((i) => (
                  <div
                    key={i}
                    className="flex items-center justify-between gap-4 rounded border border-border bg-muted p-4"
                  >
                    <div className="flex-1 space-y-2">
                      <div className="h-5 w-48 animate-pulse rounded bg-card" />
                      <div className="h-4 w-32 animate-pulse rounded bg-card" />
                    </div>
                    <div className="h-9 w-24 animate-pulse rounded bg-card" />
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
