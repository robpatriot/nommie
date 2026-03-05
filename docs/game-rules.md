i===== game-rules.md =====

# Game Rules

This project implements Nomination Whist with a fixed ruleset.  
All game logic, validation, and UI must conform to these rules.

## Players

- Exactly **4 players**.
- Turn order is **clockwise**.

## Rounds

The game contains **26 rounds**.

Hand sizes per round:

13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3,  
2, 2, 2, 2,  
3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13

Rules:

- Each round uses a freshly shuffled 52-card deck.
- The **dealer rotates clockwise** each round.

## Bidding

Each player submits exactly one bid per round.

Valid bids:

0 … hand_size

Rules:

- The **dealer bids last**.
- The dealer may not choose a bid that causes:

sum(all bids) == hand_size

- A player may bid **0**, but not **more than three rounds in a row**.
- After three consecutive 0 bids, that player must bid at least 1.

After bidding:

- The **highest bid** wins the right to select trump.
- If tied, the earliest bidder among the tied players wins.

## Trump Selection

The winning bidder selects the trump suit.

Valid choices:

- Spades
- Hearts
- Diamonds
- Clubs
- No Trumps

If **No Trumps** is selected:

- No cards have trump priority.
- Tricks are won by the highest card of the lead suit.

## Trick Play

- The player to the **left of the dealer** leads the first trick.
- Players must **follow suit if able**.
- If unable to follow suit, any card may be played.

Trick resolution:

- If a trump suit exists and at least one trump is played, the highest trump wins.
- Otherwise the highest card of the lead suit wins.

Additional rules:

- The trick winner leads the next trick.
- Each round contains exactly **hand_size tricks**.

## Scoring

Scoring uses only the following mechanisms.

- Each trick won = **+1 point**
- If a player wins exactly the number of tricks they bid, they receive **+10 bonus points**

Additional rules:

- The bonus applies even for a bid of **0**.
- There are **no negative scores**.
- Failing to meet a bid only forfeits the bonus.

Scores accumulate across all 26 rounds.

## Game End

After round 26:

- The game ends.
- Player(s) with the **highest total score** win.
- Ties result in **joint winners**.
