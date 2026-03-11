'use client'

import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { useTranslations } from 'next-intl'

/**
 * Admin section navigation. Extensible for future routes (e.g. /admin/system, /admin/audit).
 */
export default function AdminNav() {
  const t = useTranslations('admin.nav')
  const pathname = usePathname()
  const isOnUsers = pathname === '/admin/users'

  return (
    <nav
      className="mb-6 flex gap-2 border-b border-border/60 pb-4"
      aria-label="Admin sections"
    >
      <Link
        href="/admin/users"
        className={`rounded-xl px-4 py-2 text-sm font-medium transition ${
          isOnUsers
            ? 'bg-primary/15 text-primary'
            : 'text-muted-foreground hover:bg-muted/50 hover:text-foreground'
        }`}
      >
        {t('users')}
      </Link>
    </nav>
  )
}
