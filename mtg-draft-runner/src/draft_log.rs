use std::path::Path;

/// Prefix every line of `text` with `tag` followed by a space, so that the
/// resulting multi-line content is greppable by `tag`. Empty lines remain
/// blank (no trailing tag).
fn prefix_lines(tag: &str, text: &str) -> String {
    let mut out = String::with_capacity(text.len() + text.lines().count() * (tag.len() + 1));
    for line in text.lines() {
        if line.is_empty() {
            out.push('\n');
        } else {
            out.push_str(tag);
            out.push(' ');
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// A streaming log writer that writes through the global game_log.
/// Thread-safe because game_log uses a single Mutex internally.
///
/// Use the `log_*!` macros instead of calling methods directly — they automatically
/// capture file and line number at the call site.
pub struct DraftLogger;

impl DraftLogger {
    pub fn new(path: &Path) -> Self {
        mtg_player::game_log::init(&path.to_string_lossy());
        Self
    }

    pub fn header(&self, set_name: &str, players: usize, best_of: usize, models: &[String], file: &str, line: u32) {
        let all_same = models.iter().all(|m| m == &models[0]);

        let mut lines: Vec<String> = Vec::new();
        lines.push(format!("{} Draft", set_name));
        lines.push(format!("{} players", players));
        lines.push(format!("best-of-{}", best_of));
        if all_same {
            lines.push(format!("model: {}", models[0]));
        } else {
            for (seat, model) in models.iter().enumerate() {
                lines.push(format!("Seat {}: {}", seat, model));
            }
        }

        // Border at least as long as the longest content line (chars, not bytes).
        let inner_width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) + 4;
        let bar = "═".repeat(inner_width);

        let mut content = format!("╔{}╗\n", bar);
        for line_text in &lines {
            let pad = inner_width - line_text.chars().count() - 4;
            content.push_str(&format!("║  {}{}  ║\n", line_text, " ".repeat(pad)));
        }
        content.push_str(&format!("╚{}╝", bar));

        mtg_player::game_log::write(file, line, "HEADER", &content);
    }

    pub fn section(&self, title: &str, file: &str, line: u32) {
        let bar = "═".repeat(60);
        let content = format!("{}\n  {}\n{}", bar, title, bar);
        mtg_player::game_log::write(file, line, "SECTION", &content);
    }

    pub fn subsection(&self, title: &str, file: &str, line: u32) {
        mtg_player::game_log::write(file, line, &format!("--- {} ---", title), "");
    }

    pub fn system_prompt(&self, prompt: &str, file: &str, line: u32) {
        mtg_player::game_log::write(file, line, "DRAFT SYSTEM PROMPT", prompt);
    }

    pub fn pack_contents(&self, seat: usize, pack_num: usize, cards: &[String], file: &str, line: u32) {
        let tag = format!("[Seat {}]", seat);
        let mut content = String::new();
        for (i, card) in cards.iter().enumerate() {
            let name = card.split(" // ").next().unwrap_or(card);
            content.push_str(&format!("{:2}. {}\n", i, name));
        }
        mtg_player::game_log::write(
            file, line,
            &format!("{} PACK Pack {} ({} cards)", tag, pack_num, cards.len()),
            &prefix_lines(&tag, &content),
        );
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
        let tag = format!("[Seat {}]", seat);
        // Order: prompt → response → pick summary, all tagged with the seat
        // so `grep "Seat 7"` shows every line that pertains to seat 7.
        mtg_player::game_log::write(
            file, line,
            &format!("{} PROMPT Pack {} Pick {}", tag, pack, pick),
            &prefix_lines(&tag, prompt),
        );
        mtg_player::game_log::write(
            file, line,
            &format!("{} RESPONSE Pack {} Pick {}", tag, pack, pick),
            &prefix_lines(&tag, response),
        );
        mtg_player::game_log::write(
            file, line,
            &format!("{} PICK Pack {} Pick {} | Chose: {} (from {} cards)",
                tag, pack, pick, chosen_name, available.len()),
            "",
        );
    }

    pub fn pool_summary(&self, seat: usize, pool: &[String], file: &str, line: u32) {
        let tag = format!("[Seat {}]", seat);
        let mut content = String::new();
        for card in pool {
            let name = card.split(" // ").next().unwrap_or(card);
            content.push_str(&format!("- {}\n", name));
        }
        mtg_player::game_log::write(
            file, line,
            &format!("{} POOL ({} cards)", tag, pool.len()),
            &prefix_lines(&tag, &content),
        );
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
        let tag = format!("[Seat {}]", seat);
        let total = maindeck.len() + lands.values().sum::<u32>() as usize;
        let mut content = String::from("Maindeck:\n");
        for card in maindeck {
            content.push_str(&format!("  {}\n", card));
        }
        content.push_str("Lands:\n");
        let mut lands_sorted: Vec<_> = lands.iter().collect();
        lands_sorted.sort_by_key(|(name, _)| (*name).clone());
        for (name, count) in lands_sorted {
            content.push_str(&format!("  {} {}\n", count, name));
        }
        if !sideboard.is_empty() {
            content.push_str(&format!("Sideboard ({} cards):\n", sideboard.len()));
            for card in sideboard {
                let name = card.split(" // ").next().unwrap_or(card);
                content.push_str(&format!("  {}\n", name));
            }
        }
        mtg_player::game_log::write(
            file, line,
            &format!("{} DECK_RESPONSE ({} retries)", tag, retries),
            &prefix_lines(&tag, response),
        );
        mtg_player::game_log::write(
            file, line,
            &format!("{} DECK ({} cards)", tag, total),
            &prefix_lines(&tag, &content),
        );
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
        mtg_player::game_log::write(
            file, line,
            &format!("MATCH Round {} — Seat {} vs Seat {}: {}-{} ({})",
                round, seat_a, seat_b, wins_a, wins_b, winner_str),
            "",
        );
    }

    pub fn game_log(&self, _round: usize, game_num: usize, seat_a: usize, seat_b: usize, log: &[String], file: &str, line: u32) {
        let content = log.join("\n");
        mtg_player::game_log::write(
            file, line,
            &format!("GAME {} (Seat {} vs Seat {})", game_num, seat_a, seat_b),
            &content,
        );
    }

    pub fn bye(&self, round: usize, seat: usize, file: &str, line: u32) {
        mtg_player::game_log::write(
            file, line,
            &format!("BYE Round {} — Seat {} gets a bye", round, seat),
            "",
        );
    }

    pub fn standings(&self, standings: &[(usize, usize, usize, usize)], file: &str, line: u32) {
        let mut content = String::new();
        for (rank, &(seat, match_wins, match_losses, game_wins)) in standings.iter().enumerate() {
            content.push_str(&format!(
                "{}. Seat {} — {}-{} ({} game wins)\n",
                rank + 1, seat, match_wins, match_losses, game_wins
            ));
        }
        mtg_player::game_log::write(file, line, "FINAL STANDINGS", &content);
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
macro_rules! log_system_prompt {
    ($log:expr, $($args:expr),+ $(,)?) => { $log.system_prompt($($args),+, file!(), line!()) }
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
