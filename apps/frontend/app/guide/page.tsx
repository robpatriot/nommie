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

        <section className="rounded-3xl border border-border/50 bg-card/70 p-8 shadow-elevated">
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

        <section className="rounded-3xl border border-border/50 bg-card/70 p-8 shadow-elevated">
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

        <section className="rounded-3xl border border-border/50 bg-card/70 p-8 shadow-elevated">
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
