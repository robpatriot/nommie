import { describe, expect, it } from 'vitest'
import {
  getTwoRowLayout,
  createSignature,
} from '@/app/game/[gameId]/_components/game-room/PlayerHand'

describe('PlayerHand Layout Algorithm', () => {
  describe('getTwoRowLayout - optimal layout', () => {
    it('prefers balanced rows when suits can stay together', () => {
      const cards = ['2H', '3H', '5S', '7S', '4C', '9C', '6D', '8D']
      const result = getTwoRowLayout(cards, null)

      expect(result.topRow.length).toBe(4)
      expect(result.bottomRow.length).toBe(4)
      expect(result.splitSuit).toBeNull()
    })

    it('keeps suits together when possible', () => {
      const cards = ['2H', '3H', '5H', '7S', '4C']
      const result = getTwoRowLayout(cards, null)

      const topHearts = result.topRow.filter((c) => c.endsWith('H'))
      const bottomHearts = result.bottomRow.filter((c) => c.endsWith('H'))

      // All hearts should be in one row
      expect(topHearts.length === 0 || bottomHearts.length === 0).toBe(true)
      expect(topHearts.length + bottomHearts.length).toBe(3)
    })

    it('must split suit when it has more than 7 cards', () => {
      const cards = [
        '2H',
        '3H',
        '4H',
        '5H',
        '6H',
        '7H',
        '8H',
        '9H', // 8 hearts
        '2S',
        '3S',
      ]
      const result = getTwoRowLayout(cards, null)

      expect(result.splitSuit).toBe('H')
      expect(result.splitTopCount).toBeGreaterThan(0)
      expect(result.splitTopCount).toBeLessThan(8)
    })

    it('keeps all suits together when possible', () => {
      const cards = ['2H', '3H', '5S', '7S', '4C', '9C']
      const result = getTwoRowLayout(cards, null)

      // All suits should be together (no suit split across rows)
      const topSuits = new Set(result.topRow.map((c) => c.slice(-1)))
      const bottomSuits = new Set(result.bottomRow.map((c) => c.slice(-1)))
      const splitSuits = Array.from(topSuits).filter((suit) =>
        bottomSuits.has(suit)
      )
      expect(splitSuits.length).toBe(0) // No suits should be split
    })

    it('respects 7-card-per-row limit', () => {
      const cards = [
        '2H',
        '3H',
        '4H',
        '5H',
        '6H',
        '7H',
        '8H',
        '9H', // 8 hearts (must split)
        '2S',
        '3S',
        '4S',
        '5S',
        '6S', // 5 spades (total: 13 cards, max possible)
      ]
      const result = getTwoRowLayout(cards, null)

      expect(result.topRow.length).toBeLessThanOrEqual(7)
      expect(result.bottomRow.length).toBeLessThanOrEqual(7)
      expect(result.topRow.length + result.bottomRow.length).toBe(13)
    })
  })

  describe('getTwoRowLayout - pattern maintenance', () => {
    it('maintains previous pattern when hand unchanged', () => {
      const cards = ['2H', '3H', '5S', '7S']
      const initial = getTwoRowLayout(cards, null)
      const signature = createSignature(cards, initial)

      // Same hand should return same layout
      const result = getTwoRowLayout(cards, signature)

      expect(result.topRow).toEqual(initial.topRow)
      expect(result.bottomRow).toEqual(initial.bottomRow)
    })

    it('maintains pattern when card is removed', () => {
      const originalCards = ['2H', '3H', '5H', '7S', '4C', '9C']
      const original = getTwoRowLayout(originalCards, null)
      const signature = createSignature(originalCards, original)

      // Remove one card
      const newCards = ['2H', '3H', '5H', '7S', '4C']
      const result = getTwoRowLayout(newCards, signature)

      // Should maintain same suit grouping pattern
      const originalTopSuits = new Set(original.topRow.map((c) => c.slice(-1)))
      const resultTopSuits = new Set(result.topRow.map((c) => c.slice(-1)))

      // If hearts were on top originally, they should still be on top
      if (originalTopSuits.has('H')) {
        expect(resultTopSuits.has('H')).toBe(true)
      }
    })

    it('improves from split to perfect when possible', () => {
      // Start with a split layout
      const originalCards = [
        '2H',
        '3H',
        '4H',
        '5H',
        '6H',
        '7H',
        '8H',
        '9H', // 8 hearts (must split)
        '2S',
        '3S',
      ]
      const original = getTwoRowLayout(originalCards, null)
      const signature = createSignature(originalCards, original)

      expect(signature.splitSuit).toBe('H')

      // Remove cards so hearts can fit together
      const newCards = ['2H', '3H', '4H', '5H', '6H', '7S']
      const result = getTwoRowLayout(newCards, signature)

      // Should improve to perfect (no split)
      expect(result.splitSuit).toBeNull()
    })

    it('maintains split pattern when improvement not possible', () => {
      const originalCards = [
        '2H',
        '3H',
        '4H',
        '5H',
        '6H',
        '7H',
        '8H',
        '9H', // 8 hearts
        '2S',
        '3S',
      ]
      const original = getTwoRowLayout(originalCards, null)
      const signature = createSignature(originalCards, original)

      // Still have 8 hearts, so split must remain
      const newCards = [
        '2H',
        '3H',
        '4H',
        '5H',
        '6H',
        '7H',
        '8H',
        '9H',
        '2S', // One less spade
      ]
      const result = getTwoRowLayout(newCards, signature)

      // Should maintain split pattern
      expect(result.splitSuit).toBe('H')
      expect(result.splitTopCount).toBe(signature.splitTopCount)
    })

    it('maintains perfect pattern when hand changes but still perfect', () => {
      const originalCards = ['2H', '3H', '5S', '7S', '4C']
      const original = getTwoRowLayout(originalCards, null)
      const signature = createSignature(originalCards, original)

      expect(signature.splitSuit).toBeNull() // Was perfect

      // Remove a card, still can be perfect
      const newCards = ['2H', '3H', '5S', '7S']
      const result = getTwoRowLayout(newCards, signature)

      // Should maintain pattern (hearts on same row as before)
      const originalHeartsRow = original.topRow.some((c) => c.endsWith('H'))
        ? 'top'
        : 'bottom'
      const resultHeartsRow = result.topRow.some((c) => c.endsWith('H'))
        ? 'top'
        : 'bottom'

      expect(resultHeartsRow).toBe(originalHeartsRow)
    })
  })
})
