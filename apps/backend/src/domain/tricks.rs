use crate::domain::rules::PLAYERS;
use crate::domain::state::{
    next_player, require_hand_size, require_trick_no, require_turn, GameState, Phase, PlayerId,
    RoundState,
};
use crate::domain::{card_beats, hand_has_suit, Card, Trump};
use crate::errors::domain::{DomainError, ValidationKind};

/// Result of playing a card, describing what state changes occurred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayCardResult {
    /// Whether a trick was completed (4 cards played).
    pub trick_completed: bool,
    /// Winner of the completed trick, if one was completed.
    pub trick_winner: Option<PlayerId>,
    /// Trick number after this play (may have incremented if trick completed).
    pub trick_no_after: u8,
    /// Phase transitioned to, if any (None means still in Trick phase).
    pub phase_transitioned: Option<Phase>,
}

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
pub fn play_card(
    state: &mut GameState,
    who: PlayerId,
    card: Card,
) -> Result<PlayCardResult, DomainError> {
    // Phase check
    let Phase::Trick {
        trick_no: trick_no_before_phase,
    } = state.phase
    else {
        return Err(DomainError::validation(
            ValidationKind::PhaseMismatch,
            "Phase mismatch",
        ));
    };

    // Invariant: when in Trick phase, state.trick_no must be set and match the phase payload.
    let trick_no_before = require_trick_no(state, "play_card")?;
    if trick_no_before != trick_no_before_phase {
        return Err(DomainError::validation_other(
            "Invariant violated: state.trick_no must match Phase::Trick.trick_no",
        ));
    }

    // Turn check
    let turn = require_turn(state, "play_card")?;
    if turn != who {
        return Err(DomainError::validation(
            ValidationKind::OutOfTurn,
            "Out of turn",
        ));
    }

    // Card in hand (immutable check first to avoid borrow conflicts)
    let pos_opt = state.hands[who as usize].iter().position(|&c| c == card);
    let Some(pos) = pos_opt else {
        return Err(DomainError::validation(
            ValidationKind::CardNotInHand,
            "Card not in hand",
        ));
    };

    // Suit following check using an immutable borrow only
    let legal = legal_moves(state, who);
    if !legal.contains(&card) {
        return Err(DomainError::validation(
            ValidationKind::MustFollowSuit,
            "Must follow suit",
        ));
    }

    // On first play, set lead + leader
    if state.round.trick_plays.is_empty() {
        state.round.trick_lead = Some(card.suit);
        state.leader = Some(who); // remember who led this trick
    }

    // Move card to plays (now take mutable borrow)
    let removed = {
        let hand_mut = &mut state.hands[who as usize];
        hand_mut.remove(pos)
    };
    state.round.trick_plays.push((who, removed));

    // Advance turn explicitly
    state.turn = Some(next_player(who));

    let trick_completed = state.round.trick_plays.len() == 4;
    let mut result = PlayCardResult {
        trick_completed,
        trick_winner: None,
        trick_no_after: trick_no_before,
        phase_transitioned: None,
    };

    if !trick_completed {
        return Ok(result);
    }

    // Resolve completed trick
    if let Some(winner) = resolve_current_trick(&state.round) {
        state.round.tricks_won[winner as usize] += 1;
        state.leader = Some(winner);
        state.turn = Some(winner);
        result.trick_winner = Some(winner);
    }

    // Prepare next trick
    state.round.trick_plays.clear();
    state.round.trick_lead = None;

    let hand_size = require_hand_size(state, "play_card trick_complete")?;
    let next_trick_no = trick_no_before.saturating_add(1);

    if next_trick_no > hand_size {
        state.phase = Phase::Scoring;
        state.turn = None;
        state.leader = None;
        state.trick_no = None;

        result.trick_no_after = next_trick_no;
        result.phase_transitioned = Some(Phase::Scoring);
        return Ok(result);
    }

    state.trick_no = Some(next_trick_no);
    state.phase = Phase::Trick {
        trick_no: next_trick_no,
    };
    result.trick_no_after = next_trick_no;

    Ok(result)
}

/// Resolve the current trick winner if complete.
pub fn resolve_current_trick(state: &RoundState) -> Option<PlayerId> {
    if state.trick_plays.len() < 4 {
        return None;
    }
    let lead = state.trick_lead?;

    let trump = state.trump;
    // Determine best play index per rules
    let mut best_idx = 0usize;
    for i in 1..PLAYERS {
        let (_, card_i) = state.trick_plays[i];
        let (_, card_best) = state.trick_plays[best_idx];
        let trump_for_comparison = match trump {
            Some(tr) => tr,
            None => Trump::NoTrumps, // No trump chosen yet: treat as NoTrumps
        };
        let better = card_beats(card_i, card_best, lead, trump_for_comparison);
        if better {
            best_idx = i;
        }
    }
    Some(state.trick_plays[best_idx].0)
}
