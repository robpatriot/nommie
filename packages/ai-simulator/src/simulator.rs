//! In-memory game simulator for AI training and evaluation.
//!
//! This module provides a fast, validation-free game execution engine
//! that runs games entirely in memory for AI development and testing.

use backend::ai::{AiError, AiPlayer};
use backend::domain::bidding::{place_bid, set_trump, Bid};
use backend::domain::cards_types::Card;
use backend::domain::game_context::GameContext;
use backend::domain::player_view::{CurrentRoundInfo, GameHistory, RoundHistory, RoundScoreDetail};
use backend::domain::round_memory::{PlayMemory, RoundMemory, TrickMemory};
use backend::domain::rules::hand_size_for_round;
use backend::domain::scoring::apply_round_scoring;
use backend::domain::seed_derivation::derive_dealing_seed;
use backend::domain::state::{GameState, Phase, RoundState};
use backend::domain::tricks::play_card;
use backend::domain::deal_hands;

/// Result of simulating a complete game.
#[derive(Debug, Clone)]
pub struct GameResult {
    /// Final scores for each player (indexed by seat 0-3)
    pub final_scores: [i16; 4],
    /// Complete game history
    #[allow(dead_code)] // Reserved for future detailed metrics
    pub history: GameHistory,
    /// Number of rounds played
    #[allow(dead_code)] // Reserved for future detailed metrics
    pub rounds_played: u8,
}

/// In-memory game simulator.
///
/// Manages game state, builds AI context, and executes game phases
/// without database or validation overhead.
pub struct Simulator {
    /// Current game state
    state: GameState,
    /// Game history (built incrementally)
    history: GameHistory,
    /// Round memory for each player (for AI context)
    round_memories: [Option<RoundMemory>; 4],
    /// Completed tricks in current round (for building round memory)
    completed_tricks: Vec<Vec<(u8, Card)>>,
    /// Game seed (for deterministic dealing)
    game_seed: i64,
    /// Game ID (for context, can be arbitrary)
    game_id: i64,
}

impl Simulator {
    /// Create a new simulator with initial game state.
    pub fn new(game_seed: i64, game_id: i64) -> Self {
        Self {
            state: GameState {
                phase: Phase::Init,
                round_no: 0,
                hand_size: 0,
                hands: [vec![], vec![], vec![], vec![]],
                turn_start: 0,
                turn: 0,
                leader: 0,
                trick_no: 0,
                scores_total: [0; 4],
                round: RoundState::empty(),
            },
            history: GameHistory { rounds: vec![] },
            round_memories: [None, None, None, None],
            completed_tricks: vec![],
            game_seed,
            game_id,
        }
    }

    /// Simulate a complete game with the given AI players.
    ///
    /// Returns the final game result with scores and history.
    pub fn simulate_game(
        mut self,
        ais: &[Box<dyn AiPlayer>; 4],
    ) -> Result<GameResult, SimulatorError> {
        // Play all 26 rounds
        for round_no in 1..=26 {
            self.deal_round(round_no)?;
            self.play_round(ais)?;
        }

        Ok(GameResult {
            final_scores: self.state.scores_total,
            history: self.history,
            rounds_played: 26,
        })
    }

    /// Deal a new round.
    fn deal_round(&mut self, round_no: u8) -> Result<(), SimulatorError> {
        let hand_size = hand_size_for_round(round_no)
            .ok_or_else(|| SimulatorError::InvalidRound(round_no))?;

        // Derive dealing seed from game seed
        let dealing_seed = derive_dealing_seed(self.game_seed, round_no);

        // Deal hands
        let hands = deal_hands(4, hand_size, dealing_seed)
            .map_err(|e| SimulatorError::DomainError(format!("Deal failed: {e}")))?;

        // Determine dealer (rotates each round)
        let dealer_pos = ((round_no - 1) % 4) as u8;

        // Initialize round state
        self.state.round_no = round_no;
        self.state.hand_size = hand_size;
        self.state.hands = hands;
        self.state.turn_start = dealer_pos;
        self.state.turn = (dealer_pos + 1) % 4; // First bidder is to left of dealer
        self.state.leader = (dealer_pos + 1) % 4;
        self.state.trick_no = 0;
        self.state.phase = Phase::Bidding;
        self.state.round = RoundState::empty();
        self.completed_tricks.clear();
        self.round_memories = [None, None, None, None];

        Ok(())
    }

    /// Play a complete round (bidding, trump selection, tricks, scoring).
    fn play_round(&mut self, ais: &[Box<dyn AiPlayer>; 4]) -> Result<(), SimulatorError> {
        // Bidding phase
        self.play_bidding_phase(ais)?;

        // Trump selection phase
        self.play_trump_selection(ais)?;

        // Trick play phase
        self.play_tricks_phase(ais)?;

        // Scoring phase
        self.score_round()?;

        // Add round to history
        self.add_round_to_history();

        Ok(())
    }

