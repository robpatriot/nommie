'use client'

import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslations } from 'next-intl'
import {
  searchAdminUsersAction,
  grantAdminAction,
  revokeAdminAction,
} from '@/app/actions/admin-user-actions'
import type { AdminUserSummary } from '@/lib/api/admin-users'
import { useToast } from '@/hooks/useToast'
import Toast from '@/components/Toast'

const DEBOUNCE_MS = 300

type AdminUsersClientProps = {
  currentUserId?: number | null
}

export default function AdminUsersClient({
  currentUserId = null,
}: AdminUsersClientProps) {
  const t = useTranslations('admin.users')
  const { toasts, showToast, hideToast } = useToast()
  const [query, setQuery] = useState('')
  const [debouncedQuery, setDebouncedQuery] = useState('')
  const [items, setItems] = useState<AdminUserSummary[]>([])
  const [nextCursor, setNextCursor] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [loadingMore, setLoadingMore] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const search = useCallback(async (q: string, cursor?: string | null) => {
    const trimmed = q.trim()
    if (!trimmed) {
      setItems([])
      setNextCursor(null)
      setError(null)
      return
    }

    const isLoadMore = !!cursor
    if (isLoadMore) {
      setLoadingMore(true)
    } else {
      setLoading(true)
    }
    setError(null)

    const result = await searchAdminUsersAction({
      q: trimmed,
      limit: 20,
      cursor: cursor ?? undefined,
    })

    if (isLoadMore) {
      setLoadingMore(false)
    } else {
      setLoading(false)
    }

    if (result.kind === 'ok') {
      if (cursor) {
        setItems((prev) => [...prev, ...result.data.items])
      } else {
        setItems(result.data.items)
      }
      setNextCursor(result.data.next_cursor)
    } else {
      setError(result.message)
      if (!cursor) setItems([])
    }
  }, [])

  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current)
    debounceRef.current = setTimeout(() => {
      setDebouncedQuery(query)
      debounceRef.current = null
    }, DEBOUNCE_MS)
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current)
    }
  }, [query])

  useEffect(() => {
    if (!debouncedQuery.trim()) return
    void Promise.resolve().then(() => search(debouncedQuery))
  }, [debouncedQuery, search])

  const handleLoadMore = useCallback(() => {
    if (nextCursor && debouncedQuery.trim()) {
      search(debouncedQuery, nextCursor)
    }
  }, [nextCursor, debouncedQuery, search])

  const handleGrantAdmin = useCallback(
    async (userId: number) => {
      const result = await grantAdminAction(userId)
      if (result.kind === 'ok') {
        setItems((prev) =>
          prev.map((u) =>
            u.id === userId ? { ...u, role: 'admin' as const } : u
          )
        )
        showToast(
          result.data.changed ? t('adminGranted') : t('alreadyAdmin'),
          'success'
        )
      } else {
        showToast(result.message, 'error')
      }
    },
    [showToast, t]
  )

  const handleRevokeAdmin = useCallback(
    async (userId: number) => {
      const result = await revokeAdminAction(userId)
      if (result.kind === 'ok') {
        setItems((prev) =>
          prev.map((u) =>
            u.id === userId ? { ...u, role: 'user' as const } : u
          )
        )
        showToast(
          result.data.changed ? t('adminRevoked') : t('alreadyUser'),
          'success'
        )
      } else {
        const msg =
          result.code === 'LAST_ADMIN_PROTECTION'
            ? t('lastAdminProtection')
            : result.code === 'CANNOT_REVOKE_OWN_ADMIN'
              ? t('cannotRevokeOwn')
              : result.message
        showToast(msg, 'error')
      }
    },
    [showToast, t]
  )

  const myUserId = currentUserId ?? null
  const trimmedQuery = query.trim()
  const canSearch = trimmedQuery.length > 0
  const hasActiveQuery = debouncedQuery.trim().length > 0
  const displayItems = hasActiveQuery ? items : []
  const displayError = hasActiveQuery ? error : null
  const displayNextCursor = hasActiveQuery ? nextCursor : null

  return (
    <section className="rounded-3xl border border-border/50 bg-card/70 p-8 shadow-elevated">
      <h2 className="mb-6 text-2xl font-semibold text-foreground">
        {t('title')}
      </h2>

      <div className="mb-6 flex gap-2">
        <input
          type="search"
          placeholder={t('searchPlaceholder')}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          className="flex-1 rounded-2xl border border-border/70 bg-card px-4 py-2 text-foreground placeholder:text-muted-foreground focus:border-primary/50 focus:outline-none focus:ring-2 focus:ring-primary/20"
          aria-label={t('searchPlaceholder')}
        />
      </div>

      {displayError && (
        <p className="mb-4 text-sm text-destructive" role="alert">
          {displayError}
        </p>
      )}

      {loading ? (
        <p className="text-muted-foreground">{t('loading')}</p>
      ) : !canSearch ? (
        <p className="text-muted-foreground">{t('searchPlaceholder')}</p>
      ) : displayItems.length === 0 ? (
        <p className="text-muted-foreground">{t('noUsers')}</p>
      ) : (
        <>
          <div className="overflow-x-auto">
            <table className="w-full text-left text-sm">
              <thead>
                <tr className="border-b border-border/60">
                  <th className="py-3 pr-4 font-semibold">{t('id')}</th>
                  <th className="py-3 pr-4 font-semibold">
                    {t('displayName')}
                  </th>
                  <th className="py-3 pr-4 font-semibold">{t('email')}</th>
                  <th className="py-3 pr-4 font-semibold">{t('role')}</th>
                  <th className="py-3 font-semibold">{t('actions')}</th>
                </tr>
              </thead>
              <tbody>
                {displayItems.map((user) => (
                  <tr
                    key={user.id}
                    className="border-b border-border/40 last:border-0"
                  >
                    <td className="py-3 pr-4">{user.id}</td>
                    <td className="py-3 pr-4">
                      {user.display_name ?? user.email ?? '—'}
                    </td>
                    <td className="py-3 pr-4">{user.email ?? '—'}</td>
                    <td className="py-3 pr-4 capitalize">{user.role}</td>
                    <td className="py-3">
                      <div className="flex gap-2">
                        {user.role === 'user' ? (
                          <button
                            type="button"
                            onClick={() => handleGrantAdmin(user.id)}
                            className="rounded-xl border border-primary/50 px-3 py-1.5 text-sm font-medium text-primary hover:bg-primary/10"
                          >
                            {t('grantAdmin')}
                          </button>
                        ) : (
                          <button
                            type="button"
                            onClick={() => {
                              if (
                                window.confirm(
                                  `${t('revokeConfirmTitle')} ${t('revokeConfirmMessage')}`
                                )
                              ) {
                                handleRevokeAdmin(user.id)
                              }
                            }}
                            disabled={myUserId === user.id}
                            className="rounded-xl border border-destructive/50 px-3 py-1.5 text-sm font-medium text-destructive hover:bg-destructive/10 disabled:opacity-50 disabled:cursor-not-allowed"
                            title={
                              myUserId === user.id
                                ? t('cannotRevokeOwn')
                                : undefined
                            }
                          >
                            {t('revokeAdmin')}
                          </button>
                        )}
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {displayNextCursor && (
            <div className="mt-4">
              <button
                type="button"
                onClick={handleLoadMore}
                disabled={loadingMore}
                className="rounded-xl border border-border/70 bg-card px-4 py-2 text-sm font-medium text-foreground hover:bg-muted/50 disabled:opacity-50"
              >
                {loadingMore ? t('loading') : t('loadMore')}
              </button>
            </div>
          )}
        </>
      )}
      <Toast toasts={toasts} onClose={hideToast} />
    </section>
  )
}
