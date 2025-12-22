import { auth } from '@/auth'
import { redirect } from 'next/navigation'
import { getTranslations } from 'next-intl/server'
import { PageContainer } from '@/components/PageContainer'
import { SurfaceCard } from '@/components/SurfaceCard'
import { StatCard } from '@/components/StatCard'
import { signInWithGoogleAction } from '@/app/actions/auth-actions'

export default async function Home({
  searchParams,
}: {
  searchParams?: Promise<{ accessDenied?: string }>
}) {
  const t = await getTranslations('common')
  const session = await auth()
  const params = (await searchParams) ?? {}
  const showAccessDenied = params.accessDenied === 'true'

  // If authenticated, redirect to lobby
  // (Note: if accessDenied=true is present, the user should have been signed out
  //  by the Route Handler, so session should be null. But we check anyway.)
  if (session) {
    redirect('/lobby')
  }

  const appName = process.env.NEXT_PUBLIC_APP_NAME || 'Nommie'

  return (
    <PageContainer className="flex-1 justify-center gap-0 pb-16 pt-10 sm:pt-16">
      {/* Show access denied message if present */}
      {showAccessDenied && (
        <SurfaceCard
          padding="lg"
          tone="strong"
          className="mb-6 max-w-4xl mx-auto"
        >
          <h2 className="text-xl font-semibold mb-2">
            {t('home.accessDenied.title')}
          </h2>
          <p className="text-sm text-muted">
            {t('home.accessDenied.description')}
          </p>
        </SurfaceCard>
      )}

      <div className="grid w-full gap-8 lg:grid-cols-[minmax(0,1fr)_360px]">
        <SurfaceCard as="section" padding="lg" tone="strong">
          <p className="text-sm font-semibold uppercase tracking-[0.3em] text-subtle">
            {t('home.hero.kicker')}
          </p>
          <h1 className="mt-3 text-4xl font-semibold tracking-tight text-foreground sm:text-5xl lg:text-6xl">
            {t('home.hero.title', { appName })}
          </h1>
          <p className="mt-4 text-lg text-muted sm:text-xl">
            {t('home.hero.description')}
          </p>
          <div className="mt-8 flex flex-col gap-3 sm:flex-row">
            <form action={signInWithGoogleAction} className="sm:flex-1">
              <button
                type="submit"
                className="flex w-full items-center justify-center gap-2 rounded-2xl bg-primary px-6 py-3 text-base font-semibold text-primary-foreground shadow-lg shadow-primary/30 transition hover:bg-primary/90"
              >
                <span role="img" aria-hidden>
                  🚪
                </span>
                {t('home.hero.primaryCta')}
              </button>
            </form>
            <div className="flex items-center justify-center rounded-2xl border border-border/60 bg-surface px-4 py-3 text-sm font-semibold text-muted shadow-inner shadow-black/10 sm:w-60">
              {t('home.hero.secondaryCta')}
            </div>
          </div>

          <div className="mt-10 grid gap-4 sm:grid-cols-3">
            <StatCard
              align="start"
              label={t('home.stats.readTheTable.label')}
              value={t('home.stats.readTheTable.value')}
              valueClassName="text-sm font-semibold text-foreground"
            />
            <StatCard
              align="start"
              label={t('home.stats.promptedTurns.label')}
              value={t('home.stats.promptedTurns.value')}
              valueClassName="text-sm font-semibold text-foreground"
            />
            <StatCard
              align="start"
              label={t('home.stats.resumeSwiftly.label')}
              value={t('home.stats.resumeSwiftly.value')}
              valueClassName="text-sm font-semibold text-foreground"
            />
          </div>
        </SurfaceCard>

        <SurfaceCard
          as="section"
          padding="lg"
          tone="subtle"
          className="relative flex flex-col border-white/20 bg-gradient-to-br from-surface/70 to-surface-strong/40 shadow-elevated"
        >
          <div className="text-sm font-semibold uppercase tracking-[0.4em] text-muted">
            {t('home.aside.kicker')}
          </div>
          <div className="mt-6 flex flex-1 items-center justify-center">
            <div className="card-wrapper-home relative w-full max-w-xs rounded-[32px] border border-white/20 p-6 shadow-2xl before:absolute before:inset-0 before:rounded-[32px] before:bg-gradient-to-br before:from-white/5 before:to-transparent before:pointer-events-none overflow-hidden">
              <div className="relative flex h-full flex-col text-card-cream z-10 p-4 justify-center gap-4">
                {/* Header with round info */}
                <div className="text-center relative px-2">
                  <div className="absolute inset-0 -top-2 -bottom-2 bg-gradient-to-b from-white/5 via-transparent to-transparent rounded-full blur-xl" />
                  <p className="text-xs uppercase tracking-[0.4em] text-card-cream opacity-80 relative">
                    {t('home.aside.roundLabel')}
                  </p>
                  <p className="mt-1.5 text-xl font-bold tracking-tight relative">
                    {t('home.aside.roundValue')}
                  </p>
                </div>

                {/* Table view with player positions */}
                <div className="relative flex items-center justify-center overflow-visible">
                  <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
                    <div
                      className="w-28 h-28 rounded-full border border-white/10"
                      style={{
                        background:
                          'radial-gradient(circle, rgba(255,255,255,0.05) 0%, transparent 70%)',
                      }}
                    />
                  </div>
                  <div className="relative w-full aspect-square max-w-[140px] mx-auto">
                    {/* Center card display */}
                    <div className="absolute inset-0 flex items-center justify-center z-20">
                      <div className="relative rounded-2xl bg-gradient-to-br from-white/25 to-white/10 backdrop-blur-md border-2 border-white/30 p-2.5 min-w-[65px] shadow-lg shadow-black/20 transform hover:scale-105 transition-transform">
                        <div className="absolute inset-0 rounded-2xl bg-gradient-to-br from-white/10 to-transparent" />
                        <div className="relative text-center">
                          <span
                            className="text-xl leading-none drop-shadow-lg"
                            aria-hidden="true"
                          >
                            {t('home.aside.centerCardSuit')}
                          </span>
                          <p className="text-xs font-bold mt-0.5 tracking-tight">
                            {t('home.aside.centerCardRank')}
                          </p>
                        </div>
                        <div className="absolute top-0.5 left-1 text-[9px] font-bold opacity-60">
                          {t('home.aside.centerCardRank')}
                        </div>
                        <div className="absolute bottom-0.5 right-1 text-[9px] font-bold opacity-60 rotate-180">
                          {t('home.aside.centerCardRank')}
                        </div>
                      </div>
                    </div>

                    {/* Player positions around table */}
                    <div className="absolute inset-0">
                      {/* North */}
                      <div className="absolute top-0 left-1/2 -translate-x-1/2 -translate-y-1/2 z-10">
                        <div className="rounded-lg bg-gradient-to-br from-white/15 to-white/5 backdrop-blur-md px-2.5 py-1 border border-white/20 shadow-md">
                          <p className="text-[9px] uppercase tracking-wider font-medium opacity-90">
                            {t('home.aside.playerNorth')}
                          </p>
                        </div>
                      </div>
                      {/* East */}
                      <div className="absolute right-0 top-1/2 translate-x-1/2 -translate-y-1/2 z-10">
                        <div className="rounded-lg bg-gradient-to-br from-white/15 to-white/5 backdrop-blur-md px-2.5 py-1 border border-white/20 shadow-md">
                          <p className="text-[9px] uppercase tracking-wider font-medium opacity-90">
                            {t('home.aside.playerEast')}
                          </p>
                        </div>
                      </div>
                      {/* South (You) */}
                      <div className="absolute bottom-0 left-1/2 -translate-x-1/2 translate-y-1/2 z-20">
                        <div className="rounded-lg bg-gradient-to-br from-white/30 to-white/15 backdrop-blur-md px-2.5 py-1 border-2 border-white/35 shadow-lg ring-2 ring-white/20">
                          <p className="text-[9px] uppercase tracking-wider font-bold">
                            {t('home.aside.playerSouth')}
                          </p>
                        </div>
                      </div>
                      {/* West */}
                      <div className="absolute left-0 top-1/2 -translate-x-1/2 -translate-y-1/2 z-10">
                        <div className="rounded-lg bg-gradient-to-br from-white/15 to-white/5 backdrop-blur-md px-2.5 py-1 border border-white/20 shadow-md">
                          <p className="text-[9px] uppercase tracking-wider font-medium opacity-90">
                            {t('home.aside.playerWest')}
                          </p>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>

                {/* Game state info */}
                <div className="space-y-2.5 px-1">
                  <div className="flex items-center justify-between rounded-xl bg-gradient-to-r from-white/15 to-white/5 backdrop-blur-md p-2.5 border border-white/20 shadow-md">
                    <div className="flex items-center gap-2">
                      <span
                        className="text-base drop-shadow-md"
                        aria-hidden="true"
                      >
                        {t('home.aside.trumpSuitIcon')}
                      </span>
                      <p className="text-[10px] uppercase tracking-wider text-card-cream opacity-80 font-medium">
                        {t('home.aside.trumpLabel')}
                      </p>
                    </div>
                    <p className="text-sm font-bold">
                      {t('home.aside.trumpValue')}
                    </p>
                  </div>
                  <div className="grid grid-cols-2 gap-2.5">
                    <div className="rounded-xl bg-gradient-to-br from-white/15 to-white/5 backdrop-blur-md p-2.5 border border-white/20 shadow-md">
                      <p className="text-[10px] uppercase tracking-wider text-card-cream opacity-80 mb-1 font-medium">
                        {t('home.aside.trickLabel')}
                      </p>
                      <p className="text-base font-bold">
                        {t('home.aside.trickValue')}
                      </p>
                    </div>
                    <div className="rounded-xl bg-gradient-to-br from-white/15 to-white/5 backdrop-blur-md p-2.5 border border-white/20 shadow-md">
                      <p className="text-[10px] uppercase tracking-wider text-card-cream opacity-80 mb-1 font-medium">
                        {t('home.aside.bidLabel')}
                      </p>
                      <p className="text-base font-bold">
                        {t('home.aside.bidValue')}
                      </p>
                    </div>
                  </div>
                </div>

                {/* Footer */}
                <div className="text-center pt-2 relative px-1">
                  <div className="absolute inset-x-1 top-0 h-px bg-gradient-to-r from-transparent via-white/20 to-transparent" />
                  <p className="text-[10px] uppercase tracking-[0.5em] text-card-cream opacity-70 font-medium relative pt-2.5">
                    {t('home.aside.footer')}
                  </p>
                </div>
              </div>
            </div>
          </div>
        </SurfaceCard>
      </div>
    </PageContainer>
  )
}