    /// Play the bidding phase.
    fn play_bidding_phase(&mut self, ais: &[Box<dyn AiPlayer>; 4]) -> Result<(), SimulatorError> {
        while self.state.phase == Phase::Bidding {
            let player_seat = self.state.turn;
            let current_info = self.build_current_round_info(player_seat)?;
            let game_context = self.build_game_context(player_seat)?;

            let ai = &ais[player_seat as usize];
            let bid_value = ai
                .choose_bid(&current_info, &game_context)
                .map_err(|e| SimulatorError::AiError(player_seat, "bid", e))?;

            // Apply bid (no validation - trust AI)
            let result = place_bid(&mut self.state, player_seat, Bid(bid_value))
                .map_err(|e| SimulatorError::DomainError(format!("Place bid failed: {e}")))?;

            // Check if phase transitioned
            if let Some(Phase::TrumpSelect) = result.phase_transitioned {
                // Phase transitioned to TrumpSelect
                break;
            }
        }

        Ok(())
    }

    /// Play the trump selection phase.
    fn play_trump_selection(&mut self, ais: &[Box<dyn AiPlayer>; 4]) -> Result<(), SimulatorError> {
        if self.state.phase != Phase::TrumpSelect {
            return Ok(());
        }

        let winning_bidder = self
            .state
            .round
            .winning_bidder
            .ok_or_else(|| SimulatorError::InvalidState("No winning bidder".into()))?;

        let current_info = self.build_current_round_info(winning_bidder)?;
        let game_context = self.build_game_context(winning_bidder)?;

        let ai = &ais[winning_bidder as usize];
        let trump = ai
            .choose_trump(&current_info, &game_context)
            .map_err(|e| SimulatorError::AiError(winning_bidder, "trump", e))?;

        // Apply trump selection (no validation - trust AI)
        set_trump(&mut self.state, winning_bidder, trump)
            .map_err(|e| SimulatorError::DomainError(format!("Set trump failed: {e}")))?;

        // Fix leader and turn: first trick is led by player to left of dealer
        // (set_trump sets both to turn_start/dealer, but leader should be dealer+1)
        self.state.leader = (self.state.turn_start + 1) % 4;
        self.state.turn = self.state.leader;

        Ok(())
    }

    /// Play all tricks in the round.
    fn play_tricks_phase(&mut self, ais: &[Box<dyn AiPlayer>; 4]) -> Result<(), SimulatorError> {
        while matches!(self.state.phase, Phase::Trick { .. }) {
            // Play one complete trick (4 cards)
            while self.state.round.trick_plays.len() < 4
                && matches!(self.state.phase, Phase::Trick { .. })
            {
                let player_seat = self.state.turn;
                let current_info = self.build_current_round_info(player_seat)?;
                let game_context = self.build_game_context(player_seat)?;

                let ai = &ais[player_seat as usize];
                let card = ai
                    .choose_play(&current_info, &game_context)
                    .map_err(|e| SimulatorError::AiError(player_seat, "play", e))?;

                // If this will complete the trick, save the plays before they're cleared
                let will_complete = self.state.round.trick_plays.len() == 3;
                let trick_plays_before = if will_complete {
                    let mut plays = self.state.round.trick_plays.clone();
                    plays.push((player_seat, card));
                    Some(plays)
                } else {
                    None
                };

                // Apply play (no validation - trust AI)
                let result = play_card(&mut self.state, player_seat, card)
                    .map_err(|e| SimulatorError::DomainError(format!("Play card failed: {e}")))?;

                // If trick completed, record it
                if result.trick_completed {
                    if let Some(trick_plays) = trick_plays_before {
                        self.completed_tricks.push(trick_plays.clone());
                        // Update round memories for all players
                        self.update_round_memories(&trick_plays);
                    }
                }
            }
        }

        Ok(())
    }

    /// Score the current round.
    fn score_round(&mut self) -> Result<(), SimulatorError> {
        if self.state.phase != Phase::Scoring {
            return Ok(());
        }

        apply_round_scoring(&mut self.state);
        Ok(())
    }

