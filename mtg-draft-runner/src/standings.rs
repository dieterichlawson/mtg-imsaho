//! The standings as both the runner and the lobby print them: one row per
//! seat, with the qualifiers a row has to carry (byes, substituted
//! answers, a substitute deck, forfeits), and the per-match score line.

use std::fmt::Write as _;

use mtg_draft::tournament::{GameOutcome, MatchResult, Standing};

/// One row of the final standings, written once and printed by both surfaces
/// that show them — stderr and the log's FINAL STANDINGS block.
///
/// The row carries the seat's full match record and marks the wins that were
/// byes rather than matches played. Without the marker a seat that sat out a
/// round reads exactly like a seat that beat somebody, and the block does not
/// reconcile against the matches above it (issue #486, the shape of #195 and
/// #200).
#[must_use]
pub fn standings_row(rank: usize, s: &Standing, tags: &RowTags) -> String {
    standings_row_with(rank, s, tags, &|seat| format!("Seat {seat}"))
}

/// `standings_row` with the seat named by `who`: the hosted table's
/// terminal names a person as the page does, "Ann (seat 3)", where the log
/// and the runner keep `Seat N`, which their readers parse (#816).
#[must_use]
pub fn standings_row_with(rank: usize, s: &Standing, tags: &RowTags, who: &dyn Fn(usize) -> String) -> String {
    let draws = if s.match_draws > 0 {
        format!("-{}", s.match_draws)
    } else {
        String::new()
    };
    format!(
        "{}. {} — {}-{}{draws} ({} game win{}{}){}",
        rank, who(s.seat), s.match_wins, s.match_losses, s.game_wins, if s.game_wins == 1 { "" } else { "s" },
        tiebreakers(s), row_notes(s, tags),
    )
}

/// "; OMW 55.6%, GW 85.7%, OGW 47.2%": the tiebreakers that ordered seats
/// level on points, said where the order is read. A standing that has not
/// been through `sorted_standings` has none to say (every one it fills is
/// at least the 33% floor).
#[must_use]
pub fn tiebreakers(s: &Standing) -> String {
    if s.omw <= 0.0 {
        return String::new();
    }
    format!("; OMW {:.1}%, GW {:.1}%, OGW {:.1}%", 100.0 * s.omw, 100.0 * s.gw, 100.0 * s.ogw)
}

/// Everything a standings row says beside the numbers: " [1 bye]" and the
/// tags. The terminal, the log and the hosted table's view all print this,
/// so none of them can drop what another says (#486, #744).
#[must_use]
pub fn row_notes(s: &Standing, tags: &RowTags) -> String {
    let byes = match s.byes {
        0 => String::new(),
        1 => " [1 bye]".to_string(),
        n => format!(" [{n} byes]"),
    };
    format!("{byes}{}", tags.render())
}

/// What a standings row says about a result that is not wholly the seat's
/// own, in the same `[...]` form as a bye (#486). The sections after the
/// standings say which and why; the row is where a reader is looking when
/// it ranks the seat (issue #588).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RowTags {
    /// Answers the harness could not use and chose for the seat.
    pub answers_substituted: u64,
    /// Decisions the seat's backend never answered at all (#587).
    pub never_answered: u64,
    /// The seat's deck was substituted: its builder failed (#200), or at
    /// the hosted table it submitted none before its timer ran out (#747).
    /// "runner-built" was wrong there: the table built it.
    pub runner_built_deck: bool,
    /// Games the watchdog forfeited for this seat (#488).
    pub games_forfeited: usize,
    /// Matches carried over from a snapshot rather than played by this
    /// process (#581).
    pub matches_from_snapshot: usize,
}

impl RowTags {
    /// The row's tags, " [1 game forfeited] [substitute deck]", as the
    /// terminal, the log and the hosted table's view all print them (#744).
    #[must_use]
    pub fn render(&self) -> String {
        let plural = |n: u64, one: &str, many: &str| if n == 1 { one.to_string() } else { many.to_string() };
        let mut out = String::new();
        if self.answers_substituted > 0 {
            let n = self.answers_substituted;
            let _ = write!(out, " [{n} {} substituted]", plural(n, "answer", "answers"));
        }
        if self.never_answered > 0 {
            let n = self.never_answered;
            let _ = write!(out, " [{n} {} never answered]", plural(n, "decision", "decisions"));
        }
        if self.runner_built_deck {
            out.push_str(" [substitute deck]");
        }
        if self.games_forfeited > 0 {
            let n = self.games_forfeited as u64;
            let _ = write!(out, " [{n} {} forfeited]", plural(n, "game", "games"));
        }
        if self.matches_from_snapshot > 0 {
            let n = self.matches_from_snapshot as u64;
            let _ = write!(out, " [{n} {} from snapshot]", plural(n, "match", "matches"));
        }
        out
    }
}

