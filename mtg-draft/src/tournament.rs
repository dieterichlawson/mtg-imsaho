use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct TournamentConfig {
    pub best_of: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatchResult {
    pub player_a: usize,
    pub player_b: usize,
    pub wins_a: usize,
    pub wins_b: usize,
    pub games: Vec<GameOutcome>,
}

impl MatchResult {
    pub fn winner(&self) -> Option<usize> {
        match self.wins_a.cmp(&self.wins_b) {
            std::cmp::Ordering::Greater => Some(self.player_a),
            std::cmp::Ordering::Less => Some(self.player_b),
            std::cmp::Ordering::Equal => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GameOutcome {
    pub winner: Option<usize>,
    pub turns: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub game_log: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Standing {
    pub seat: usize,
    pub match_wins: usize,
    pub match_losses: usize,
    pub match_draws: usize,
    pub game_wins: usize,
    pub game_losses: usize,
}

impl Standing {
    pub fn match_points(&self) -> usize {
        self.match_wins * 3 + self.match_draws
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TournamentRound {
    pub round_number: usize,
    pub pairings: Vec<(usize, usize)>,
    pub results: Vec<MatchResult>,
}

/// Swiss tournament manager.
pub struct Tournament {
    pub config: TournamentConfig,
    pub num_players: usize,
    pub rounds: Vec<TournamentRound>,
    pub standings: Vec<Standing>,
    /// Track which pairs have already played to avoid rematches.
    played_pairs: Vec<(usize, usize)>,
}

impl Tournament {
    pub fn new(num_players: usize, config: TournamentConfig) -> Self {
        let standings = (0..num_players)
            .map(|seat| Standing {
                seat,
                match_wins: 0,
                match_losses: 0,
                match_draws: 0,
                game_wins: 0,
                game_losses: 0,
            })
            .collect();

        Self {
            config,
            num_players,
            rounds: Vec::new(),
            standings,
            played_pairs: Vec::new(),
        }
    }

    /// Number of Swiss rounds: ceil(log2(num_players)).
    pub fn total_rounds(&self) -> usize {
        if self.num_players <= 1 {
            return 0;
        }
        let mut rounds = 0;
        let mut n = 1;
        while n < self.num_players {
            n *= 2;
            rounds += 1;
        }
        rounds
    }

    /// Generate pairings for the next round.
    /// Swiss: sort by match points descending, pair adjacent players.
    /// Avoid rematches when possible. Lowest-ranked odd player gets a bye.
    pub fn generate_pairings(&self) -> Vec<(usize, usize)> {
        // Sort players by match points (descending), then by seat (tiebreak)
        let mut ranked: Vec<usize> = (0..self.num_players).collect();
        ranked.sort_by(|&a, &b| {
            self.standings[b]
                .match_points()
                .cmp(&self.standings[a].match_points())
                .then(a.cmp(&b))
        });

        let mut pairings = Vec::new();
        let mut paired = vec![false; self.num_players];

        // Pair top-down, avoiding rematches when possible
        for i in 0..ranked.len() {
            if paired[ranked[i]] {
                continue;
            }

            // Find the best available opponent (next unpaired, prefer no rematch)
            let mut best_j = None;
            let mut best_j_rematch = None;

            for j in (i + 1)..ranked.len() {
                if paired[ranked[j]] {
                    continue;
                }

                let pair = normalize_pair(ranked[i], ranked[j]);
                if self.played_pairs.contains(&pair) {
                    if best_j_rematch.is_none() {
                        best_j_rematch = Some(j);
                    }
                } else {
                    best_j = Some(j);
                    break;
                }
            }

            let opponent_idx = best_j.or(best_j_rematch);

            if let Some(j) = opponent_idx {
                pairings.push((ranked[i], ranked[j]));
                paired[ranked[i]] = true;
                paired[ranked[j]] = true;
            }
            // If no opponent found, this player gets a bye (handled below)
        }

        // Handle byes: any unpaired player gets a bye (paired with usize::MAX sentinel)
        for (seat, is_paired) in paired.iter().enumerate().take(self.num_players) {
            if !is_paired {
                pairings.push((seat, usize::MAX));
            }
        }

        pairings
    }

    /// Record the results of a completed round.
    pub fn record_round(&mut self, pairings: Vec<(usize, usize)>, results: Vec<MatchResult>) {
        let round_number = self.rounds.len() + 1;

        for result in &results {
            // Update standings
            let a = result.player_a;
            let b = result.player_b;

            self.standings[a].game_wins += result.wins_a;
            self.standings[a].game_losses += result.wins_b;
            self.standings[b].game_wins += result.wins_b;
            self.standings[b].game_losses += result.wins_a;

            match result.winner() {
                Some(w) if w == a => {
                    self.standings[a].match_wins += 1;
                    self.standings[b].match_losses += 1;
                }
                Some(_) => {
                    self.standings[b].match_wins += 1;
                    self.standings[a].match_losses += 1;
                }
                None => {
                    self.standings[a].match_draws += 1;
                    self.standings[b].match_draws += 1;
                }
            }

            self.played_pairs.push(normalize_pair(a, b));
        }

        // Handle byes (sentinel value usize::MAX)
        for &(a, b) in &pairings {
            if b == usize::MAX {
                // Bye: auto-win for player a
                self.standings[a].match_wins += 1;
                self.standings[a].game_wins += self.config.best_of / 2 + 1;
            }
        }

        self.rounds.push(TournamentRound {
            round_number,
            pairings,
            results,
        });
    }

    /// Check if the tournament is complete.
    pub fn is_complete(&self) -> bool {
        self.rounds.len() >= self.total_rounds()
    }

    /// Return standings sorted by match points (descending).
    pub fn sorted_standings(&self) -> Vec<Standing> {
        let mut sorted = self.standings.clone();
        sorted.sort_by(|a, b| {
            b.match_points()
                .cmp(&a.match_points())
                .then(b.game_wins.cmp(&a.game_wins))
                .then(a.game_losses.cmp(&b.game_losses))
        });
        sorted
    }
}

fn normalize_pair(a: usize, b: usize) -> (usize, usize) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_total_rounds() {
        assert_eq!(Tournament::new(4, TournamentConfig { best_of: 3 }).total_rounds(), 2);
        assert_eq!(Tournament::new(8, TournamentConfig { best_of: 3 }).total_rounds(), 3);
        assert_eq!(Tournament::new(2, TournamentConfig { best_of: 1 }).total_rounds(), 1);
    }

    #[test]
    fn test_pairings_4_players() {
        let tournament = Tournament::new(4, TournamentConfig { best_of: 3 });
        let pairings = tournament.generate_pairings();

        // 4 players, no byes, 2 matches
        assert_eq!(pairings.len(), 2);
        let mut all_players: Vec<usize> = pairings
            .iter()
            .flat_map(|&(a, b)| vec![a, b])
            .collect();
        all_players.sort_unstable();
        assert_eq!(all_players, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_pairings_odd_players() {
        let tournament = Tournament::new(5, TournamentConfig { best_of: 3 });
        let pairings = tournament.generate_pairings();

        // 5 players: 2 matches + 1 bye
        assert_eq!(pairings.len(), 3);
        let bye_pair = pairings.iter().find(|&&(_, b)| b == usize::MAX);
        assert!(bye_pair.is_some(), "Should have a bye");
    }

    #[test]
    fn test_record_round() {
        let mut tournament = Tournament::new(4, TournamentConfig { best_of: 3 });
        let pairings = vec![(0, 1), (2, 3)];
        let results = vec![
            MatchResult {
                player_a: 0,
                player_b: 1,
                wins_a: 2,
                wins_b: 1,
                games: vec![
                    GameOutcome { winner: Some(0), turns: 10, game_log: vec![] },
                    GameOutcome { winner: Some(1), turns: 8, game_log: vec![] },
                    GameOutcome { winner: Some(0), turns: 12, game_log: vec![] },
                ],
            },
            MatchResult {
                player_a: 2,
                player_b: 3,
                wins_a: 0,
                wins_b: 2,
                games: vec![
                    GameOutcome { winner: Some(3), turns: 7, game_log: vec![] },
                    GameOutcome { winner: Some(3), turns: 9, game_log: vec![] },
                ],
            },
        ];

        tournament.record_round(pairings, results);

        assert_eq!(tournament.standings[0].match_wins, 1);
        assert_eq!(tournament.standings[0].game_wins, 2);
        assert_eq!(tournament.standings[1].match_losses, 1);
        assert_eq!(tournament.standings[3].match_wins, 1);
        assert_eq!(tournament.standings[3].game_wins, 2);
    }

    #[test]
    fn test_avoid_rematches() {
        let mut tournament = Tournament::new(4, TournamentConfig { best_of: 3 });

        // Round 1: 0v1, 2v3
        tournament.record_round(
            vec![(0, 1), (2, 3)],
            vec![
                MatchResult {
                    player_a: 0, player_b: 1, wins_a: 2, wins_b: 0,
                    games: vec![
                        GameOutcome { winner: Some(0), turns: 5, game_log: vec![] },
                        GameOutcome { winner: Some(0), turns: 5, game_log: vec![] },
                    ],
                },
                MatchResult {
                    player_a: 2, player_b: 3, wins_a: 2, wins_b: 0,
                    games: vec![
                        GameOutcome { winner: Some(2), turns: 5, game_log: vec![] },
                        GameOutcome { winner: Some(2), turns: 5, game_log: vec![] },
                    ],
                },
            ],
        );

        // Round 2: should pair 0v2 and 1v3 (avoiding 0v1 and 2v3 rematches)
        let pairings = tournament.generate_pairings();
        for &(a, b) in &pairings {
            assert!(
                !((a == 0 && b == 1) || (a == 1 && b == 0)),
                "Should avoid 0v1 rematch"
            );
            assert!(
                !((a == 2 && b == 3) || (a == 3 && b == 2)),
                "Should avoid 2v3 rematch"
            );
        }
    }
}
