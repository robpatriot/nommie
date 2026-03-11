import { getTranslations } from 'next-intl/server'
import { BreadcrumbSetter } from '@/components/header-breadcrumbs'

export default async function AdminUsersPage() {
  const t = await getTranslations('admin.users')

  return (
    <>
      <BreadcrumbSetter crumbs={[{ label: t('title') }]} />
      <section className="rounded-3xl border border-border/50 bg-card/70 p-8 shadow-elevated">
        <h2 className="mb-6 text-2xl font-semibold text-foreground">
          {t('title')}
        </h2>
        <p className="text-muted-foreground">{t('comingSoon')}</p>
      </section>
    </>
  )
}
