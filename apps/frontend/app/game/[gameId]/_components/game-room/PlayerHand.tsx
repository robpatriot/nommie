'use client'

import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from 'react'
import { useTranslations } from 'next-intl'
import type { Card, PhaseSnapshot, Seat } from '@/lib/game-room/types'
import type { GameRoomViewProps } from '../game-room-view'
import { cn } from '@/lib/cn'
import { PlayingCard, CARD_DIMENSIONS } from './PlayingCard'
import { useMediaQuery } from '@/hooks/useMediaQuery'
import { isTrickPhase as checkIsTrickPhase } from './phase-helpers'

export type LayoutVariant = 'default' | 'scaled'

interface PlayerHandProps {
  viewerHand: Card[]
  phase: PhaseSnapshot
  playerNames: [string, string, string, string]
  viewerSeat: Seat
  playState?: GameRoomViewProps['playState']
  selectedCard: Card | null
  onSelectCard: (card: Card | null) => void
  onPlayCard?: (card: Card) => Promise<void> | void
  className?: string
  requireCardConfirmation?: boolean
  layoutVariant?: LayoutVariant
  viewportRef?: React.RefObject<HTMLDivElement | null>
}

const CARD_WIDTH = CARD_DIMENSIONS.md.width
const CARD_HEIGHT = CARD_DIMENSIONS.md.height
const CARD_PADDING = 8 // padding + border
const EFFECTIVE_CARD_WIDTH = CARD_WIDTH + CARD_PADDING

const MIN_OVERLAP = 4
// Allow more aggressive overlap: keep only a very small portion of the top-left corner visible
const MAX_OVERLAP = CARD_WIDTH - 17
const AESTHETIC_OVERLAP = 8
const BLEND_THRESHOLD = 20 // Pixels of remaining space before we start blending
// Second row overlaps top row by 40% of card height
// Increased to account for card shadows that create visual gap
const ROW_OVERLAP = CARD_HEIGHT * 0.4
const SELECTED_CARD_LIFT = 8 // How much selected card lifts (translateY)
const MAX_CARDS_PER_ROW = 7 // Maximum cards allowed in a single row
const OVERLAP_TOLERANCE = 0.1 // Tolerance for comparing overlap values (in pixels)

type LayoutMode = 'singleRow' | 'twoRow'

interface CardPosition {
  left: number
  top: number
  zIndex: number
}

interface LayoutResult {
  mode: LayoutMode
  positions: CardPosition[]
  minHeight: number
  scale?: number // Optional scale factor for card scaling
  twoRowSplit?: TwoRowSplit // The split used for two-row layouts (to avoid recomputation)
  topOverlap?: number // Overlap value for top row (to avoid recomputation)
  bottomOverlap?: number // Overlap value for bottom row (to avoid recomputation)
}

// Suit-aware layout types
interface SuitGroup {
  suit: string
  cards: Card[]
  count: number
}

interface TwoRowSplit {
  topRow: Card[]
  bottomRow: Card[]
  splitSuit: string | null
  splitTopCount: number | null
}

interface LayoutSignature {
  handHash: string
  topRowSuits: { suit: string; count: number }[]
  bottomRowSuits: { suit: string; count: number }[]
  splitSuit: string | null
  splitTopCount: number | null
}

// Export types for testing
export type { LayoutSignature, TwoRowSplit, SuitGroup }

// Calculate overlap for a single row given viewport width and card count
function calculateOverlapForRow(
  viewportWidth: number,
  cardCount: number
): number {
  if (cardCount <= 1) {
    return 0
  }

  const totalWidthNeeded = EFFECTIVE_CARD_WIDTH * cardCount
  const spaceRemaining = viewportWidth - totalWidthNeeded

  if (spaceRemaining >= BLEND_THRESHOLD) {
    // Plenty of space: use aesthetic overlap
    return AESTHETIC_OVERLAP
  }

  if (spaceRemaining > 0) {
    // Tight but fits: blend between aesthetic and calculated
    const overlapNeeded = (totalWidthNeeded - viewportWidth) / (cardCount - 1)
    const calculatedOverlap = Math.max(
      MIN_OVERLAP,
      Math.min(MAX_OVERLAP, overlapNeeded)
    )
    const blend = 1 - spaceRemaining / BLEND_THRESHOLD
    return AESTHETIC_OVERLAP + (calculatedOverlap - AESTHETIC_OVERLAP) * blend
  }

  // Doesn't fit: calculate required overlap
  const overlapNeeded = (totalWidthNeeded - viewportWidth) / (cardCount - 1)
  return Math.max(MIN_OVERLAP, Math.min(MAX_OVERLAP, overlapNeeded))
}

// Calculate the minimum width needed for a row to reach max overlap
function calculateMinWidthForMaxOverlap(cardCount: number): number {
  if (cardCount <= 1) {
    return EFFECTIVE_CARD_WIDTH
  }
  return (
    EFFECTIVE_CARD_WIDTH +
    (cardCount - 1) * (EFFECTIVE_CARD_WIDTH - MAX_OVERLAP)
  )
}