    /// Build CurrentRoundInfo for a specific player.
    fn build_current_round_info(&self, player_seat: u8) -> Result<CurrentRoundInfo, SimulatorError> {
        if player_seat >= 4 {
            return Err(SimulatorError::InvalidState(format!(
                "Invalid player seat: {player_seat}"
            )));
        }

        // Build current trick plays
        let current_trick_plays: Vec<(u8, Card)> = self
            .state
            .round
            .trick_plays
            .iter()
            .copied()
            .collect();

        // Determine trick leader
        // For first trick: leader is player to left of dealer
        // For subsequent tricks: leader is winner of previous trick (stored in state.leader)
        let trick_leader = if matches!(self.state.phase, Phase::Trick { .. }) {
            if current_trick_plays.is_empty() {
                // First play of trick - use state.leader (set correctly by domain logic)
                Some(self.state.leader)
            } else {
                // Trick in progress - leader is first player
                Some(self.state.round.trick_plays[0].0)
            }
        } else {
            None
        };

        Ok(CurrentRoundInfo {
            game_id: self.game_id,
            player_seat,
            game_state: self.state.phase,
            current_round: self.state.round_no,
            hand_size: self.state.hand_size,
            dealer_pos: self.state.turn_start,
            hand: self.state.hands[player_seat as usize].clone(),
            bids: self.state.round.bids,
            trump: self.state.round.trump,
            trick_no: self.state.trick_no,
            current_trick_plays,
            scores: self.state.scores_total,
            tricks_won: self.state.round.tricks_won,
            trick_leader,
        })
    }

    /// Build GameContext for a specific player.
    fn build_game_context(&self, player_seat: u8) -> Result<GameContext, SimulatorError> {
        let mut context = GameContext::new(self.game_id);
        context = context.with_history(self.history.clone());

        // Add round memory if available
        if let Some(memory) = &self.round_memories[player_seat as usize] {
            context = context.with_round_memory(Some(memory.clone()));
        }

        Ok(context)
    }

    /// Update round memories for all players after a trick completes.
    fn update_round_memories(&mut self, trick_plays: &[(u8, Card)]) {
        // For now, use full memory (perfect recall) for all players
        // This can be extended later to support different memory levels
        let trick_no = self.completed_tricks.len() as u8;
        let plays: Vec<(u8, PlayMemory)> = trick_plays
            .iter()
            .map(|(seat, card)| (*seat, PlayMemory::Exact(*card)))
            .collect();

        let trick_memory = TrickMemory::new(trick_no, plays);

        // Update memory for all players (same memory for now)
        for i in 0..4 {
            let existing = self.round_memories[i].take();
            let mut tricks = if let Some(mem) = existing {
                mem.tricks
            } else {
                vec![]
            };
            tricks.push(trick_memory.clone());
            self.round_memories[i] = Some(RoundMemory::new(
                backend::ai::memory::MemoryMode::Full,
                tricks,
            ));
        }
    }

    /// Add the completed round to game history.
    fn add_round_to_history(&mut self) {
        let round_score_deltas = self.calculate_round_scores();
        let round_history = RoundHistory {
            round_no: self.state.round_no,
            hand_size: self.state.hand_size,
            dealer_seat: self.state.turn_start,
            bids: self.state.round.bids,
            trump_selector_seat: self.state.round.winning_bidder,
            trump: self.state.round.trump,
            scores: round_score_deltas
                .iter()
                .enumerate()
                .map(|(i, &delta)| RoundScoreDetail {
                    round_score: delta as u8,
                    cumulative_score: self.state.scores_total[i],
                })
                .collect::<Vec<_>>()
                .try_into()
                .unwrap_or([RoundScoreDetail {
                    round_score: 0,
                    cumulative_score: 0,
                }; 4]),
        };

        self.history.rounds.push(round_history);
    }

    /// Calculate round score deltas (for history).
    fn calculate_round_scores(&self) -> [i16; 4] {
        let mut deltas = [0i16; 4];
        for i in 0..4 {
            let tricks = self.state.round.tricks_won[i] as i16;
            let bid = self.state.round.bids[i].unwrap_or(0) as i16;
            let bonus = if tricks == bid { 10 } else { 0 };
            deltas[i] = tricks + bonus;
        }
        deltas
    }
}

/// Errors that can occur during simulation.
#[derive(Debug)]
pub enum SimulatorError {
    /// AI returned an error
    AiError(u8, &'static str, AiError),
    /// Domain logic error
    DomainError(String),
    /// Invalid round number
    InvalidRound(u8),
    /// Invalid game state
    InvalidState(String),
}

impl std::fmt::Display for SimulatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimulatorError::AiError(seat, action, err) => {
                write!(f, "AI error (seat {seat}, {action}): {err}")
            }
            SimulatorError::DomainError(msg) => write!(f, "Domain error: {msg}"),
            SimulatorError::InvalidRound(round) => write!(f, "Invalid round number: {round}"),
            SimulatorError::InvalidState(msg) => write!(f, "Invalid state: {msg}"),
        }
    }
}

impl std::error::Error for SimulatorError {}

