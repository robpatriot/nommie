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
          className="relative hidden border-white/20 bg-gradient-to-br from-surface/70 to-surface-strong/40 shadow-elevated lg:flex lg:flex-col"
        >
          <div className="text-sm font-semibold uppercase tracking-[0.4em] text-muted">
            {t('home.aside.kicker')}
          </div>
          <div className="mt-6 flex flex-1 items-center justify-center">
            <div className="card-wrapper-home relative aspect-[4/5] w-full max-w-xs rounded-[32px] border border-border/80 p-6 shadow-2xl">
              <div className="absolute inset-6 rounded-[28px] border border-white/10" />
              <div className="relative flex h-full flex-col items-center justify-between text-center text-card-cream">
                <div className="w-full">
                  <p className="text-xs uppercase tracking-[0.4em] text-card-cream opacity-70">
                    {t('home.aside.yourSeat')}
                  </p>
                  <p className="mt-2 text-2xl font-semibold">
                    {t('home.aside.dealerStandingBy')}
                  </p>
                </div>
                <div className="grid w-full grid-cols-2 gap-3 text-left text-sm">
                  <div className="rounded-2xl bg-white/10 p-3 backdrop-blur">
                    <p className="text-xs uppercase tracking-widest text-card-cream opacity-60">
                      {t('home.aside.liveTurn')}
                    </p>
                    <p className="text-lg font-semibold">
                      {t('home.aside.yourTurn')}
                    </p>
                  </div>
                  <div className="rounded-2xl bg-white/10 p-3 backdrop-blur">
                    <p className="text-xs uppercase tracking-widest text-card-cream opacity-60">
                      {t('home.aside.tricksPlayed')}
                    </p>
                    <p className="text-lg font-semibold">
                      {t('home.aside.tricksValue')}
                    </p>
                  </div>
                  <div className="col-span-2 rounded-2xl bg-white/10 p-3 backdrop-blur">
                    <p className="text-xs uppercase tracking-widest text-card-cream opacity-60">
                      {t('home.aside.deviceSwap')}
                    </p>
                    <p className="text-lg font-semibold">
                      {t('home.aside.deviceSwapValue')}
                    </p>
                  </div>
                </div>
                <div className="text-xs uppercase tracking-[0.5em] text-card-cream opacity-60">
                  {t('home.aside.footer')}
                </div>
              </div>
            </div>
          </div>
        </SurfaceCard>
      </div>
    </PageContainer>
  )
}