// Check if cards fit in one row with acceptable overlap
function canFitInOneRow(viewportWidth: number, cardCount: number): boolean {
  if (cardCount <= 1) {
    return true
  }
  // If viewport is below absolute minimum, we need two rows
  return viewportWidth >= calculateMinWidthForMaxOverlap(cardCount)
}

// Suit-aware layout helper functions
// Cache for card suit extraction to avoid repeated string operations
const cardSuitCache = new Map<Card, string>()

function getCardSuit(card: Card): string {
  let suit = cardSuitCache.get(card)
  if (suit === undefined) {
    suit = card.slice(-1).toUpperCase()
    cardSuitCache.set(card, suit)
  }
  return suit
}

// Clear suit cache when hand changes significantly (called from component)
export function clearCardSuitCache(): void {
  cardSuitCache.clear()
}

function computeHandHash(cards: Card[]): string {
  // Create deterministic hash from sorted card list
  return [...cards].sort().join(',')
}

function groupCardsBySuit(cards: Card[]): SuitGroup[] {
  const groups = new Map<string, Card[]>()

  // Group cards by suit, preserving original order within each suit
  for (const card of cards) {
    const suit = getCardSuit(card)
    if (!groups.has(suit)) {
      groups.set(suit, [])
    }
    groups.get(suit)!.push(card)
  }

  // Convert to array format
  return Array.from(groups.entries()).map(([suit, cards]) => ({
    suit,
    cards,
    count: cards.length,
  }))
}

function scoreCombination(combo: TwoRowSplit): number {
  let score = 0

  // Priority 1: Prefer balanced rows (same size)
  const rowDiff = Math.abs(combo.topRow.length - combo.bottomRow.length)
  score += 1000 - rowDiff * 100 // Higher score for smaller difference

  // Tie-breaker: Among equally balanced options, prefer keeping more suits together
  // Count how many suits are split across rows (fewer is better)
  if (rowDiff === 0) {
    const topSuits = new Set(combo.topRow.map((c) => getCardSuit(c)))
    const bottomSuits = new Set(combo.bottomRow.map((c) => getCardSuit(c)))
    const splitSuits = Array.from(topSuits).filter((suit) =>
      bottomSuits.has(suit)
    )
    score += 20 - splitSuits.length * 5 // Bonus for fewer split suits
  }

  return score
}

function generateValidCombinations(
  suitGroups: SuitGroup[],
  maxCardsPerRow: number
): TwoRowSplit[] {
  const combinations: TwoRowSplit[] = []
  const n = suitGroups.length

  // Generate all possible ways to assign suits to rows (2^n combinations)
  for (let i = 0; i < Math.pow(2, n); i++) {
    const topRow: Card[] = []
    const bottomRow: Card[] = []

    for (let j = 0; j < n; j++) {
      const isTopRow = (i & (1 << j)) !== 0
      const suitGroup = suitGroups[j]

      if (isTopRow) {
        topRow.push(...suitGroup.cards)
      } else {
        bottomRow.push(...suitGroup.cards)
      }
    }

    // Check constraints: no row exceeds maxCardsPerRow
    if (topRow.length <= maxCardsPerRow && bottomRow.length <= maxCardsPerRow) {
      combinations.push({
        topRow,
        bottomRow,
        splitSuit: null,
        splitTopCount: null,
      })
    }
  }

  return combinations
}

function computePerfectLayout(suitGroups: SuitGroup[]): TwoRowSplit | null {
  const validCombinations = generateValidCombinations(
    suitGroups,
    MAX_CARDS_PER_ROW
  )

  if (validCombinations.length === 0) {
    return null
  }

  // Score each combination
  let bestScore = -Infinity
  let bestSolution: TwoRowSplit | null = null

  for (const combo of validCombinations) {
    const score = scoreCombination(combo)
    if (score > bestScore) {
      bestScore = score
      bestSolution = combo
    }
  }

  return bestSolution
}

