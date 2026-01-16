import type { Card } from '@/lib/game-room/types'

// --- Types & Constants ---

export interface LayoutStructure {
  topRow: Card[]
  bottomRow: Card[]
}

const SUITS = ['S', 'H', 'C', 'D'] as const
type Suit = (typeof SUITS)[number]

const SUIT_INDICES: Record<Suit, number> = {
  S: 0,
  H: 1,
  C: 2,
  D: 3,
}

// S, C are Black (false). H, D are Red (true).
// S=0 (B), H=1 (R), C=2 (B), D=3 (R)
function isRed(suit: Suit): boolean {
  return suit === 'H' || suit === 'D'
}

type SuitGroup = {
  suit: Suit
  cards: Card[]
  count: number
  idx: number
}

// --- Helper Functions ---

function getSuit(card: Card): Suit {
  return card.slice(-1) as Suit
}

function getSuitGroups(cards: Card[]): SuitGroup[] {
  const groups: Record<Suit, Card[]> = {
    S: [],
    H: [],
    C: [],
    D: [],
  }

  // Group by suit, preserving rank order (assumed input is rank-sorted within suits or we don't touch it)
  // Actually, standard practice is to respect the incoming card order for ranks.
  for (const card of cards) {
    const s = getSuit(card)
    if (groups[s]) {
      groups[s].push(card)
    }
  }

  // Return strictly in canonical order S, H, C, D
  return SUITS.map((s) => ({
    suit: s,
    cards: groups[s],
    count: groups[s].length,
    idx: SUIT_INDICES[s],
  }))
}

// Check if balance constraint is met: abs(a - b) <= 2
// The prompt said: "balanced(a, b) := abs(a - b) in {1,2}" but actually usually implies <= 2 or <=1.
// Usually "balanced" means as close as possible. User spec: "balance predicate (< 3 means 1 or 2)"
// Wait, user wrote: "balanced(a, b) := abs(a - b) in {1,2}".
// However, exact balance (0) is surely better?
// "If T is even: targets are (T/2, T/2) ... If T is odd: targets are (floor, ceil)" -> This implies diff 0 or 1.
// But the predicate definition "in {1,2}" is odd if it excludes 0.
// I will assume "balanced" means abs(diff) <= 2, but prefer 0/1.
// Let's stick to the user's specific text if possible, but 0-diff must be allowed for perfect splits (4 vs 4).
// I will interpret "in {1,2}" as inclusive of 0 logic (which is < 1).
// PROPOSAL: Allow diff <= 2.
function isBalanced(countA: number, countB: number): boolean {
  return Math.abs(countA - countB) <= 2
}

// Sort cards within a suit-group list for a single row
// "For any row... both orders either satisfy adjacency... Choose [x,y] if idx(x) < idx(y)"
// "If row has 3 or 4 groups... evaluate all permutations... min violations -> tie lexicographically"
function flattenGroups(groups: SuitGroup[]): Card[] {
  return groups.flatMap((g) => g.cards)
}

function countAdjacencyViolations(groups: SuitGroup[]): number {
  let violations = 0
  for (let i = 0; i < groups.length - 1; i++) {
    const s1 = groups[i].suit
    const s2 = groups[i + 1].suit
    if (isRed(s1) === isRed(s2)) {
      violations++
    }
  }
  return violations
}

