# 🎲 Nommie Game Rules

This project implements *Nomination Whist* with the following fixed ruleset.  
These rules are **locked** — all game logic, extractors, tests, and UI must conform.

---

## Players
- Always exactly **4 players**.  
- Fixed **turn order** (clockwise).  

---

## Rounds
- The game lasts **26 rounds**.  
- Hand size schedule:  
  13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2,  
  2, 2, 2,  
  3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13
- Each round is dealt from a freshly shuffled standard 52-card deck.  
- **Dealer** rotates clockwise each round.  

---

## Bidding
- Each player must make exactly **one bid** per round.  
- Valid bids are integers `0 … hand_size`.  
- The **dealer always bids last**.  
  - Dealer’s restriction: cannot choose a bid that would make  
    `sum(all 4 bids) == hand_size`.  
- A player may bid **0**, but **not more than 3 rounds in a row**.  
- Once all 4 bids are in:  
  - **Highest bid** wins the right to select trump.  
  - If tied, the earliest bidder among the tied players wins.  

---

## Trump Selection
- The winning bidder chooses the **trump suit** for that round.  

---

## Trick Play
- Players must **follow suit** if able.  
- If no card of the lead suit, they may play any card.  
- Trick winner =  
  - highest trump played, or  
  - if no trumps, highest card of the lead suit.  
- Trick winner leads the next trick.  
- Each round has exactly **hand_size tricks**.  

---

## Scoring
- Each trick won = **+1 point**.  
- If a player wins **exactly as many tricks as they bid**, they gain a **+10 bonus**.  
  - Applies even to a bid of 0 (if they take no tricks).  
- Scores are cumulative across all 26 rounds.  

---

## Game End
- After Round 26, the game ends.  
- Player(s) with the **highest total score** win.  
- If multiple players tie, they are declared **joint winners**.  
