pub type PlayerId = u8; // 0..=3

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Phase {
    Bidding,
    TrumpSelect,
    Trick { trick_no: u8 },
    Scoring,
    Complete,
}

#[derive(Debug, Clone)]
pub struct RoundState {
    pub round_no: u8,
}

#[derive(Debug, Clone)]
pub struct GameState {
    pub phase: Phase,
    pub round: RoundState,
}


