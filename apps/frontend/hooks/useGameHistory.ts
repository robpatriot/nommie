'use client'

import { useCallback, useEffect, useState } from 'react'

import type {
  GameHistorySummary,
  RoundHistoryEntry,
  Seat,
  Trump,
} from '@/lib/game-room/types'
import { isValidSeat } from '@/utils/seat-validation'
import {
  getGameHistoryAction,
  type GameHistoryApiResponse,
  type GameHistoryApiRound,
} from '@/app/actions/game-actions'

interface FetchHistoryOptions {
  force?: boolean
}

export function useGameHistory(gameId?: number) {
  const [history, setHistory] = useState<GameHistorySummary | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(false)

  useEffect(() => {
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

      setIsLoading(true)
      setError(null)

      try {
        const result = await getGameHistoryAction(gameId)

        if (result.kind === 'error') {
          throw new Error(result.message)
        }

        const mapped = mapGameHistory(result.data)
        setHistory(mapped)
        return mapped
      } catch (err) {
        const message =
          err instanceof Error ? err.message : 'Unable to load score history'
        setError(message)
        return null
      } finally {
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
    trump: round.trump as Trump | null,
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
