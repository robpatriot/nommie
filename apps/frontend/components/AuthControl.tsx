'use client'

import { useTranslations } from 'next-intl'

interface AuthControlProps {
  state: 'unauthenticated' | 'authenticated' | 'loading'
  onLogin?: () => void
  onLogout?: () => void
}

export default function AuthControl({
  state,
  onLogin,
  onLogout,
}: AuthControlProps) {
  const t = useTranslations('nav')

  if (state === 'loading') {
    return (
      <div role="status" aria-label={t('auth.statusAria')}>
        <button disabled aria-label={t('auth.loadingAria')}>
          {t('auth.loading')}
        </button>
      </div>
    )
  }

  if (state === 'authenticated') {
    return (
      <div role="status" aria-label={t('auth.statusAria')}>
        <button onClick={onLogout} aria-label={t('auth.signOutAria')}>
          {t('auth.signOut')}
        </button>
      </div>
    )
  }

  // unauthenticated state
  return (
    <div role="status" aria-label={t('auth.statusAria')}>
      <button onClick={onLogin} aria-label={t('auth.signInAria')}>
        {t('auth.signIn')}
      </button>
    </div>
  )
}
