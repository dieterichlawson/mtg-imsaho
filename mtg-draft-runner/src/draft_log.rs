use std::path::Path;
use std::fmt::Write;

/// A streaming log writer that writes through the global `game_log`.
/// Thread-safe because `game_log` uses a single Mutex internally.
///
/// Use the `log_*!` macros instead of calling methods directly — they automatically
/// capture file and line number at the call site.
pub struct DraftLogger;

impl DraftLogger {
    pub fn new(path: &Path) -> Self {
        if let Err(e) = mtg_player::game_log::init(&path.to_string_lossy()) {
            eprintln!("Error: failed to open draft log '{}': {e}", path.display());
            std::process::exit(1);
        }
        Self
    }

    pub fn header(set_name: &str, players: usize, best_of: usize, models: &[String], file: &str, line: u32) {
        let all_same = models.iter().all(|m| m == &models[0]);

        let mut lines: Vec<String> = Vec::new();
        lines.push(format!("{set_name} Draft"));
        lines.push(format!("{players} players"));
        lines.push(format!("best-of-{best_of}"));
        if all_same {
            lines.push(format!("model: {}", models[0]));
        } else {
            for (seat, model) in models.iter().enumerate() {
                lines.push(format!("Seat {seat}: {model}"));
            }
        }

        // Border at least as long as the longest content line (chars, not bytes).
        let inner_width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) + 4;
        let bar = "═".repeat(inner_width);

        let mut content = format!("╔{bar}╗\n");
        for line_text in &lines {
            let pad = inner_width - line_text.chars().count() - 4;
            writeln!(content, "║  {}{}  ║", line_text, " ".repeat(pad)).unwrap();
        }
        write!(content, "╚{bar}╝").unwrap();

