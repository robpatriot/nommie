#[test]
fn test_winner_counts_to_winners_list() {
    let tricks_per_player = [3, 4, 2, 4];
    let mut winners = Vec::new();

    for (seat, &count) in tricks_per_player.iter().enumerate() {
        for _ in 0..count {
            winners.push(seat as i16);
        }
    }

    assert_eq!(winners.len(), 13);
    assert_eq!(winners[0..3], [0, 0, 0]); // P0 wins first 3
    assert_eq!(winners[3..7], [1, 1, 1, 1]); // P1 wins next 4
    assert_eq!(winners[7..9], [2, 2]); // P2 wins next 2
    assert_eq!(winners[9..13], [3, 3, 3, 3]); // P3 wins last 4
}
