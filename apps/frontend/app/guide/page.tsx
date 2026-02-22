import Link from 'next/link'
import { AiSeatManagerPreview } from './AiSeatManagerPreview'
import { TrickAreaHeaderPreview } from './TrickAreaHeaderPreview'
import { auth } from '@/auth'
import { redirect } from 'next/navigation'
import { getTranslations } from 'next-intl/server'
import { BreadcrumbSetter } from '@/components/header-breadcrumbs'
import { PageContainer } from '@/components/PageContainer'

export default async function GuidePage() {
  const t = await getTranslations('guide')
  const tSidebar = await getTranslations('game.gameRoom.sidebar')
  const session = await auth()

  if (!session) {
    redirect('/')
  }

  return (
    <PageContainer>
      <BreadcrumbSetter crumbs={[{ label: t('breadcrumbs.guide') }]} />
      <div className="mx-auto flex w-full max-w-4xl flex-col gap-8">
        <div className="overflow-hidden rounded-3xl border border-border/50 bg-gradient-to-br from-card via-card to-muted/40 p-8 shadow-elevated sm:p-10">
          <h1 className="text-3xl font-semibold tracking-tight text-foreground sm:text-4xl">
            {t('title')}
          </h1>
          <p className="mt-3 text-base leading-relaxed text-muted-foreground sm:text-lg">
            {t('description')}
          </p>
        </div>

        <nav
          aria-label={t('toc.title')}
          className="rounded-2xl border border-border/40 bg-card/50 px-5 py-4 shadow-sm backdrop-blur-sm"
        >
          <h2 className="mb-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            {t('toc.title')}
          </h2>
          <ol className="flex flex-wrap gap-2">
            <li>
              <Link
                href="#rules"
                className="inline-flex rounded-full border border-border/60 bg-card px-4 py-1.5 text-sm font-medium text-foreground transition hover:border-primary/40 hover:bg-primary/10 hover:text-primary"
              >
                {t('toc.rules')}
              </Link>
            </li>
            <li>
              <Link
                href="#ui"
                className="inline-flex rounded-full border border-border/60 bg-card px-4 py-1.5 text-sm font-medium text-foreground transition hover:border-primary/40 hover:bg-primary/10 hover:text-primary"
              >
                {t('toc.ui')}
              </Link>
            </li>
            <li>
              <Link
                href="#setup"
                className="inline-flex rounded-full border border-border/60 bg-card px-4 py-1.5 text-sm font-medium text-foreground transition hover:border-primary/40 hover:bg-primary/10 hover:text-primary"
              >
                {t('toc.setup')}
              </Link>
            </li>
            <li>
              <Link
                href="#features"
                className="inline-flex rounded-full border border-border/60 bg-card px-4 py-1.5 text-sm font-medium text-foreground transition hover:border-primary/40 hover:bg-primary/10 hover:text-primary"
              >
                {t('toc.features')}
              </Link>
            </li>
          </ol>
        </nav>

        <section
          id="rules"
          className="rounded-3xl border border-border/50 bg-card/70 p-8 shadow-elevated scroll-mt-6 sm:p-10"
        >
          <div className="mb-8">
            <p className="text-xs font-semibold uppercase tracking-wider text-accent">
              {t('sections.rules.kicker')}
            </p>
            <h2 className="mt-1 text-2xl font-semibold tracking-tight text-foreground sm:text-3xl">
              {t('sections.rules.title')}
            </h2>
            <p className="mt-3 text-sm leading-relaxed text-muted-foreground">
              {t('sections.rules.description')}
            </p>
          </div>
          <div className="space-y-6 text-foreground">
            {[
              'players',
              'rounds',
              'bidding',
              'trumpSelection',
              'trickPlay',
              'scoring',
            ].map((key) => (
              <div key={key}>
                <h3 className="font-semibold text-foreground">
                  {t(`sections.rules.${key}.title`)}
                </h3>
                <p className="mt-1.5 text-sm leading-relaxed text-muted-foreground">
                  {t(`sections.rules.${key}.description`)}
                </p>
              </div>
            ))}
          </div>
        </section>

        <section
          id="ui"
          className="rounded-3xl border border-border/50 bg-card/70 p-8 shadow-elevated scroll-mt-6 sm:p-10"
        >
          <div className="mb-8">
            <p className="text-xs font-semibold uppercase tracking-wider text-accent">
              {t('sections.ui.kicker')}
            </p>
            <h2 className="mt-1 text-2xl font-semibold tracking-tight text-foreground sm:text-3xl">
              {t('sections.ui.title')}
            </h2>
            <p className="mt-3 text-sm leading-relaxed text-muted-foreground">
              {t('sections.ui.description')}
            </p>
          </div>
          <div className="space-y-6 text-foreground">
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.ui.playerHand.title')}
              </h3>
              <p className="mt-1.5 text-sm leading-relaxed text-muted-foreground">
                {t('sections.ui.playerHand.description')}
              </p>
            </div>
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.ui.seatCards.title')}
              </h3>
              <p className="mt-1.5 text-sm leading-relaxed text-muted-foreground">
                {t('sections.ui.seatCards.description')}
              </p>
            </div>
            <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:gap-4">
              <div className="min-w-0 flex-1">
                <h3 className="font-semibold text-foreground">
                  {t('sections.ui.trickArea.title')}
                </h3>
                <p className="mt-1.5 text-sm leading-relaxed text-muted-foreground">
                  {t('sections.ui.trickArea.description')}
                </p>
              </div>
              <div className="shrink-0">
                <div className="w-[280px] max-w-full">
                  <TrickAreaHeaderPreview />
                </div>
              </div>
            </div>
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.ui.biddingPanel.title')}
              </h3>
              <p className="mt-1.5 text-sm leading-relaxed text-muted-foreground">
                {t('sections.ui.biddingPanel.description')}
              </p>
            </div>
            <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:gap-4">
              <div className="min-w-0 flex-1">
                <h3 className="font-semibold text-foreground">
                  {t('sections.ui.scoreSidebar.title')}
                </h3>
                <p className="mt-1.5 text-sm leading-relaxed text-muted-foreground">
                  {t('sections.ui.scoreSidebar.description')}
                </p>
              </div>
              <div className="shrink-0">
                <span
                  role="img"
                  aria-label={tSidebar('scoreboard.showHistoryAria')}
                  className="inline-flex items-center gap-2 rounded-full border border-border/60 bg-card/60 px-6 py-2 text-[22px] font-semibold text-foreground"
                >
                  <span>{tSidebar('scoreboard.history')}</span>
                  <svg
                    aria-hidden="true"
                    className="h-6 w-6"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth={1.8}
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M6 2h9l5 5v15H6z" />
                    <path d="M14 2v6h6" />
                    <path d="M8 13h8" />
                    <path d="M8 17h5" />
                  </svg>
                </span>
              </div>
            </div>
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.ui.gameInfo.title')}
              </h3>
              <p className="mt-1.5 text-sm leading-relaxed text-muted-foreground">
                {t('sections.ui.gameInfo.description')}
              </p>
            </div>
          </div>
        </section>

        <section
          id="setup"
          className="rounded-3xl border border-border/50 bg-card/70 p-8 shadow-elevated scroll-mt-6 sm:p-10"
        >
          <div className="mb-8">
            <p className="text-xs font-semibold uppercase tracking-wider text-accent">
              {t('sections.setup.kicker')}
            </p>
            <h2 className="mt-1 text-2xl font-semibold tracking-tight text-foreground sm:text-3xl">
              {t('sections.setup.title')}
            </h2>
            <p className="mt-3 text-sm leading-relaxed text-muted-foreground">
              {t('sections.setup.description')}
            </p>
          </div>
          <div className="space-y-6 text-foreground">
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.setup.startingGame.title')}
              </h3>
              <p className="mt-1.5 text-sm leading-relaxed text-muted-foreground">
                {t('sections.setup.startingGame.description')}
              </p>
            </div>
            <div id="ai-players" className="scroll-mt-6">
              <h3 className="font-semibold text-foreground">
                {t('sections.setup.aiPlayers.title')}
              </h3>
              <p className="mt-1.5 text-sm leading-relaxed text-muted-foreground">
                {t('sections.setup.aiPlayers.description')}
              </p>
              <ul className="mt-3 space-y-2 pl-1 text-sm text-muted-foreground">
                <li className="flex gap-2">
                  <span className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-accent/60" />
                  <span>{t('sections.setup.aiPlayers.add')}</span>
                </li>
                <li className="flex gap-2">
                  <span className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-accent/60" />
                  <span>{t('sections.setup.aiPlayers.remove')}</span>
                </li>
                <li className="flex gap-2">
                  <span className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-accent/60" />
                  <span>{t('sections.setup.aiPlayers.changeType')}</span>
                </li>
              </ul>
              <AiSeatManagerPreview />
            </div>
          </div>
        </section>

        <section
          id="features"
          className="rounded-3xl border border-border/50 bg-card/70 p-8 shadow-elevated scroll-mt-6 sm:p-10"
        >
          <div className="mb-8">
            <p className="text-xs font-semibold uppercase tracking-wider text-accent">
              {t('sections.features.kicker')}
            </p>
            <h2 className="mt-1 text-2xl font-semibold tracking-tight text-foreground sm:text-3xl">
              {t('sections.features.title')}
            </h2>
            <p className="mt-3 text-sm leading-relaxed text-muted-foreground">
              {t('sections.features.description')}
            </p>
          </div>
          <div className="space-y-6 text-foreground">
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.features.leavingGame.title')}
              </h3>
              <p className="mt-1.5 text-sm leading-relaxed text-muted-foreground">
                {t('sections.features.leavingGame.description')}
              </p>
              <ul className="mt-3 space-y-2 pl-1 text-sm text-muted-foreground">
                <li className="flex gap-2">
                  <span className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-accent/60" />
                  <span>{t('sections.features.leavingGame.inLobby')}</span>
                </li>
                <li className="flex gap-2">
                  <span className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-accent/60" />
                  <span>{t('sections.features.leavingGame.activeGame')}</span>
                </li>
                <li className="flex gap-2">
                  <span className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-accent/60" />
                  <span>{t('sections.features.leavingGame.spectator')}</span>
                </li>
              </ul>
            </div>
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.features.nextGameButton.title')}
              </h3>
              <p className="mt-1.5 text-sm leading-relaxed text-muted-foreground">
                {t('sections.features.nextGameButton.description')}
              </p>
            </div>
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.features.spectators.title')}
              </h3>
              <p className="mt-1.5 text-sm leading-relaxed text-muted-foreground">
                {t('sections.features.spectators.description')}
              </p>
            </div>
          </div>
        </section>
      </div>
    </PageContainer>
  )
}
