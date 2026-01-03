import { auth } from '@/auth'
import { redirect } from 'next/navigation'
import { getTranslations } from 'next-intl/server'
import { BreadcrumbSetter } from '@/components/header-breadcrumbs'
import { AppearanceSelector } from '@/components/AppearanceSelector'
import { CardConfirmationToggle } from '@/components/CardConfirmationToggle'
import { LanguageSelector } from '@/components/LanguageSelector'
import { TrickDisplayDurationInput } from '@/components/TrickDisplayDurationInput'
import { getUserOptions } from '@/lib/api/user-options'
import {
  handleAllowlistError,
  handleStaleSessionError,
} from '@/lib/auth/allowlist'
import { logError } from '@/lib/logging/error-logger'
import type { ThemeMode } from '@/components/theme-provider'
import type { SupportedLocale } from '@/i18n/locale'

export default async function SettingsPage() {
  const t = await getTranslations('settings')
  const session = await auth()

  if (!session) {
    redirect('/')
  }

  let requireCardConfirmation = true
  let preferredLocale: SupportedLocale | null = null
  let preferredAppearance: ThemeMode | null = null
  let trickDisplayDurationSeconds: number | null = null
  try {
    const options = await getUserOptions()
    requireCardConfirmation = options.require_card_confirmation
    preferredLocale = options.locale
    trickDisplayDurationSeconds = options.trick_display_duration_seconds
    // Treat 'system' as null (no explicit preference) for consistency with locale
    preferredAppearance =
      options.appearance_mode === 'system' ? null : options.appearance_mode
  } catch (error) {
    // Try to handle allowlist/session errors (these will redirect if they match)
    await handleAllowlistError(error)
    await handleStaleSessionError(error)

    // If we reach here, the error wasn't handled by allowlist/session handlers
    // (they redirect if they match, so execution stops there)
    // Log unexpected errors for debugging
    logError('Failed to load user options on settings page', error, {
      action: 'getUserOptions',
    })
    // Fall back to defaults
  }

  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-6 px-4 py-10">
      <BreadcrumbSetter crumbs={[{ label: t('breadcrumbs.settings') }]} />
      <section className="rounded-3xl border border-border/50 bg-surface/70 p-8 shadow-elevated">
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
      <section className="rounded-3xl border border-border/50 bg-surface/70 p-8 shadow-elevated">
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
      <section className="rounded-3xl border border-border/50 bg-surface/70 p-8 shadow-elevated">
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
      <section className="rounded-3xl border border-border/50 bg-surface/70 p-8 shadow-elevated">
        <div className="mb-6">
          <p className="text-sm uppercase tracking-wide text-subtle">
            {t('sections.gameplay.kicker')}
          </p>
          <h2 className="text-2xl font-semibold text-foreground">
            {t('sections.gameplay.trickDisplayDuration.title')}
          </h2>
          <p className="mt-2 text-sm text-muted">
            {t('sections.gameplay.trickDisplayDuration.description')}
          </p>
        </div>
        <TrickDisplayDurationInput initialValue={trickDisplayDurationSeconds} />
      </section>
    </div>
  )
}
