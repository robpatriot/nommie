use crate::domain::cards::{card_beats, hand_has_suit, Card, Trump};
use crate::domain::errors::DomainError;
use crate::domain::rules::PLAYERS;
use crate::domain::state::{advance_turn, GameState, Phase, PlayerId, RoundState};

/// Compute legal cards the player may play, independent of turn enforcement.
pub fn legal_moves(state: &GameState, who: PlayerId) -> Vec<Card> {
    // If not in Trick phase, the set is empty.
    let Phase::Trick { .. } = state.phase else {
        return Vec::new();
    };
    let hand = &state.hands[who as usize];
    if hand.is_empty() {
        return Vec::new();
    }
    if let Some(lead) = state.round.trick_lead {
        if hand_has_suit(hand, lead) {
            let mut v: Vec<Card> = hand.iter().copied().filter(|c| c.suit == lead).collect();
            v.sort();
            return v;
        }
    }
    let mut any = hand.clone();
    any.sort();
    any
}

/// Play a card into the current trick, enforcing turn, suit-following, and phase.
pub fn play_card(state: &mut GameState, who: PlayerId, card: Card) -> Result<(), DomainError> {
    // Phase check
    let Phase::Trick { .. } = state.phase else {
        return Err(DomainError::PhaseMismatch);
    };
    // Turn check
    if state.turn != who {
        return Err(DomainError::OutOfTurn);
    }
    // Card in hand (immutable check first to avoid borrow conflicts)
    let pos_opt = state.hands[who as usize].iter().position(|&c| c == card);
    if let Some(pos) = pos_opt {
        // Suit following check using an immutable borrow only
        let legal = legal_moves(state, who);
        if !legal.contains(&card) {
            return Err(DomainError::MustFollowSuit);
        }
        // On first play, set lead
        if state.round.trick_plays.is_empty() {
            state.round.trick_lead = Some(card.suit);
            state.leader = who; // remember who led this trick
        }
        // Move card to plays (now take mutable borrow)
        let removed = {
            let hand_mut = &mut state.hands[who as usize];
            hand_mut.remove(pos)
        };
        state.round.trick_plays.push((who, removed));
        // Advance turn
        advance_turn(state);
        // If 4 plays, resolve
        if state.round.trick_plays.len() == 4 {
            if let Some(winner) = resolve_current_trick(&state.round) {
                state.round.tricks_won[winner as usize] += 1;
                state.leader = winner;
                state.turn = winner;
            }
            // Prepare next trick
            state.round.trick_plays.clear();
            state.round.trick_lead = None;
            state.trick_no += 1;
            if state.trick_no > state.hand_size {
                state.phase = Phase::Scoring;
            } else {
                state.phase = Phase::Trick {
                    trick_no: state.trick_no,
                };
            }
        }
        Ok(())
    } else {
        Err(DomainError::CardNotInHand)
    }
}

/// Resolve the current trick winner if complete.
pub fn resolve_current_trick(state: &RoundState) -> Option<PlayerId> {
    if state.trick_plays.len() < 4 {
        return None;
    }
    let lead = state.trick_lead?;

    // Debug assertion for trick_lead invariant
    debug_assert_eq!(
        state.trick_plays[0].1.suit, lead,
        "First card's suit ({:?}) must match trick_lead ({:?})",
        state.trick_plays[0].1.suit, lead
    );

    let trump = state.trump;
    // Determine best play index per rules
    let mut best_idx = 0usize;
    for i in 1..PLAYERS {
        let (_, card_i) = state.trick_plays[i];
        let (_, card_best) = state.trick_plays[best_idx];
        let trump_for_comparison = match trump {
            Some(tr) => tr,
            None => Trump::NoTrump, // No trump chosen yet: treat as NoTrump
        };
        let better = card_beats(card_i, card_best, lead, trump_for_comparison);
        if better {
            best_idx = i;
        }
    }
    Some(state.trick_plays[best_idx].0)
}