        mtg_player::game_log::write(file, line, "HEADER", &content);
    }

    pub fn section(title: &str, file: &str, line: u32) {
        let bar = "═".repeat(60);
        let content = format!("{bar}\n  {title}\n{bar}");
        mtg_player::game_log::write(file, line, "SECTION", &content);
    }

    pub fn subsection(title: &str, file: &str, line: u32) {
        mtg_player::game_log::write(file, line, &format!("--- {title} ---"), "");
    }

    pub fn system_prompt(prompt: &str, file: &str, line: u32) {
        mtg_player::game_log::write(file, line, "DRAFT SYSTEM PROMPT", prompt);
    }

    pub fn pack_contents(seat: usize, pack_num: usize, cards: &[String], file: &str, line: u32) {
        let mut content = String::new();
        for (i, card) in cards.iter().enumerate() {
            let name = card.split(" // ").next().unwrap_or(card);
            writeln!(content, "{i:2}. {name}").unwrap();
        }
        mtg_player::game_log::write(
            file, line,
            &format!("[Seat {}] PACK Pack {} ({} cards)", seat, pack_num, cards.len()),
            &content,
        );
    }

    pub fn draft_pick(
        seat: usize,
        pack: usize,
        pick_index: usize,
        available: &[String],
        chosen: &str,
        prompt: &str,
        response: &str,
        file: &str,
        line: u32,
    ) {
        let chosen_name = chosen.split(" // ").next().unwrap_or(chosen);
        // Order: prompt → response → pick summary. The seat tag lives in
        // each entry's label field; body lines are raw content with no
        // per-line prefix.
        mtg_player::game_log::write(
            file, line,
            &format!("[Seat {seat}] PROMPT Pack {pack} Pick {pick_index}"),
            prompt,
        );
        mtg_player::game_log::write(
            file, line,
            &format!("[Seat {seat}] RESPONSE Pack {pack} Pick {pick_index}"),
            response,
        );
        mtg_player::game_log::write(
            file, line,
            &format!("[Seat {}] PICK Pack {} Pick {} | Chose: {} (from {} cards)",
                seat, pack, pick_index, chosen_name, available.len()),
            "",
        );
    }

    /// A pick the run had to make on a seat's behalf, because the seat's
    /// answer could not be used. Logged next to the PROMPT/RESPONSE/PICK
    /// entries it belongs with, so a reader scanning the draft sees it
    /// where it happened — and `grep WARN` finds it, which used to return
    /// nothing at all for a run where no seat ever picked (issue #195).
    pub fn draft_pick_warning(
        seat: usize,
        pack: usize,
        pick_index: usize,
        substituted: &str,
        response: &str,
        file: &str,
        line: u32,
    ) {
        mtg_player::game_log::write(
            file, line,
            &format!("[Seat {seat}] WARN Pack {pack} Pick {pick_index} | \
unusable response, substituted {substituted} (the first card)"),
            response,
        );
    }

    pub fn pool_summary(seat: usize, pool: &[String], file: &str, line: u32) {
        let mut content = String::new();
        for card in pool {
            let name = card.split(" // ").next().unwrap_or(card);
            writeln!(content, "- {name}").unwrap();
        }
        mtg_player::game_log::write(
            file, line,
            &format!("[Seat {}] POOL ({} cards)", seat, pool.len()),
            &content,
        );
    }

    pub fn deck_building(
        seat: usize,
        maindeck: &[String],
        lands: &std::collections::HashMap<String, u32>,
        sideboard: &[String],
        attempts: &[(&str, &str, Option<&str>)],
        retries: usize,
        fallback: bool,
        file: &str,
        line: u32,
    ) {
        let total = maindeck.len() + lands.values().sum::<u32>() as usize;

        // Log every prompt → response round-trip in order. The first
        // attempt's prompt is the full deckbuilding ask; later attempts
        // include the validation error and re-ask.
        let n = attempts.len();
        for (i, (prompt, response, error)) in attempts.iter().enumerate() {
            let label_prompt = format!("[Seat {}] DECK_PROMPT attempt {}/{}", seat, i + 1, n);
            mtg_player::game_log::write(file, line, &label_prompt, prompt);

            let response_label = match error {
                Some(e) => format!("[Seat {}] DECK_RESPONSE attempt {}/{} (invalid: {})", seat, i + 1, n, e),
                None => format!("[Seat {}] DECK_RESPONSE attempt {}/{} (accepted)", seat, i + 1, n),
            };
            mtg_player::game_log::write(file, line, &response_label, response);
        }

        // Final structured deck summary.
        let mut content = String::from("Maindeck:\n");
        for card in maindeck {
            writeln!(content, "  {card}").unwrap();
        }
        content.push_str("Lands:\n");
        let mut lands_sorted: Vec<_> = lands.iter().collect();
        lands_sorted.sort_by_key(|(name, _)| (*name).clone());
        for (name, count) in lands_sorted {
            writeln!(content, "  {count} {name}").unwrap();
        }
        if !sideboard.is_empty() {
            writeln!(content, "Sideboard ({} cards):", sideboard.len()).unwrap();
            for card in sideboard {
                let name = card.split(" // ").next().unwrap_or(card);
                writeln!(content, "  {name}").unwrap();
            }
        }
        // A deck nobody built must not read like one that took a few tries
        // to get right. The header says which it is, and a fallback also
        // gets its own WARN line so `grep WARN` finds it next to the
        // unusable picks (issue #200).
        if fallback {
            mtg_player::game_log::write(
                file, line,
                &format!("[Seat {seat}] WARN deck building failed after {retries} attempt(s); \
the runner substituted a deck — this seat's deck and results are not a built one"),
                "",
            );
        }
        mtg_player::game_log::write(file, line, &deck_header(seat, total, retries, fallback), &content);
    }

    pub fn match_result(
        round: usize,
        seat_a: usize,
        seat_b: usize,
        wins_a: usize,
        wins_b: usize,
        winner: Option<usize>,
        file: &str,
        line: u32,
    ) {
        let winner_str = winner.map_or_else(|| "Draw".to_string(), |w| format!("Seat {w} wins"));
        mtg_player::game_log::write(
            file, line,
            &format!("MATCH Round {round} — Seat {seat_a} vs Seat {seat_b}: {wins_a}-{wins_b} ({winner_str})"),
            "",
        );
    }

    pub fn game_log(_round: usize, game_num: usize, seat_a: usize, seat_b: usize, log: &[String], file: &str, line: u32) {
        let content = log.join("\n");
        mtg_player::game_log::write(
            file, line,
            &format!("GAME {game_num} (Seat {seat_a} vs Seat {seat_b})"),
            &content,
        );
    }

    pub fn bye(round: usize, seat: usize, file: &str, line: u32) {
        mtg_player::game_log::write(
            file, line,
            &format!("BYE Round {round} — Seat {seat} gets a bye"),
            "",
        );
    }

    pub fn standings(standings: &[(usize, usize, usize, usize)], file: &str, line: u32) {
        let mut content = String::new();
        for (rank, &(seat, match_wins, match_losses, game_wins)) in standings.iter().enumerate() {
            writeln!(content, "{}. Seat {} — {}-{} ({} game wins)",
                rank + 1, seat, match_wins, match_losses, game_wins
            ).unwrap();
        }
        mtg_player::game_log::write(file, line, "FINAL STANDINGS", &content);
    }
}

