import { auth } from '@/auth'
import { redirect } from 'next/navigation'
import { getMe } from '@/lib/api/user-me'
import AdminNav from './AdminNav'

export default async function AdminLayout({
  children,
}: {
  children: React.ReactNode
}) {
  const session = await auth()

  if (!session) {
    redirect('/')
  }

  const me = await getMe()
  if (!me || me.role !== 'admin') {
    redirect('/')
  }

  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-6 px-4 py-10">
      <AdminNav />
      {children}
    </div>
  )
}
