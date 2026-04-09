use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

/// A streaming log writer that appends human-readable entries as the draft progresses.
/// Thread-safe via Mutex so parallel operations can log safely.
pub struct DraftLogger {
    file: Mutex<File>,
    start: Instant,
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
            start: Instant::now(),
        }
    }

    fn timestamp(&self) -> String {
        let elapsed = self.start.elapsed();
        let secs = elapsed.as_secs();
        let mins = secs / 60;
        let secs = secs % 60;
        format!("{:02}:{:02}", mins, secs)
    }

    pub fn header(&self, set_name: &str, players: usize, best_of: usize, model: &str) {
        self.raw_write(&format!(
            "╔══════════════════════════════════════════════════════════╗\n\
             ║  {} Draft — {} players, best-of-{}, model: {}\n\
             ╚══════════════════════════════════════════════════════════╝\n\n",
            set_name, players, best_of, model
        ));
    }

    pub fn section(&self, title: &str) {
        let bar = "═".repeat(60);
        self.raw_write(&format!("\n{}\n  {}\n{}\n\n", bar, title, bar));
    }

    pub fn subsection(&self, title: &str) {
        self.log_line("", &format!("--- {} ---\n", title));
    }

    pub fn pack_contents(&self, seat: usize, pack_num: usize, cards: &[String]) {
        let mut buf = format!("[{}] Seat {} — Pack {} ({} cards):\n",
            self.timestamp(), seat, pack_num, cards.len());
        for (i, card) in cards.iter().enumerate() {
            let name = card.split(" // ").next().unwrap_or(card);
            buf.push_str(&format!("  {:2}. {}\n", i, name));
        }
        buf.push('\n');
        self.raw_write(&buf);
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
        let mut buf = format!(
            "[{}] Seat {} | Pack {} Pick {} | Chose: {} (from {} cards)\n",
            self.timestamp(), seat, pack, pick, chosen_name, available.len()
        );
        // Log the LLM's reasoning (indent it, marked as LLM output)
        for line in response.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                buf.push_str(&format!("  [LLM Seat {}] {}\n", seat, trimmed));
            }
        }
        buf.push('\n');
        self.raw_write(&buf);
    }

    pub fn pool_summary(&self, seat: usize, pool: &[String]) {
        let mut buf = format!("[{}] Seat {} — Final pool ({} cards):\n",
            self.timestamp(), seat, pool.len());
        for card in pool {
            let name = card.split(" // ").next().unwrap_or(card);
            buf.push_str(&format!("  - {}\n", name));
        }
        buf.push('\n');
        self.raw_write(&buf);
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
        let mut buf = format!(
            "[{}] Seat {} — Deck ({} cards, {} retries)\n",
            self.timestamp(), seat, total, retries
        );

        buf.push_str("  Maindeck:\n");
        for card in maindeck {
            buf.push_str(&format!("    {}\n", card));
        }

        buf.push_str("  Lands:\n");
        let mut lands_sorted: Vec<_> = lands.iter().collect();
        lands_sorted.sort_by_key(|(name, _)| (*name).clone());
        for (name, count) in lands_sorted {
            buf.push_str(&format!("    {} {}\n", count, name));
        }

        if !sideboard.is_empty() {
            buf.push_str(&format!("  Sideboard ({} cards):\n", sideboard.len()));
            for card in sideboard {
                let name = card.split(" // ").next().unwrap_or(card);
                buf.push_str(&format!("    {}\n", name));
            }
        }

        buf.push_str("\n  LLM reasoning:\n");
        for line in response.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                buf.push_str(&format!("    [LLM Seat {}] {}\n", seat, trimmed));
            }
        }
        buf.push('\n');
        self.raw_write(&buf);
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
        self.log_line("", &format!(
            "Round {} — Seat {} vs Seat {}: {}-{} ({})",
            round, seat_a, seat_b, wins_a, wins_b, winner_str
        ));
    }

    pub fn game_log(&self, _round: usize, game_num: usize, seat_a: usize, seat_b: usize, log: &[String]) {
        let mut buf = format!(
            "[{}] Game {} (Seat {} vs Seat {}):\n",
            self.timestamp(), game_num, seat_a, seat_b
        );
        for entry in log {
            buf.push_str(&format!("    {}\n", entry));
        }
        buf.push('\n');
        self.raw_write(&buf);
    }

    pub fn bye(&self, round: usize, seat: usize) {
        self.log_line("", &format!("Round {} — Seat {} gets a bye", round, seat));
    }

    pub fn standings(&self, standings: &[(usize, usize, usize, usize)]) {
        let mut buf = format!("[{}] Final Standings:\n", self.timestamp());
        for (rank, &(seat, match_wins, match_losses, game_wins)) in standings.iter().enumerate() {
            buf.push_str(&format!(
                "  {}. Seat {} — {}-{} ({} game wins)\n",
                rank + 1, seat, match_wins, match_losses, game_wins
            ));
        }
        buf.push('\n');
        self.raw_write(&buf);
    }

    fn log_line(&self, _src: &str, msg: &str) {
        self.raw_write(&format!("[{}] {}\n", self.timestamp(), msg));
    }

    fn raw_write(&self, text: &str) {
        if let Ok(mut f) = self.file.lock() {
            let _ = f.write_all(text.as_bytes());
            let _ = f.flush();
        }
    }
}