function computeBestSplitLayout(
  suitGroups: SuitGroup[],
  suitToSplit: SuitGroup
): TwoRowSplit {
  const otherSuits = suitGroups.filter((sg) => sg.suit !== suitToSplit.suit)
  let bestScore = -Infinity
  let bestSolution: TwoRowSplit | null = null

  // Try all ways to split the suit that must be split
  for (let topCount = 1; topCount < suitToSplit.count; topCount++) {
    const bottomCount = suitToSplit.count - topCount

    // Check if this split is valid (both parts fit in rows)
    if (topCount > MAX_CARDS_PER_ROW || bottomCount > MAX_CARDS_PER_ROW) {
      continue
    }

    // Try all ways to assign other suits
    const otherCombinations = generateValidCombinations(
      otherSuits,
      MAX_CARDS_PER_ROW
    )

    for (const otherCombo of otherCombinations) {
      const topRow = [
        ...suitToSplit.cards.slice(0, topCount),
        ...otherCombo.topRow,
      ]
      const bottomRow = [
        ...suitToSplit.cards.slice(topCount),
        ...otherCombo.bottomRow,
      ]

      // Check constraints
      if (
        topRow.length <= MAX_CARDS_PER_ROW &&
        bottomRow.length <= MAX_CARDS_PER_ROW
      ) {
        const combo: TwoRowSplit = {
          topRow,
          bottomRow,
          splitSuit: suitToSplit.suit,
          splitTopCount: topCount,
        }

        const score = scoreCombination(combo)
        if (score > bestScore) {
          bestScore = score
          bestSolution = combo
        }
      }
    }
  }

  // Fallback: if no valid split found, use balanced split
  if (!bestSolution) {
    const topCount = Math.ceil(suitToSplit.count / 2)
    const topRow = suitToSplit.cards.slice(0, topCount)
    const bottomRow = suitToSplit.cards.slice(topCount)

    // Distribute other suits
    const otherTop: Card[] = []
    const otherBottom: Card[] = []

    for (const otherSuit of otherSuits) {
      if (otherTop.length + topRow.length <= MAX_CARDS_PER_ROW) {
        otherTop.push(...otherSuit.cards)
      } else {
        otherBottom.push(...otherSuit.cards)
      }
    }

    return {
      topRow: [...topRow, ...otherTop],
      bottomRow: [...bottomRow, ...otherBottom],
      splitSuit: suitToSplit.suit,
      splitTopCount: topCount,
    }
  }

  return bestSolution
}

function computeOptimalLayout(
  cards: Card[],
  suitGroups?: SuitGroup[]
): TwoRowSplit {
  // Accept pre-computed suit groups to avoid recomputation
  const groups = suitGroups ?? groupCardsBySuit(cards)

  // Check if any suit > 7 (must be split)
  const suitOver7 = groups.find((sg) => sg.count > MAX_CARDS_PER_ROW)

  if (suitOver7) {
    // Must split this suit
    return computeBestSplitLayout(groups, suitOver7)
  }

  // Try perfect layout (all suits together)
  const perfect = computePerfectLayout(groups)
  if (perfect) {
    return perfect
  }

  // Fallback: balanced split (shouldn't happen with max 13 cards, but safety)
  const topCount = Math.ceil(cards.length / 2)
  return {
    topRow: cards.slice(0, topCount),
    bottomRow: cards.slice(topCount),
    splitSuit: null,
    splitTopCount: null,
  }
}

// Export for testing - helper to create signatures for test fixtures
export function createSignature(
  cards: Card[],
  split: TwoRowSplit
): LayoutSignature {
  const topRowSuits = groupCardsBySuit(split.topRow).map((sg) => ({
    suit: sg.suit,
    count: sg.count,
  }))
  const bottomRowSuits = groupCardsBySuit(split.bottomRow).map((sg) => ({
    suit: sg.suit,
    count: sg.count,
  }))

  return {
    handHash: computeHandHash(cards),
    topRowSuits,
    bottomRowSuits,
    splitSuit: split.splitSuit,
    splitTopCount: split.splitTopCount,
  }
}

function maintainPattern(
  cards: Card[],
  previous: LayoutSignature,
  suitGroups?: SuitGroup[]
): TwoRowSplit {
  // Accept pre-computed suit groups to avoid recomputation
  const groups = suitGroups ?? groupCardsBySuit(cards)
  const topRow: Card[] = []
  const bottomRow: Card[] = []

  // Assign suits to rows based on previous pattern
  for (const suitGroup of groups) {
    const wasOnTop = previous.topRowSuits.some((s) => s.suit === suitGroup.suit)

    if (wasOnTop) {
      topRow.push(...suitGroup.cards)
    } else {
      bottomRow.push(...suitGroup.cards)
    }
  }

  // Verify constraints (should always pass since pattern is maintainable)
  if (
    topRow.length > MAX_CARDS_PER_ROW ||
    bottomRow.length > MAX_CARDS_PER_ROW
  ) {
    // Fallback to optimal if somehow invalid
    return computeOptimalLayout(cards, groups)
  }

  return {
    topRow,
    bottomRow,
    splitSuit: null,
    splitTopCount: null,
  }
}

