import { auth } from '@/auth'
import { redirect } from 'next/navigation'
import { getTranslations } from 'next-intl/server'
import { BreadcrumbSetter } from '@/components/header-breadcrumbs'
import { AppearanceSelector } from '@/components/AppearanceSelector'
import { CardConfirmationToggle } from '@/components/CardConfirmationToggle'
import { LanguageSelector } from '@/components/LanguageSelector'
import { getUserOptions } from '@/lib/api/user-options'
import { handleAllowlistError } from '@/lib/auth/allowlist'
import type { ThemeMode } from '@/components/theme-provider'

export default async function SettingsPage() {
  const t = await getTranslations('settings')
  const session = await auth()

  if (!session) {
    redirect('/')
  }

  let requireCardConfirmation = true
  let preferredLocale: string | null = null
  let preferredAppearance: ThemeMode | null = null
  try {
    const options = await getUserOptions()
    requireCardConfirmation = options.require_card_confirmation
    preferredLocale = options.locale
    // Treat 'system' as null (no explicit preference) for consistency with locale
    preferredAppearance =
      options.appearance_mode === 'system' ? null : options.appearance_mode
  } catch (error) {
    await handleAllowlistError(error)
    // Swallow other errors and fall back to default
  }

  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-6 px-4 py-10">
      <BreadcrumbSetter crumbs={[{ label: t('breadcrumbs.settings') }]} />
      <section className="rounded-3xl border border-border/50 bg-surface/70 p-8">
        <div className="mb-6">
          <p className="text-sm uppercase tracking-wide text-subtle">
            {t('sections.display.kicker')}
          </p>
          <h2 className="text-2xl font-semibold text-foreground">
            {t('sections.display.appearance.title')}
          </h2>
          <p className="mt-2 text-sm text-muted">
            {t('sections.display.appearance.description')}
          </p>
        </div>
        <AppearanceSelector preferredAppearance={preferredAppearance} />
      </section>
      <section className="rounded-3xl border border-border/50 bg-surface/70 p-8">
        <div className="mb-6">
          <p className="text-sm uppercase tracking-wide text-subtle">
            {t('sections.language.kicker')}
          </p>
          <h2 className="text-2xl font-semibold text-foreground">
            {t('sections.language.title')}
          </h2>
          <p className="mt-2 text-sm text-muted">
            {t('sections.language.description')}
          </p>
        </div>
        <LanguageSelector preferredLocale={preferredLocale} />
      </section>
      <section className="rounded-3xl border border-border/50 bg-surface/70 p-8">
        <div className="mb-6">
          <p className="text-sm uppercase tracking-wide text-subtle">
            {t('sections.gameplay.kicker')}
          </p>
          <h2 className="text-2xl font-semibold text-foreground">
            {t('sections.gameplay.cardConfirmation.title')}
          </h2>
          <p className="mt-2 text-sm text-muted">
            {t('sections.gameplay.cardConfirmation.description')}
          </p>
        </div>
        <CardConfirmationToggle initialEnabled={requireCardConfirmation} />
      </section>
    </div>
  )
}
