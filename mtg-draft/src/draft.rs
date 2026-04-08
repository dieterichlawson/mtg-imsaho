use crate::pack::BoosterPack;
use serde::Serialize;

/// A single draft pick record.
#[derive(Debug, Clone, Serialize)]
pub struct DraftPick {
    pub pack_number: usize,
    pub pick_number: usize,
    pub available: Vec<String>,
    pub chosen: String,
}

/// Per-player draft state.
#[derive(Debug, Clone, Serialize)]
pub struct DraftPlayer {
    pub seat: usize,
    pub picks: Vec<DraftPick>,
    pub pool: Vec<String>,
}

/// The draft state machine.
/// Manages pack distribution, picks, and rotation for a standard booster draft.
pub struct DraftState {
    pub pod_size: usize,
    pub players: Vec<DraftPlayer>,
    /// Current packs in front of each player (cards remaining).
    current_packs: Vec<Vec<String>>,
    /// Which pack round we're on (0, 1, 2).
    pub current_pack_round: usize,
    /// Which pick within the current round (0-based).
    pub current_pick: usize,
    /// Original pack contents for logging.
    pub original_packs: Vec<Vec<Vec<String>>>,
}

impl DraftState {
    /// Create a new draft from generated packs.
    /// `packs` is indexed as packs[player][pack_round].
    pub fn new(packs: Vec<Vec<BoosterPack>>) -> Self {
        let pod_size = packs.len();

        // Store original pack contents for logging
        let original_packs: Vec<Vec<Vec<String>>> = packs
            .iter()
            .map(|player_packs| {
                player_packs.iter().map(|p| p.all_cards()).collect()
            })
            .collect();

        // Convert first pack round into current packs
        let current_packs: Vec<Vec<String>> = packs
            .iter()
            .map(|player_packs| player_packs[0].all_cards())
            .collect();

        let players = (0..pod_size)
            .map(|seat| DraftPlayer {
                seat,
                picks: Vec::new(),
                pool: Vec::new(),
            })
            .collect();

        // Store all packs for later rounds
        let mut state = Self {
            pod_size,
            players,
            current_packs,
            current_pack_round: 0,
            current_pick: 0,
            original_packs,
        };

        // Store remaining pack data for rounds 1 and 2
        // We'll reconstruct current_packs when starting a new round
        state.store_future_packs(&packs);

        state
    }

    fn store_future_packs(&mut self, _packs: &[Vec<BoosterPack>]) {
        // Already loaded round 0 into current_packs.
        // Rounds 1 and 2 are stored in original_packs for logging
        // and reloaded when starting a new pack round.
    }

    /// Get the current pack contents for a player.
    pub fn current_pack_for(&self, seat: usize) -> &[String] {
        &self.current_packs[seat]
    }

    /// How many cards are in the current pack for a player.
    pub fn cards_remaining(&self, seat: usize) -> usize {
        self.current_packs[seat].len()
    }

    /// Make a pick for a player. The `card_name` must be in their current pack.
    pub fn make_pick(&mut self, seat: usize, card_name: &str) -> Result<(), String> {
        let pack = &mut self.current_packs[seat];
        let pos = pack
            .iter()
            .position(|c| c == card_name)
            .ok_or_else(|| format!("Card '{}' not in pack for seat {}", card_name, seat))?;

        let available = pack.clone();
        let chosen = pack.remove(pos);

        self.players[seat].picks.push(DraftPick {
            pack_number: self.current_pack_round + 1,
            pick_number: self.current_pick + 1,
            available,
            chosen: chosen.clone(),
        });

        self.players[seat].pool.push(chosen);

        Ok(())
    }

    /// After all players have made their pick for this round, rotate packs.
    /// Pack 1: pass left (seat N -> seat N+1)
    /// Pack 2: pass right (seat N -> seat N-1)
    /// Pack 3: pass left
    pub fn rotate_packs(&mut self) {
        let direction: isize = if self.current_pack_round % 2 == 0 {
            1 // left
        } else {
            -1 // right
        };

        let n = self.pod_size as isize;
        let mut new_packs = vec![Vec::new(); self.pod_size];

        for seat in 0..self.pod_size {
            let target = ((seat as isize + direction + n) % n) as usize;
            new_packs[target] = std::mem::take(&mut self.current_packs[seat]);
        }

        self.current_packs = new_packs;
        self.current_pick += 1;
    }