function maintainSplitPattern(
  cards: Card[],
  previous: LayoutSignature,
  suitGroups?: SuitGroup[]
): TwoRowSplit {
  // Accept pre-computed suit groups to avoid recomputation
  const groups = suitGroups ?? groupCardsBySuit(cards)
  const topRow: Card[] = []
  const bottomRow: Card[] = []

  // Assign suits based on previous pattern
  for (const suitGroup of groups) {
    if (
      suitGroup.suit === previous.splitSuit &&
      previous.splitTopCount !== null
    ) {
      // Split this suit the same way
      const splitPoint = previous.splitTopCount
      topRow.push(...suitGroup.cards.slice(0, splitPoint))
      bottomRow.push(...suitGroup.cards.slice(splitPoint))
    } else {
      // Keep suit together based on previous pattern
      const wasOnTop = previous.topRowSuits.some(
        (s) => s.suit === suitGroup.suit
      )
      if (wasOnTop) {
        topRow.push(...suitGroup.cards)
      } else {
        bottomRow.push(...suitGroup.cards)
      }
    }
  }

  // Verify constraints (should always pass)
  if (
    topRow.length > MAX_CARDS_PER_ROW ||
    bottomRow.length > MAX_CARDS_PER_ROW
  ) {
    // Fallback to optimal if somehow invalid
    return computeOptimalLayout(cards, groups)
  }

  return {
    topRow,
    bottomRow,
    splitSuit: previous.splitSuit,
    splitTopCount: previous.splitTopCount,
  }
}

// Export for testing - main layout algorithm entry point
export function getTwoRowLayout(
  cards: Card[],
  previousSignature: LayoutSignature | null,
  suitGroups?: SuitGroup[]
): TwoRowSplit {
  const currentHash = computeHandHash(cards)

  // Hand unchanged - use previous layout exactly
  if (previousSignature && previousSignature.handHash === currentHash) {
    // Rebuild from signature
    if (previousSignature.splitSuit === null) {
      return maintainPattern(cards, previousSignature, suitGroups)
    } else {
      return maintainSplitPattern(cards, previousSignature, suitGroups)
    }
  }

  // Compute optimal for current hand (pass suit groups if provided)
  const optimal = computeOptimalLayout(cards, suitGroups)

  // No previous state - use optimal
  if (!previousSignature) {
    return optimal
  }

  // Previous was perfect - maintain pattern
  if (previousSignature.splitSuit === null) {
    return maintainPattern(cards, previousSignature, suitGroups)
  }

  // Previous had split - check for improvement
  if (previousSignature.splitSuit !== null) {
    const canBePerfect = optimal.splitSuit === null

    if (canBePerfect) {
      // Improvement! Use new perfect layout
      return optimal
    } else {
      // Maintain previous split pattern
      return maintainSplitPattern(cards, previousSignature, suitGroups)
    }
  }

  return optimal
}

/**
 * Layout strategy interface for PlayerHand layouts.
 * Each strategy computes card positions, dimensions, and optional scaling.
 */
interface LayoutStrategy {
  computeLayout(
    viewportWidth: number,
    cards: Card[],
    previousSignature: LayoutSignature | null
  ): LayoutResult
}

/**
 * Layout engine for PlayerHand that avoids horizontal scrolling entirely.
 *
 * Layout modes:
 * - singleRow: Cards arranged in one centered row with overlap. Used when cards
 *   fit within viewport width using acceptable overlap (respects MAX_OVERLAP ceiling).
 * - twoRow: Cards split into two balanced rows when single-row would exceed overlap
 *   limits. Each row is independently centered and uses its own overlap calculation.
 *
 * Design principles:
 * - No horizontal scrolling: overflow-x is always hidden. When cards don't fit in one
 *   row, we switch to two-row layout rather than enabling scrolling.
 * - Deterministic positioning: All positions computed from viewport width and card
 *   count, avoiding DOM measurement jitter during resize/rotation.
 * - Visual effects preserved: Drop shadows and selected-card lift/scale are not
 *   clipped by using overflow-hidden (not overflow-y-hidden) and adequate minHeight.
 * - Stable z-index: Selected cards render above neighbors (including across rows)
 *   via deterministic z-index boost.
 */