/// What the score line adds about the match's games that were not played
/// out: a forfeit is a seat the watchdog caught (#488), or one the hosted
/// table forfeited before a game began, whose log says why ("forfeit: the
/// seat is away", "forfeit: no game page for seat 0: ..." — #759, which read
/// "a seat stalled" when nothing had); an abandoned game is one the runner
/// stopped at its action budget with no winner (#630).
#[must_use]
pub fn unplayed_games_note(games: &[GameOutcome]) -> String {
    let count = |n: usize, what: &str| match n {
        0 => String::new(),
        1 => format!(" [1 game {what}]"),
        n => format!(" [{n} games {what}]"),
    };
    let mut forfeits: Vec<(String, usize)> = Vec::new();
    for g in games.iter().filter(|g| g.stalled_seat.is_some()) {
        let why = g.game_log.first().and_then(|l| l.strip_prefix("forfeit: "))
            .unwrap_or("a seat stalled").to_string();
        match forfeits.iter_mut().find(|(w, _)| *w == why) {
            Some((_, n)) => *n += 1,
            None => forfeits.push((why, 1)),
        }
    }
    let abandoned = games.iter().filter(|g| g.abandoned).count();
    let mut out: String = forfeits.iter().map(|(why, n)| count(*n, &format!("forfeited: {why}"))).collect();
    out.push_str(&count(abandoned, "abandoned: the action budget ran out, no winner"));
    out
}

/// A match's score from one side: games won, games lost, and the drawn
/// games as a third figure when there were any, as MTR records a match
/// (wins-losses-draws). "2-0" for a match of a draw and two wins left the
/// draw in no count, and a match of three draws read "0-0", a match
/// nobody played (#919). An abandoned game is not a draw (#743): the
/// unplayed-games note counts it.
#[must_use]
pub fn match_score(won: usize, lost: usize, games: &[GameOutcome]) -> String {
    match games.iter().filter(|g| g.winner.is_none() && !g.abandoned).count() {
        0 => format!("{won}-{lost}"),
        drawn => format!("{won}-{lost}-{drawn}"),
    }
}

/// The per-match progress line on stderr. A forfeited game is a game nobody
/// played; the score line is where a reader is looking when it happens
/// (#488). A level match has no winner to name (#650).
#[must_use]
pub fn match_score_line(result: &MatchResult) -> String {
    match_score_line_with(result, &|seat| format!("Seat {seat}"))
}

/// `match_score_line` with the seats named by `who` (#816).
#[must_use]
pub fn match_score_line_with(result: &MatchResult, who: &dyn Fn(usize) -> String) -> String {
    let outcome = match result.winner() {
        Some(w) => format!("winner: {}", who(w)),
        None => "drawn".to_string(),
    };
    format!(
        "  {} vs {}: {} ({outcome}){}",
        who(result.player_a),
        who(result.player_b),
        match_score(result.wins_a, result.wins_b, &result.games),
        unplayed_games_note(&result.games),
    )
}


#[cfg(test)]
mod standings_row_tests {
    use super::{standings_row, RowTags, Standing};

    fn standing(seat: usize, match_wins: usize, match_losses: usize, game_wins: usize, byes: usize) -> Standing {
        Standing {
            seat,
            match_wins,
            match_losses,
            match_draws: 0,
            game_wins,
            game_losses: 0,
            byes,
            game_draws: 0,
            omw: 0.0,
            gw: 0.0,
            ogw: 0.0,
        }
    }

    /// #486: seat 0 went 1-1 in matches it played; seat 1 lost its only match
    /// and was given a bye. Both are "1-1" in the counters, so the row has to
    /// say which win was awarded — otherwise the seat that lost to seat 0
    /// prints identically to seat 0.
    #[test]
    fn a_bye_is_not_printed_as_a_won_match() {
        let played = standings_row(2, &standing(0, 1, 1, 1, 0), &RowTags::default());
        let byed = standings_row(3, &standing(1, 1, 1, 1, 1), &RowTags::default());

        assert_eq!(played, "2. Seat 0 — 1-1 (1 game win)");
        assert_eq!(byed, "3. Seat 1 — 1-1 (1 game win) [1 bye]");
    }

