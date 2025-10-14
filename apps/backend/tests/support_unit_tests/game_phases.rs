use crate::support::game_phases::*;

#[test]
fn test_find_winning_bidder_simple() {
    let bids = [3, 5, 2, 4];
    let dealer_pos = 0;
    assert_eq!(find_winning_bidder(&bids, dealer_pos), 1); // Seat 1 has highest bid
}

#[test]
fn test_find_winning_bidder_tie_early_wins() {
    let bids = [3, 4, 4, 2]; // Seats 1 and 2 both bid 4
    let dealer_pos = 0;
    // Bidding order: 1, 2, 3, 0
    // Seat 1 bids first with 4, seat 2 bids second with 4
    // Seat 1 should win (earlier bidder)
    assert_eq!(find_winning_bidder(&bids, dealer_pos), 1);
}

#[test]
fn test_find_winning_bidder_dealer_not_first() {
    let bids = [5, 3, 2, 4];
    let dealer_pos = 1;
    // Bidding order: 2, 3, 0, 1
    // Seat 0 has highest bid (5)
    assert_eq!(find_winning_bidder(&bids, dealer_pos), 0);
}