class DefaultLayoutStrategy implements LayoutStrategy {
  computeLayout(
    viewportWidth: number,
    cards: Card[],
    previousSignature: LayoutSignature | null
  ): LayoutResult {
    const cardCount = cards.length

    if (cardCount === 0) {
      return {
        mode: 'singleRow',
        positions: [],
        minHeight: CARD_HEIGHT + 16,
      }
    }

    // Check if we can fit in one row
    if (canFitInOneRow(viewportWidth, cardCount)) {
      // Single row layout
      const overlap = calculateOverlapForRow(viewportWidth, cardCount)
      const cardStep = EFFECTIVE_CARD_WIDTH - overlap
      const stripWidth = EFFECTIVE_CARD_WIDTH + (cardCount - 1) * cardStep
      const baseOffset = (viewportWidth - stripWidth) / 2

      const positions: CardPosition[] = []
      for (let i = 0; i < cardCount; i++) {
        positions.push({
          left: baseOffset + i * cardStep,
          top: 0,
          zIndex: i, // Base z-index by index
        })
      }

      return {
        mode: 'singleRow',
        positions,
        minHeight: CARD_HEIGHT + 16 + SELECTED_CARD_LIFT,
      }
    }

    // Two row layout - use suit-aware algorithm
    // Group cards by suit once and pass to avoid recomputation
    const suitGroups = groupCardsBySuit(cards)
    const twoRowSplit = getTwoRowLayout(cards, previousSignature, suitGroups)
    const topRowCount = twoRowSplit.topRow.length
    const bottomRowCount = twoRowSplit.bottomRow.length

    // Calculate overlap for each row independently
    const topOverlap = calculateOverlapForRow(viewportWidth, topRowCount)
    const bottomOverlap =
      bottomRowCount > 0
        ? calculateOverlapForRow(viewportWidth, bottomRowCount)
        : 0

    const topCardStep = EFFECTIVE_CARD_WIDTH - topOverlap
    const bottomCardStep =
      bottomRowCount > 0 ? EFFECTIVE_CARD_WIDTH - bottomOverlap : 0

    // Center each row
    const topStripWidth = EFFECTIVE_CARD_WIDTH + (topRowCount - 1) * topCardStep
    const bottomStripWidth =
      bottomRowCount > 0
        ? EFFECTIVE_CARD_WIDTH + (bottomRowCount - 1) * bottomCardStep
        : 0

    const topBaseOffset = (viewportWidth - topStripWidth) / 2
    const bottomBaseOffset = (viewportWidth - bottomStripWidth) / 2

    const topRowY = 0
    const bottomRowY = CARD_HEIGHT - ROW_OVERLAP

    // Create a map from card to its position in the split
    const cardToRowMap = new Map<
      Card,
      { row: 'top' | 'bottom'; index: number }
    >()
    twoRowSplit.topRow.forEach((card, index) => {
      cardToRowMap.set(card, { row: 'top', index })
    })
    twoRowSplit.bottomRow.forEach((card, index) => {
      cardToRowMap.set(card, { row: 'bottom', index })
    })

    const positions: CardPosition[] = []
    for (let i = 0; i < cardCount; i++) {
      const card = cards[i]
      const rowInfo = cardToRowMap.get(card)

      if (!rowInfo) {
        // Fallback (shouldn't happen)
        positions.push({
          left: 0,
          top: 0,
          zIndex: i,
        })
        continue
      }

      const { row, index: rowIndex } = rowInfo
      const isTopRow = row === 'top'

      // Z-index strategy: bottom row cards render above top row cards in overlap area
      // Within each row, later cards have higher z-index (for horizontal overlap)
      // Bottom row gets base 100, top row gets base 0
      const zIndex = isTopRow
        ? rowIndex // Top row: 0-99 range
        : 100 + rowIndex // Bottom row: 100-199 range

      positions.push({
        left: isTopRow
          ? topBaseOffset + rowIndex * topCardStep
          : bottomBaseOffset + rowIndex * bottomCardStep,
        top: isTopRow ? topRowY : bottomRowY,
        zIndex,
      })
    }

    return {
      mode: 'twoRow',
      positions,
      minHeight:
        CARD_HEIGHT + (CARD_HEIGHT - ROW_OVERLAP) + 16 + SELECTED_CARD_LIFT,
      twoRowSplit: twoRowSplit,
      topOverlap: topOverlap,
      bottomOverlap: bottomOverlap,
    }
  }
}

/**
 * Scaled layout strategy that applies card scaling below a threshold width.
 * When viewport is narrow, cards scale down to fit better while maintaining readability.
 * Maintains the same overlap ratio as full-size cards by scaling overlap proportionally.
 */
class ScaledLayoutStrategy implements LayoutStrategy {
  private readonly MIN_SCALE = 0.75 // Don't scale below this factor
  private readonly baseStrategy = new DefaultLayoutStrategy()

