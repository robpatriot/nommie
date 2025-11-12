'use client'

import { useState, FormEvent } from 'react'

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

  if (!isOpen) return null

  const defaultName = `${creatorName} game`

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault()
    setIsSubmitting(true)

    try {
      // Send trimmed name (empty string will be handled by parent to use default)
      await onCreateGame(name.trim())
      // Reset form
      setName('')
      onClose()
    } catch (error) {
      // Error handling is done in parent component via toast
      console.error('Failed to create game:', error)
    } finally {
      setIsSubmitting(false)
    }
  }

  const handleCancel = () => {
    setName('')
    onClose()
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-foreground/20 backdrop-blur-sm">
      <div className="mx-4 w-full max-w-md rounded-lg border border-border bg-surface-strong shadow-elevated">
        <div className="p-6">
          <h2 className="mb-4 text-2xl font-bold text-foreground">
            Create New Game
          </h2>

          <form onSubmit={handleSubmit}>
            <div className="mb-4">
              <label
                htmlFor="game-name"
                className="mb-1 block text-sm font-medium text-foreground"
              >
                Game Name <span className="text-muted">(optional)</span>
              </label>
              <input
                type="text"
                id="game-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder={defaultName}
                className="w-full rounded-md border border-border bg-background px-3 py-2 text-foreground shadow-inner focus:outline-none focus:ring-2 focus:ring-primary"
                disabled={isSubmitting}
              />
              <p className="mt-1 text-xs text-subtle">
                Default: &quot;{defaultName}&quot;
              </p>
            </div>

            <div className="flex justify-end space-x-3">
              <button
                type="button"
                onClick={handleCancel}
                disabled={isSubmitting}
                className="rounded-md bg-surface px-4 py-2 text-sm font-medium text-muted transition-colors hover:bg-surface-strong hover:text-foreground disabled:cursor-not-allowed disabled:opacity-50"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={isSubmitting}
                className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {isSubmitting ? 'Creating...' : 'Create Game'}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  )
}
