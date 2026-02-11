/**
 * Phase 1 game-over stats computed from round history.
 * All derived from existing history API; no backend changes.
 */

import type { RoundHistoryEntry, Seat, Trump } from '@/lib/game-room/types'

export interface PlayerStats {
  seat: Seat
  totalTricks: number
  bidAccuracy: number
  bidAccuracyDenominator: number
  roundsWon: number
  biggestRound: number
  perfectRounds: number
  finalScore: number
}

export interface GameOverStats {
  mostCommonTrump: Trump | null
  mostBidWins: Seat | null
  mostBidWinsCount: number
  playerStats: [PlayerStats, PlayerStats, PlayerStats, PlayerStats]
  totalRounds: number
}

function getRoundScore(
  round: RoundHistoryEntry,
  prevRound: RoundHistoryEntry | null,
  seatIndex: number
): number {
  const curr = round.cumulativeScores[seatIndex]
  const prev = prevRound?.cumulativeScores[seatIndex] ?? 0
  return Math.max(0, curr - prev)
}

function getTricksFromRoundScore(
  roundScore: number,
  bid: number | null
): number {
  if (bid === null) return roundScore
  if (roundScore >= 10 && roundScore - 10 === bid) return bid
  return roundScore
}

function isBidMet(roundScore: number, bid: number | null): boolean {
  if (bid === null) return false
  const tricks = getTricksFromRoundScore(roundScore, bid)
  return tricks === bid
}

function isPerfectRound(roundScore: number, bid: number | null): boolean {
  if (bid === null) return false
  const tricks = getTricksFromRoundScore(roundScore, bid)
  return bid === 0 ? tricks === 0 : tricks === bid
}

function isRoundWinner(
  round: RoundHistoryEntry,
  prevRound: RoundHistoryEntry | null,
  seatIndex: number
): boolean {
  const scores = [0, 1, 2, 3].map((s) => getRoundScore(round, prevRound, s))
  const myScore = scores[seatIndex]
  const maxScore = Math.max(...scores)
  return myScore === maxScore && myScore > 0
}

export function computeGameOverStats(
  rounds: RoundHistoryEntry[]
): GameOverStats | null {
  if (rounds.length === 0) return null

  const trumpCounts = new Map<string, number>()
  const bidWinCounts: [number, number, number, number] = [0, 0, 0, 0] as const

  const playerStats: PlayerStats[] = [
    {
      seat: 0,
      totalTricks: 0,
      bidAccuracy: 0,
      bidAccuracyDenominator: 0,
      roundsWon: 0,
      biggestRound: 0,
      perfectRounds: 0,
      finalScore: 0,
    },
    {
      seat: 1,
      totalTricks: 0,
      bidAccuracy: 0,
      bidAccuracyDenominator: 0,
      roundsWon: 0,
      biggestRound: 0,
      perfectRounds: 0,
      finalScore: 0,
    },
    {
      seat: 2,
      totalTricks: 0,
      bidAccuracy: 0,
      bidAccuracyDenominator: 0,
      roundsWon: 0,
      biggestRound: 0,
      perfectRounds: 0,
      finalScore: 0,
    },
    {
      seat: 3,
      totalTricks: 0,
      bidAccuracy: 0,
      bidAccuracyDenominator: 0,
      roundsWon: 0,
      biggestRound: 0,
      perfectRounds: 0,
      finalScore: 0,
    },
  ]

  let prevRound: RoundHistoryEntry | null = null

  for (const round of rounds) {
    if (round.trump) {
      const key = round.trump
      trumpCounts.set(key, (trumpCounts.get(key) ?? 0) + 1)
    }

    if (round.trumpSelectorSeat !== null) {
      const seat = round.trumpSelectorSeat
      if (seat >= 0 && seat <= 3) {
        bidWinCounts[seat] += 1
      }
    }

    for (let seatIndex = 0; seatIndex < 4; seatIndex++) {
      const roundScore = getRoundScore(round, prevRound, seatIndex)
      const bid = round.bids[seatIndex]

      const tricks = getTricksFromRoundScore(roundScore, bid)
      playerStats[seatIndex].totalTricks += tricks

      if (bid !== null) {
        playerStats[seatIndex].bidAccuracyDenominator += 1
        if (isBidMet(roundScore, bid)) {
          playerStats[seatIndex].bidAccuracy += 1
        }
      }

      if (isRoundWinner(round, prevRound, seatIndex)) {
        playerStats[seatIndex].roundsWon += 1
      }

      if (roundScore > playerStats[seatIndex].biggestRound) {
        playerStats[seatIndex].biggestRound = roundScore
      }

      if (isPerfectRound(roundScore, bid)) {
        playerStats[seatIndex].perfectRounds += 1
      }

      playerStats[seatIndex].finalScore = round.cumulativeScores[seatIndex]
    }

    prevRound = round
  }

  let mostCommonTrump: Trump | null = null
  let maxTrumpCount = 0
  for (const [trump, count] of trumpCounts) {
    if (count > maxTrumpCount) {
      maxTrumpCount = count
      mostCommonTrump = trump as Trump
    }
  }

  let mostBidWins: Seat | null = null
  let mostBidWinsCount = 0
  for (let s = 0; s < 4; s++) {
    if (bidWinCounts[s] > mostBidWinsCount) {
      mostBidWinsCount = bidWinCounts[s]
      mostBidWins = s as Seat
    }
  }

  return {
    mostCommonTrump,
    mostBidWins,
    mostBidWinsCount,
    playerStats: playerStats as [
      PlayerStats,
      PlayerStats,
      PlayerStats,
      PlayerStats,
    ],
    totalRounds: rounds.length,
  }
}
