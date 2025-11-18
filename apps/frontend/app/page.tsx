import { auth, signIn } from '@/auth'
import { redirect } from 'next/navigation'

export default async function Home() {
  const session = await auth()

  // If authenticated, redirect to lobby
  if (session) {
    redirect('/lobby')
  }

  const appName = process.env.NEXT_PUBLIC_APP_NAME || 'Nommie'

  return (
    <main className="flex flex-1 items-center justify-center px-4 pb-16 pt-10 sm:pt-16">
      <div className="mx-auto grid w-full max-w-6xl gap-8 lg:grid-cols-[minmax(0,1fr)_360px]">
        <section className="rounded-3xl border border-white/15 bg-surface/80 p-8 shadow-[0_40px_120px_rgba(0,0,0,0.25)] backdrop-blur">
          <p className="text-sm font-semibold uppercase tracking-[0.3em] text-subtle">
            Steady Nomination Whist evenings
          </p>
          <h1 className="mt-3 text-4xl font-semibold tracking-tight text-foreground sm:text-5xl lg:text-6xl">
            {appName} seats your table and keeps the count so you can simply
            play.
          </h1>
          <p className="mt-4 text-lg text-muted sm:text-xl">
            Deal the cards, declare your bid, and let the table cue each action
            wherever your friends are seated.
          </p>
          <div className="mt-8 flex flex-col gap-3 sm:flex-row">
            <form
              action={async () => {
                'use server'
                await signIn('google')
              }}
              className="sm:flex-1"
            >
              <button
                type="submit"
                className="flex w-full items-center justify-center gap-2 rounded-2xl bg-primary px-6 py-3 text-base font-semibold text-primary-foreground shadow-lg shadow-primary/30 transition hover:bg-primary/90"
              >
                <span role="img" aria-hidden>
                  🚪
                </span>
                Enter the lobby
              </button>
            </form>
            <div className="flex items-center justify-center rounded-2xl border border-border/60 bg-surface px-4 py-3 text-sm font-semibold text-muted shadow-inner shadow-black/10 sm:w-60">
              Take your seat from any device
            </div>
          </div>

          <dl className="mt-10 grid gap-4 sm:grid-cols-3">
            <div className="rounded-2xl border border-border/60 bg-surface px-4 py-3">
              <dt className="text-xs font-semibold uppercase tracking-wide text-subtle">
                Read the table
              </dt>
              <dd className="mt-1 text-sm text-foreground">
                Follow each seat and trick at a glance, no fuss.
              </dd>
            </div>
            <div className="rounded-2xl border border-border/60 bg-surface px-4 py-3">
              <dt className="text-xs font-semibold uppercase tracking-wide text-subtle">
                Prompted turns
              </dt>
              <dd className="mt-1 text-sm text-foreground">
                Play each card with clear prompts so the table keeps a reliable
                pace.
              </dd>
            </div>
            <div className="rounded-2xl border border-border/60 bg-surface px-4 py-3">
              <dt className="text-xs font-semibold uppercase tracking-wide text-subtle">
                Resume swiftly
              </dt>
              <dd className="mt-1 text-sm text-foreground">
                Rejoin your previous game from the header and resume the count
                straightaway.
              </dd>
            </div>
          </dl>
        </section>

        <section className="relative hidden rounded-3xl border border-white/20 bg-gradient-to-br from-surface/70 to-surface-strong/40 p-6 shadow-[0_30px_90px_rgba(0,0,0,0.35)] lg:flex lg:flex-col">
          <div className="text-sm font-semibold uppercase tracking-[0.4em] text-muted">
            At the table
          </div>
          <div className="mt-6 flex flex-1 items-center justify-center">
            <div className="relative aspect-[4/5] w-full max-w-xs rounded-[32px] border border-border/80 bg-gradient-to-b from-[#1f4639] to-[#0f2a21] p-6 shadow-2xl">
              <div className="absolute inset-6 rounded-[28px] border border-white/10" />
              <div className="relative flex h-full flex-col items-center justify-between text-center text-card-cream">
                <div className="w-full">
                  <p className="text-xs uppercase tracking-[0.4em] text-card-cream opacity-70">
                    your seat
                  </p>
                  <p className="mt-2 text-2xl font-semibold">
                    Dealer standing by
                  </p>
                </div>
                <div className="grid w-full grid-cols-2 gap-3 text-left text-sm">
                  <div className="rounded-2xl bg-white/10 p-3 backdrop-blur">
                    <p className="text-xs uppercase tracking-widest text-card-cream opacity-60">
                      Live turn
                    </p>
                    <p className="text-lg font-semibold">Your turn</p>
                  </div>
                  <div className="rounded-2xl bg-white/10 p-3 backdrop-blur">
                    <p className="text-xs uppercase tracking-widest text-card-cream opacity-60">
                      Tricks played
                    </p>
                    <p className="text-lg font-semibold">3 of 7</p>
                  </div>
                  <div className="col-span-2 rounded-2xl bg-white/10 p-3 backdrop-blur">
                    <p className="text-xs uppercase tracking-widest text-card-cream opacity-60">
                      Device swap
                    </p>
                    <p className="text-lg font-semibold">
                      Swap devices mid-round and stay with the play.
                    </p>
                  </div>
                </div>
                <div className="text-xs uppercase tracking-[0.5em] text-card-cream opacity-60">
                  play on
                </div>
              </div>
            </div>
          </div>
        </section>
      </div>
    </main>
  )
}
