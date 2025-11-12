export default function GameRoomLoading() {
  return (
    <div className="flex min-h-screen flex-col bg-background text-foreground">
      {/* Header skeleton */}
      <header className="border-b border-border bg-surface/80 backdrop-blur">
        <div className="mx-auto flex w-full max-w-7xl flex-wrap items-center justify-between gap-2 px-4 py-4 sm:px-6 lg:px-10">
          <div className="flex flex-1 flex-col gap-1">
            <div className="h-5 w-24 animate-pulse rounded bg-surface" />
            <div className="h-8 w-40 animate-pulse rounded bg-surface" />
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <div className="h-9 w-20 animate-pulse rounded bg-surface" />
            <div className="h-9 w-32 animate-pulse rounded bg-surface" />
            <div className="h-9 w-28 animate-pulse rounded bg-surface" />
          </div>
        </div>
      </header>

      <main className="flex flex-1 flex-col gap-6 px-4 py-6 sm:px-6 lg:px-10">
        {/* Phase header skeleton */}
        <section className="flex flex-col gap-4 rounded-xl border border-border bg-surface/70 p-4 shadow-elevated">
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div className="h-6 w-32 animate-pulse rounded bg-surface" />
            <div className="h-6 w-24 animate-pulse rounded bg-surface" />
          </div>
        </section>

        {/* Table area skeleton */}
        <section className="mx-auto flex w-full max-w-7xl flex-col gap-6">
          <div className="relative mx-auto grid h-full w-full max-w-4xl grid-cols-3 grid-rows-3 gap-4">
            {/* Seat skeletons */}
            {[0, 1, 2, 3].map((i) => (
              <div
                key={i}
                className="flex flex-col items-center justify-center gap-2 rounded-xl border border-border bg-surface/70 p-4"
              >
                <div className="h-5 w-24 animate-pulse rounded bg-surface" />
                <div className="h-4 w-16 animate-pulse rounded bg-surface" />
              </div>
            ))}
            {/* Trick area skeleton */}
            <div className="col-start-2 row-start-2 flex items-center justify-center rounded-xl border border-border bg-surface/50 p-8">
              <div className="h-20 w-32 animate-pulse rounded bg-surface" />
            </div>
          </div>

          {/* Hand skeleton */}
          <div className="mx-auto flex w-full max-w-4xl flex-col gap-3 rounded-2xl border border-border bg-surface/70 p-4">
            <div className="flex items-center justify-between">
              <div className="h-5 w-24 animate-pulse rounded bg-surface" />
              <div className="h-4 w-32 animate-pulse rounded bg-surface" />
            </div>
            <div className="flex flex-wrap justify-center gap-2">
              {[1, 2, 3, 4, 5, 6, 7, 8].map((i) => (
                <div
                  key={i}
                  className="h-12 w-16 animate-pulse rounded-xl border border-border bg-surface"
                />
              ))}
            </div>
          </div>

          {/* Actions skeleton */}
          <div className="mx-auto flex w-full max-w-4xl flex-col gap-4 rounded-2xl border border-primary/40 bg-primary/10 p-4">
            <div className="h-6 w-32 animate-pulse rounded bg-surface" />
            <div className="h-10 w-full animate-pulse rounded bg-surface" />
          </div>
        </section>

        {/* Sidebar skeleton */}
        <aside className="mx-auto w-full max-w-md rounded-xl border border-border bg-surface/70 p-4">
          <div className="space-y-4">
            <div className="h-6 w-24 animate-pulse rounded bg-surface" />
            <div className="space-y-2">
              {[1, 2, 3, 4].map((i) => (
                <div
                  key={i}
                  className="flex items-center justify-between rounded border border-border bg-surface-strong p-2"
                >
                  <div className="h-4 w-20 animate-pulse rounded bg-surface" />
                  <div className="h-4 w-8 animate-pulse rounded bg-surface" />
                </div>
              ))}
            </div>
          </div>
        </aside>
      </main>
    </div>
  )
}