  computeLayout(
    viewportWidth: number,
    cards: Card[],
    previousSignature: LayoutSignature | null
  ): LayoutResult {
    const baseLayout = this.baseStrategy.computeLayout(
      viewportWidth,
      cards,
      previousSignature
    )

    // Single row mode: no scaling, return base layout
    if (baseLayout.mode === 'singleRow') {
      return {
        ...baseLayout,
        scale: 1,
      }
    }

    // Two-row mode: check if scaling is needed
    // Reuse split and overlaps from baseLayout to avoid recomputation
    const twoRowSplit = baseLayout.twoRowSplit!
    const topOverlap = baseLayout.topOverlap!
    const bottomOverlap = baseLayout.bottomOverlap ?? 0
    const topRowCount = twoRowSplit.topRow.length
    const bottomRowCount = twoRowSplit.bottomRow.length

    const topRowAtMaxOverlap =
      topRowCount > 1 && Math.abs(topOverlap - MAX_OVERLAP) < OVERLAP_TOLERANCE
    const bottomRowAtMaxOverlap =
      bottomRowCount > 1 &&
      Math.abs(bottomOverlap - MAX_OVERLAP) < OVERLAP_TOLERANCE
    const eitherRowAtMaxOverlap = topRowAtMaxOverlap || bottomRowAtMaxOverlap

    // Calculate the width where max overlap would be reached
    const minWidthForTwoRowMaxOverlap = Math.max(
      calculateMinWidthForMaxOverlap(topRowCount),
      calculateMinWidthForMaxOverlap(bottomRowCount)
    )

    // Determine scale: only scale when max overlap is reached and viewport is below threshold
    let scale = 1
    if (eitherRowAtMaxOverlap && viewportWidth < minWidthForTwoRowMaxOverlap) {
      scale = Math.max(
        this.MIN_SCALE,
        viewportWidth / minWidthForTwoRowMaxOverlap
      )
    }

    // If no scaling needed, return base layout
    if (scale === 1) {
      return {
        ...baseLayout,
        scale: 1,
      }
    }

    // Calculate positions using effective viewport width to maintain overlap ratio
    // Then scale positions back to actual viewport coordinates
    // Use the original split to ensure consistency (scaling shouldn't change card rows)
    const effectiveViewportWidth = viewportWidth / scale
    const scaledLayout = this.baseStrategy.computeLayout(
      effectiveViewportWidth,
      cards,
      previousSignature
    )

    // Scale positions to actual viewport coordinates
    const scaledCardHeight = CARD_HEIGHT * scale
    const scaledSelectedLift = SELECTED_CARD_LIFT * scale
    const scaledRowOverlap = ROW_OVERLAP * scale

    // Reuse card-to-row mapping from original split (scaling shouldn't change row assignment)
    const cardToRowMap = new Map<Card, 'top' | 'bottom'>()
    twoRowSplit.topRow.forEach((card) => {
      cardToRowMap.set(card, 'top')
    })
    twoRowSplit.bottomRow.forEach((card) => {
      cardToRowMap.set(card, 'bottom')
    })

    const positions: CardPosition[] = scaledLayout.positions.map(
      (pos, index) => {
        const card = cards[index]
        const isTopRow = cardToRowMap.get(card) === 'top'
        return {
          ...pos,
          left: pos.left * scale,
          // Top row stays at 0, bottom row uses scaled overlap
          top: isTopRow ? 0 : scaledCardHeight - scaledRowOverlap,
        }
      }
    )

    return {
      mode: scaledLayout.mode,
      positions,
      minHeight:
        scaledCardHeight +
        (scaledCardHeight - scaledRowOverlap) +
        16 +
        scaledSelectedLift,
      scale,
      // Preserve original split and overlaps (scaling preserves the split)
      twoRowSplit: twoRowSplit,
      topOverlap: topOverlap,
      bottomOverlap: bottomOverlap,
    }
  }
}

// Strategy registry
const layoutStrategies: Record<LayoutVariant, LayoutStrategy> = {
  default: new DefaultLayoutStrategy(),
  scaled: new ScaledLayoutStrategy(),
}