    /// Check if the current pack round is complete (all cards picked).
    pub fn is_pack_round_complete(&self) -> bool {
        self.current_packs.iter().all(|p| p.is_empty())
    }

    /// Start the next pack round. Returns false if the draft is complete.
    pub fn start_next_pack_round(&mut self) -> bool {
        if self.current_pack_round >= 2 {
            return false;
        }

        self.current_pack_round += 1;
        self.current_pick = 0;

        // Load the next round's packs from original_packs
        self.current_packs = self
            .original_packs
            .iter()
            .map(|player_packs| player_packs[self.current_pack_round].clone())
            .collect();

        true
    }

    /// Check if the entire draft is complete (all 3 rounds done).
    pub fn is_draft_complete(&self) -> bool {
        self.current_pack_round >= 2 && self.is_pack_round_complete()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::{generate_draft_packs, SheetData};
    use crate::set_data::SetData;
    use std::path::PathBuf;

    fn load_sheets() -> SheetData {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("data/sets/isd.json");
        let data = SetData::load(&path).expect("Failed to load ISD set data");
        SheetData::from_set_data(&data).expect("Failed to build sheet data")
    }

    #[test]
    fn test_full_4_player_draft() {
        let sheets = load_sheets();
        let mut rng = rand::thread_rng();
        let packs = generate_draft_packs(&sheets, 4, &mut rng);
        let mut draft = DraftState::new(packs);

        // Run all 3 pack rounds
        for round in 0..3 {
            if round > 0 {
                assert!(draft.start_next_pack_round());
            }

            // Each round: pick until packs are empty
            let initial_cards = draft.cards_remaining(0);
            for _pick in 0..initial_cards {
                // Each player picks the first available card
                for seat in 0..4 {
                    let card = draft.current_pack_for(seat)[0].clone();
                    draft.make_pick(seat, &card).unwrap();
                }
                draft.rotate_packs();
            }

            assert!(draft.is_pack_round_complete());
        }

        assert!(draft.is_draft_complete());

        // Each player should have picked from all 3 packs
        for player in &draft.players {
            // Pool size = sum of cards in each of the 3 packs
            assert!(
                player.pool.len() >= 40,
                "Player {} has only {} cards (expected ~42-45)",
                player.seat,
                player.pool.len()
            );
        }
    }

    #[test]
    fn test_pack_rotation_direction() {
        let sheets = load_sheets();
        let mut rng = rand::thread_rng();
        let packs = generate_draft_packs(&sheets, 4, &mut rng);
        let mut draft = DraftState::new(packs);

        // After all players pick from pack 1, packs should pass left
        // Player 0's pack should go to player 1
        let pack0_cards: Vec<String> = draft.current_pack_for(0).to_vec();

        // All players pick their first card
        for seat in 0..4 {
            let card = draft.current_pack_for(seat)[0].clone();
            draft.make_pick(seat, &card).unwrap();
        }
        draft.rotate_packs();

        // Player 1 should now have the remains of player 0's pack
        let player1_pack: Vec<String> = draft.current_pack_for(1).to_vec();
        // The pack that was in front of player 0 (minus the picked card)
        // should now be in front of player 1
        let pack0_remaining: Vec<String> = pack0_cards[1..].to_vec();
        assert_eq!(player1_pack, pack0_remaining, "Pack 1 should pass left");
    }

    #[test]
    fn test_invalid_pick() {
        let sheets = load_sheets();
        let mut rng = rand::thread_rng();
        let packs = generate_draft_packs(&sheets, 4, &mut rng);
        let mut draft = DraftState::new(packs);

        let result = draft.make_pick(0, "Nonexistent Card");
        assert!(result.is_err());
    }
}
