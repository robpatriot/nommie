# 🎲 Nommie — Game Rules

This project implements **Nomination Whist** with the following fixed ruleset.  

---

## 1) Core Rules

- **Players:** Always exactly 4 players.

### Bidding
- Each player makes a public bid (nomination) in turn.  
- The highest bid wins the right to choose the trump suit.  
- If multiple players tie for highest, the first in turn order wins.  

### Trump Selection
- The winning bidder chooses the trump suit.  

### Scoring
- Each player scores **1 point per trick won**.  
- If a player wins **exactly** the number of tricks they bid, they earn a **+10 bonus**.  

### Round Structure
- Game starts with **13 cards per player**.  
- Each round after, the hand size decreases by 1 until reaching 2.  
- At 2 cards per player, there are **4 rounds of 2 cards**.  
- Then the hand size increases again by 1 each round until back to 13.  
- **Total rounds: 26.**  

### Card Play
- Players must **follow suit if able**.  
- Trick winner = highest trump played; if no trumps, highest card of lead suit.  

---

## 2) AI Behavior
- AI players must follow the same rules as humans.  
- All AI actions (bidding and plays) must be valid under the rules above.  

---

## 3) Testable Invariants
- Total tricks in a round = cards dealt per player.  
- A player with a card of the lead suit **must** play that suit.  
- If any trump is played, winner ∈ {trump cards} with highest rank.  
- Scores are deterministic given bids, trump, and trick sequence.  

---

## 4) Glossary
- **Lead suit:** Suit of the first card played in a trick.  
- **Trump:** Suit that overrides lead suit hierarchy in a trick.  
- **Exact bid:** Number of tricks won equals the number of tricks bid.  

---
