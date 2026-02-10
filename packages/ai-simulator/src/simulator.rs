//! In-memory game simulator for AI training and evaluation.
//!
//! This module provides a fast, validation-free game execution engine
//! that runs games entirely in memory for AI development and testing.

use backend::ai::{AiError, AiPlayer};
use backend::domain::bidding::{place_bid, set_trump, Bid};
use backend::domain::cards_types::Card;
use backend::domain::deal_hands;
use backend::domain::game_context::GameContext;
use backend::domain::player_view::{CurrentRoundInfo, GameHistory, RoundHistory, RoundScoreDetail};
use backend::domain::round_memory::{PlayMemory, RoundMemory, TrickMemory};
use backend::domain::rules::hand_size_for_round;
use backend::domain::scoring::apply_round_scoring;
use backend::domain::seed_derivation::derive_dealing_seed;
use backend::domain::state::{GameState, Phase, PlayerId, RoundState};
use backend::domain::tricks::play_card;

const PLAYERS: usize = 4;

// -----------------------------------------------------------------------------
// Local helper (file-scoped)
//
// We intentionally keep this copy local because:
// - this crate cannot access backend's unit-test helpers
// - we don't want backend to expose test-only helpers or use feature flags
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct MakeGameStateArgs {
    phase: Phase,

    round_no: Option<u8>,
    hand_size: Option<u8>,
    dealer: Option<PlayerId>,

    turn: Option<PlayerId>,
    leader: Option<PlayerId>,
    trick_no: Option<u8>,

    scores_total: [i16; PLAYERS],
}

impl Default for MakeGameStateArgs {
    fn default() -> Self {
        Self {
            phase: Phase::Init,
            round_no: None,
            hand_size: None,
            dealer: None,
            turn: None,
            leader: None,
            trick_no: None,
            scores_total: [0; PLAYERS],
        }
    }
}

/// Build a GameState with Option-honest semantics.
///
/// Notes:
/// - `turn: Option<PlayerId>` is the sole "someone must act" signal.
/// - Round-start seat is derived as `dealer.map(next_player)`; we do not store `turn_start`.
fn make_game_state(hands: [Vec<Card>; 4], args: MakeGameStateArgs) -> GameState {
    GameState {
        phase: args.phase,

        round_no: args.round_no,
        hand_size: args.hand_size,
        dealer: args.dealer,

        turn: args.turn,
        leader: args.leader,
        trick_no: args.trick_no,

        hands,
        scores_total: args.scores_total,
        round: RoundState::empty(),
    }
}

// -----------------------------------------------------------------------------
// Simulator
// -----------------------------------------------------------------------------

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
    game_seed: [u8; 32],
    /// Game ID (for context, can be arbitrary)
    game_id: i64,
}

impl Simulator {
    /// Create a new simulator with initial game state.
    pub fn new(game_seed: [u8; 32], game_id: i64) -> Self {
        Self {
            state: make_game_state(
                [vec![], vec![], vec![], vec![]],
                MakeGameStateArgs::default(),
            ),
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
        // 26 rounds total (13 down to 2, four 2-card rounds, then back up to 13)
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
        let hand_size =
            hand_size_for_round(round_no).ok_or_else(|| SimulatorError::InvalidRound(round_no))?;

        // Derive dealing seed from game seed
        let dealing_seed = derive_dealing_seed(&self.game_seed, round_no)
            .map_err(|e| SimulatorError::DomainError(format!("Seed derivation failed: {e}")))?;

        // Deal hands
        let hands = deal_hands(PLAYERS, hand_size, dealing_seed)
            .map_err(|e| SimulatorError::DomainError(format!("Deal failed: {e}")))?;

        // Determine dealer (rotates each round)
        let dealer: PlayerId = ((round_no - 1) % 4) as PlayerId;

        // Preserve cumulative score across rounds
        let scores_total = self.state.scores_total;

        // Rebuild state for the new round (Option-honest)
        self.state = make_game_state(
            hands,
            MakeGameStateArgs {
                phase: Phase::Bidding,
                round_no: Some(round_no),
                hand_size: Some(hand_size),
                dealer: Some(dealer),

                // First bidder is to left of dealer
                turn: Some((dealer + 1) % 4),

                // Leader is not meaningful during bidding; it will be set for trick play.
                leader: None,

                // Track current trick number as a convenience for AI context/history.
                trick_no: Some(0),

                scores_total,
            },
        );

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
            let player_seat = self
                .state
                .turn
                .expect("simulator: expected Some(turn) in Bidding phase");

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

        // Ensure first trick leader/turn is left-of-dealer.
        // (Domain logic may set a placeholder; simulator enforces canonical round-start seat.)
        let dealer = self
            .state
            .dealer
            .expect("simulator: expected Some(dealer) after dealing round");
        let first = (dealer + 1) % 4;
        self.state.leader = Some(first);
        self.state.turn = Some(first);
        self.state.trick_no = Some(1);

        Ok(())
    }

    /// Play all tricks in the round.
    fn play_tricks_phase(&mut self, ais: &[Box<dyn AiPlayer>; 4]) -> Result<(), SimulatorError> {
        while matches!(self.state.phase, Phase::Trick { .. }) {
            // Play one complete trick (4 cards)
            while self.state.round.trick_plays.len() < 4
                && matches!(self.state.phase, Phase::Trick { .. })
            {
                let player_seat = self
                    .state
                    .turn
                    .expect("simulator: expected Some(turn) in Trick phase");

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
    fn build_current_round_info(
        &self,
        player_seat: u8,
    ) -> Result<CurrentRoundInfo, SimulatorError> {
        if player_seat >= 4 {
            return Err(SimulatorError::InvalidState(format!(
                "Invalid player seat: {player_seat}"
            )));
        }

        let current_round = self
            .state
            .round_no
            .expect("simulator: expected Some(round_no) outside Init");
        let hand_size = self
            .state
            .hand_size
            .expect("simulator: expected Some(hand_size) outside Init");
        let dealer_pos = self
            .state
            .dealer
            .expect("simulator: expected Some(dealer) outside Init");
        let trick_no = self
            .state
            .trick_no
            .expect("simulator: expected Some(trick_no) during round");

        // Build current trick plays
        let current_trick_plays: Vec<(u8, Card)> =
            self.state.round.trick_plays.iter().copied().collect();

        // Determine trick leader
        // For first play of a trick: use state.leader (set by domain/simulator)
        // For trick in progress: leader is first player in trick_plays
        let trick_leader = if matches!(self.state.phase, Phase::Trick { .. }) {
            if current_trick_plays.is_empty() {
                self.state.leader
            } else {
                Some(self.state.round.trick_plays[0].0)
            }
        } else {
            None
        };

        Ok(CurrentRoundInfo {
            game_id: self.game_id,
            player_seat,
            game_state: self.state.phase,
            current_round,
            hand_size,
            dealer_pos,
            hand: self.state.hands[player_seat as usize].clone(),
            bids: self.state.round.bids,
            trump: self.state.round.trump,
            trick_no,
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
        let round_no = self
            .state
            .round_no
            .expect("simulator: expected Some(round_no) when adding history");
        let hand_size = self
            .state
            .hand_size
            .expect("simulator: expected Some(hand_size) when adding history");
        let dealer_seat = self
            .state
            .dealer
            .expect("simulator: expected Some(dealer) when adding history");

        let round_score_deltas = self.calculate_round_scores();
        let round_history = RoundHistory {
            round_no,
            hand_size,
            dealer_seat,
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
                .unwrap_or(
                    [RoundScoreDetail {
                        round_score: 0,
                        cumulative_score: 0,
                    }; 4],
                ),
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
