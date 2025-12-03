import { auth } from '@/auth'
import { redirect } from 'next/navigation'
import { BreadcrumbSetter } from '@/components/header-breadcrumbs'
import { AppearanceSelector } from '@/components/AppearanceSelector'
import { CardConfirmationToggle } from '@/components/CardConfirmationToggle'
import { getUserOptions } from '@/lib/api/user-options'
import { handleAllowlistError } from '@/lib/auth/allowlist'

export default async function SettingsPage() {
  const session = await auth()

  if (!session) {
    redirect('/')
  }

  let requireCardConfirmation = true
  try {
    const options = await getUserOptions()
    requireCardConfirmation = options.require_card_confirmation
  } catch (error) {
    await handleAllowlistError(error)
    // Swallow other errors and fall back to default
  }

  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-6 px-4 py-10">
      <BreadcrumbSetter crumbs={[{ label: 'Settings' }]} />
      <section className="rounded-3xl border border-border/50 bg-surface/70 p-8">
        <div className="mb-6">
          <p className="text-sm uppercase tracking-wide text-subtle">Display</p>
          <h2 className="text-2xl font-semibold text-foreground">Appearance</h2>
          <p className="mt-2 text-sm text-muted">
            Choose how Nommie looks across all devices. The appearance setting
            applies everywhere you&apos;re signed in.
          </p>
        </div>
        <AppearanceSelector />
      </section>
      <section className="rounded-3xl border border-border/50 bg-surface/70 p-8">
        <div className="mb-6">
          <p className="text-sm uppercase tracking-wide text-subtle">
            Gameplay
          </p>
          <h2 className="text-2xl font-semibold text-foreground">
            Card confirmation
          </h2>
          <p className="mt-2 text-sm text-muted">
            Keep the confirmation button if you prefer a two-step action, or
            turn it off to play a legal card immediately when you click it.
          </p>
        </div>
        <CardConfirmationToggle initialEnabled={requireCardConfirmation} />
      </section>
    </div>
  )
}
