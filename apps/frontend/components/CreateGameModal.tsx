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
      await onCreateGame(name.trim() || defaultName)
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
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50">
      <div className="bg-white rounded-lg shadow-xl max-w-md w-full mx-4">
        <div className="p-6">
          <h2 className="text-2xl font-bold text-gray-900 mb-4">
            Create New Game
          </h2>

          <form onSubmit={handleSubmit}>
            <div className="mb-4">
              <label
                htmlFor="game-name"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                Game Name <span className="text-gray-500">(optional)</span>
              </label>
              <input
                type="text"
                id="game-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder={defaultName}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                disabled={isSubmitting}
              />
              <p className="mt-1 text-xs text-gray-500">
                Default: &quot;{defaultName}&quot;
              </p>
            </div>

            <div className="flex justify-end space-x-3">
              <button
                type="button"
                onClick={handleCancel}
                disabled={isSubmitting}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={isSubmitting}
                className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
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