// Groups: list of SuitGroups to arrange in one row
function arrangeRowGroups(groups: SuitGroup[]): SuitGroup[] {
  if (groups.length <= 1) return groups

  if (groups.length === 2) {
    const [a, b] = groups
    // Only 2 orders: [a,b] or [b,a]
    // User spec: "Both orders either satisfy... or both violate... Choose [x,y] if idx(x)<idx(y)"
    // So actually, just strict canonical sort always breaks ties deterministicly.
    // The only case where order matters for adjacency is if colors differ (which they always do if RB).
    // If RB, then R-B and B-R are both valid (0 violations).
    // If RR, then R-R is invalid (1 violation).
    // User rule: "Choose [x,y] if (idx(x), idx(y)) is lexicographically smaller"
    // This implies simple canonical sort is the rule for 2-groups.
    if (a.idx < b.idx) return [a, b]
    return [b, a]
  }

  if (groups.length === 3) {
    // 3-suit row logic (Step 2 specific logic handles the 3-group case for 3+1 splits)
    // But generic row ordering (Step 4) needs to handle 3 groups too.
    // Spec: "Evaluate all permutations... pick min adjacency violations -> tie lexicographically"

    // Permutations of 3
    const perms = permute(groups)
    return pickBestPermutation(perms)
  }

  if (groups.length === 4) {
    const perms = permute(groups)
    return pickBestPermutation(perms)
  }

  return groups
}

function permute<T>(arr: T[]): T[][] {
  if (arr.length <= 1) return [arr]
  const result: T[][] = []
  for (let i = 0; i < arr.length; i++) {
    const rest = [...arr.slice(0, i), ...arr.slice(i + 1)]
    const subPerms = permute(rest)
    for (const sub of subPerms) {
      result.push([arr[i], ...sub])
    }
  }
  return result
}

function pickBestPermutation(perms: SuitGroup[][]): SuitGroup[] {
  let best = perms[0]
  let minViolations = Infinity

  // Tie-breaker: lexicographically smallest by idx sequence
  perLoop: for (const p of perms) {
    const v = countAdjacencyViolations(p)
    if (v < minViolations) {
      minViolations = v
      best = p
    } else if (v === minViolations) {
      // Compare lexicographically
      for (let i = 0; i < p.length; i++) {
        if (p[i].idx < best[i].idx) {
          best = p
          continue perLoop
        }
        if (p[i].idx > best[i].idx) {
          continue perLoop
        }
      }
    }
  }
  return best
}

// "Row ordering (deterministic): Prefer the row with more cards first. If equal, prefer row whose ordered suit list is lexicographically smaller"
function orderRows(rowA: Card[], rowB: Card[]): LayoutStructure {
  if (rowA.length > rowB.length) return { topRow: rowA, bottomRow: rowB }
  if (rowB.length > rowA.length) return { topRow: rowB, bottomRow: rowA }

  // Equal length: compare first card's suit (since rows are internally sorted by suit groups)
  // Actually comparisons should be based on the suit-group sequence.
  // But checking the first card's suit is a decent proxy for "suit list lexicographically smaller"
  // if we assume the rows are built from flat groups.
  // Let's deduce the leading suit of key.
  if (rowA.length === 0) return { topRow: rowA, bottomRow: rowB }

  const sA = getSuit(rowA[0])
  const sB = getSuit(rowB[0])
  if (SUIT_INDICES[sA] < SUIT_INDICES[sB])
    return { topRow: rowA, bottomRow: rowB }
  return { topRow: rowB, bottomRow: rowA }
}

// --- Main Layout Steps ---

// Layout Engine Constants
import { CARD_DIMENSIONS } from './PlayingCard'
const CARD_WIDTH = CARD_DIMENSIONS.md.width
const MAX_OVERLAP = CARD_WIDTH - 17
const EFFECTIVE_CARD_WIDTH = CARD_WIDTH + 8 // Padding 8

function calculateMinWidth(count: number): number {
  if (count <= 1) return EFFECTIVE_CARD_WIDTH
  return (
    EFFECTIVE_CARD_WIDTH + (count - 1) * (EFFECTIVE_CARD_WIDTH - MAX_OVERLAP)
  )
}

function fitsInOneRow(count: number, viewportWidth: number): boolean {
  if (count === 0) return true
  return viewportWidth >= calculateMinWidth(count)
}

