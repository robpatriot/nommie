import Link from 'next/link'
import { auth } from '@/auth'
import { redirect } from 'next/navigation'
import { getTranslations } from 'next-intl/server'
import { BreadcrumbSetter } from '@/components/header-breadcrumbs'
import { PageContainer } from '@/components/PageContainer'

export default async function GuidePage() {
  const t = await getTranslations('guide')
  const session = await auth()

  if (!session) {
    redirect('/')
  }

  return (
    <PageContainer>
      <BreadcrumbSetter crumbs={[{ label: t('breadcrumbs.guide') }]} />
      <div className="mx-auto flex w-full max-w-4xl flex-col gap-6">
        <div className="rounded-3xl border border-border/50 bg-card/70 p-8 shadow-elevated">
          <h1 className="text-3xl font-semibold text-foreground">
            {t('title')}
          </h1>
          <p className="mt-2 text-muted-foreground">{t('description')}</p>
        </div>

        <nav
          aria-label={t('toc.title')}
          className="rounded-3xl border border-border/50 bg-card/70 p-6 shadow-elevated"
        >
          <h2 className="mb-4 text-sm font-semibold uppercase tracking-wide text-muted-foreground">
            {t('toc.title')}
          </h2>
          <ol className="flex flex-wrap gap-x-6 gap-y-2 text-sm">
            <li>
              <Link
                href="#rules"
                className="font-medium text-foreground underline-offset-4 transition hover:text-primary hover:underline"
              >
                {t('toc.rules')}
              </Link>
            </li>
            <li>
              <Link
                href="#ui"
                className="font-medium text-foreground underline-offset-4 transition hover:text-primary hover:underline"
              >
                {t('toc.ui')}
              </Link>
            </li>
            <li>
              <Link
                href="#setup"
                className="font-medium text-foreground underline-offset-4 transition hover:text-primary hover:underline"
              >
                {t('toc.setup')}
              </Link>
            </li>
            <li>
              <Link
                href="#features"
                className="font-medium text-foreground underline-offset-4 transition hover:text-primary hover:underline"
              >
                {t('toc.features')}
              </Link>
            </li>
          </ol>
        </nav>

        <section
          id="rules"
          className="rounded-3xl border border-border/50 bg-card/70 p-8 shadow-elevated scroll-mt-6"
        >
          <div className="mb-6">
            <p className="text-sm uppercase tracking-wide text-muted-foreground">
              {t('sections.rules.kicker')}
            </p>
            <h2 className="text-2xl font-semibold text-foreground">
              {t('sections.rules.title')}
            </h2>
            <p className="mt-2 text-sm text-muted-foreground">
              {t('sections.rules.description')}
            </p>
          </div>
          <div className="space-y-4 text-foreground">
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.rules.players.title')}
              </h3>
              <p className="mt-1 text-sm text-muted-foreground">
                {t('sections.rules.players.description')}
              </p>
            </div>
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.rules.rounds.title')}
              </h3>
              <p className="mt-1 text-sm text-muted-foreground">
                {t('sections.rules.rounds.description')}
              </p>
            </div>
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.rules.bidding.title')}
              </h3>
              <p className="mt-1 text-sm text-muted-foreground">
                {t('sections.rules.bidding.description')}
              </p>
            </div>
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.rules.trumpSelection.title')}
              </h3>
              <p className="mt-1 text-sm text-muted-foreground">
                {t('sections.rules.trumpSelection.description')}
              </p>
            </div>
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.rules.trickPlay.title')}
              </h3>
              <p className="mt-1 text-sm text-muted-foreground">
                {t('sections.rules.trickPlay.description')}
              </p>
            </div>
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.rules.scoring.title')}
              </h3>
              <p className="mt-1 text-sm text-muted-foreground">
                {t('sections.rules.scoring.description')}
              </p>
            </div>
          </div>
        </section>

        <section
          id="ui"
          className="rounded-3xl border border-border/50 bg-card/70 p-8 shadow-elevated scroll-mt-6"
        >
          <div className="mb-6">
            <p className="text-sm uppercase tracking-wide text-muted-foreground">
              {t('sections.ui.kicker')}
            </p>
            <h2 className="text-2xl font-semibold text-foreground">
              {t('sections.ui.title')}
            </h2>
            <p className="mt-2 text-sm text-muted-foreground">
              {t('sections.ui.description')}
            </p>
          </div>
          <div className="space-y-4 text-foreground">
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.ui.playerHand.title')}
              </h3>
              <p className="mt-1 text-sm text-muted-foreground">
                {t('sections.ui.playerHand.description')}
              </p>
            </div>
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.ui.seatCards.title')}
              </h3>
              <p className="mt-1 text-sm text-muted-foreground">
                {t('sections.ui.seatCards.description')}
              </p>
            </div>
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.ui.trickArea.title')}
              </h3>
              <p className="mt-1 text-sm text-muted-foreground">
                {t('sections.ui.trickArea.description')}
              </p>
            </div>
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.ui.biddingPanel.title')}
              </h3>
              <p className="mt-1 text-sm text-muted-foreground">
                {t('sections.ui.biddingPanel.description')}
              </p>
            </div>
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.ui.scoreSidebar.title')}
              </h3>
              <p className="mt-1 text-sm text-muted-foreground">
                {t('sections.ui.scoreSidebar.description')}
              </p>
            </div>
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.ui.gameInfo.title')}
              </h3>
              <p className="mt-1 text-sm text-muted-foreground">
                {t('sections.ui.gameInfo.description')}
              </p>
            </div>
          </div>
        </section>

        <section
          id="setup"
          className="rounded-3xl border border-border/50 bg-card/70 p-8 shadow-elevated scroll-mt-6"
        >
          <div className="mb-6">
            <p className="text-sm uppercase tracking-wide text-muted-foreground">
              {t('sections.setup.kicker')}
            </p>
            <h2 className="text-2xl font-semibold text-foreground">
              {t('sections.setup.title')}
            </h2>
            <p className="mt-2 text-sm text-muted-foreground">
              {t('sections.setup.description')}
            </p>
          </div>
          <div className="space-y-4 text-foreground">
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.setup.startingGame.title')}
              </h3>
              <p className="mt-1 text-sm text-muted-foreground">
                {t('sections.setup.startingGame.description')}
              </p>
            </div>
            <div id="ai-players" className="scroll-mt-6">
              <h3 className="font-semibold text-foreground">
                {t('sections.setup.aiPlayers.title')}
              </h3>
              <p className="mt-1 text-sm text-muted-foreground">
                {t('sections.setup.aiPlayers.description')}
              </p>
              <ul className="mt-2 ml-4 list-disc space-y-1 text-sm text-muted-foreground">
                <li>{t('sections.setup.aiPlayers.add')}</li>
                <li>{t('sections.setup.aiPlayers.remove')}</li>
                <li>{t('sections.setup.aiPlayers.changeType')}</li>
              </ul>
            </div>
          </div>
        </section>

        <section
          id="features"
          className="rounded-3xl border border-border/50 bg-card/70 p-8 shadow-elevated scroll-mt-6"
        >
          <div className="mb-6">
            <p className="text-sm uppercase tracking-wide text-muted-foreground">
              {t('sections.features.kicker')}
            </p>
            <h2 className="text-2xl font-semibold text-foreground">
              {t('sections.features.title')}
            </h2>
            <p className="mt-2 text-sm text-muted-foreground">
              {t('sections.features.description')}
            </p>
          </div>
          <div className="space-y-4 text-foreground">
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.features.leavingGame.title')}
              </h3>
              <p className="mt-1 text-sm text-muted-foreground">
                {t('sections.features.leavingGame.description')}
              </p>
              <ul className="mt-2 ml-4 list-disc space-y-1 text-sm text-muted-foreground">
                <li>{t('sections.features.leavingGame.inLobby')}</li>
                <li>{t('sections.features.leavingGame.activeGame')}</li>
                <li>{t('sections.features.leavingGame.spectator')}</li>
              </ul>
            </div>
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.features.nextGameButton.title')}
              </h3>
              <p className="mt-1 text-sm text-muted-foreground">
                {t('sections.features.nextGameButton.description')}
              </p>
            </div>
            <div>
              <h3 className="font-semibold text-foreground">
                {t('sections.features.spectators.title')}
              </h3>
              <p className="mt-1 text-sm text-muted-foreground">
                {t('sections.features.spectators.description')}
              </p>
            </div>
          </div>
        </section>
      </div>
    </PageContainer>
  )
}
