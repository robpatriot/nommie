use crate::domain::rules::PLAYERS;
use crate::domain::state::{next_player, GameState, Phase, PlayerId, RoundState};
use crate::domain::Card;

#[derive(Debug, Clone, Copy)]
pub struct MakeGameStateArgs {
    pub phase: Phase,

    pub round_no: Option<u8>,
    pub hand_size: Option<u8>,
    pub dealer: Option<PlayerId>,

    pub turn: Option<PlayerId>,
    pub leader: Option<PlayerId>,
    pub trick_no: Option<u8>,

    pub scores_total: [i16; PLAYERS],
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

pub fn make_game_state(hands: [Vec<Card>; PLAYERS], mut args: MakeGameStateArgs) -> GameState {
    match args.phase {
        Phase::Init => {
            args.round_no = None;
            args.hand_size = None;
            args.turn = None;
            args.leader = None;
            args.trick_no = None;
        }

        Phase::Bidding => {
            if args.round_no.is_none() {
                args.round_no = Some(1);
            }
            if args.turn.is_none() {
                if let Some(dealer) = args.dealer {
                    args.turn = Some(next_player(dealer));
                }
            }
            args.leader = None;
            args.trick_no = None;
        }

        Phase::TrumpSelect => {
            if args.round_no.is_none() {
                args.round_no = Some(1);
            }
            args.leader = None;
            args.trick_no = None;
        }

        Phase::Trick { trick_no } => {
            if args.round_no.is_none() {
                args.round_no = Some(1);
            }
            if args.trick_no.is_none() {
                args.trick_no = Some(trick_no);
            }
            if args.leader.is_none() {
                if let Some(dealer) = args.dealer {
                    args.leader = Some(next_player(dealer));
                }
            }
            if args.turn.is_none() {
                args.turn = args.leader;
            }
        }

        Phase::Scoring | Phase::Complete => {
            if args.round_no.is_none() {
                args.round_no = Some(1);
            }
            args.turn = None;
            args.leader = None;
            args.trick_no = None;
        }

        Phase::GameOver => {
            args.turn = None;
            args.leader = None;
            args.trick_no = None;
        }
    }

    GameState {
        phase: args.phase,
        round_no: args.round_no,
        hand_size: args.hand_size,
        hands,
        dealer: args.dealer,
        turn: args.turn,
        leader: args.leader,
        trick_no: args.trick_no,
        scores_total: args.scores_total,
        round: RoundState::empty(),
    }
}
