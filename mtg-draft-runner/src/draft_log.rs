use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

/// A streaming log writer that appends human-readable entries as the draft progresses.
/// Thread-safe via Mutex so parallel operations can log safely.
pub struct DraftLogger {
    file: Mutex<File>,
}

impl DraftLogger {
    pub fn new(path: &Path) -> Self {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .unwrap_or_else(|e| panic!("Failed to create log file {}: {}", path.display(), e));
        Self {
            file: Mutex::new(file),
        }
    }

    pub fn header(&self, set_name: &str, players: usize, best_of: usize, model: &str) {
        self.write(&format!(
            "╔══════════════════════════════════════════════════════════╗\n\
             ║  {} Draft — {} players, best-of-{}, model: {}\n\
             ╚══════════════════════════════════════════════════════════╝\n\n",
            set_name, players, best_of, model
        ));
    }

    pub fn section(&self, title: &str) {
        let bar = "═".repeat(60);
        self.write(&format!("\n{}\n  {}\n{}\n\n", bar, title, bar));
    }

    pub fn subsection(&self, title: &str) {
        self.write(&format!("\n--- {} ---\n\n", title));
    }

    pub fn pack_contents(&self, seat: usize, pack_num: usize, cards: &[String]) {
        self.write(&format!("Seat {} — Pack {} ({} cards):\n", seat, pack_num, cards.len()));
        for (i, card) in cards.iter().enumerate() {
            let name = card.split(" // ").next().unwrap_or(card);
            self.write(&format!("  {:2}. {}\n", i, name));
        }
        self.write("\n");
    }

    pub fn draft_pick(
        &self,
        seat: usize,
        pack: usize,
        pick: usize,
        available: &[String],
        chosen: &str,
        response: &str,
    ) {
        let chosen_name = chosen.split(" // ").next().unwrap_or(chosen);
        self.write(&format!(
            "Seat {} | Pack {} Pick {} | Chose: {} (from {} cards)\n",
            seat, pack, pick, chosen_name, available.len()
        ));
        // Log the LLM's reasoning (indent it)
        for line in response.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                self.write(&format!("  > {}\n", trimmed));
            }
        }
        self.write("\n");
    }

    pub fn pool_summary(&self, seat: usize, pool: &[String]) {
        self.write(&format!("Seat {} — Final pool ({} cards):\n", seat, pool.len()));
        for card in pool {
            let name = card.split(" // ").next().unwrap_or(card);
            self.write(&format!("  - {}\n", name));
        }
        self.write("\n");
    }

    pub fn deck_building(
        &self,
        seat: usize,
        maindeck: &[String],
        lands: &std::collections::HashMap<String, u32>,
        sideboard: &[String],
        response: &str,
        retries: usize,
    ) {
        let total = maindeck.len() + lands.values().sum::<u32>() as usize;
        self.write(&format!(
            "Seat {} — Deck ({} cards, {} retries)\n",
            seat, total, retries
        ));

        self.write("  Maindeck:\n");
        for card in maindeck {
            self.write(&format!("    {}\n", card));
        }

        self.write("  Lands:\n");
        let mut lands_sorted: Vec<_> = lands.iter().collect();
        lands_sorted.sort_by_key(|(name, _)| (*name).clone());
        for (name, count) in lands_sorted {
            self.write(&format!("    {} {}\n", count, name));
        }

        if !sideboard.is_empty() {
            self.write(&format!("  Sideboard ({} cards):\n", sideboard.len()));
            for card in sideboard {
                let name = card.split(" // ").next().unwrap_or(card);
                self.write(&format!("    {}\n", name));
            }
        }

        self.write("\n  LLM reasoning:\n");
        for line in response.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                self.write(&format!("    > {}\n", trimmed));
            }
        }
        self.write("\n");
    }

    pub fn match_result(
        &self,
        round: usize,
        seat_a: usize,
        seat_b: usize,
        wins_a: usize,
        wins_b: usize,
        winner: Option<usize>,
    ) {
        let winner_str = winner
            .map(|w| format!("Seat {} wins", w))
            .unwrap_or_else(|| "Draw".to_string());
        self.write(&format!(
            "Round {} — Seat {} vs Seat {}: {}-{} ({})\n",
            round, seat_a, seat_b, wins_a, wins_b, winner_str
        ));
    }

    pub fn game_log(&self, _round: usize, game_num: usize, seat_a: usize, seat_b: usize, log: &[String]) {
        self.write(&format!(
            "\n  Game {} (Seat {} vs Seat {}):\n",
            game_num, seat_a, seat_b
        ));
        for entry in log {
            self.write(&format!("    {}\n", entry));
        }
        self.write("\n");
    }

    pub fn bye(&self, round: usize, seat: usize) {
        self.write(&format!("Round {} — Seat {} gets a bye\n", round, seat));
    }

    pub fn standings(&self, standings: &[(usize, usize, usize, usize)]) {
        self.write("Final Standings:\n");
        for (rank, &(seat, match_wins, match_losses, game_wins)) in standings.iter().enumerate() {
            self.write(&format!(
                "  {}. Seat {} — {}-{} ({} game wins)\n",
                rank + 1,
                seat,
                match_wins,
                match_losses,
                game_wins
            ));
        }
        self.write("\n");
    }

    fn write(&self, text: &str) {
        if let Ok(mut f) = self.file.lock() {
            let _ = f.write_all(text.as_bytes());
            let _ = f.flush();
        }
    }
}
