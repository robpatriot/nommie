'use client'

import { useCallback, useEffect, useRef, useState } from 'react'

import type {
  GameHistorySummary,
  RoundHistoryEntry,
  Seat,
  Trump,
} from '@/lib/game-room/types'
import { isValidSeat } from '@/utils/seat-validation'

interface GameHistoryApiRound {
  round_no: number
  hand_size: number
  dealer_seat: number
  trump_selector_seat: number | null
  trump: Trump | null
  bids: [number | null, number | null, number | null, number | null]
  cumulative_scores: [number, number, number, number]
}

interface GameHistoryApiResponse {
  rounds: GameHistoryApiRound[]
}

interface FetchHistoryOptions {
  force?: boolean
}

export function useGameHistory(gameId?: number) {
  const [history, setHistory] = useState<GameHistorySummary | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const abortRef = useRef<AbortController | null>(null)

  useEffect(() => {
    return () => {
      abortRef.current?.abort()
    }
  }, [])

  useEffect(() => {
    abortRef.current?.abort()
    setHistory(null)
    setError(null)
    setIsLoading(false)
  }, [gameId])

  const fetchHistory = useCallback(
    async (options: FetchHistoryOptions = {}) => {
      if (!gameId) {
        return null
      }

      if (history && !options.force) {
        return history
      }

      abortRef.current?.abort()
      const controller = new AbortController()
      abortRef.current = controller

      setIsLoading(true)
      setError(null)

      try {
        const response = await fetch(`/api/games/${gameId}/history`, {
          method: 'GET',
          credentials: 'include',
          cache: 'no-store',
          signal: controller.signal,
        })

        if (!response.ok) {
          throw new Error('Failed to load score history')
        }

        const payload = (await response.json()) as GameHistoryApiResponse
        const mapped = mapGameHistory(payload)
        setHistory(mapped)
        return mapped
      } catch (err) {
        if (err instanceof DOMException && err.name === 'AbortError') {
          return null
        }
        const message =
          err instanceof Error ? err.message : 'Unable to load score history'
        setError(message)
        return null
      } finally {
        if (abortRef.current === controller) {
          abortRef.current = null
        }
        setIsLoading(false)
      }
    },
    [gameId, history]
  )

  return {
    history,
    isLoading,
    error,
    hasLoaded: Boolean(history),
    fetchHistory,
  }
}

function mapGameHistory(response: GameHistoryApiResponse): GameHistorySummary {
  const rounds = response.rounds.map(mapRound)
  return { rounds }
}

function mapRound(round: GameHistoryApiRound): RoundHistoryEntry {
  return {
    roundNo: round.round_no,
    handSize: round.hand_size,
    dealerSeat: ensureSeat(round.dealer_seat),
    trumpSelectorSeat: toSeatOrNull(round.trump_selector_seat),
    trump: round.trump,
    bids: round.bids,
    cumulativeScores: round.cumulative_scores,
  }
}

function ensureSeat(value: number): Seat {
  if (isValidSeat(value)) {
    return value as Seat
  }
  console.warn(`Invalid seat index from history payload: ${value}`)
  return 0
}

function toSeatOrNull(value: number | null | undefined): Seat | null {
  if (typeof value !== 'number') {
    return null
  }
  return isValidSeat(value) ? (value as Seat) : null
}
