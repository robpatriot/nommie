'use client'

import { useState, useEffect, useCallback, FormEvent } from 'react'

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
  const [name, setName] = useState('')
  const [isSubmitting, setIsSubmitting] = useState(false)
  const defaultName = `${creatorName}'s game`

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
      setName('')
      onClose()
    } catch (error) {
      const { logError } = await import('@/lib/logging/error-logger')
      logError('Failed to create game', error, { action: 'createGame' })
    } finally {
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
        className="absolute inset-0 bg-black/50 backdrop-blur-md"
        onClick={handleCancel}
        aria-hidden
      />
      <div className="relative z-10 w-full max-w-lg rounded-[32px] border border-white/20 bg-surface/90 p-6 shadow-[0_30px_120px_rgba(0,0,0,0.4)]">
        <header className="flex items-start justify-between gap-4">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.4em] text-subtle">
              New Table
            </p>
            <h2
              id="create-game-title"
              className="mt-2 text-2xl font-semibold text-foreground"
            >
              Name your game
            </h2>
            <p className="text-sm text-muted">
              Keep it casual. We will fall back to &quot;{defaultName}&quot; if
              you leave it blank.
            </p>
          </div>
          <button
            type="button"
            onClick={handleCancel}
            className="rounded-full border border-border/60 bg-surface px-3 py-1 text-sm font-medium text-subtle transition hover:text-foreground"
            aria-label="Close create game modal"
          >
            ✕
          </button>
        </header>

        <form onSubmit={handleSubmit} className="mt-6 space-y-5">
          <label className="flex flex-col gap-2 text-sm font-medium text-foreground">
            Game name <span className="text-muted">(optional)</span>
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

          <p className="text-xs text-subtle">
            Name the table so your group knows where to sit; everything else
            stays steady.
          </p>

          <div className="grid gap-3 sm:grid-cols-2">
            <button
              type="button"
              onClick={handleCancel}
              disabled={isSubmitting}
              className="rounded-2xl border border-border/70 bg-surface px-4 py-3 text-sm font-semibold text-muted transition hover:border-primary/50 hover:text-foreground disabled:cursor-not-allowed disabled:opacity-50"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={isSubmitting}
              className="rounded-2xl bg-primary px-4 py-3 text-sm font-semibold text-primary-foreground shadow-lg shadow-primary/30 transition hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-50"
            >
              {isSubmitting ? 'Creating…' : 'Create game'}
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}
