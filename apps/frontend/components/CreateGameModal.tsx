'use client'

import type { FormEvent } from 'react'
import { useState, useEffect, useCallback } from 'react'
import { useTranslations } from 'next-intl'
import { Button } from '@/components/ui/Button'

interface CreateGameModalProps {
  isOpen: boolean
  onClose: () => void
  onCreateGame: (name: string) => Promise<void>
  creatorName: string
}

export default function CreateGameModal({
  isOpen,
  onClose,
  onCreateGame,
  creatorName,
}: CreateGameModalProps) {
  const t = useTranslations('lobby')
  const [name, setName] = useState('')
  const [isSubmitting, setIsSubmitting] = useState(false)
  const defaultName = t('createGame.defaultName', { name: creatorName })

  const handleCancel = useCallback(() => {
    setName('')
    onClose()
  }, [onClose])

  useEffect(() => {
    if (!isOpen) return
    const listener = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault()
        handleCancel()
      }
    }
    window.addEventListener('keydown', listener)
    return () => window.removeEventListener('keydown', listener)
  }, [isOpen, handleCancel])

  if (!isOpen) return null

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault()
    setIsSubmitting(true)

    try {
      await onCreateGame(name.trim())
      // Don't close modal or reset name - navigation will unmount this component
      // Modal will stay visible with loading state until navigation completes
    } catch {
      // Error is already logged by handleCreateGame in LobbyClient
      // Just reset submitting state so user can try again
      setIsSubmitting(false)
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center px-4 py-6"
      role="dialog"
      aria-modal="true"
      aria-labelledby="create-game-title"
    >
      <div
        className="absolute inset-0 bg-overlay/50 backdrop-blur-md"
        onClick={handleCancel}
        aria-hidden
      />
      <div className="relative z-10 w-full max-w-lg rounded-[32px] border border-border/70 bg-card/90 p-6 shadow-elevated">
        <header className="flex items-start justify-between gap-4">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.4em] text-muted-foreground">
              {t('createModal.kicker')}
            </p>
            <h2
              id="create-game-title"
              className="mt-2 text-2xl font-semibold text-foreground"
            >
              {t('createModal.title')}
            </h2>
            <p className="text-sm text-muted-foreground">
              {t('createModal.description', { defaultName })}
            </p>
          </div>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={handleCancel}
            className="rounded-full px-3 py-1 text-sm font-medium text-muted-foreground hover:text-foreground"
            aria-label={t('createModal.closeAria')}
          >
            ✕
          </Button>
        </header>

        <form onSubmit={handleSubmit} className="mt-6 space-y-5">
          <label className="flex flex-col gap-2 text-sm font-medium text-foreground">
            {t('createModal.gameNameLabel')}{' '}
            <span className="text-muted-foreground">
              ({t('createModal.optional')})
            </span>
            <input
              type="text"
              id="game-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={defaultName}
              className="rounded-2xl border border-border/70 bg-background px-4 py-3 text-base text-foreground shadow-inner focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50"
              disabled={isSubmitting}
              autoFocus
            />
          </label>

          <p className="text-xs text-muted-foreground">
            {t('createModal.helper')}
          </p>

          <div className="grid gap-3 sm:grid-cols-2">
            <Button
              type="button"
              variant="outline"
              size="lg"
              onClick={handleCancel}
              disabled={isSubmitting}
              className="w-full text-muted-foreground hover:text-foreground"
            >
              {t('createModal.cancel')}
            </Button>
            <Button
              type="submit"
              variant="primary"
              size="lg"
              disabled={isSubmitting}
              className="w-full"
            >
              {isSubmitting
                ? t('createModal.creating')
                : t('createModal.create')}
            </Button>
          </div>
        </form>
      </div>
    </div>
  )
}
