//! DTOs for bids_sea adapter.

/// DTO for creating a bid.
#[derive(Debug, Clone)]
pub struct BidCreate {
    pub round_id: i64,
    pub player_seat: u8,
    pub bid_value: u8,
    pub bid_order: u8,
}
