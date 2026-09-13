use serde::Serialize;

/// The opponent a bye is paired against: a seat number no pod can contain.
pub const BYE: usize = usize::MAX;

#[derive(Debug, Clone, Serialize)]
pub struct TournamentConfig {
    pub best_of: usize,
}

/// Game wins that decide a match of `best_of` games: more than half of them.
///
/// A match also ends once `best_of` games have been played, drawn games
/// included (MTR 6.5), so this target is not always reached — a match that
/// runs out of games with the wins level is a drawn match.
#[must_use]
pub fn wins_needed(best_of: usize) -> usize {
    best_of / 2 + 1
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
    #[must_use]
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
    /// How many of `match_wins` are byes rather than matches played.
    pub byes: usize,
}

impl Standing {
    #[must_use]
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
    #[must_use]
    pub fn new(num_players: usize, config: TournamentConfig) -> Self {
        let standings = (0..num_players)
            .map(|seat| Standing {
                seat,
                match_wins: 0,
                match_losses: 0,
                match_draws: 0,
                game_wins: 0,
                game_losses: 0,
                byes: 0,
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

    /// Number of Swiss rounds: `ceil(log2(num_players))`.
    #[must_use]
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
    ///
    /// Seats are ranked by match points (descending), seat number breaking
    /// ties, and paired in rank order. Two properties the greedy first-fit
    /// scan this replaced could not hold (#480, #482):
    ///
    /// - **No rematch unless the field forces one.** Pairing is a search over
    ///   the whole field with backtracking, so a choice high in the standings
    ///   that would strand the last two seats in a rematch is undone instead
    ///   of shipped. A rematch appears only when every way of pairing the
    ///   field contains one, and then the pairing chosen has the fewest.
    /// - **The bye is chosen, not left over.** It goes to the lowest-ranked
    ///   seat that has not already had one. A seat never takes a second bye
    ///   while some seat has had none, whatever that costs in rematches: only
    ///   seats on the fewest byes are candidates at all. Among them the
    ///   lowest-ranked one that admits a rematch-free pairing of the rest
    ///   takes it, and if none does, the lowest-ranked candidate does and the
    ///   round takes its minimum number of rematches.
    #[must_use]
    pub fn generate_pairings(&self) -> Vec<(usize, usize)> {
        let ranked = self.ranked_seats();

        if ranked.len() % 2 == 0 {
            return self.best_matching(&ranked).1;
        }

        // Odd field: someone sits out. Only seats on the fewest byes are
        // eligible, lowest-ranked first.
        let fewest_byes = ranked
            .iter()
            .map(|&seat| self.standings[seat].byes)
            .min()
            .unwrap_or(0);
        let candidates = ranked
            .iter()
            .rev()
            .copied()
            .filter(|&seat| self.standings[seat].byes == fewest_byes);

        let mut fallback: Option<(usize, Vec<(usize, usize)>)> = None;
        for bye_seat in candidates {
            let rest: Vec<usize> = ranked.iter().copied().filter(|&s| s != bye_seat).collect();
            let (rematches, mut pairings) = self.best_matching(&rest);
            pairings.push((bye_seat, BYE));
            if rematches == 0 {
                return pairings;
            }
            if fallback.is_none() {
                fallback = Some((rematches, pairings));
            }
        }

        fallback.map_or_else(Vec::new, |(_, pairings)| pairings)
    }

    /// Seats in pairing order: match points descending, seat number ascending.
    fn ranked_seats(&self) -> Vec<usize> {
        let mut ranked: Vec<usize> = (0..self.num_players).collect();
        ranked.sort_by(|&a, &b| {
            self.standings[b]
                .match_points()
                .cmp(&self.standings[a].match_points())
                .then(a.cmp(&b))
        });
        ranked
    }

    /// Pair every seat in `seats` (which must be rank-ordered and of even
    /// length), minimizing rematches. Returns the number of rematches in the
    /// pairing along with the pairing itself.
    fn best_matching(&self, seats: &[usize]) -> (usize, Vec<(usize, usize)>) {
        // The search explores partners in rank order, so the first pairing it
        // reaches is the greedy one and the first *best* one it reaches is the
        // most Swiss-like pairing at that cost.
        let mut used = vec![false; seats.len()];
        let mut current = Vec::with_capacity(seats.len() / 2);
        let mut best = None;
        // A field of n seats has (n-1)!! pairings. Small pods search all of
        // them in microseconds; the budget only matters for a pod big enough
        // that exhaustive search would hang, where it stops at the best
        // pairing found so far. The first descent already yields a complete
        // pairing, so there is always one to fall back on.
        let mut budget = 200_000_usize;
        self.search_matching(seats, &mut used, 0, &mut current, &mut best, &mut budget);
        best.unwrap_or_else(|| (0, Vec::new()))
    }

    fn search_matching(
        &self,
        seats: &[usize],
        used: &mut [bool],
        cost: usize,
        current: &mut Vec<(usize, usize)>,
        best: &mut Option<(usize, Vec<(usize, usize)>)>,
        budget: &mut usize,
    ) {
        if let Some((best_cost, _)) = best {
            // Cost only grows as the pairing is extended, so a branch already
            // at the best cost cannot beat it.
            if cost >= *best_cost {
                return;
            }
        }
        let Some(i) = (0..seats.len()).find(|&i| !used[i]) else {
            *best = Some((cost, current.clone()));
            return;
        };

        used[i] = true;
        for j in (i + 1)..seats.len() {
            if used[j] || *budget == 0 {
                continue;
            }
            *budget -= 1;
            let rematch = self.played_pairs.contains(&normalize_pair(seats[i], seats[j]));
            used[j] = true;
            current.push((seats[i], seats[j]));
            self.search_matching(seats, used, cost + usize::from(rematch), current, best, budget);
            current.pop();
            used[j] = false;
        }
        used[i] = false;
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

        // Handle byes (sentinel value BYE)
        for &(a, b) in &pairings {
            if b == BYE {
                // Bye: auto-win for player a, scored 2-0 the way MTR 6.4 does.
                // Counted separately as well, so the standings can say which
                // wins were played and which were awarded.
                self.standings[a].match_wins += 1;
                self.standings[a].game_wins += wins_needed(self.config.best_of);
                self.standings[a].byes += 1;
            }
        }

        self.rounds.push(TournamentRound {
            round_number,
            pairings,
            results,
        });
    }

    /// Check if the tournament is complete.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.rounds.len() >= self.total_rounds()
    }

    /// Return standings sorted by match points (descending).
    #[must_use]
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

    /// Results for a played match, shaped so the standings arithmetic is
    /// exercised without a game engine.
    fn mr(a: usize, b: usize, wins_a: usize, wins_b: usize) -> MatchResult {
        MatchResult {
            player_a: a,
            player_b: b,
            wins_a,
            wins_b,
            games: (0..(wins_a + wins_b))
                .map(|_| GameOutcome { winner: None, turns: 1, game_log: vec![] })
                .collect(),
        }
    }

    /// Every unordered pair in a pairing list, byes dropped.
    fn played(pairings: &[(usize, usize)]) -> Vec<(usize, usize)> {
        pairings
            .iter()
            .filter(|&&(_, b)| b != BYE)
            .map(|&(a, b)| normalize_pair(a, b))
            .collect()
    }

    /// #480: the greedy first-fit scan paired the top of the standings without
    /// looking ahead, so the last two unpaired seats were handed a rematch
    /// even when the whole field could have been paired without one.
    #[test]
    fn round_3_does_not_repeat_a_round_2_pairing() {
        let mut t = Tournament::new(6, TournamentConfig { best_of: 1 });
        t.record_round(
            vec![(0, 1), (2, 3), (4, 5)],
            vec![mr(0, 1, 1, 0), mr(2, 3, 1, 0), mr(4, 5, 1, 0)],
        );
        t.record_round(
            vec![(0, 2), (4, 1), (3, 5)],
            vec![mr(0, 2, 1, 0), mr(4, 1, 1, 0), mr(3, 5, 1, 0)],
        );

        // Played: 0-1 2-3 4-5 0-2 1-4 3-5.  Points: 0:6 4:6 2:3 3:3 1:0 5:0.
        // (3,5) was the greedy scan's leftover pair and is a rematch; a
        // rematch-free pairing of the whole field exists.
        let pairings = t.generate_pairings();
        for pair in played(&pairings) {
            assert!(
                !t.played_pairs.contains(&pair),
                "round 3 pairs {pair:?} again: {pairings:?}"
            );
        }
    }

    /// #482: the bye was whatever seat the pairing scan happened to leave
    /// over, so one seat could take two byes while another had none, and a
    /// seat on points could take one while a pointless seat played.
    #[test]
    fn the_bye_goes_to_the_lowest_ranked_seat_that_has_not_had_one() {
        let mut t = Tournament::new(5, TournamentConfig { best_of: 1 });

        let r1 = t.generate_pairings();
        assert_eq!(r1, vec![(0, 1), (2, 3), (4, BYE)]);
        t.record_round(r1, vec![mr(0, 1, 1, 0), mr(2, 3, 1, 0)]);

        let r2 = t.generate_pairings();
        assert_eq!(r2, vec![(0, 2), (4, 1), (3, BYE)]);
        t.record_round(r2, vec![mr(0, 2, 1, 0), mr(4, 1, 1, 0)]);

        // Points 0:6 4:6 2:3 3:3 1:0.  Seats 4 and 3 have had the byes, so
        // neither is eligible; seat 1 is the lowest-ranked seat that has not.
        let r3 = t.generate_pairings();
        assert_eq!(r3.iter().find(|&&(_, b)| b == BYE), Some(&(1, BYE)));
        for pair in played(&r3) {
            assert!(!t.played_pairs.contains(&pair), "round 3 rematch: {r3:?}");
        }
    }

    /// Both properties over a swept field rather than one arranged tournament:
    /// no round takes a rematch that some pairing of that field could have
    /// avoided (checked by brute force over every bye choice and every perfect
    /// matching), and no seat takes a second bye while a seat has had none.
    #[test]
    fn swept_tournaments_avoid_avoidable_rematches_and_spread_byes() {
        /// Every rematch-free way to pair `seats`, or rather whether one
        /// exists at all.
        fn rematch_free_matching_exists(seats: &[usize], played_pairs: &[(usize, usize)]) -> bool {
            let Some((&first, rest)) = seats.split_first() else {
                return true;
            };
            rest.iter().enumerate().any(|(i, &other)| {
                if played_pairs.contains(&normalize_pair(first, other)) {
                    return false;
                }
                let remaining: Vec<usize> = rest
                    .iter()
                    .enumerate()
                    .filter(|&(j, _)| j != i)
                    .map(|(_, &s)| s)
                    .collect();
                rematch_free_matching_exists(&remaining, played_pairs)
            })
        }

        let mut rng: u64 = 0x2026_0913;
        let mut next = move || {
            rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            (rng >> 33) as usize
        };

        for players in 2..=9 {
            for _ in 0..200 {
                let mut t = Tournament::new(players, TournamentConfig { best_of: 3 });
                while !t.is_complete() {
                    let pairings = t.generate_pairings();

                    // Every seat appears exactly once, as a player or a bye.
                    let mut seen: Vec<usize> = pairings
                        .iter()
                        .flat_map(|&(a, b)| if b == BYE { vec![a] } else { vec![a, b] })
                        .collect();
                    seen.sort_unstable();
                    assert_eq!(seen, (0..players).collect::<Vec<_>>(), "{pairings:?}");

                    // A rematch only where the field left no choice.
                    let rematches = played(&pairings)
                        .iter()
                        .filter(|pair| t.played_pairs.contains(pair))
                        .count();
                    if rematches > 0 {
                        let seats: Vec<usize> = (0..players).collect();
                        let avoidable = if players % 2 == 0 {
                            rematch_free_matching_exists(&seats, &t.played_pairs)
                        } else {
                            seats.iter().any(|&bye| {
                                let rest: Vec<usize> =
                                    seats.iter().copied().filter(|&s| s != bye).collect();
                                rematch_free_matching_exists(&rest, &t.played_pairs)
                            })
                        };
                        assert!(
                            !avoidable,
                            "{players} seats: {pairings:?} rematches although a \
                             rematch-free pairing of the field exists"
                        );
                    }

                    // The bye goes to a seat on the fewest byes so far.
                    if let Some(&(bye_seat, _)) = pairings.iter().find(|&&(_, b)| b == BYE) {
                        let fewest = (0..players)
                            .map(|s| t.standings[s].byes)
                            .min()
                            .unwrap_or(0);
                        assert_eq!(
                            t.standings[bye_seat].byes, fewest,
                            "seat {bye_seat} takes a bye on {} while some seat is on {fewest}",
                            t.standings[bye_seat].byes
                        );
                    }

                    let results = pairings
                        .iter()
                        .filter(|&&(_, b)| b != BYE)
                        .map(|&(a, b)| match next() % 3 {
                            0 => mr(a, b, 2, 0),
                            1 => mr(a, b, 1, 2),
                            _ => mr(a, b, 1, 1),
                        })
                        .collect();
                    t.record_round(pairings, results);
                }

                // No seat ends the event with two byes while another has none.
                let byes: Vec<usize> = (0..players).map(|s| t.standings[s].byes).collect();
                let (lo, hi) = (byes.iter().min().unwrap(), byes.iter().max().unwrap());
                assert!(hi - lo <= 1, "{players} seats, byes {byes:?}");
            }
        }
    }

    /// #486: a bye is a match win, but the standings have to be able to say so
    /// — the counter is what the printed `(1 bye)` marker reads.
    #[test]
    fn a_bye_is_counted_as_a_bye_as_well_as_a_match_win() {
        let mut t = Tournament::new(3, TournamentConfig { best_of: 1 });
        t.record_round(vec![(0, 1), (2, BYE)], vec![mr(0, 1, 1, 0)]);

        assert_eq!(t.standings[2].match_wins, 1);
        assert_eq!(t.standings[2].byes, 1);
        assert_eq!(t.standings[2].game_wins, 1);
        // A match that was played is not a bye.
        assert_eq!(t.standings[0].match_wins, 1);
        assert_eq!(t.standings[0].byes, 0);
        assert_eq!(t.standings[1].byes, 0);
    }

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
        let bye_pair = pairings.iter().find(|&&(_, b)| b == BYE);
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
