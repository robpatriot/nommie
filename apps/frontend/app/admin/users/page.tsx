import { getTranslations } from 'next-intl/server'
import { BreadcrumbSetter } from '@/components/header-breadcrumbs'
import { getMe } from '@/lib/api/user-me'
import AdminUsersClient from './AdminUsersClient'

export default async function AdminUsersPage() {
  const t = await getTranslations('admin.users')
  const me = await getMe()

  return (
    <>
      <BreadcrumbSetter crumbs={[{ label: t('title') }]} />
      <AdminUsersClient currentUserId={me?.id} />
    </>
  )
}