export function PlayerHand({
  viewerHand,
  phase,
  playerNames,
  viewerSeat,
  playState,
  selectedCard,
  onSelectCard,
  onPlayCard,
  className,
  requireCardConfirmation = true,
  layoutVariant = 'default',
  viewportRef: externalViewportRef,
}: PlayerHandProps) {
  const t = useTranslations('game.gameRoom.play')
  const tHand = useTranslations('game.gameRoom.hand')
  const tSuitAbbrev = useTranslations('game.gameRoom.hand.suitAbbrev')
  const isTrickPhase = checkIsTrickPhase(phase) && !!playState
  const viewerTurn =
    isTrickPhase && playState
      ? phase.data.to_act === playState.viewerSeat
      : false
  const playableCards = useMemo(
    () => new Set(playState?.playable ?? []),
    [playState]
  )
  const waitingOnSeat = isTrickPhase ? phase.data.to_act : null
  const waitingOnName =
    waitingOnSeat === null
      ? null
      : waitingOnSeat === viewerSeat
        ? tHand('you')
        : playerNames[waitingOnSeat]

  const readOnlyPreviewText = tHand('status.readOnlyPreview')
  let handStatus = readOnlyPreviewText

  if (!viewerHand.length) {
    handStatus = t('status.handWillAppear')
  } else if (isTrickPhase && !viewerTurn && !requireCardConfirmation) {
    // Show waiting message when confirmation is disabled and not viewer's turn
    handStatus = waitingOnName
      ? t('status.waitingFor', { name: waitingOnName })
      : t('status.waitingForNext')
  } else if (viewerTurn && !requireCardConfirmation) {
    handStatus = t('status.tapToPlayImmediate')
  }

  // Responsive visibility: hide title below 400px, legal plays below 320px
  // Exception: always show title when it's not the player's turn (for waiting message)
  const showTitle = useMediaQuery('(min-width: 400px)')
  const showLegalPlays = useMediaQuery('(min-width: 320px)')
  const isNotViewerTurn = isTrickPhase && playState && !viewerTurn
  const shouldShowTitle = isNotViewerTurn || showTitle

  // Memoize legal cards display calculation
  const legalCardsDisplay = useMemo(() => {
    if (!isTrickPhase || !playState || !viewerTurn) {
      return null
    }

    if (!playState.playable.length) {
      return '—'
    }

    // Get lead card if trick has started
    const leadCard =
      phase.data.current_trick.length > 0
        ? phase.data.current_trick[0][1]
        : null

    if (leadCard) {
      // Rule 2: If player has cards matching lead suit, only that suit is legal
      const leadSuit = leadCard.slice(-1).toUpperCase()
      const hasLeadSuit = viewerHand.some(
        (card) => card.slice(-1).toUpperCase() === leadSuit
      )

      if (hasLeadSuit) {
        // Only one suit is legal (the lead suit) - translate the suit abbreviation
        return tSuitAbbrev(leadSuit as 'S' | 'C' | 'H' | 'D')
      }
    }

    // Rule 3: No lead card or player doesn't have lead suit - all cards are legal
    return tHand('legal.all')
  }, [
    isTrickPhase,
    playState,
    viewerTurn,
    phase,
    viewerHand,
    tSuitAbbrev,
    tHand,
  ])

  const internalViewportRef = useRef<HTMLDivElement | null>(null)
  const viewportRef = externalViewportRef ?? internalViewportRef
  const layoutSignatureRef = useRef<LayoutSignature | null>(null)
  const [layout, setLayout] = useState<LayoutResult>({
    mode: 'singleRow',
    positions: [],
    minHeight: CARD_HEIGHT + 16,
  })

  // Clear card suit cache when hand changes
  useEffect(() => {
    clearCardSuitCache()
  }, [viewerHand])

  useLayoutEffect(() => {
    const viewport = viewportRef.current
    if (!viewport) {
      return
    }

    const strategy = layoutStrategies[layoutVariant] ?? layoutStrategies.default

    const updateLayout = () => {
      const width = viewport.clientWidth

      // Compute layout using previous signature
      const newLayout = strategy.computeLayout(
        width,
        viewerHand,
        layoutSignatureRef.current
      )

      // Update signature after computing layout (for next render)
      // Reuse the split from layout result if available (avoids recomputation)
      if (newLayout.mode === 'twoRow' && newLayout.twoRowSplit) {
        layoutSignatureRef.current = createSignature(
          viewerHand,
          newLayout.twoRowSplit
        )
      } else {
        // Reset signature for single-row layouts
        layoutSignatureRef.current = null
      }

      setLayout(newLayout)
    }

    updateLayout()

    const resizeObserver = new ResizeObserver(() => {
      updateLayout()
    })
    resizeObserver.observe(viewport)

    return () => {
      resizeObserver.disconnect()
    }
    // viewportRef is stable (refs don't change identity), but included so effect re-runs
    // if a different external ref is passed. ResizeObserver handles size changes.
  }, [viewerHand, layoutVariant, viewportRef])

  const handleCardClick = useCallback(
    (card: Card) => {
      if (!isTrickPhase || !playState) {
        return
      }

      const isPlayable = playableCards.has(card)
      if (!viewerTurn || !isPlayable || playState.isPending) {
        return
      }

      if (!requireCardConfirmation) {
        onSelectCard(null)
        if (onPlayCard) {
          void onPlayCard(card)
        }
        return
      }

      onSelectCard(selectedCard === card ? null : card)
    },
    [
      isTrickPhase,
      playState,
      playableCards,
      viewerTurn,
      requireCardConfirmation,
      onSelectCard,
      onPlayCard,
      selectedCard,
    ]
  )

  return (
    <section
      className={cn(
        'flex w-full flex-col gap-3 rounded-[28px] border border-white/15 bg-surface/80 p-4 text-foreground shadow-elevated backdrop-blur',
        className
      )}
    >
      <header className="flex items-center gap-3">
        {/* Title and subtitle: only show when not waiting (or when requireCardConfirmation is false and waiting) */}
        {(!isTrickPhase ||
          !playState ||
          viewerTurn ||
          !requireCardConfirmation) && (
          <div className="flex flex-col gap-1 min-w-0">
            {shouldShowTitle && (
              <span className="text-[11px] font-semibold uppercase tracking-[0.4em] text-subtle break-words">
                {tHand('title')}
              </span>
            )}
            {shouldShowTitle && handStatus !== readOnlyPreviewText && (
              <p className="text-xs text-muted break-words" aria-live="polite">
                {handStatus}
              </p>
            )}
          </div>
        )}
        {isTrickPhase && playState && requireCardConfirmation ? (
          <div className="flex justify-center flex-1 min-w-0">
            <button
              type="button"
              data-selected-card-exempt
              onClick={async () => {
                if (onPlayCard && selectedCard && viewerTurn) {
                  await onPlayCard(selectedCard)
                  onSelectCard(null)
                }
              }}
              disabled={
                !viewerTurn ||
                playState.isPending ||
                !selectedCard ||
                !playState.playable.includes(selectedCard)
              }
              className="rounded-2xl bg-primary px-4 py-1.5 text-sm font-semibold text-primary-foreground shadow-lg shadow-primary/40 transition hover:bg-primary/90 disabled:cursor-not-allowed disabled:bg-primary/40 disabled:text-primary-foreground/70"
              aria-label={
                playState.isPending
                  ? t('button.aria.playing')
                  : selectedCard
                    ? t('button.aria.playSelected', { card: selectedCard })
                    : t('button.aria.selectCard')
              }
            >
              {playState.isPending ? (
                t('button.playing')
              ) : viewerTurn ? (
                <>
                  <span className="sm:hidden">{t('button.playCard')}</span>
                  <span className="hidden sm:inline">
                    {t('button.playSelectedCard')}
                  </span>
                </>
              ) : waitingOnName ? (
                t('button.waitingFor', { name: waitingOnName })
              ) : (
                t('button.waitingForNext')
              )}
            </button>
          </div>
        ) : null}
        {isTrickPhase &&
        playState &&
        viewerTurn &&
        (showLegalPlays || !requireCardConfirmation) ? (
          <div
            className="ml-auto flex items-center justify-end gap-2 flex-shrink-0"
            style={{ minWidth: 'max-content' }}
          >
            {playState.playable.length > 0 && legalCardsDisplay ? (
              <div className="rounded-full bg-black/20 px-3 py-1">
                <span className="text-sm font-medium text-muted">
                  <span className="sm:hidden">{t('legal.short')}</span>
                  <span className="hidden sm:inline">{t('legal.long')}</span>
                </span>
                <span className="ml-1.5 text-sm font-medium text-foreground">
                  {legalCardsDisplay}
                </span>
              </div>
            ) : null}
          </div>
        ) : null}
      </header>

      <div
        ref={viewportRef}
        className="relative w-full pb-2 pt-4 overflow-visible"
        style={{ minHeight: layout.minHeight }}
      >
        {viewerHand.length === 0 ? (
          <div className="flex h-full items-center justify-center">
            <span className="text-sm text-subtle">{tHand('empty')}</span>
          </div>
        ) : (
          <div className="relative w-full h-full">
            {viewerHand.map((card, index) => {
              const isPlayable = playableCards.has(card)
              const isSelected = selectedCard === card
              const isDisabled =
                !isTrickPhase ||
                !playState ||
                !isPlayable ||
                !viewerTurn ||
                playState.isPending

              const cardLabel = isPlayable
                ? tHand('cardAria', {
                    card,
                    state: isSelected
                      ? tHand('cardState.selected')
                      : tHand('cardState.playable'),
                  })
                : tHand('cardAria', {
                    card,
                    state: isDisabled
                      ? tHand('cardState.notPlayable')
                      : tHand('cardState.playable'),
                  })

              const position = layout.positions[index]

              // Skip rendering if position not yet computed (initial render before useLayoutEffect runs)
              if (!position) {
                return null
              }

              // Selected card gets z-index boost to render above neighbors (including across rows)
              const baseZIndex = position.zIndex
              const zIndex = isSelected ? baseZIndex + 1000 : baseZIndex

              // Apply scale if present in layout result
              const scale = layout.scale ?? 1
              const baseTransform = `scale(${scale})`
              const selectedTransform = `translateY(-${SELECTED_CARD_LIFT * scale}px) scale(${scale * 1.1})`
              const hoverTransform = `translateY(-${1 * scale}px) scale(${scale * 1.05})`

              return (
                <button
                  key={card}
                  type="button"
                  data-selected-card-exempt
                  onClick={() => handleCardClick(card)}
                  disabled={isDisabled}
                  className={cn(
                    'absolute focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/60 disabled:cursor-not-allowed',
                    'transition-[transform] duration-300 ease-out'
                  )}
                  style={{
                    left: position.left,
                    top: position.top,
                    zIndex,
                    transform: isSelected ? selectedTransform : baseTransform,
                    transformOrigin: 'top left',
                  }}
                  // Direct style manipulation is necessary here because each card has a unique baseZIndex
                  // that changes dynamically. CSS classes can't express per-instance z-index calculations.
                  onMouseEnter={(e) => {
                    if (!isSelected) {
                      e.currentTarget.style.zIndex = String(baseZIndex + 50)
                      e.currentTarget.style.transform = hoverTransform
                    }
                  }}
                  onMouseLeave={(e) => {
                    if (!isSelected) {
                      e.currentTarget.style.zIndex = String(baseZIndex)
                      e.currentTarget.style.transform = baseTransform
                    }
                  }}
                  aria-label={cardLabel}
                  aria-pressed={isSelected}
                >
                  <div
                    className={cn(
                      'rounded-[1.45rem] border-2 p-[2px] transition-all',
                      isSelected ? 'border-primary' : 'border-transparent'
                    )}
                  >
                    <PlayingCard
                      card={card}
                      size="md"
                      isDimmed={isDisabled && !isSelected}
                      isSelected={isSelected}
                    />
                  </div>
                </button>
              )
            })}
          </div>
        )}
      </div>
    </section>
  )
}