    #[test]
    fn several_byes_and_draws_are_both_reported() {
        let mut s = standing(4, 2, 1, 2, 2);
        s.match_draws = 1;
        assert_eq!(standings_row(1, &s, &RowTags::default()), "1. Seat 4 — 2-1-1 (2 game wins) [2 byes]");
    }

    /// Issue #588: a result that is not wholly the seat's own says so on
    /// the row the seat is ranked by, the way a bye does — not in a section
    /// under a heading about tokens.
    #[test]
    fn every_qualifier_is_on_the_row() {
        let s = standing(1, 3, 0, 6, 0);
        let one = |t: RowTags| standings_row(1, &s, &t);
        assert_eq!(one(RowTags { answers_substituted: 39, ..RowTags::default() }),
            "1. Seat 1 — 3-0 (6 game wins) [39 answers substituted]");
        assert_eq!(one(RowTags { never_answered: 1, ..RowTags::default() }),
            "1. Seat 1 — 3-0 (6 game wins) [1 decision never answered]");
        assert_eq!(one(RowTags { runner_built_deck: true, ..RowTags::default() }),
            "1. Seat 1 — 3-0 (6 game wins) [substitute deck]");
        assert_eq!(one(RowTags { games_forfeited: 1, ..RowTags::default() }),
            "1. Seat 1 — 3-0 (6 game wins) [1 game forfeited]");
        assert_eq!(one(RowTags { matches_from_snapshot: 2, ..RowTags::default() }),
            "1. Seat 1 — 3-0 (6 game wins) [2 matches from snapshot]");
        let mut byed = s.clone();
        byed.byes = 1;
        assert_eq!(standings_row(1, &byed, &RowTags { games_forfeited: 2, runner_built_deck: true, ..RowTags::default() }),
            "1. Seat 1 — 3-0 (6 game wins) [1 bye] [substitute deck] [2 games forfeited]");
    }
}

#[cfg(test)]
mod match_score_line_tests {
    use super::match_score_line;
    use mtg_draft::tournament::MatchResult;

    fn result(wins_a: usize, wins_b: usize) -> MatchResult {
        MatchResult { player_a: 0, player_b: 1, wins_a, wins_b, games: Vec::new() }
    }

    /// #650: a level match printed "(winner: Seat draw)".
    #[test]
    fn a_drawn_match_names_no_winner() {
        assert_eq!(match_score_line(&result(1, 1)), "  Seat 0 vs Seat 1: 1-1 (drawn)");
        assert_eq!(match_score_line(&result(0, 2)), "  Seat 0 vs Seat 1: 0-2 (winner: Seat 1)");
    }

    /// #919: the drawn games of a match were in no count. A draw and two
    /// wins read "2-0", three draws read "0-0" like a match nobody played.
    /// MTR's record is wins-losses-draws; an abandoned game is not a draw
    /// and keeps its own note.
    #[test]
    fn a_matchs_drawn_games_are_in_its_score() {
        let game = |winner: Option<usize>, abandoned: bool| mtg_draft::tournament::GameOutcome {
            winner, turns: 9, game_log: vec![], stalled_seat: None, abandoned, decks: None };
        let with = |wins_a, wins_b, games| MatchResult { games, ..result(wins_a, wins_b) };
        assert_eq!(match_score_line(&with(0, 2, vec![game(None, false), game(Some(1), false), game(Some(1), false)])),
            "  Seat 0 vs Seat 1: 0-2-1 (winner: Seat 1)");
        assert_eq!(match_score_line(&with(0, 0, vec![game(None, false); 3])), "  Seat 0 vs Seat 1: 0-0-3 (drawn)");
        assert_eq!(match_score_line(&with(1, 1, vec![game(Some(0), false), game(Some(1), false), game(None, true)])),
            "  Seat 0 vs Seat 1: 1-1 (drawn) [1 game abandoned: the action budget ran out, no winner]");
        assert_eq!(super::match_score(2, 0, &[game(Some(0), false), game(Some(0), false)]), "2-0");
    }
}