export function computeLayout(
  cards: Card[],
  viewportWidth: number
): LayoutStructure {
  // Step 0: One row check
  if (fitsInOneRow(cards.length, viewportWidth)) {
    // Return single row in canonical order S, H, C, D
    const groups = getSuitGroups(cards) // Already S,H,C,D
    return {
      topRow: flattenGroups(groups),
      bottomRow: [],
    }
  }

  const groups = getSuitGroups(cards)
  const T = cards.length

  // S=0, H=1, C=2, D=3
  const S = groups[0],
    H = groups[1],
    C = groups[2],
    D = groups[3]
  const nS = S.count,
    nH = H.count,
    nC = C.count,
    nD = D.count

  // Step 1: Priority 1 - No Split, 2+2, RB/RB, Balanced
  // Candidate A: {S, H} / {C, D}
  const sizeA1 = nS + nH
  const sizeB1 = nC + nD
  if (isBalanced(sizeA1, sizeB1)) {
    // Canonical sort within rows: [S, H], [C, D]
    // These are already sorted because getSuitGroups returns S,H,C,D
    return orderRows(
      flattenGroups([S, H]), // S(0)<H(1)
      flattenGroups([C, D]) // C(2)<D(3)
    )
  }

  // Candidate B: {S, D} / {C, H}
  const sizeA2 = nS + nD
  const sizeB2 = nC + nH
  if (isBalanced(sizeA2, sizeB2)) {
    // Canonical sort: [S, D], [H, C]
    return orderRows(flattenGroups([S, D]), flattenGroups([H, C]))
  }

  // Step 2: Priority 2 - No Split, 3+1, Balanced
  // Check singleton suit in canonical order S, H, C, D
  for (const singleton of [S, H, C, D]) {
    const a = singleton.count
    const b = T - a
    if (isBalanced(a, b)) {
      const rest = groups.filter((g) => g.suit !== singleton.suit)
      // "Odd-colour suit in the middle. Ends are two same-colour suits in canonical order"
      // rest has 3 suits. 2 same color, 1 odd.
      const reds = rest.filter((g) => isRed(g.suit))
      const blacks = rest.filter((g) => !isRed(g.suit))

      let row3: SuitGroup[] = []
      if (reds.length === 1) {
        // Red is odd. Blacks are ends.
        // Blacks sorted: [0] < [1]
        row3 = [blacks[0], reds[0], blacks[1]]
      } else {
        // Black is odd (must be 1 black, 2 reds).
        // Reds sorted.
        row3 = [reds[0], blacks[0], reds[1]]
      }

      return orderRows(flattenGroups([singleton]), flattenGroups(row3))
    }
  }

  // Step 3: Priority 3 - No Split, 2+2 RR/BB, Balanced
  // Black Row: {S, C}, Red Row: {H, D}
  const sizeBlack = nS + nC
  const sizeRed = nH + nD
  if (isBalanced(sizeBlack, sizeRed)) {
    return orderRows(flattenGroups([S, C]), flattenGroups([H, D]))
  }

  // Step 4: Split exactly one suit (Last Resort)
  // 4.1 Choose suit to split (Max count, tie-break canonical)
  let splitSuit = S
  for (const g of [H, C, D]) {
    if (g.count > splitSuit.count) {
      splitSuit = g
    }
  }
  // Tie-break is implicit because we iterated S->H->C->D and strict > used.
  // Wait, strict > keeps the first one if equal. S has priority. Correct.

  // 4.3 Candidate split points
  const nX = splitSuit.count
  const OtherCount = T - nX
  const tSmall = Math.floor(T / 2)
  const tLarge = Math.ceil(T / 2)

  // k = target - Other
  const kCandidates = new Set<number>()
  const k1 = tSmall - OtherCount
  if (k1 >= 1 && k1 < nX) kCandidates.add(k1)
  const k2 = tLarge - OtherCount
  if (k2 >= 1 && k2 < nX) kCandidates.add(k2)

  // Fallback if no valid k (e.g. constraints)
  if (kCandidates.size === 0) {
    const kDefault = Math.max(1, Math.min(nX - 1, Math.round(nX / 2)))
    if (nX > 1) kCandidates.add(kDefault)
  }

  // If nX <= 1, we can't split this suit!
  // This implies we failed Step 1-3 but have no splittable suit?
  // Should essentially be impossible for T >= 4 unless distributions are weird (e.g. 1-1-1-1 is handled by Step 1 or 2).
  // If T=3 (1-1-1), Step 2 usually catches it (Singleton + 2).
  // If we are here, we MUST split.

  let bestLayout: LayoutStructure | null = null
  let bestScore = {
    balanceDiff: Infinity,
    totalViolations: Infinity,
    canonicalScore: Infinity, // Lower is better
  }

  const otherSuits = groups.filter((g) => g.suit !== splitSuit.suit)

  for (const k of kCandidates) {
    // Fragments
    // Assumes card list for suit is sorted by rank? Yes, groups preserve order.
    // X_A = first k, X_B = rest
    const cardsRankSorted = splitSuit.cards
    const X_A: SuitGroup = {
      ...splitSuit,
      cards: cardsRankSorted.slice(0, k),
      count: k,
    }
    const X_B: SuitGroup = {
      ...splitSuit,
      cards: cardsRankSorted.slice(k),
      count: nX - k,
    }

    // Option 4+1: Row1 = Others + X_A, Row2 = X_B (or vice versa)
    // We construct the row sets, then optimize their internal order.
    const set1_4plus1 = [...otherSuits, X_A]
    const set2_4plus1 = [X_B]
    considerSplitOption(set1_4plus1, set2_4plus1)

    // Option 3+2: Move one non-X suit to join X_B
    for (const mover of otherSuits) {
      const set1_3plus2 = [
        ...otherSuits.filter((s) => s.suit !== mover.suit),
        X_A,
      ]
      const set2_3plus2 = [X_B, mover]
      considerSplitOption(set1_3plus2, set2_3plus2)
    }
  }

  function considerSplitOption(setA: SuitGroup[], setB: SuitGroup[]) {
    // Optimize internal row orders
    const rowAOrdered = arrangeRowGroups(setA)
    const rowBOrdered = arrangeRowGroups(setB)

    const cardsA = flattenGroups(rowAOrdered)
    const cardsB = flattenGroups(rowBOrdered)

    // Order rows (Longer first)
    const layout = orderRows(cardsA, cardsB)
    const top = layout.topRow
    const bot = layout.bottomRow

    // Score
    const balDiff = Math.abs(top.length - bot.length)
    const viol =
      countAdjacencyViolations(rowAOrdered) +
      countAdjacencyViolations(rowBOrdered)

    // Canonical score: Check first suits of top row then bottom row
    // We construct a simple numeric value: TopFirstIdx * 100 + BotFirstIdx
    // This favors S on top, then H on top, etc.
    const topFirst = top.length > 0 ? getSuit(top[0]) : 'S'
    const botFirst = bot.length > 0 ? getSuit(bot[0]) : 'S'
    const canon = SUIT_INDICES[topFirst] * 10 + SUIT_INDICES[botFirst]

    // Compare to best
    // Prioritize violations over balance (as long as balance is reasonable, which Step 4 candidates are)
    let isBetter = false
    if (bestLayout === null) isBetter = true
    else {
      if (viol < bestScore.totalViolations) isBetter = true
      else if (viol === bestScore.totalViolations) {
        if (balDiff < bestScore.balanceDiff) isBetter = true
        else if (balDiff === bestScore.balanceDiff) {
          if (canon < bestScore.canonicalScore) isBetter = true
        }
      }
    }

    if (isBetter) {
      bestLayout = layout
      bestScore = {
        balanceDiff: balDiff,
        totalViolations: viol,
        canonicalScore: canon,
      }
    }
  }

  return bestLayout!
}