// Macros that auto-capture file!() and line!() at the call site.

#[macro_export]
macro_rules! log_header {
    ($log:expr, $($args:expr),+ $(,)?) => {{ let _ = &$log; $crate::draft_log::DraftLogger::header($($args),+, file!(), line!()) }}
}
#[macro_export]
macro_rules! log_section {
    ($log:expr, $($args:expr),+ $(,)?) => {{ let _ = &$log; $crate::draft_log::DraftLogger::section($($args),+, file!(), line!()) }}
}
#[macro_export]
macro_rules! log_subsection {
    ($log:expr, $($args:expr),+ $(,)?) => {{ let _ = &$log; $crate::draft_log::DraftLogger::subsection($($args),+, file!(), line!()) }}
}
#[macro_export]
macro_rules! log_system_prompt {
    ($log:expr, $($args:expr),+ $(,)?) => {{ let _ = &$log; $crate::draft_log::DraftLogger::system_prompt($($args),+, file!(), line!()) }}
}
#[macro_export]
macro_rules! log_pack_contents {
    ($log:expr, $($args:expr),+ $(,)?) => {{ let _ = &$log; $crate::draft_log::DraftLogger::pack_contents($($args),+, file!(), line!()) }}
}
#[macro_export]
macro_rules! log_draft_pick {
    ($log:expr, $($args:expr),+ $(,)?) => {{ let _ = &$log; $crate::draft_log::DraftLogger::draft_pick($($args),+, file!(), line!()) }}
}
#[macro_export]
macro_rules! log_draft_warning {
    ($log:expr, $($args:expr),+ $(,)?) => {{ let _ = &$log; $crate::draft_log::DraftLogger::draft_pick_warning($($args),+, file!(), line!()) }}
}
#[macro_export]
macro_rules! log_pool_summary {
    ($log:expr, $($args:expr),+ $(,)?) => {{ let _ = &$log; $crate::draft_log::DraftLogger::pool_summary($($args),+, file!(), line!()) }}
}
#[macro_export]
macro_rules! log_deck_building {
    ($log:expr, $($args:expr),+ $(,)?) => {{ let _ = &$log; $crate::draft_log::DraftLogger::deck_building($($args),+, file!(), line!()) }}
}
#[macro_export]
macro_rules! log_match_result {
    ($log:expr, $($args:expr),+ $(,)?) => {{ let _ = &$log; $crate::draft_log::DraftLogger::match_result($($args),+, file!(), line!()) }}
}
#[macro_export]
macro_rules! log_game_log {
    ($log:expr, $($args:expr),+ $(,)?) => {{ let _ = &$log; $crate::draft_log::DraftLogger::game_log($($args),+, file!(), line!()) }}
}
#[macro_export]
macro_rules! log_bye {
    ($log:expr, $($args:expr),+ $(,)?) => {{ let _ = &$log; $crate::draft_log::DraftLogger::bye($($args),+, file!(), line!()) }}
}
#[macro_export]
macro_rules! log_standings {
    ($log:expr, $($args:expr),+ $(,)?) => {{ let _ = &$log; $crate::draft_log::DraftLogger::standings($($args),+, file!(), line!()) }}
}

/// The label on a seat's final DECK entry.
///
/// A deck the runner substituted must not read like one a seat took a few
/// tries to get right — "(59 cards, 10 retries)" said nothing about the fact
/// that attempt 10 failed too and no seat built this (issue #200).
fn deck_header(seat: usize, total: usize, retries: usize, fallback: bool) -> String {
    if fallback {
        format!("[Seat {seat}] DECK ({total} cards, FALLBACK after {retries} failed attempts)")
    } else {
        format!("[Seat {seat}] DECK ({total} cards, {retries} retries)")
    }
}

#[cfg(test)]
mod tests {
    use super::deck_header;

    #[test]
    fn a_built_deck_reports_its_retries() {
        assert_eq!(
            deck_header(0, 40, 2, false),
            "[Seat 0] DECK (40 cards, 2 retries)"
        );
    }

    /// Issue #200: the fallback used to be indistinguishable from a deck
    /// that took ten tries and then worked.
    #[test]
    fn a_substituted_deck_says_so_in_the_log() {
        let header = deck_header(0, 40, 10, true);
        assert!(header.contains("FALLBACK"), "header: {header}");
        assert!(header.contains("10 failed attempts"), "header: {header}");
        assert!(!header.contains("retries"), "\"retries\" reads as a deck that got built: {header}");
    }
}
