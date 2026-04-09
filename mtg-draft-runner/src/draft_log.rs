use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

/// A streaming log writer that appends human-readable entries as the draft progresses.
/// Thread-safe via Mutex so parallel operations can log safely.
///
/// Use the `log_*!` macros instead of calling methods directly — they automatically
/// capture file and line number at the call site.
pub struct DraftLogger {
    file: Mutex<File>,
    start: Instant,
}

/// Format a source location as "filename:line" (strips the path to just the filename).
fn src(file: &str, line: u32) -> String {
    let filename = file.rsplit('/').next().unwrap_or(file);
    format!("{}:{}", filename, line)
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

    fn prefix(&self) -> String {
        let elapsed = self.start.elapsed();
        let total_secs = elapsed.as_secs();
        let millis = elapsed.subsec_millis();
        let hrs = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        let secs = total_secs % 60;
        let tid = std::thread::current().id();
        format!("{:02}:{:02}:{:02}.{:03} {:?}", hrs, mins, secs, millis, tid)
    }

    pub fn header(&self, set_name: &str, players: usize, best_of: usize, model: &str, file: &str, line: u32) {
        self.raw_write(&format!(
            "[{}] [{}] ╔═══��══════════════════════════════════════════════════════╗\n\
             [{}] [{}] ║  {} Draft — {} players, best-of-{}, model: {}\n\
             [{}] [{}] ╚══════════════════════════════════════════════════════════╝\n\n",
            self.prefix(), src(file, line),
            self.prefix(), src(file, line),
            set_name, players, best_of, model,
            self.prefix(), src(file, line),
        ));
    }

    pub fn section(&self, title: &str, file: &str, line: u32) {
        let bar = "═".repeat(60);
        self.raw_write(&format!("\n[{}] [{}] {}\n[{}] [{}]   {}\n[{}] [{}] {}\n\n",
            self.prefix(), src(file, line), bar,
            self.prefix(), src(file, line), title,
            self.prefix(), src(file, line), bar,
        ));
    }

    pub fn subsection(&self, title: &str, file: &str, line: u32) {
        self.raw_write(&format!("[{}] [{}] --- {} ---\n\n", self.prefix(), src(file, line), title));
    }

    pub fn pack_contents(&self, seat: usize, pack_num: usize, cards: &[String], file: &str, line: u32) {
        let s = src(file, line);
        let ts = self.prefix();
        let mut buf = format!("[{}] [{}] Seat {} — Pack {} ({} cards):\n", ts, s, seat, pack_num, cards.len());
        for (i, card) in cards.iter().enumerate() {
            let name = card.split(" // ").next().unwrap_or(card);
            buf.push_str(&format!("[{}] [{}]   {:2}. {}\n", ts, s, i, name));
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
        prompt: &str,
        response: &str,
        file: &str,
        line: u32,
    ) {
        let chosen_name = chosen.split(" // ").next().unwrap_or(chosen);
        let s = src(file, line);
        let ts = self.prefix();
        let mut buf = format!(
            "[{}] [{}] Seat {} | Pack {} Pick {} | Chose: {} (from {} cards)\n",
            ts, s, seat, pack, pick, chosen_name, available.len()
        );
        // Log the prompt sent to the LLM
        buf.push_str(&format!("[{}] [{}] [PROMPT→Seat {}]\n", ts, s, seat));
        for ln in prompt.lines() {
            buf.push_str(&format!("[{}] [{}]   {}\n", ts, s, ln));
        }
        // Log the LLM's response
        buf.push_str(&format!("[{}] [{}] [RESPONSE←Seat {}]\n", ts, s, seat));
        for ln in response.lines() {
            let trimmed = ln.trim();
            if !trimmed.is_empty() {
                buf.push_str(&format!("[{}] [{}]   {}\n", ts, s, trimmed));
            }
        }
        buf.push('\n');
        self.raw_write(&buf);
    }

    pub fn pool_summary(&self, seat: usize, pool: &[String], file: &str, line: u32) {
        let s = src(file, line);
        let ts = self.prefix();
        let mut buf = format!("[{}] [{}] Seat {} — Final pool ({} cards):\n", ts, s, seat, pool.len());
        for card in pool {
            let name = card.split(" // ").next().unwrap_or(card);
            buf.push_str(&format!("[{}] [{}]   - {}\n", ts, s, name));
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
        file: &str,
        line: u32,
    ) {
        let total = maindeck.len() + lands.values().sum::<u32>() as usize;
        let s = src(file, line);
        let ts = self.prefix();
        let mut buf = format!("[{}] [{}] Seat {} — Deck ({} cards, {} retries)\n", ts, s, seat, total, retries);

        buf.push_str(&format!("[{}] [{}]   Maindeck:\n", ts, s));
        for card in maindeck {
            buf.push_str(&format!("[{}] [{}]     {}\n", ts, s, card));
        }

        buf.push_str(&format!("[{}] [{}]   Lands:\n", ts, s));
        let mut lands_sorted: Vec<_> = lands.iter().collect();
        lands_sorted.sort_by_key(|(name, _)| (*name).clone());
        for (name, count) in lands_sorted {
            buf.push_str(&format!("[{}] [{}]     {} {}\n", ts, s, count, name));
        }

        if !sideboard.is_empty() {
            buf.push_str(&format!("[{}] [{}]   Sideboard ({} cards):\n", ts, s, sideboard.len()));
            for card in sideboard {
                let name = card.split(" // ").next().unwrap_or(card);
                buf.push_str(&format!("[{}] [{}]     {}\n", ts, s, name));
            }
        }

        buf.push_str(&format!("[{}] [{}]   [PROMPT→Seat {}]\n", ts, s, seat));
        // The prompt is embedded in the response for deck building since we don't
        // pass it separately. Log the full response which contains both reasoning
        // and the decklist.
        buf.push_str(&format!("[{}] [{}]   [RESPONSE←Seat {}]\n", ts, s, seat));
        for ln in response.lines() {
            let trimmed = ln.trim();
            if !trimmed.is_empty() {
                buf.push_str(&format!("[{}] [{}]     {}\n", ts, s, trimmed));
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
        file: &str,
        line: u32,
    ) {
        let winner_str = winner
            .map(|w| format!("Seat {} wins", w))
            .unwrap_or_else(|| "Draw".to_string());
        self.raw_write(&format!(
            "[{}] [{}] Round {} — Seat {} vs Seat {}: {}-{} ({})\n",
            self.prefix(), src(file, line), round, seat_a, seat_b, wins_a, wins_b, winner_str
        ));
    }

    pub fn game_log(&self, _round: usize, game_num: usize, seat_a: usize, seat_b: usize, log: &[String], file: &str, line: u32) {
        let s = src(file, line);
        let ts = self.prefix();
        let mut buf = format!("[{}] [{}] Game {} (Seat {} vs Seat {}):\n", ts, s, game_num, seat_a, seat_b);
        for entry in log {
            buf.push_str(&format!("[{}] [{}]     {}\n", ts, s, entry));
        }
        buf.push('\n');
        self.raw_write(&buf);
    }

    pub fn bye(&self, round: usize, seat: usize, file: &str, line: u32) {
        self.raw_write(&format!(
            "[{}] [{}] Round {} — Seat {} gets a bye\n",
            self.prefix(), src(file, line), round, seat
        ));
    }

    pub fn standings(&self, standings: &[(usize, usize, usize, usize)], file: &str, line: u32) {
        let s = src(file, line);
        let ts = self.prefix();
        let mut buf = format!("[{}] [{}] Final Standings:\n", ts, s);
        for (rank, &(seat, match_wins, match_losses, game_wins)) in standings.iter().enumerate() {
            buf.push_str(&format!(
                "[{}] [{}]   {}. Seat {} — {}-{} ({} game wins)\n",
                ts, s, rank + 1, seat, match_wins, match_losses, game_wins
            ));
        }
        buf.push('\n');
        self.raw_write(&buf);
    }

    fn raw_write(&self, text: &str) {
        if let Ok(mut f) = self.file.lock() {
            let _ = f.write_all(text.as_bytes());
            let _ = f.flush();
        }
    }
}

// Macros that auto-capture file!() and line!() at the call site.

#[macro_export]
macro_rules! log_header {
    ($log:expr, $($args:expr),+ $(,)?) => { $log.header($($args),+, file!(), line!()) }
}
#[macro_export]
macro_rules! log_section {
    ($log:expr, $($args:expr),+ $(,)?) => { $log.section($($args),+, file!(), line!()) }
}
#[macro_export]
macro_rules! log_subsection {
    ($log:expr, $($args:expr),+ $(,)?) => { $log.subsection($($args),+, file!(), line!()) }
}
#[macro_export]
macro_rules! log_pack_contents {
    ($log:expr, $($args:expr),+ $(,)?) => { $log.pack_contents($($args),+, file!(), line!()) }
}
#[macro_export]
macro_rules! log_draft_pick {
    ($log:expr, $($args:expr),+ $(,)?) => { $log.draft_pick($($args),+, file!(), line!()) }
}
#[macro_export]
macro_rules! log_pool_summary {
    ($log:expr, $($args:expr),+ $(,)?) => { $log.pool_summary($($args),+, file!(), line!()) }
}
#[macro_export]
macro_rules! log_deck_building {
    ($log:expr, $($args:expr),+ $(,)?) => { $log.deck_building($($args),+, file!(), line!()) }
}
#[macro_export]
macro_rules! log_match_result {
    ($log:expr, $($args:expr),+ $(,)?) => { $log.match_result($($args),+, file!(), line!()) }
}
#[macro_export]
macro_rules! log_game_log {
    ($log:expr, $($args:expr),+ $(,)?) => { $log.game_log($($args),+, file!(), line!()) }
}
#[macro_export]
macro_rules! log_bye {
    ($log:expr, $($args:expr),+ $(,)?) => { $log.bye($($args),+, file!(), line!()) }
}
#[macro_export]
macro_rules! log_standings {
    ($log:expr, $($args:expr),+ $(,)?) => { $log.standings($($args),+, file!(), line!()) }
}
