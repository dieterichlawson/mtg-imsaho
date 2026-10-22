use std::env;
use std::path::PathBuf;
use std::process::Command;

use reqwest::blocking::Client;

use mtg_player::llm::Cost;

/// Per-model token usage tracking. Thread-safe via Mutex.
use std::sync::Mutex;
use std::collections::HashMap;
use std::fmt::Write;

use crate::card_lines::CardLines;

// Derived, not written out field by field: a hand-copied `Clone` and a
// hand-copied conversion are how `rejected` went missing from this summary
// in the first place (#489).
#[derive(Default, Debug, Clone)]
pub struct ModelUsage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_create: u64,
    pub calls: u64,
    /// Calls that came back but whose answer the harness could not use. A
    /// rejected answer is a *successful* call, so without this the summary
    /// reads as a healthy seat even when the harness chose every move
    /// itself — the game phase's counter used to be dropped one line before
    /// it would have been printed (issue #489, the shape of #211/#399).
    pub rejected: u64,
    /// Decisions the backend never answered at all. Carried for the same
    /// reason `rejected` is: the field that was not carried is the one that
    /// went missing (#489, #587).
    pub unanswered: u64,
}

static MODEL_USAGE: std::sync::LazyLock<Mutex<HashMap<String, ModelUsage>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Picks this run replayed out of a `--resume` snapshot instead of paying
/// for them.
///
/// `MODEL_USAGE` is process-local, so a resumed run's counters cover that
/// process only. The summary read identically whether the run had drafted
/// all 84 picks or 34 of them, so a draft split across resumes could not be
/// costed: the number you had did not say it was a fragment, and the runs
/// that died published no number to add to it (issue #578).
static REPLAYED_PICKS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Tell the summary how many picks came out of a snapshot rather than a
/// model call. Called once, by `main`, when a resume replays one.
pub fn note_replayed_picks(picks: usize) {
    REPLAYED_PICKS.store(picks as u64, std::sync::atomic::Ordering::Relaxed);
}

fn record_model_usage(model: &str, input: u64, output: u64, cache_read: u64, cache_create: u64) {
    let mut map = MODEL_USAGE.lock().unwrap();
    let entry = map.entry(model.to_string()).or_default();
    entry.calls += 1;
    entry.input += input;
    entry.output += output;
    entry.cache_read += cache_read;
    entry.cache_create += cache_create;
}

/// Record token usage from an Anthropic API response.
pub fn record_anthropic_usage(model: &str, usage: &serde_json::Value) {
    record_model_usage(
        model,
        usage["input_tokens"].as_u64().unwrap_or(0),
        usage["output_tokens"].as_u64().unwrap_or(0),
        usage["cache_read_input_tokens"].as_u64().unwrap_or(0),
        usage["cache_creation_input_tokens"].as_u64().unwrap_or(0),
    );
}

/// Record token usage from a Gemini Interactions API response.
pub fn record_gemini_usage(model: &str, usage: &serde_json::Value) {
    let input_tokens = usage["total_input_tokens"].as_u64().unwrap_or(0);
    let cached_tokens = usage["total_cached_tokens"].as_u64().unwrap_or(0);
    let uncached_input = input_tokens.saturating_sub(cached_tokens);
    record_model_usage(
        model,
        uncached_input,
        usage["total_output_tokens"].as_u64().unwrap_or(0),
        cached_tokens,
        0, // Gemini interactions API manages caching server-side
    );
}

/// Get the per-model usage map for external reporting.
pub fn get_model_usage() -> HashMap<String, ModelUsage> {
    MODEL_USAGE.lock().unwrap().clone()
}

/// Known model pricing ($/`MTok`). (input, output, `cache_read`, `cache_write`)
/// Anthropic: platform.claude.com/docs/en/about-claude/pricing (verified 2026-04-08)

/// This crate's usage record as the shared cost model reads it. Costing
/// reads the token counters only, but every field this struct has is
/// carried across, because the one that was not is the one that went
/// missing (#489).
fn as_llm_usage(u: &ModelUsage) -> mtg_player::llm::LlmModelUsage {
    mtg_player::llm::LlmModelUsage {
        input: u.input,
        output: u.output,
        cache_read: u.cache_read,
        cache_create: u.cache_create,
        calls: u.calls,
        rejected: u.rejected,
        unanswered: u.unanswered,
        // Not a counter this crate keeps: the draft seats report no
        // thinking breakdown, and the game seats' own record carries it.
        thinking: 0,
    }
}

fn usage_cost(u: &ModelUsage, model: &str) -> Cost {
    mtg_player::llm::cost(model, &as_llm_usage(u))
}

/// What a phase's usage came to, summed the way the shared model sums a run:
/// plan-quota seats contribute no dollars, and one model with no rate on file
/// makes the phase total unknown rather than an understatement.
fn phase_cost(usage: &HashMap<String, ModelUsage>) -> Cost {
    let converted: HashMap<String, mtg_player::llm::LlmModelUsage> =
        usage.iter().map(|(m, u)| (m.clone(), as_llm_usage(u))).collect();
    mtg_player::llm::total_cost(&converted)
}

/// A cost as the summary prints it: for a plan-quota seat, what the same
/// tokens would have cost on a key, said as such (`mtg-runner` prints the
/// same), since "n/a" alone is not a number a run can be compared by.
fn cost_text(cost: Cost, api_equivalent: Option<f64>) -> String {
    match (cost, api_equivalent) {
        (Cost::PlanQuota, Some(usd)) => format!("n/a (plan quota; ${usd:.4} at API rates)"),
        (cost, _) => cost.to_string(),
    }
}

/// One phase's tokens and cost: the draft's picks and deck builds, or the
/// tournament's games. The per-model lines add the two together, so
/// without this a run could say how many calls each phase made and not
/// what either cost.
fn phase_line(usage: &HashMap<String, ModelUsage>) -> String {
    let sum = |f: fn(&ModelUsage) -> u64| usage.values().map(f).sum::<u64>();
    let api: Option<f64> = usage.iter()
        .map(|(m, u)| mtg_player::llm::api_equivalent(m, &as_llm_usage(u)))
        .try_fold(0.0, |acc, v| v.map(|v| acc + v));
    let api = api.filter(|_| usage.keys().all(|m| mtg_player::llm::is_plan_quota(m)));
    format!("{} calls, {} in / {} out / {} cache_read / {} cache_write, {}",
        sum(|u| u.calls), sum(|u| u.input), sum(|u| u.output), sum(|u| u.cache_read),
        sum(|u| u.cache_create), cost_text(phase_cost(usage), api))
}

/// How a run reached its usage summary.
///
/// The summary used to be a statement on the happy path — one call at the
/// very end of `main`, after the standings — so every other way a run can
/// end skipped it, and the `claude -p` calls those runs had already paid
/// for were reported nowhere at all (issue #578).
#[derive(Clone, Copy)]
pub enum RunOutcome {
    /// The tournament finished, having played `total_games` games.
    Finished { total_games: usize },
    /// The run stopped early: a seat's fatal, a config error, a panic. What
    /// it spent up to that point is still owed an account, and there is no
    /// finished game count to average a metered phase over.
    Stopped,
}

/// Print a summary of all token usage and estimated cost, broken down by model and phase.
///
/// Prints once per process: the run that finishes and then hits a fatal on
/// the way out must not account for itself twice, and two seats racing into
/// the fatal path must not each print a copy.
pub fn print_usage_summary(outcome: RunOutcome) {
    let draft_usage = get_model_usage();
    let game_usage = mtg_player::llm::get_llm_model_usage();

    // A run that stopped before it called anything has nothing to account
    // for, and a bare `Error: unknown flag` should not grow a table of
    // zeroes. A *finished* run always reports, even an all-zero one: that
    // the seats never called is itself the result.
    let spent: u64 = draft_usage.values().map(|u| u.calls).sum::<u64>()
        + game_usage.values().map(|u| u.calls).sum::<u64>();
    if matches!(outcome, RunOutcome::Stopped) && spent == 0 {
        return;
    }
    static SUMMARIZED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if SUMMARIZED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    let total_games = match outcome {
        RunOutcome::Finished { total_games } => total_games,
        RunOutcome::Stopped => 0,
    };

    let draft_calls: u64 = draft_usage.values().map(|u| u.calls).sum();

    // Game phase cost
    let game_usage: HashMap<String, ModelUsage> = game_usage.iter().map(|(m, u)| (m.clone(), ModelUsage {
        calls: u.calls, input: u.input, output: u.output, cache_read: u.cache_read,
        cache_create: u.cache_create, rejected: u.rejected, unanswered: u.unanswered,
    })).collect();
    let game_cost = phase_cost(&game_usage);
    let game_calls: u64 = game_usage.values().map(|u| u.calls).sum();

    // Combined per-model
    let mut combined: HashMap<String, ModelUsage> = HashMap::new();
    for (model, u) in draft_usage.iter().chain(game_usage.iter()) {
        let entry = combined.entry(model.clone()).or_default();
        entry.calls += u.calls;
        entry.input += u.input;
        entry.output += u.output;
        entry.cache_read += u.cache_read;
        entry.cache_create += u.cache_create;
        entry.rejected += u.rejected;
        entry.unanswered += u.unanswered;
    }

    // Build summary string for both stderr and log file. A run that
    // stopped early says so in the heading, because the same table from a
    // finished run and from a dead one is the thing that cannot be added
    // up afterwards (#578).
    let mut summary = String::from(match outcome {
        RunOutcome::Finished { .. } => "=== Token Usage ===\n",
        RunOutcome::Stopped => "=== Token Usage (run stopped early) ===\n",
    });

    writeln!(summary, "  Draft:  {}", phase_line(&draft_usage)).unwrap();
    // A per-game average only means anything for a metered phase; a
    // plan-quota or unpriced one has no dollars to divide.
    let per_game = match (game_cost, total_games) {
        (Cost::Usd(v), n) if n > 0 => format!(" ({n} games, ${:.4}/game avg)",
            v / f64::from(u32::try_from(n).unwrap_or(u32::MAX))),
        _ => String::new(),
    };
    writeln!(summary, "  Games:  {}{per_game}", phase_line(&game_usage)).unwrap();
    summary.push('\n');

    let mut models: Vec<_> = combined.iter().collect();
    models.sort_by_key(|(name, _)| (*name).clone());
    let total_cost = phase_cost(&combined);

    for (model, u) in &models {
        let cost = cost_text(usage_cost(u, model), mtg_player::llm::api_equivalent(model, &as_llm_usage(u)));
        // The same sentence `mtg-runner` prints, from the same function: a
        // call count alone reads as a healthy seat even when every answer
        // was thrown away (#489).
        let rejected = mtg_player::llm::rejected_note(u.rejected);
        // A dead backend and a bad answer are different events with
        // different remedies, so they are different numbers (#587).
        let unanswered = mtg_player::llm::unanswered_note(u.unanswered);
        writeln!(summary, "  {}: {} calls, {}in/{}out/{}cached/{}written = {cost}{rejected}{unanswered}",
            model, u.calls, u.input, u.output, u.cache_read, u.cache_create
        ).unwrap();
    }
    writeln!(summary, "  ---\n  Total: {} calls, {total_cost}", draft_calls + game_calls).unwrap();

    // These counters are process-local, so a resumed run's totals are a
    // fragment of the draft's real cost. Say so next to them: the number
    // the operator is reading covers this process, and the calls the run
    // that died spent on the replayed picks are in no total here (#578).
    let replayed = REPLAYED_PICKS.load(std::sync::atomic::Ordering::Relaxed);
    if replayed > 0 {
        writeln!(summary, "  (this run only: {replayed} pick{} came from a snapshot, and what \
the earlier run(s) spent making them is counted in no number above)",
            if replayed == 1 { "" } else { "s" }).unwrap();
    }

    // Whose answers those were. Every `cc` seat shares one model label, so
    // the per-model line above can say that answers were rejected and not
    // by whom — while the draft phase names the seat for a substituted pick
    // or deck. This is the same statement for the tournament (#489).
    let mut by_seat: Vec<(String, u64)> = mtg_player::llm::get_rejected_by_seat()
        .into_iter()
        .filter(|(_, n)| *n > 0)
        .collect();
    if !by_seat.is_empty() {
        by_seat.sort();
        summary.push_str("\n  Answers the harness could not use, by seat:\n");
        for (seat, n) in &by_seat {
            writeln!(summary, "    {seat}: {n} answer{} rejected — the harness chose for it, \
so this seat's games are not wholly its own", if *n == 1 { "" } else { "s" }).unwrap();
        }
        summary.push_str("  (grep the log for MALFORMED to see each one)\n");
    }

    // And the seats whose backend never answered, which is a broken CLI
    // rather than a seat playing badly (#587).
    let mut mute: Vec<(String, u64)> = mtg_player::llm::get_unanswered_by_seat()
        .into_iter()
        .filter(|(_, n)| *n > 0)
        .collect();
    if !mute.is_empty() {
        mute.sort();
        summary.push_str("\n  Decisions the seat's backend never answered:\n");
        for (seat, n) in &mute {
            // A seat given up for replies it could not read (#898) lands
            // here too: that is the seat not answering in the format, not
            // the CLI failing, and the API_ERROR line says which.
            writeln!(summary, "    {seat}: {n} decision{} got no answer at all — the CLI or \
API failed, or the seat stopped answering in the format it is asked for", if *n == 1 { "" } else { "s" }).unwrap();
        }
        summary.push_str("  (grep the log for NO_ANSWER to see each one, and API_ERROR for why)\n");
    }

    // Write to stderr
    mtg_player::stderr_text!("\n{summary}");

    // Write to log file
    mtg_player::game_log::write(file!(), line!(), "TOKEN USAGE", &summary);
}

/// LLM client for draft picks and deck building.
/// Maintains a multi-turn conversation for the draft process.
/// Backend trait for draft LLM communication. Both `send_pick` and
/// `send_deck_building` return the raw JSON response (pretty-printed)
/// from the model, with the schema enforced via structured output on
/// both Gemini and Anthropic.
trait DraftBackend: Send {
    /// Send a draft pick message. `num_cards` is the number of cards in
    /// the current pack, used to build an enum-constrained `pick` index
    /// schema. Returns the raw JSON response text.
    fn send_pick(&mut self, message: &str, num_cards: usize) -> String;
    /// Send a deck building message. `pool` is the seat's drafted cards,
    /// whose rows (`deck_rows`) the `maindeck` counts answer. Returns the
    /// raw JSON response text.
    fn send_deck_building(&mut self, message: &str, pool: &[String]) -> String;
    /// Send a between-games message: the deck for the next game, as a deck
    /// build answers it, and whether to play first.
    fn send_sideboard(&mut self, message: &str, pool: &[String]) -> String;
    /// The full system prompt this backend will send to the model.
    fn system_prompt(&self) -> &str;
    /// The reasoning behind the last answer, for a backend that reads it
    /// from a channel outside the JSON (the Messages API's thinking block).
    /// Taken once. The others leave it in the response's `thoughts`.
    fn take_thinking(&mut self) -> Option<String> { None }
}

/// Build a pick schema with the `pick` field constrained to the valid
/// `0..num_cards` range via an enum.
///
/// The two Anthropic backends send it without the enum
/// ([`mtg_player::llm::cache_stable_schema`]): the schema is part of the
/// cached prefix, the pack shrinks by a card every pick, and a new schema
/// every pick wrote the whole system prompt — the set's card reference —
/// again on every one of a seat's 42 picks. [`DraftLlmClient::send_pick_message`]
/// holds the answer to the enum instead.
fn pick_schema_for(num_cards: usize) -> serde_json::Value {
    let valid_indices: Vec<serde_json::Value> = (0..num_cards)
        .map(|i| serde_json::json!(i))
        .collect();
    serde_json::json!({
        "type": "object",
        "properties": {
            "thoughts": {
                "type": "string",
                "description": "At most two sentences: what decided the pick, and what you are drafting towards."
            },
            "pick": {
                "type": "integer",
                "enum": valid_indices,
                "description": "0-indexed number of the card to pick"
            }
        },
        "required": ["thoughts", "pick"]
    })
}

/// Build the deck-building schema: `maindeck` is one count per row of
/// [`mtg_draft::deckbuilding::deck_rows`], each held to `0..=copies` by its
/// `prefixItems` slot and the list to the row count. The `lands` object has
/// fixed keys for the five basic lands.
///
/// The two Anthropic backends send it through
/// [`mtg_player::llm::cache_stable_schema`], which leaves the slots and the
/// length out, so every seat's deck question is the same schema. It used to key `maindeck` by the pool's card names
/// with the counts as enums, sent as built: no two pools are alike, so each
/// seat's deck build wrote the cached prefix again (issue #867, #857's
/// rule). The answer is read back through the rows and checked against the
/// pool (`parse_deck_response_by_rows`, `validate_deck`).
fn deck_schema_for(pool: &[String]) -> serde_json::Value {
    let slots: Vec<serde_json::Value> = mtg_draft::deckbuilding::deck_rows(pool)
        .iter()
        .map(|(_, copies)| serde_json::json!({
            "type": "integer",
            "enum": (0..=*copies).collect::<Vec<u32>>()
        }))
        .collect();
    let rows = slots.len();
    let maindeck = serde_json::json!({
        "type": "array",
        "items": { "type": "integer" },
        "prefixItems": slots,
        "minItems": rows,
        "maxItems": rows,
        "description": "One count per numbered row of your pool, in order: how many copies of that card to include in the maindeck (0 to skip). Basic lands are not rows — they go in `lands`."
    });

    let basic_land_count = serde_json::json!({ "type": "integer" });
    serde_json::json!({
        "type": "object",
        "properties": {
            "thoughts": {
                "type": "string",
                "description": "At most two sentences: the deck's plan and what you cut."
            },
            "maindeck": maindeck,
            "lands": {
                "type": "object",
                "properties": {
                    "Plains":   basic_land_count.clone(),
                    "Island":   basic_land_count.clone(),
                    "Swamp":    basic_land_count.clone(),
                    "Mountain": basic_land_count.clone(),
                    "Forest":   basic_land_count.clone()
                },
                "description": "Number of each basic land to add, drafted or not (typically 16-18 of them in a 40-card deck)"
            }
        },
        "required": ["thoughts", "maindeck", "lands"]
    })
}

/// The between-games schema: a deck build's answer — one count per pool
/// row and the basic lands, the deck for the next game — and whether to
/// play first. The same shape whatever the pool, as the deck build's is,
/// so it is one cached schema for the whole tournament (#867's rule).
#[must_use]
pub fn sideboard_schema_for(pool: &[String]) -> serde_json::Value {
    let mut schema = deck_schema_for(pool);
    schema["properties"]["thoughts"]["description"] =
        "At most two sentences: what you changed for the next game and why, or why nothing.".into();
    schema["properties"]["play_first"] = serde_json::json!({
        "type": "boolean",
        "description": "true to play first in the next game, false to draw first; read only when you choose"
    });
    schema["required"] = serde_json::json!(["thoughts", "maindeck", "lands", "play_first"]);
    schema
}

/// Response-format instructions for every backend: each keeps `thoughts` in
/// the schema. The `claude -p` seat's result object carries no thinking
/// block, the Gemini seat's sanitizer never stripped the field, and the
/// Messages API seat runs with thinking off by default, or gets its thinking
/// block back empty where the model omits it — so the JSON payload is the
/// one place a pick's reasoning reliably lands.
///
/// The draft log's `RESPONSE` record is the whole durable account of why a
/// seat took a card (D5's reconstruction question), and the fallback paths in
/// this crate substitute picks quietly (#195, #536): with the reasoning
/// erased, a seat that drafted and a seat whose answers were all substituted
/// differ only in a `WARN` (issue #607).
const THOUGHTS_IN_JSON_RESPONSE_FORMAT: &str = "\n\n## Response format\nAnswer with exactly the JSON object the schema asks for. Its `thoughts` field is one or two short sentences: the deciding reason and your plan. It is the draft's only record of why you picked what you did, and it comes back to you as notes with your next pick.";

/// What a drafting seat is and is not told about the table it is at.
///
/// The pod is 2..=8 seats, and the rules text used to describe an 8-pod
/// whatever the pod actually was — the same bytes at `--players 2`, where
/// "passing left" and "passing right" are the same neighbour, a pack wheels
/// after one other pick rather than seven, and "read the signals" is advice
/// about inferring seven drafters' colours from one opponent's single pick
/// (issue #485). Nothing said which seat the reader was either (#115's shape,
/// fixed for the CLI and never asked of the draft).
#[derive(Debug, Clone, Copy)]
pub struct Table {
    pub seat: usize,
    pub pod_size: usize,
    pub pack_size: usize,
}

impl Table {
    /// The seat this one passes a pack to in `pack_number`, and the word for
    /// that direction — `None` in a one-seat pod, where a pack is not passed
    /// to anybody.
    ///
    /// The arithmetic is `(seat + 1) % pod_size`, which at `pod_size == 1`
    /// lands on the seat itself, so every pick prompt of a `--players 1` run
    /// told the seat it was passing the pack to itself (issue #608).
    #[must_use]
    fn passes_to(self, pack_number: usize) -> Option<(&'static str, usize)> {
        if self.pod_size <= 1 {
            return None;
        }
        // Packs 1 and 3 pass left (seat N -> N+1), pack 2 right — the same
        // rotation `DraftState::rotate_packs` performs.
        Some(if pack_number % 2 == 1 {
            ("LEFT", (self.seat + 1) % self.pod_size)
        } else {
            ("RIGHT", (self.seat + self.pod_size - 1) % self.pod_size)
        })
    }

    /// How the table reads from this seat: who else is here, when a pack
    /// comes back, and what can be read off it when it does.
    fn description(self) -> String {
        let Table { seat, pod_size, pack_size } = self;
        let others = pod_size.saturating_sub(1);

        // A one-seat pod is not a draft with fewer opponents, it is a draft
        // with none: no cards leave the packs, so the seat ends up with all
        // of them and there is nothing to read off a pack coming back. The
        // rest of this function assumes at least one opponent, and a 1-seat
        // pod used to fall into the `else` below — told it was reading
        // signals off "the 0 cards the other seats took" (issue #608, the
        // same shape as #485 one pod size lower down).
        if pod_size <= 1 {
            return format!(
                "\n## Your seat\n\
                 - You are the only drafter: a {pod_size}-seat pod, with no opponents at this table\n\
                 - Nobody takes cards out of your packs, so every card you are shown is a card \
you can still have — there are no signals to read and no colour anybody else is cutting\n\
                 \n## How drafting works\n\
                 - You'll open 3 packs of {pack_size} cards each and make {picks} picks in all\n\
                 - No pack is passed to another seat: what you leave comes back to you on your \
next pick, so by the end of a pack you will have taken all {pack_size} of its cards\n\
                 - The order you take them in therefore does not change your pool: every \
card in these three packs ends up in it. The deck you build from the {picks} cards \
afterwards is the whole exercise\n\
                 - After drafting, you'll build a 40-card deck from your picks plus basic lands\n",
                picks = 3 * pack_size,
            );
        }

        let mut s = format!(
            "\n## Your seat\n\
             - You are seat {seat} of a {pod_size}-seat pod, drafting against {others} other seat(s)\n\
             - Every seat picks at the same time, and you are never shown another seat's pick\n"
        );
        if pod_size == 2 {
            s.push_str(
                "- With one opponent, \"passing left\" and \"passing right\" are the same neighbour: the two packs shuttle between you, so the pack you pass comes back to you two picks later with exactly one card gone — and that card is your opponent's pick. You can read their pool exactly rather than guessing at signals\n",
            );
        } else {
            let _ = write!(
                s,
                "- A pack you pass comes back to you {pod_size} picks later, with the {others} cards the other seats took gone. Which cards those are is the signal: if strong cards of a colour keep coming back, nobody upstream is taking them and that colour is open\n"
            );
        }
        let _ = write!(
            s,
            "\n## How drafting works\n\
             - You'll open 3 packs of {pack_size} cards each and make {} picks in all\n\
             - For each pack, pick one card; the rest passes to the next seat\n\
             - Pack 1 passes left, Pack 2 passes right, Pack 3 passes left\n\
             - After drafting, you'll build a 40-card deck from your picks plus basic lands\n",
            3 * pack_size,
        );
        s
    }
}

/// Shared draft rules (used by all backends).
fn build_draft_rules(
    set_name: &str,
    guide: Option<&str>,
    card_reference: &str,
    table: Table,
) -> String {
    let guide_section = guide
        .map(|g| format!("\n## Draft Guide\n\n{g}\n"))
        .unwrap_or_default();
    format!(
        r#"You are drafting Magic: The Gathering cards from {set_name}.
{guide_section}{table_section}
## How to pick
- Build toward 2 colors (sometimes splashing a 3rd)
- Value bombs (powerful rares), removal, evasion (flying), then curve fillers
- Every card is listed as `name cost | type line size`, and a card in a pack ends with its rarity in brackets — `[common]`, `[uncommon]`, `[rare]`, `[mythic]`
- Cards with "//" are double-faced cards; both faces are on the line, and you evaluate the front face for casting
- Your pool is restated at every pick, with its colour counts and its curve, and so is the tail of your own reasoning from the pick before ("Your notes from your last pick"). {kept}

## Card reference

{card_reference}"#,
        table_section = table.description(),
        kept = history_note(mtg_player::llm::history_exchanges()),
    )
}

/// What the conversation keeps, as the draft rules say it: nothing by
/// default, the last N exchanges under `MTG_LLM_HISTORY=N`, which the rules
/// used to deny (#858).
fn history_note(history: usize) -> String {
    match history {
        0 => "Earlier picks are not kept in the conversation; the pool and the notes are what carries over".to_string(),
        n => format!("Up to {n} of your earlier exchanges are kept in the conversation and older ones are not; the pool and the notes are what carries over"),
    }
}

/// A card reference for the whole set: every card's name, cost, type line,
/// size and rules text, both faces of a double-faced card included.
///
/// This is `mtg_player`'s reference, not a second one — the draft used to
/// keep its own copy, which had already drifted (it dropped supertypes, so
/// a legendary creature's type line read "Creature").
pub fn build_card_reference(
    card_names: &[String],
    registry: &mtg_engine::cards::CardRegistry,
) -> String {
    mtg_player::llm::build_card_reference(card_names, registry)
}

/// Anthropic draft backend.
struct AnthropicDraftBackend {
    client: Client,
    api_key: String,
    model: String,
    system_prompt: String,
    /// The last `window` exchanges, re-sent with the next request. Every
    /// pick prompt restates the pack, the pool and the seat's place (#481),
    /// and the deck prompt restates the pool, so the exchanges before are
    /// the same information again — and 45 picks of growing history were
    /// a quadratic bill. Bounded like the game seat (`MTG_LLM_HISTORY`).
    conversation: Vec<serde_json::Value>,
    window: usize,
    /// The thinking block of the last answer, for the seat's notes.
    last_thinking: Option<String>,
}

impl AnthropicDraftBackend {
    fn new(model: &str, set_name: &str, guide: Option<&str>, card_reference: &str, table: Table) -> Self {
        let api_key = env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set");
        let system_prompt = format!(
            "{}{THOUGHTS_IN_JSON_RESPONSE_FORMAT}",
            build_draft_rules(set_name, guide, card_reference, table)
        );
        Self {
            client: Client::new(),
            api_key,
            model: model.to_string(),
            system_prompt,
            conversation: Vec::new(),
            window: mtg_player::llm::history_exchanges(),
            last_thinking: None,
        }
    }

    /// Send a message to Anthropic with the given JSON schema as
    /// structured output. Returns the raw response text (which should
    /// be valid JSON matching the schema), and the thinking block if the
    /// answer carried one.
    fn call_api_structured(
        &self,
        messages: &[serde_json::Value],
        schema: &serde_json::Value,
    ) -> (String, Option<String>) {
        let system = serde_json::json!([{
            "type": "text",
            "text": &self.system_prompt,
            "cache_control": {"type": "ephemeral"}
        }]);

        let mut msgs = messages.to_vec();
        if msgs.len() >= 2 {
            let idx = msgs.len() - 2;
            if let Some(content) = msgs[idx].get("content").and_then(|c| c.as_str()).map(std::string::ToString::to_string) {
                msgs[idx] = serde_json::json!({
                    "role": msgs[idx]["role"],
                    "content": [{"type": "text", "text": content, "cache_control": {"type": "ephemeral"}}]
                });
            }
        }

        // `thoughts` is kept, as on the game seat and this crate's
        // `claude -p` seat: at MTG_LLM_THINKING=off there is no thinking
        // block, and a model that omits its thinking returns the block
        // empty, so the field is where the pick's reasoning and the next
        // pick's notes come from.
        let sanitized = mtg_player::llm::sanitize_schema_for_anthropic(schema, true);
        let mut body = serde_json::json!({
            "model": &self.model,
            "system": system,
            "messages": msgs,
            "output_config": {
                "format": {
                    "type": "json_schema",
                    "schema": sanitized
                }
            }
        });
        // The same thinking level as the game seat (`MTG_LLM_THINKING`):
        // this is the request path that has missed fixes before (#404).
        mtg_player::llm::apply_thinking(&mut body, &self.model, mtg_player::llm::thinking_level());

        // The draft's own budget, through the loop every HTTP seat uses: six
        // fixed attempts over about half a minute ended a draft that the
        // `claude -p` seat would have ridden out (#218, #719). Past it the
        // draft stops, as it always has — quietly picking card 0 is worse.
        let (client, api_key, model) = (&self.client, &self.api_key, &self.model);
        let outcome = mtg_player::llm::call_within_budget(retry_budget(), |attempt| {
            let response = client
                .post(format!("{}/v1/messages", mtg_player::llm::anthropic_base_url()))
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send();
            match response {
                Ok(resp) if resp.status().is_success() => {
                    let json: serde_json::Value = resp.json().unwrap_or_default();
                    record_anthropic_usage(model, &json["usage"]);
                    // The text block, wherever it is: with thinking on, the
                    // first block is the thinking block, and reading
                    // `content[0].text` found nothing there and retried a
                    // perfectly good answer as "empty text".
                    let blocks = json["content"].as_array().cloned().unwrap_or_default();
                    let text = blocks.iter()
                        .find(|b| b["type"] == "text")
                        .and_then(|b| b["text"].as_str())
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    let thinking = blocks.iter()
                        .find(|b| b["type"] == "thinking")
                        .and_then(|b| b["thinking"].as_str())
                        .map(str::trim)
                        .filter(|t| !t.is_empty())
                        .map(str::to_string);
                    if text.is_empty() {
                        return mtg_player::llm::CallAttempt::Transient(
                            format!("Anthropic returned empty text (attempt {attempt})"));
                    }
                    mtg_player::llm::CallAttempt::Answer((text, thinking))
                }
                Ok(resp) => {
                    let code = resp.status().as_u16();
                    let text = resp.text().unwrap_or_default();
                    mtg_player::llm::classify_http_status(code, format!(
                        "Anthropic API error {code} (attempt {attempt}): {}",
                        text.chars().take(200).collect::<String>()))
                }
                Err(e) => mtg_player::llm::CallAttempt::Transient(
                    format!("Anthropic request failed (attempt {attempt}): {e}")),
            }
        }, |_, msg, retried| {
            crate::progress::say(msg);
            let label = if retried { "API_RETRY" } else { "API_ERROR" };
            mtg_player::game_log::write_at(mtg_player::game_log::LogLevel::Error, file!(), line!(), label, msg);
        });
        match outcome {
            mtg_player::llm::CallOutcome::Answer(answer) => answer,
            mtg_player::llm::CallOutcome::Failed(why) => fatal(&why),
            mtg_player::llm::CallOutcome::GaveUp(why) => fatal(&format!("Anthropic draft API {why}")),
        }
    }

    fn send_conv_structured(&mut self, message: &str, schema: &serde_json::Value) -> String {
        self.conversation.push(serde_json::json!({"role": "user", "content": message}));
        let (resp, thinking) = self.call_api_structured(&self.conversation.clone(), schema);
        self.conversation.push(serde_json::json!({"role": "assistant", "content": &resp}));
        // Keep the window and nothing older.
        let keep = self.window * 2;
        if self.conversation.len() > keep {
            let drop = self.conversation.len() - keep;
            self.conversation.drain(..drop);
        }
        self.last_thinking = thinking;
        resp
    }
}

impl DraftBackend for AnthropicDraftBackend {
    fn send_pick(&mut self, message: &str, num_cards: usize) -> String {
        let schema = mtg_player::llm::cache_stable_schema(&pick_schema_for(num_cards));
        self.send_conv_structured(message, &schema)
    }

    // Deckbuilding is the next turn of the same (bounded) conversation; the
    // prompt restates the whole pool, which is what the build is made of.
    fn send_deck_building(&mut self, message: &str, pool: &[String]) -> String {
        let schema = mtg_player::llm::cache_stable_schema(&deck_schema_for(pool));
        self.send_conv_structured(message, &schema)
    }

    fn send_sideboard(&mut self, message: &str, pool: &[String]) -> String {
        let schema = mtg_player::llm::cache_stable_schema(&sideboard_schema_for(pool));
        self.send_conv_structured(message, &schema)
    }

    fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    fn take_thinking(&mut self) -> Option<String> {
        self.last_thinking.take()
    }
}

/// The prefix this seat's scratch directory is named with. Not the one the
/// game seat uses, which is why nothing ever swept these (#206, #404).
const CLAUDE_CODE_WORKDIR_PREFIX: &str = "mtg-draft-claude-code-";
/// How long a seat keeps retrying a failing `claude -p` before the draft
/// gives up: the one budget the game seat uses too (#218, #587).
use mtg_player::llm::retry_backoff;

/// Overrides the retry budget, so the give-up path can be tested in seconds.
/// Not something a run should set.
const RETRY_BUDGET_ENV: &str = "MTG_DRAFT_RETRY_BUDGET_SECS";

fn retry_budget() -> std::time::Duration {
    mtg_player::llm::retry_budget(RETRY_BUDGET_ENV)
}

/// Prefix on the panic payload of a fatal LLM failure.
///
/// Exhausting the retries is deliberately fatal — a draft that quietly
/// picks card 0 for the rest of the run is worse than one that stops — but
/// it is an operational condition, not a bug, and the operator used to get
/// a worker-thread panic with a backtrace followed by a second panic
/// reading `called Result::unwrap() on an Err value: Any { .. }`. The
/// panic is still how the worker thread unwinds, but `main` recognises this
/// prefix, prints one line naming the seat, pack and pick, and exits
/// through `die` (issue #218).
pub const FATAL_MARKER: &str = "llm seat fatal: ";

/// Report a fatal LLM failure and unwind out of the seat's worker thread.
fn fatal(msg: &str) -> ! {
    mtg_player::game_log::write_at(
        mtg_player::game_log::LogLevel::Error, file!(), line!(), "API_FATAL", msg);
    panic!("{FATAL_MARKER}{msg}")
}

/// Claude Code draft backend: the same draft protocol as
/// [`AnthropicDraftBackend`], spoken through the `claude -p` CLI instead of
/// the Messages API. Whatever the CLI is logged into pays for it, so a
/// subscription seat spends plan quota and no API money — which is the only
/// reason to ask for this seat. Mirrors the game-side backend in mtg-player:
/// one subprocess per decision, the whole draft kept in a single CLI session
/// (`--session-id` on the first call, `--resume` after), every tool disabled,
/// and the pick/deck schema handed over as `--json-schema` — or, under
/// `MTG_CLAUDE_CODE_DRAFT_ANSWERS=text`, the answer's fields said at the end
/// of the prompt (`mtg_player::llm::answer_text`).
struct ClaudeCodeDraftBackend {
    binary: String,
    /// Model alias/name passed as `--model`; `None` leaves the CLI default.
    /// This is a CLI alias ("opus", "sonnet"), not an API model id.
    model: Option<String>,
    /// Label used for usage accounting.
    label: String,
    system_prompt: String,
    /// The session id once the first call has created it; `--resume`d for
    /// `window` more decisions, then replaced (see `MTG_LLM_HISTORY`): a
    /// resumed session re-reads its whole history on every call, and every
    /// pick prompt restates the pack and the pool anyway (#481).
    session_id: Option<String>,
    window: usize,
    /// Decisions the current session has carried.
    turns: usize,
    /// Scratch working directory for the subprocess, so no project
    /// `CLAUDE.md`, settings, or hooks from the caller's cwd leak into the
    /// draft prompt.
    workdir: PathBuf,
    /// A pick's full schema while its call is out, for a stand-in CLI
    /// (`mtg_player::llm::CLAUDE_CODE_ANSWER_SCHEMA_ENV`).
    answer_schema: Option<String>,
    /// Ask for the answer in the prompt rather than with `--json-schema`.
    text_answers: bool,
    /// Write this call's cache at the hour TTL. A sideboarding decision
    /// comes a game after the seat's last, past five minutes, so its prefix
    /// — the rules and the set's card reference, about 15,000 tokens — was
    /// written whole on every one; at the hour the seat's first writes it
    /// and the rest read it (`reports/llm-cost.md`, phase 5).
    hour_cache: bool,
}

/// `MTG_CLAUDE_CODE_DRAFT_ANSWERS`: `schema` (the default) or `text`, how a
/// `claude -p` draft seat is asked for its answer.
///
/// The game seat answers in text (`mtg_player::llm::answer_text`); a pick
/// does not by default. Text saved a pick 6% — its price is reading the
/// set's card reference, not the answer's round trip — and on the kept
/// pick eval it agreed with the reference on 200 of 278 settled picks where
/// `--json-schema` agreed on 210, lower in both samples
/// (`reports/llm-cost.md`, phase 5).
pub const DRAFT_ANSWERS_ENV: &str = "MTG_CLAUDE_CODE_DRAFT_ANSWERS";

/// Whether a `claude -p` draft seat is asked for its answer in text.
fn draft_text_answers() -> bool {
    std::env::var(DRAFT_ANSWERS_ENV).as_deref() == Ok("text")
}

/// The draft rules' answer note for a seat asked in text: the CLI has no
/// schema to hold the answer to, so it is told the shape is the whole reply.
const TEXT_ANSWER_NOTE: &str = "\nReply with only that JSON object — no words before or after it, no code \
fence. Its fields are listed at the end of each question.";

impl ClaudeCodeDraftBackend {
    fn new(model: Option<&str>, set_name: &str, guide: Option<&str>, card_reference: &str, table: Table) -> Self {
        Self::with_binary(&mtg_player::llm::claude_code_binary(), model, set_name, guide, card_reference, table)
    }

    fn with_binary(
        binary: &str,
        model: Option<&str>,
        set_name: &str,
        guide: Option<&str>,
        card_reference: &str,
        table: Table,
    ) -> Self {
        let workdir = mtg_player::llm::claude_code_prepare_seat(CLAUDE_CODE_WORKDIR_PREFIX);
        let label = match model {
            Some(m) => format!("claude-code:{m}"),
            None => "claude-code".to_string(),
        };
        let text_answers = draft_text_answers();
        Self {
            binary: binary.to_string(),
            model: model.map(std::string::ToString::to_string),
            label,
            system_prompt: format!(
                "{}{THOUGHTS_IN_JSON_RESPONSE_FORMAT}{}",
                build_draft_rules(set_name, guide, card_reference, table),
                if text_answers { TEXT_ANSWER_NOTE } else { "" },
            ),
            session_id: None,
            window: mtg_player::llm::history_exchanges(),
            turns: 0,
            workdir,
            answer_schema: None,
            text_answers,
            hour_cache: false,
        }
    }

    /// A fresh RFC 4122 version-4 id for `--session-id`.
    fn fresh_session_id() -> String {
        let mut b = rand::random::<u128>().to_be_bytes();
        b[6] = (b[6] & 0x0f) | 0x40;
        b[8] = (b[8] & 0x3f) | 0x80;
        let h: Vec<String> = b.iter().map(|x| format!("{x:02x}")).collect();
        format!(
            "{}{}{}{}-{}{}-{}{}-{}{}-{}{}{}{}{}{}",
            h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7],
            h[8], h[9], h[10], h[11], h[12], h[13], h[14], h[15]
        )
    }

    /// One schema-constrained decision. Returns the model's object
    /// pretty-printed, which is what both callers parse. Retries a spawn
    /// failure, a non-zero exit, a timeout, unparsable output, or an
    /// `is_error` result; exhausting the attempts is fatal, exactly as it is
    /// on the API path — a draft that quietly picks card 0 for the rest of
    /// the run is worse than one that stops.
    fn decide(&mut self, message: &str, schema: &serde_json::Value) -> String {
        // `thoughts` is KEPT for this backend, exactly as the game side keeps
        // it (#213). The CLI's result object gives the harness no thinking
        // block to read, so a schema stripped of the field left the seat's
        // reasoning recorded nowhere at all — 42 picks and a deck build with
        // no THOUGHT and no reasoning in the `RESPONSE` record, in the same
        // log where the tournament recorded 202 (issue #607). This is the
        // seat the project actually drafts with.
        let sanitized = mtg_player::llm::sanitize_schema_for_anthropic(schema, true);
        // A session that has carried its window of decisions is not
        // resumed: every call to it would re-read all of them.
        if self.turns > self.window {
            self.session_id = None;
            self.turns = 0;
        }
        let began = std::time::Instant::now();
        let deadline = began + retry_budget();
        let mut attempt = 0u32;
        // The reason the last attempt failed, for the fatal record. Each
        // attempt's reason goes to stderr as it happens, but the `--log`'s
        // `API_FATAL` record is the durable one — after the terminal has
        // scrolled it is all a reader has, and carrying only a count meant
        // a draft log could not tell a hung CLI from a crashing one (#585).
        let mut last_failure = String::new();
        loop {
            if attempt > 0 {
                let backoff = retry_backoff(attempt);
                // Sleeping past the budget would spend the whole window on
                // one wait; stop instead and report.
                if std::time::Instant::now() + backoff > deadline {
                    break;
                }
                std::thread::sleep(backoff);
            }
            attempt += 1;
            let started = std::time::Instant::now();
            match self.call_once(message, &sanitized) {
                Ok(json) => {
                    if json["is_error"].as_bool() == Some(true) {
                        let msg = format!(
                            "claude -p reported an error (attempt {}, {}ms): {}",
                            attempt, started.elapsed().as_millis(),
                            json["result"].as_str().unwrap_or("").chars().take(200).collect::<String>()
                        );
                        crate::progress::say(&msg);
                        mtg_player::game_log::write_at(mtg_player::game_log::LogLevel::Error, file!(), line!(), "API_RETRY", &msg);
                        last_failure = format!("reported an error: {}",
                            json["result"].as_str().unwrap_or("").chars().take(200).collect::<String>());
                        continue;
                    }
                    if let Some(sid) = json["session_id"].as_str() {
                        // The id the CLI reports is authoritative; on the
                        // first call it is the one we asked for.
                        //
                        // Written down the first time it is seen. A seat
                        // mints a fresh uuid per conversation and `--resume`s
                        // it for the rest of the draft, and none of them
                        // appeared in the log, the snapshot, or on stderr —
                        // so the CLI's own transcript of a seat's 42 picks
                        // existed and was unfindable, and "were these calls
                        // one session?" could only be answered by re-running
                        // the draft under a wrapper (issue #542).
                        if self.session_id.as_deref() != Some(sid) {
                            mtg_player::game_log::write_at(
                                mtg_player::game_log::LogLevel::Info,
                                file!(), line!(), "SESSION", sid);
                        }
                        self.session_id = Some(sid.to_string());
                    }
                    self.turns += 1;
                    let usage = &json["usage"];
                    record_model_usage(
                        &self.label,
                        usage["input_tokens"].as_u64().unwrap_or(0),
                        usage["output_tokens"].as_u64().unwrap_or(0),
                        usage["cache_read_input_tokens"].as_u64().unwrap_or(0),
                        usage["cache_creation_input_tokens"].as_u64().unwrap_or(0),
                    );
                    if let Some(object) = Self::structured(&json) {
                        return serde_json::to_string_pretty(&object)
                            .unwrap_or_else(|_| object.to_string());
                    }
                    let msg = format!(
                        "claude -p returned no structured object (attempt {}): {}",
                        attempt,
                        json["result"].as_str().unwrap_or("").chars().take(200).collect::<String>()
                    );
                    crate::progress::say(&format!("WARN: {msg}"));
                    mtg_player::game_log::write_at(mtg_player::game_log::LogLevel::Error, file!(), line!(), "API_WARN", &msg);
                    last_failure = format!("returned no structured object: {}",
                        json["result"].as_str().unwrap_or("").chars().take(200).collect::<String>());
                }
                Err(e) => {
                    let msg = format!(
                        "claude -p failed (attempt {}, {}ms): {e}",
                        attempt, started.elapsed().as_millis()
                    );
                    crate::progress::say(&msg);
                    mtg_player::game_log::write_at(mtg_player::game_log::LogLevel::Error, file!(), line!(), "API_ERROR", &msg);
                    last_failure = e;
                }
            }
        }
        fatal(&Self::gave_up_message(attempt, began.elapsed(), retry_budget(), &last_failure))
    }

    /// The one sentence an operator gets when a draft seat dies.
    ///
    /// It used to report the configured retry budget where the duration
    /// belongs — "gave up after 1 attempts over 6s" after sixty seconds of
    /// trying (#585). The loop stops as soon as the next backoff would
    /// overshoot the deadline, so the budget and the time spent are equal
    /// only by coincidence; they diverge with the ratio of the per-call
    /// timeout to the budget, and at the shipped defaults they happen to
    /// land close enough to hide it.
    ///
    /// So: the measured elapsed time, the budget named as the budget (it is
    /// the knob to turn), the attempt count pluralised, and the reason the
    /// last attempt failed.
    fn gave_up_message(
        attempts: u32,
        elapsed: std::time::Duration,
        budget: std::time::Duration,
        last_failure: &str,
    ) -> String {
        let plural = if attempts == 1 { "attempt" } else { "attempts" };
        let mut msg = format!(
            "claude -p draft seat gave up after {attempts} {plural} in {:.1}s (budget {}s)",
            elapsed.as_secs_f64(), budget.as_secs());
        if !last_failure.is_empty() {
            msg.push_str(&format!("; last failure: {last_failure}"));
        }
        msg
    }

    fn call_once(&mut self, message: &str, schema: &serde_json::Value) -> Result<serde_json::Value, String> {
        // The draft seat has its own request path — #404 is what happens
        // when a fix to the game backend never reaches this copy — so the
        // rule the game side asserts is asserted here too. A top-level
        // property key the API refuses is a 400 before the model reads
        // anything, and the seat cannot tell that apart from an answer it
        // did not like (#398). No draft schema keys anything by a card name
        // (the deck build answers by row since #867); this keeps it so.
        debug_assert!(
            schema.get("properties").and_then(serde_json::Value::as_object)
                .is_none_or(|p| p.keys().all(|k| mtg_player::llm::schema_key_is_legal(k))),
            "a top-level draft schema key is one the API will refuse (#398): {:?}",
            schema.get("properties").and_then(serde_json::Value::as_object)
                .map(|p| p.keys().filter(|k| !mtg_player::llm::schema_key_is_legal(k))
                    .cloned().collect::<Vec<_>>()));
        let mut cmd = Command::new(&self.binary);
        cmd.arg("-p")
            .args(["--output-format", "json"])
            .args(["--tools", ""])
            // The system prompt is sent every call: a resumed session keeps
            // its history, and restating the prompt is harmless.
            .args(["--system-prompt", &self.system_prompt]);
        // In text the fields end the prompt and the answer is one request;
        // `--json-schema` is a tool call and a second request after it.
        let asked = if self.text_answers {
            let full = self.answer_schema.as_deref().and_then(|t| serde_json::from_str(t).ok());
            Some(format!("{message}\n\n{}", mtg_player::llm::answer_text::answer_format(full.as_ref().unwrap_or(schema))))
        } else {
            cmd.args(["--json-schema", &schema.to_string()]);
            None
        };
        if let Some(full) = &self.answer_schema {
            cmd.env(mtg_player::llm::CLAUDE_CODE_ANSWER_SCHEMA_ENV, full);
        }
        if self.hour_cache {
            cmd.env(mtg_player::llm::CLAUDE_CODE_CACHE_TTL_ENV, "1h");
        }
        match &self.session_id {
            Some(id) => {
                cmd.args(["--resume", id]);
            }
            None => {
                cmd.args(["--session-id", &Self::fresh_session_id()]);
            }
        }
        if let Some(m) = &self.model {
            cmd.args(["--model", m]);
        }
        // The effort the API seat sends at this level, as the game's
        // `claude -p` seat passes it: without it the CLI runs its own
        // default effort, which thought ten to a hundred times as much
        // (reports/llm-cost.md, phase 3).
        if mtg_player::llm::model_takes_effort(self.model.as_deref()) {
            cmd.args(["--effort", mtg_player::llm::effort_for(mtg_player::llm::thinking_level())]);
        }
        cmd.current_dir(&self.workdir);
        // One subprocess driver for both `claude -p` seats in this
        // workspace. This was a copy of it, taken on 2026-09-04, and the
        // fixes for #203 and #206 landed in the other crate a day later and
        // never reached it: no process group, so the watchdog killed the
        // wrapper and its own child kept holding stdout; a read that ended
        // on EOF, so the call never returned; no signal handlers, so a
        // SIGTERM orphaned every seat's subprocess to init; and no scratch
        // sweep for this prefix. A hung wrapper stopped a draft mid-pick,
        // silently, forever, with nothing on the terminal or in the log
        // (issue #404).
        mtg_player::llm::claude_code_run(&mut cmd, &self.binary, asked.as_deref().unwrap_or(message))
    }

    /// The structured object of a result: the CLI's parsed
    /// `structured_output` when a schema was given, else the JSON object in
    /// the result text.
    fn structured(json: &serde_json::Value) -> Option<serde_json::Value> {
        if json["structured_output"].is_object() {
            return Some(json["structured_output"].clone());
        }
        json["result"].as_str().and_then(mtg_player::llm::answer_text::json_in_text)
    }
}

impl Drop for ClaudeCodeDraftBackend {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.workdir);
    }
}

impl DraftBackend for ClaudeCodeDraftBackend {
    fn send_pick(&mut self, message: &str, num_cards: usize) -> String {
        let full = pick_schema_for(num_cards);
        self.answer_schema = Some(full.to_string());
        let response = self.decide(message, &mtg_player::llm::cache_stable_schema(&full));
        self.answer_schema = None;
        response
    }

    // Deckbuilding is the next decision of the same (bounded) session; the
    // prompt restates the whole pool, which is what the build is made of.
    fn send_deck_building(&mut self, message: &str, pool: &[String]) -> String {
        let full = deck_schema_for(pool);
        self.answer_schema = Some(full.to_string());
        let response = self.decide(message, &mtg_player::llm::cache_stable_schema(&full));
        self.answer_schema = None;
        response
    }

    fn send_sideboard(&mut self, message: &str, pool: &[String]) -> String {
        let full = sideboard_schema_for(pool);
        self.answer_schema = Some(full.to_string());
        self.hour_cache = true;
        let response = self.decide(message, &mtg_player::llm::cache_stable_schema(&full));
        self.hour_cache = false;
        self.answer_schema = None;
        response
    }

    fn system_prompt(&self) -> &str {
        &self.system_prompt
    }
}

/// Gemini draft backend using the Interactions API.
struct GeminiDraftBackend {
    client: Client,
    api_key: String,
    model: String,
    draft_thinking: Option<String>,
    system_prompt: String,
    /// The interaction chain, shared by pick and deckbuilding turns, and
    /// restarted every `window` decisions (`MTG_LLM_HISTORY`): the server
    /// re-reads the whole chain on every call, and every prompt restates
    /// what it needs.
    interaction_id: Option<String>,
    window: usize,
    /// Decisions the current chain has carried.
    chain_len: usize,
}

impl GeminiDraftBackend {
    fn new(model: &str, set_name: &str, guide: Option<&str>, draft_thinking: Option<String>, card_reference: &str, table: Table) -> Self {
        let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
        let system_prompt = format!(
            "{}{THOUGHTS_IN_JSON_RESPONSE_FORMAT}",
            build_draft_rules(set_name, guide, card_reference, table)
        );
        Self {
            client: Client::new(),
            api_key,
            model: model.to_string(),
            draft_thinking,
            system_prompt,
            interaction_id: None,
            window: mtg_player::llm::history_exchanges(),
            chain_len: 0,
        }
    }

    /// The chain to continue, if it has not carried its window yet.
    fn chain_to_continue(&mut self) -> Option<String> {
        if self.chain_len > self.window {
            self.interaction_id = None;
            self.chain_len = 0;
        }
        self.interaction_id.take()
    }

    fn call_interactions(
        &self,
        message: &str,
        schema: &serde_json::Value,
        prev_id: Option<&str>,
    ) -> (String, Option<String>) {
        let mut body = serde_json::json!({
            "model": &self.model,
            "input": message,
            "response_mime_type": "application/json",
            // The draft is the second request path, and the one that has
            // already missed a fix the game path got (#404). Same sanitizer,
            // not a second copy of it (#546).
            "response_format": mtg_player::llm::sanitize_schema_for_gemini(schema),
        });

        if let Some(ref level) = self.draft_thinking {
            body["generation_config"] = serde_json::json!({"thinking_level": level});
        }

        // Only chain if we have a non-empty previous interaction ID.
        if let Some(prev) = prev_id.filter(|s| !s.is_empty()) {
            body["previous_interaction_id"] = serde_json::json!(prev);
        } else {
            body["system_instruction"] = serde_json::json!(&self.system_prompt);
        }

        let url = format!("{}/v1beta/interactions?key={}", mtg_player::llm::gemini_base_url(), self.api_key);

        // The draft's budget through the shared loop, like the Anthropic
        // path above (#719).
        let mut fresh_retry = false;
        let (client, model, system_prompt) = (&self.client, &self.model, &self.system_prompt);
        let outcome = mtg_player::llm::call_within_budget(retry_budget(), |attempt| {
            let response = client
                .post(&url)
                .header("content-type", "application/json")
                .json(&body)
                .send();

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let json: serde_json::Value = resp.json().unwrap_or_default();
                    record_gemini_usage(model, &json["usage"]);
                    let id = json["id"].as_str()
                        .filter(|s| !s.is_empty())
                        .map(std::string::ToString::to_string);
                    let mut text = String::new();
                    if let Some(outputs) = json["outputs"].as_array() {
                        for out in outputs {
                            if out["type"].as_str() == Some("text") {
                                if let Some(t) = out["text"].as_str() {
                                    text = t.trim().to_string();
                                }
                            }
                        }
                    }
                    if text.is_empty() {
                        return mtg_player::llm::CallAttempt::Transient(format!(
                            "Gemini returned empty text (attempt {attempt}, interaction_id: {id:?})"));
                    }
                    mtg_player::llm::CallAttempt::Answer((text, id))
                }
                Ok(resp) => {
                    let code = resp.status().as_u16();
                    let text = resp.text().unwrap_or_default();
                    // If the interaction ID is invalid, fall back to a fresh conversation.
                    if code == 400 && text.contains("previous_interaction_id") && !fresh_retry {
                        let msg = format!("Invalid interaction ID, falling back to fresh conversation ({})",
                            text.chars().take(150).collect::<String>());
                        crate::progress::say(&format!("WARN: {msg}"));
                        mtg_player::game_log::write_at(mtg_player::game_log::LogLevel::Error, file!(), line!(), "API_WARN", &msg);
                        body.as_object_mut().unwrap().remove("previous_interaction_id");
                        body["system_instruction"] = serde_json::json!(system_prompt);
                        fresh_retry = true;
                        return mtg_player::llm::CallAttempt::Transient(
                            format!("Gemini 400 (attempt {attempt}): stale interaction id"));
                    }
                    // Fatal config errors — abort loudly so we don't silently produce garbage.
                    if code == 400 && (text.contains("thinking level") || text.contains("not a supported")) {
                        let msg = format!("Gemini config error: {}", text.chars().take(300).collect::<String>());
                        mtg_player::game_log::write_at(mtg_player::game_log::LogLevel::Error, file!(), line!(), "API_FATAL", &msg);
                        // Through the one fatal exit, so this stop accounts
                        // for what the run spent and sweeps the live calls
                        // the same way every other fatal does (#537, #578).
                        crate::die(&msg);
                    }
                    mtg_player::llm::classify_http_status(code, format!(
                        "Gemini API error {code} (attempt {attempt}): {}", text.chars().take(200).collect::<String>()))
                }
                Err(e) => mtg_player::llm::CallAttempt::Transient(
                    format!("Gemini request failed (attempt {attempt}): {e}")),
            }
        }, |_, msg, retried| {
            crate::progress::say(msg);
            let label = if retried { "API_RETRY" } else { "API_ERROR" };
            mtg_player::game_log::write_at(mtg_player::game_log::LogLevel::Error, file!(), line!(), label, msg);
        });
        match outcome {
            mtg_player::llm::CallOutcome::Answer(answer) => answer,
            mtg_player::llm::CallOutcome::Failed(why) => fatal(&why),
            mtg_player::llm::CallOutcome::GaveUp(why) => fatal(&format!("Gemini draft API {why}")),
        }
    }

    /// Re-serialize the Gemini JSON response to a canonical pretty form.
    /// Returns the raw response unchanged if it's not valid JSON.
    fn normalize_response(raw: &str) -> String {
        match serde_json::from_str::<serde_json::Value>(raw) {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| raw.to_string()),
            Err(_) => raw.to_string(),
        }
    }
}

impl DraftBackend for GeminiDraftBackend {
    fn send_pick(&mut self, message: &str, num_cards: usize) -> String {
        let prev = self.chain_to_continue();
        let schema = pick_schema_for(num_cards);
        let (raw, new_id) = self.call_interactions(message, &schema, prev.as_deref());
        self.interaction_id = new_id;
        self.chain_len += 1;
        Self::normalize_response(&raw)
    }

    // Deckbuilding is the next decision of the same (bounded) chain; the
    // prompt restates the whole pool, which is what the build is made of.
    fn send_deck_building(&mut self, message: &str, pool: &[String]) -> String {
        let prev = self.chain_to_continue();
        // As built, like a pick: the sanitizer keeps the row count as an
        // array length and drops the per-row slots.
        let schema = deck_schema_for(pool);
        let (raw, new_id) = self.call_interactions(message, &schema, prev.as_deref());
        self.interaction_id = new_id;
        self.chain_len += 1;
        Self::normalize_response(&raw)
    }

    fn send_sideboard(&mut self, message: &str, pool: &[String]) -> String {
        let prev = self.chain_to_continue();
        let schema = sideboard_schema_for(pool);
        let (raw, new_id) = self.call_interactions(message, &schema, prev.as_deref());
        self.interaction_id = new_id;
        self.chain_len += 1;
        Self::normalize_response(&raw)
    }

    fn system_prompt(&self) -> &str {
        &self.system_prompt
    }
}

/// The model providers a seat spec may name, for every message that has to
/// list them.
/// The key variable a metered provider's seat needs, when it is not set
/// (or is blank): `claude` bills `ANTHROPIC_API_KEY`, `gemini`
/// `GEMINI_API_KEY`; any other provider needs none. The backends `expect`
/// the variable, so a seat without it was dealt in and then panicked at its
/// first pick (#903): both binaries refuse it before anything is dealt.
#[must_use]
pub fn missing_api_key(provider: &str) -> Option<&'static str> {
    let var = metered_key(provider)?;
    if std::env::var(var).is_ok_and(|v| !v.trim().is_empty()) { None } else { Some(var) }
}

/// The key variable a provider bills per call, if it is a metered one:
/// `cc` (Claude Code) runs on plan quota and bills nothing per call.
#[must_use]
pub fn metered_key(provider: &str) -> Option<&'static str> {
    match provider {
        "claude" => Some("ANTHROPIC_API_KEY"),
        "gemini" => Some("GEMINI_API_KEY"),
        _ => None,
    }
}

/// How the host's terminal names an AI seat: its model spec, and what it
/// bills when it is metered. Start-up and `status` said only "ai", the
/// same for a plan-quota `cc` seat and one billing the host's
/// `ANTHROPIC_API_KEY` from its first pick (#902).
#[must_use]
pub fn ai_seat_label(spec: &str) -> String {
    match metered_key(spec.split(':').next().unwrap_or("")) {
        Some(var) => format!("ai {spec} (metered: bills {var} per call)"),
        None => format!("ai {spec}"),
    }
}

pub const ACCEPTED_PROVIDERS: &str = "claude[:model], gemini[:model], claude-code[:model], cc[:model]";

/// LLM client for draft picks and deck building.
pub struct DraftLlmClient {
    backend: Box<dyn DraftBackend>,
    /// The tail of the seat's reasoning from its last pick, restated at
    /// the top of its next prompt. The conversation no longer carries the
    /// earlier picks (see `MTG_LLM_HISTORY`), so this is how "I am
    /// green-white, taking removal over creatures now" survives from one
    /// pick to the next.
    notes: Option<String>,
}

impl DraftLlmClient {
    /// Create a new `DraftLlmClient`.
    /// Model spec format: "`provider:model:draft_thinking:game_thinking`"
    pub fn new(
        model_spec: &str,
        set_name: &str,
        guide: Option<&str>,
        card_reference: &str,
        table: Table,
    ) -> Self {
        let parts: Vec<&str> = model_spec.split(':').collect();
        let provider_name = parts[0];
        let model_override = parts.get(1).copied();
        let draft_thinking_override = parts.get(2).map(std::string::ToString::to_string);
        let _game_thinking_override = parts.get(3).map(std::string::ToString::to_string);

        let backend: Box<dyn DraftBackend> = match provider_name {
            "gemini" => {
                let model = model_override.unwrap_or("gemini-2.5-flash");
                let draft_thinking = draft_thinking_override.unwrap_or_else(|| "high".to_string());
                Box::new(GeminiDraftBackend::new(model, set_name, guide, Some(draft_thinking), card_reference, table))
            }
            // The whole point of this seat is that the draft does not bill an
            // API. Falling through to Anthropic here ran the picks and the
            // deck build on metered tokens while only the games used the
            // subscription, and passed a CLI alias like "opus" to the
            // Messages API as if it were a model id.
            "claude-code" | "cc" => Box::new(ClaudeCodeDraftBackend::new(
                model_override, set_name, guide, card_reference, table,
            )),
            "claude" => {
                let model = model_override.unwrap_or(mtg_player::llm::DEFAULT_ANTHROPIC_MODEL);
                Box::new(AnthropicDraftBackend::new(model, set_name, guide, card_reference, table))
            }
            // An unrecognized provider used to default to Anthropic, which
            // spent real money drafting with a model nobody asked for. Refuse
            // it the way mtg-runner refuses an unknown seat.
            other => {
                mtg_player::stderr_line!("Error: unknown model provider '{other}' (expected {ACCEPTED_PROVIDERS})");
                std::process::exit(1);
            }
        };
        Self { backend, notes: None }
    }

    /// The most characters of reasoning carried into the next prompt: the
    /// two sentences `thoughts` asks for, as on the game seat.
    const NOTES_MAX_CHARS: usize = 300;

    /// Keep the tail of this answer's reasoning as the notes for the next
    /// prompt: the response's own `thoughts`, else the thinking block where
    /// the backend reads one.
    fn keep_notes(&mut self, response: &str) {
        // The answer's own `thoughts` first: every backend asks for them,
        // and a thinking block can come back empty.
        let thinking = self.backend.take_thinking();
        let thoughts = serde_json::from_str::<serde_json::Value>(response).ok()
            .and_then(|v| v["thoughts"].as_str().map(str::to_string))
            .filter(|t| !t.trim().is_empty())
            .or(thinking);
        let Some(text) = thoughts.map(|t| t.trim().to_string()).filter(|t| !t.is_empty()) else {
            return;
        };
        let chars = text.chars().count();
        self.notes = Some(if chars <= Self::NOTES_MAX_CHARS {
            text
        } else {
            let tail: String = text.chars().skip(chars - Self::NOTES_MAX_CHARS).collect();
            let from_word = tail.find(char::is_whitespace).map_or(0, |i| i + 1);
            format!("…{}", tail[from_word..].trim_start())
        });
    }

    /// The seat's notes from its last pick, for the next prompt.
    #[must_use]
    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }

    /// Put back the notes a snapshot kept, before its picks are replayed
    /// (#836).
    pub fn restore_notes(&mut self, notes: Option<String>) {
        self.notes = notes;
    }

    /// The notes block a prompt opens with, when there are notes.
    #[must_use]
    pub fn notes_section(notes: Option<&str>) -> String {
        notes.map(|n| format!("Your notes from your last pick:\n{n}\n\n")).unwrap_or_default()
    }

    /// Build the prompt for a draft pick.
    ///
    /// Self-sufficient, because nothing else is guaranteed to carry the
    /// state: the pool used to be restated at exactly 2 of a seat's 42 picks
    /// (pack 2 pick 1 and pack 3 pick 1), with the conversation the seat's
    /// `claude -p` session holds as the only other channel — and a `--resume`
    /// starts a *fresh* session, so a resumed seat drafted nine consecutive
    /// picks knowing nothing about the cards it already owned (issue #481).
    /// Every pick now carries the pack, the pool, the pool's shape and the
    /// seat's place at the table, the way the game harness restates the whole
    /// position on every call.
    pub fn build_pick_prompt(
        table: Table,
        pack_number: usize,
        pick_index: usize,
        available: &[String],
        pool: &[String],
        cards: &CardLines,
    ) -> String {
        // A one-seat pod passes to nobody: the rest of the pack comes back to
        // this seat on its next pick (issue #608).
        let handoff = match table.passes_to(pack_number) {
            Some((direction, next_seat)) => {
                format!("after your pick this pack passes {direction} to seat {next_seat}")
            }
            None => "you are the only drafter, so the rest of this pack comes straight \
back to you on your next pick"
                .to_string(),
        };
        let mut prompt = format!(
            "Pack {pack_number} of 3, Pick {pick_index} of {}. You are seat {} of {}; \
{handoff}.\n\nAvailable ({} cards):\n",
            table.pack_size, table.seat, table.pod_size, available.len(),
        );
        for (i, card) in available.iter().enumerate() {
            writeln!(prompt, "{i}: {}", cards.pack_line(card)).unwrap();
        }
        if pool.is_empty() {
            prompt.push_str("\nYour pool is empty — this is your first pick.\n");
        } else {
            writeln!(prompt, "\nYour pool ({} cards):", pool.len()).unwrap();
            prompt.push_str(&cards.pool_listing(pool));
        }
        prompt
    }

    /// Send a pick message with the pack size, so the backend can build
    /// the pick schema for it.
    ///
    /// The Anthropic backends send that schema without its enum, so the
    /// answer is held to it here: a pick outside the pack is asked once
    /// more, with the reason, rather than substituted with the first card.
    pub fn send_pick_message(&mut self, user_message: &str, num_cards: usize) -> String {
        let mut response = self.backend.send_pick(user_message, num_cards);
        let why = serde_json::from_str::<serde_json::Value>(&response).ok()
            .and_then(|answer| mtg_player::llm::schema_violation(&pick_schema_for(num_cards), &answer));
        if let Some(why) = why {
            mtg_player::game_log::write_at(mtg_player::game_log::LogLevel::Error, file!(), line!(),
                "REASK", &format!("{why}: {}", response.trim()));
            let again = format!(
                "{user_message}\n\nYour answer {} is not one this pick allows: {why}. Answer again.",
                response.trim());
            response = self.backend.send_pick(&again, num_cards);
        }
        self.keep_notes(&response);
        response
    }

    /// Send a deck-building message with the player's full card pool, whose
    /// rows the answer counts. The copies are held to the pool when the
    /// answer is read (`parse_deck_response_by_rows`, `validate_deck`), and
    /// a bad answer is quoted back by `build_deck_with_llm`'s retry loop.
    pub fn send_deck_building_message(&mut self, user_message: &str, pool: &[String]) -> String {
        let response = self.backend.send_deck_building(user_message, pool);
        self.keep_notes(&response);
        response
    }

    /// Send a between-games message: the deck for the next game and the
    /// play-or-draw choice. A bad answer keeps the deck as it is (the
    /// caller reads it), so it is not asked again.
    pub fn send_sideboard_message(&mut self, user_message: &str, pool: &[String]) -> String {
        let response = self.backend.send_sideboard(user_message, pool);
        self.keep_notes(&response);
        response
    }

    /// The full system prompt this client will send to the model.
    pub fn system_prompt(&self) -> &str {
        self.backend.system_prompt()
    }
}

#[cfg(test)]
mod usage_summary_tests {
    use super::{as_llm_usage, ModelUsage};

    /// #489: the tournament's usage was copied into this crate's struct
    /// field by field, `rejected` was not in the list, and a run with 14
    /// unusable game answers printed a summary with no rejection in it.
    #[test]
    fn every_counter_survives_the_round_trip() {
        let usage = ModelUsage {
            input: 11,
            output: 22,
            cache_read: 33,
            cache_create: 44,
            calls: 55,
            rejected: 66,
            unanswered: 77,
        };
        let shared = as_llm_usage(&usage);

        assert_eq!(shared.input, 11);
        assert_eq!(shared.output, 22);
        assert_eq!(shared.cache_read, 33);
        assert_eq!(shared.cache_create, 44);
        assert_eq!(shared.calls, 55);
        assert_eq!(shared.rejected, 66, "a rejected answer is what the summary exists to say");
        assert_eq!(shared.unanswered, 77,
            "a decision the backend never answered is the other thing the summary exists to say");

        // And a clone keeps them all — the hand-written `Clone` that dropped
        // fields silently is how one went missing (#489).
        let copy = usage.clone();
        assert_eq!(copy.rejected, usage.rejected);
        assert_eq!(copy.unanswered, usage.unanswered);
        assert_eq!(copy.calls, usage.calls);
    }

    /// The line a reader sees, shared with `mtg-runner` so the two runners
    /// cannot say it differently — or, as here, one of them not at all.
    #[test]
    fn the_rejection_suffix_reads_the_same_in_both_runners() {
        assert_eq!(mtg_player::llm::rejected_note(0), "");
        assert_eq!(mtg_player::llm::rejected_note(1), ", 1 answer rejected → fallback");
        assert_eq!(mtg_player::llm::rejected_note(14), ", 14 answers rejected → fallback");
    }
}

#[cfg(test)]
mod draft_prompt_tests {
    /// #902: a metered seat says what it bills; a cc seat does not.
    #[test]
    fn an_ai_seat_is_named_by_its_model_and_what_it_bills() {
        assert_eq!(super::ai_seat_label("cc:claude-sonnet-4-6"), "ai cc:claude-sonnet-4-6");
        assert_eq!(super::ai_seat_label("claude:claude-haiku-4-5"),
            "ai claude:claude-haiku-4-5 (metered: bills ANTHROPIC_API_KEY per call)");
        assert_eq!(super::ai_seat_label("gemini"), "ai gemini (metered: bills GEMINI_API_KEY per call)");
    }

    use super::{build_draft_rules, deck_schema_for, DraftLlmClient, Table};
    use crate::card_lines::CardLines;

    fn cards() -> CardLines {
        let set_data = mtg_draft::set_data::SetData::load(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../data/sets/isd.json"
        )))
        .expect("ISD set data");
        let registry = mtg_engine::cards::CardRegistry::with_all_cards();
        CardLines::new(&set_data.all_card_names(), &set_data.rarities(), &registry)
    }

    fn table(seat: usize, pod_size: usize) -> Table {
        Table { seat, pod_size, pack_size: 14 }
    }

    /// #481: the pool was restated at exactly 2 of a seat's 42 picks, and a
    /// resumed seat — whose `claude -p` session is new, so the conversation
    /// carries nothing — drafted nine picks knowing nothing of its pool.
    #[test]
    fn every_pick_restates_the_pool() {
        let cards = cards();
        let pool = vec![
            "Moon Heron".to_string(),
            "Chapel Geist".to_string(),
            "Moon Heron".to_string(),
        ];
        let available = vec!["Ambush Viper".to_string(), "Abbey Griffin".to_string()];

        // Pack 2 pick 7 — a pick that used to carry the pack and nothing else.
        let prompt = DraftLlmClient::build_pick_prompt(
            table(1, 4), 2, 7, &available, &pool, &cards,
        );
        assert!(prompt.contains("Your pool (3 cards):"), "{prompt}");
        assert!(prompt.contains("2x Moon Heron {3}{U}"), "{prompt}");
        assert!(prompt.contains("1x Chapel Geist"), "{prompt}");
        // And the shape it would otherwise have to count by hand.
        assert!(prompt.contains("Colors"), "{prompt}");
        assert!(prompt.contains("Curve"), "{prompt}");

        // The first pick of all says so rather than showing an empty list.
        let first = DraftLlmClient::build_pick_prompt(
            table(1, 4), 1, 1, &available, &[], &cards,
        );
        assert!(first.contains("Your pool is empty"), "{first}");
    }

    /// #483: the pack was a list of bare names, with the cost, colour, type,
    /// size and rarity the pick is made on nowhere on the line.
    #[test]
    fn a_pack_lists_what_each_card_is() {
        let cards = cards();
        let available = vec!["Moon Heron".to_string(), "Snapcaster Mage".to_string()];
        let prompt = DraftLlmClient::build_pick_prompt(
            table(0, 8), 1, 1, &available, &[], &cards,
        );

        assert!(prompt.contains("0: Moon Heron {3}{U} | Creature — Spirit Bird 3/2 [common]"), "{prompt}");
        assert!(prompt.contains("1: Snapcaster Mage {1}{U} | Creature — Human Wizard 2/1 [rare]"), "{prompt}");
    }

    /// #485: a seat was never told the pod size or which seat it was, and
    /// the rules described an 8-pod at every pod size.
    #[test]
    fn the_prompt_describes_the_pod_the_seat_is_actually_in() {
        let cards = cards();
        let available = vec!["Moon Heron".to_string()];

        let heads_up = DraftLlmClient::build_pick_prompt(
            table(0, 2), 1, 1, &available, &[], &cards,
        );
        assert!(heads_up.contains("You are seat 0 of 2"), "{heads_up}");
        // Pack 1 passes left; in a 2-pod that is the other seat.
        assert!(heads_up.contains("passes LEFT to seat 1"), "{heads_up}");

        // Pack 2 passes the other way, which in a bigger pod is a different
        // neighbour.
        let pod8 = DraftLlmClient::build_pick_prompt(
            table(3, 8), 2, 1, &available, &[], &cards,
        );
        assert!(pod8.contains("You are seat 3 of 8"), "{pod8}");
        assert!(pod8.contains("passes RIGHT to seat 2"), "{pod8}");
        let pod8_pack1 = DraftLlmClient::build_pick_prompt(
            table(3, 8), 1, 1, &available, &[], &cards,
        );
        assert!(pod8_pack1.contains("passes LEFT to seat 4"), "{pod8_pack1}");
        // Seat 0 passing right wraps to the far end of the pod.
        let wraps = DraftLlmClient::build_pick_prompt(
            table(0, 8), 2, 1, &available, &[], &cards,
        );
        assert!(wraps.contains("passes RIGHT to seat 7"), "{wraps}");
    }

    /// #608: `--players 1` is a supported configuration — `count()` refuses
    /// only 0, and `Tournament::total_rounds` returns 0 for it deliberately —
    /// but every word the seat was told about the table was about a table it
    /// was not at. #485's `else` arm assumes at least one opponent, so a
    /// 1-seat pod was told it was reading signals off "the 0 cards the other
    /// seats took", and `(seat + 1) % pod_size` told it, 42 times, that it
    /// was passing the pack to itself.
    #[test]
    fn a_one_seat_pod_is_not_told_it_has_neighbours() {
        let cards = cards();
        let available = vec!["Moon Heron".to_string()];

        let solo = DraftLlmClient::build_pick_prompt(table(0, 1), 1, 1, &available, &[], &cards);
        assert!(!solo.contains("passes LEFT to seat 0"), "passing to itself: {solo}");
        assert!(!solo.contains("passes RIGHT to seat 0"), "passing to itself: {solo}");
        assert!(solo.contains("comes straight back to you"), "{solo}");
        // Pack 2 reverses the rotation, which is still nobody.
        let solo_p2 = DraftLlmClient::build_pick_prompt(table(0, 1), 2, 1, &available, &[], &cards);
        assert!(!solo_p2.contains("to seat 0"), "{solo_p2}");

        let rules = build_draft_rules("Innistrad", None, "", table(0, 1));
        let flat = rules.replace(" \n", " ").replace('\n', " ");
        assert!(flat.contains("You are the only drafter"), "{rules}");
        for claim in [
            "other seat(s)",
            "the other seats took",
            "that colour is open",
            "passes to the next seat",
            "Pack 1 passes left",
            "never shown another seat's pick",
        ] {
            assert!(
                !flat.contains(claim),
                "a 1-seat pod is still told {claim:?}, which describes a draft it is not in:\n{rules}"
            );
        }
        // And the pods that do have neighbours keep being told so.
        for pod in [2usize, 3, 8] {
            let other = build_draft_rules("Innistrad", None, "", table(0, pod));
            assert!(other.contains(&format!("seat 0 of a {pod}-seat pod")), "{other}");
            assert!(!other.contains("You are the only drafter"), "{other}");
        }
    }

    /// The rules a seat is given are about its own table: the two pod sizes
    /// used to produce byte-identical text.
    #[test]
    fn the_rules_differ_by_pod_size() {
        let two = build_draft_rules("Innistrad", None, "", table(0, 2));
        let eight = build_draft_rules("Innistrad", None, "", table(0, 8));
        assert_ne!(two, eight);

        assert!(two.contains("seat 0 of a 2-seat pod"), "{two}");
        assert!(two.contains("same \nneighbour") || two.contains("same neighbour"), "{two}");
        assert!(eight.contains("comes back to you 8 picks later"), "{eight}");
        assert!(eight.contains("7 \ncards the other seats took") || eight.contains("7 cards the other seats took"), "{eight}");
        // Both state the pack size and the total number of picks.
        assert!(two.contains("3 packs of 14 cards each and make 42 picks"), "{two}");
    }

    /// #858: the rules said earlier picks are not kept even when a history
    /// window kept them.
    #[test]
    fn the_draft_rules_say_what_the_history_window_keeps() {
        assert_eq!(super::history_note(0),
            "Earlier picks are not kept in the conversation; the pool and the notes are what carries over");
        let kept = super::history_note(3);
        assert!(kept.starts_with("Up to 3 of your earlier exchanges are kept"), "{kept}");
        assert!(!kept.contains("not kept in the conversation"), "{kept}");
    }

    /// #487: a drafted basic land was a key in `maindeck` and in `lands`,
    /// and filling both builds a 41-card deck with a land nobody asked for.
    #[test]
    fn a_drafted_basic_land_is_keyed_only_under_lands() {
        let pool = vec![
            "Plains".to_string(),
            "Moon Heron".to_string(),
            "Moon Heron".to_string(),
        ];
        let schema = deck_schema_for(&pool);
        let slots = schema["properties"]["maindeck"]["prefixItems"].as_array().unwrap();

        // One row, Moon Heron's, held to 0..=2; the Plains is no row.
        assert_eq!(slots.len(), 1, "{slots:?}");
        assert_eq!(slots[0]["enum"], serde_json::json!([0, 1, 2]));
        assert_eq!(mtg_draft::deckbuilding::deck_rows(&pool), vec![("Moon Heron".to_string(), 2)]);
        assert!(schema["properties"]["lands"]["properties"]["Plains"].is_object());
    }

    /// #867: `maindeck` was keyed by the pool's card names with the copy
    /// counts as enums, sent as built, so every seat's deck question was a
    /// schema nobody had sent before — a write of the whole cached prefix.
    /// What is sent is the same for any two pools.
    #[test]
    fn every_pool_sends_the_same_deck_schema() {
        let a: Vec<String> = ["Moon Heron", "Moon Heron", "Chapel Geist", "Plains"]
            .iter().map(|s| (*s).to_string()).collect();
        let b: Vec<String> = ["Brimstone Volley", "Blazing Torch"]
            .iter().map(|s| (*s).to_string()).collect();
        let sent_a = mtg_player::llm::cache_stable_schema(&deck_schema_for(&a));
        let sent_b = mtg_player::llm::cache_stable_schema(&deck_schema_for(&b));

        assert_eq!(sent_a, sent_b);
        let text = sent_a.to_string();
        assert!(!text.contains("enum") && !text.contains("prefixItems") && !text.contains("Moon Heron"),
            "{text}");
    }

    /// The question between games is one cached schema too, whatever the
    /// pool, and its keys are ones the API takes: the deck build's, and
    /// `play_first`.
    #[test]
    fn every_pool_sends_the_same_sideboard_schema() {
        use super::sideboard_schema_for;
        let a: Vec<String> = ["Moon Heron", "Chapel Geist"].iter().map(|s| (*s).to_string()).collect();
        let b: Vec<String> = ["Brimstone Volley"].iter().map(|s| (*s).to_string()).collect();
        let sent_a = mtg_player::llm::cache_stable_schema(&sideboard_schema_for(&a));
        assert_eq!(sent_a, mtg_player::llm::cache_stable_schema(&sideboard_schema_for(&b)));
        assert_ne!(sent_a, mtg_player::llm::cache_stable_schema(&deck_schema_for(&a)), "a question of its own");
        let keys: Vec<&String> = sent_a["properties"].as_object().unwrap().keys().collect();
        assert!(keys.iter().all(|k| mtg_player::llm::schema_key_is_legal(k)), "{keys:?}");
        assert!(sent_a["required"].as_array().unwrap().iter().any(|k| k == "play_first"));
    }
}

/// The Claude Code draft seat drives `claude -p` as a subprocess, so these
/// stand in a fake `claude` (a shell script that records its argv and stdin
/// and prints a canned print-mode JSON result) and check the contract the
/// real CLI is spoken to with — the same harness mtg-player uses for the
/// game-side backend.
#[cfg(all(test, unix))]
mod claude_code_tests {
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    /// A scratch directory holding the fake binary and its recordings.
    struct Fake {
        dir: PathBuf,
    }

    impl Fake {
        /// `script_body` runs with `$LOG` (argv/stdin recording file) and
        /// `$CALL` (1-based call counter) set.
        fn new(name: &str, script_body: &str) -> Fake {
            let dir = std::env::temp_dir()
                .join(format!("mtg-draft-fake-claude-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            let bin = dir.join("claude");
            let script = format!(
                "#!/bin/sh\nLOG=\"{log}\"\nCOUNT=\"{count}\"\n\
                 if [ \"$1\" = \"--version\" ]; then echo 9.9.9; exit 0; fi\n\
                 CALL=$(( $(cat \"$COUNT\" 2>/dev/null || echo 0) + 1 )); echo $CALL > \"$COUNT\"\n\
                 {{ echo \"=== call $CALL\"; for a in \"$@\"; do printf 'ARG: %s\\n' \"$a\"; done; \
                 echo '--- stdin'; cat; echo; echo '--- end'; }} >> \"$LOG\"\n\
                 {body}\n",
                log = dir.join("log.txt").display(),
                count = dir.join("count").display(),
                body = script_body
            );
            std::fs::write(&bin, script).unwrap();
            std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
            Fake { dir }
        }

        fn bin(&self) -> String {
            self.dir.join("claude").display().to_string()
        }

        fn log(&self) -> String {
            std::fs::read_to_string(self.dir.join("log.txt")).unwrap_or_default()
        }

        fn calls(&self) -> Vec<String> {
            self.log().split("=== call ").skip(1).map(str::to_string).collect()
        }
    }

    impl Drop for Fake {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    /// A fake that always succeeds, echoing back the session id it was given
    /// and a structured object that reads as a pick of card 2.
    const OK_BODY: &str = r#"
SID=""
prev=""
for a in "$@"; do
  if [ "$prev" = "--session-id" ] || [ "$prev" = "--resume" ]; then SID="$a"; fi
  prev="$a"
done
printf '{"type":"result","subtype":"success","is_error":false,"session_id":"%s","result":"{\"pick\":2}","structured_output":{"pick":2},"usage":{"input_tokens":10,"output_tokens":2,"cache_read_input_tokens":5,"cache_creation_input_tokens":1}}\n' "$SID"
"#;

    fn arg_after<'a>(call: &'a str, flag: &str) -> Option<&'a str> {
        let lines: Vec<&str> = call.lines().collect();
        let idx = lines.iter().position(|l| *l == format!("ARG: {flag}"))?;
        lines.get(idx + 1)?.strip_prefix("ARG: ")
    }

    /// The seat asked in text or with `--json-schema`, whatever the run's
    /// `MTG_CLAUDE_CODE_DRAFT_ANSWERS` says: a test pins one without racing
    /// the others on a process-wide variable.
    fn answering(mut b: super::ClaudeCodeDraftBackend, text: bool) -> super::ClaudeCodeDraftBackend {
        if let (false, Some(rest)) = (text, b.system_prompt.strip_suffix(super::TEXT_ANSWER_NOTE)) {
            b.system_prompt = rest.to_string();
        } else if text && !b.system_prompt.ends_with(super::TEXT_ANSWER_NOTE) {
            b.system_prompt.push_str(super::TEXT_ANSWER_NOTE);
        }
        b.text_answers = text;
        b
    }

    fn backend(fake: &Fake, model: Option<&str>) -> super::ClaudeCodeDraftBackend {
        super::ClaudeCodeDraftBackend::with_binary(
            &fake.bin(),
            model,
            "Innistrad",
            None,
            "Doomed Traveler {W} | Creature — Human Soldier 1/1",
            super::Table { seat: 0, pod_size: 2, pack_size: 14 },
        )
    }

    /// Every pick prompt restates the pack, the pool and the seat's place
    /// (#481), and the deck prompt the whole pool — so a session resumed
    /// across all 45 decisions re-read a growing history for nothing. Each
    /// decision is a session of its own unless a window is asked for.
    #[test]
    fn each_decision_is_its_own_session_unless_a_window_is_asked_for() {
        use super::DraftBackend;
        let fake = Fake::new("session", OK_BODY);
        let mut b = backend(&fake, None);
        b.window = 0;

        b.send_pick("Pack 1, Pick 1", 5);
        b.send_pick("Pack 1, Pick 2", 4);
        b.send_deck_building("Build a deck", &["Doomed Traveler".to_string()]);

        let calls = fake.calls();
        assert_eq!(calls.len(), 3, "three decisions, three subprocesses:\n{}", fake.log());
        let mut sids = Vec::new();
        for call in &calls {
            let sid = arg_after(call, "--session-id").expect("every decision opens its own session");
            assert_eq!(sid.len(), 36, "session id is a uuid: {sid}");
            assert!(arg_after(call, "--resume").is_none(), "no decision resumes an earlier one:\n{call}");
            sids.push(sid.to_string());
        }
        sids.dedup();
        assert_eq!(sids.len(), 3);

        // A window of one: a session carries two decisions, then is replaced.
        let fake2 = Fake::new("window", OK_BODY);
        let mut b = backend(&fake2, None);
        b.window = 1;
        for _ in 0..4 {
            b.send_pick("Pack 1, Pick N", 3);
        }
        let windowed = fake2.calls();
        let first = arg_after(&windowed[0], "--session-id").expect("fresh");
        assert_eq!(arg_after(&windowed[1], "--resume"), Some(first), "one earlier exchange");
        let third = arg_after(&windowed[2], "--session-id").expect("the window is full: fresh");
        assert_ne!(third, first);
        assert_eq!(arg_after(&windowed[3], "--resume"), Some(third));

        for call in &calls {
            assert!(call.contains("ARG: -p\n"), "print mode");
            assert_eq!(arg_after(call, "--output-format"), Some("json"));
            assert_eq!(arg_after(call, "--tools"), Some(""), "tools disabled — the seat only drafts");
            let sys = arg_after(call, "--system-prompt").expect("system prompt every call");
            assert!(sys.contains("drafting Magic: The Gathering cards from Innistrad"));
        }
        assert!(calls[0].contains("--- stdin\nPack 1, Pick 1"), "the prompt goes on stdin:\n{}", calls[0]);
    }

    /// The pick goes out as one schema whatever the pack's size — the
    /// schema is part of the cached prefix, and a pack shrinks every pick —
    /// and the pack's indices go to a stand-in CLI beside it, as the game
    /// seat's do (`cache_stable_schema`).
    #[test]
    fn a_pick_sends_one_schema_and_the_pack_beside_it() {
        use super::DraftBackend;
        let body = format!("echo \"ENV: ${}\" >> \"$LOG\"\n{OK_BODY}",
            mtg_player::llm::CLAUDE_CODE_ANSWER_SCHEMA_ENV);
        let fake = Fake::new("schema", &body);
        let mut b = answering(backend(&fake, None), false);
        let response = b.send_pick("Pack 1, Pick 1", 3);
        b.send_pick("Pack 1, Pick 2", 2);

        let calls = fake.calls();
        let call = &calls[0];
        assert_eq!(arg_after(call, "--json-schema"), arg_after(&calls[1], "--json-schema"));
        let schema: serde_json::Value =
            serde_json::from_str(arg_after(call, "--json-schema").expect("schema passed")).unwrap();
        assert!(schema["properties"]["pick"].get("enum").is_none(), "{schema}");
        let full: serde_json::Value = serde_json::from_str(
            call.lines().find_map(|l| l.strip_prefix("ENV: ")).expect("the full schema is set")).unwrap();
        assert_eq!(full["properties"]["pick"]["enum"], serde_json::json!([0, 1, 2]));
        // `thoughts` is KEPT on this path, as on every other: this seat has
        // no thinking channel for the harness to read, so the payload is the
        // only place its reasoning can go (#213, #607).
        assert!(schema["properties"].get("thoughts").is_some());
        assert!(schema["required"].as_array().unwrap().iter().any(|v| v == "thoughts"));
        assert_eq!(schema["additionalProperties"], false);
        // The response has to come back as the JSON text main.rs parses.
        assert_eq!(serde_json::from_str::<serde_json::Value>(&response).unwrap()["pick"], 2);
    }

    /// A sideboarding call comes a game after the seat's last and writes
    /// its cache at the hour, so the seat's next one reads its prefix; a
    /// pick, seconds after the last, keeps five minutes.
    #[test]
    fn sideboarding_writes_its_cache_for_the_hour_and_a_pick_for_five_minutes() {
        use super::DraftBackend;
        let body = format!("echo \"TTL: ${{{}-unset}} 5M: ${{FORCE_PROMPT_CACHING_5M-unset}}\" >> \"$LOG\"\n{OK_BODY}",
            mtg_player::llm::CLAUDE_CODE_CACHE_TTL_ENV);
        let fake = Fake::new("hour", &body);
        let mut b = answering(backend(&fake, None), false);
        b.send_pick("Pack 1, Pick 1", 3);
        b.send_sideboard("Between games", &["Doomed Traveler".to_string()]);
        b.send_pick("Pack 1, Pick 2", 2);
        let log = fake.log();
        let ttl: Vec<&str> = log.lines().filter(|l| l.starts_with("TTL: ")).collect();
        assert_eq!(ttl, ["TTL: unset 5M: 1", "TTL: 1h 5M: unset", "TTL: unset 5M: 1"], "{log}");
    }

    /// Asked in text (`MTG_CLAUDE_CODE_DRAFT_ANSWERS=text`), a pick sends no schema: the pick's
    /// fields end the prompt, with the pack's range, and the answer is read
    /// out of the result text in one request, where `--json-schema` is a
    /// tool call and a second request (`mtg_player::llm::answer_text`).
    #[test]
    fn a_pick_in_text_says_its_fields_at_the_end_of_the_prompt() {
        use super::DraftBackend;
        let body = format!("echo \"ENV: ${}\" >> \"$LOG\"\n{}",
            mtg_player::llm::CLAUDE_CODE_ANSWER_SCHEMA_ENV,
            r#"printf '{"type":"result","is_error":false,"session_id":"s","result":"{\"thoughts\": \"t\", \"pick\": 1}","usage":{}}\n'"#);
        let fake = Fake::new("text", &body);
        let mut b = answering(backend(&fake, None), true);
        let response = b.send_pick("Pack 1, Pick 1", 3);
        let call = &fake.calls()[0];
        assert!(arg_after(call, "--json-schema").is_none(), "{call}");
        assert!(call.contains("Pack 1, Pick 1\n\nAnswer with only this JSON object: {\"thoughts\": string, \"pick\": integer 0 to 2}"),
            "{call}");
        assert!(call.contains("ENV: {"), "the full schema is still set for a stand-in CLI:\n{call}");
        assert!(call.contains("Its fields are listed at the end of each question."), "the system prompt says so");
        assert_eq!(serde_json::from_str::<serde_json::Value>(&response).unwrap()["pick"], 1);
    }

    /// The deck question goes out as one schema whatever the pool, with the
    /// rows' bounds beside it for a stand-in CLI, as a pick's are (#867).
    #[test]
    fn a_deck_build_sends_one_schema_and_the_rows_beside_it() {
        use super::DraftBackend;
        let body = format!("echo \"ENV: ${}\" >> \"$LOG\"\n{OK_BODY}",
            mtg_player::llm::CLAUDE_CODE_ANSWER_SCHEMA_ENV);
        let fake = Fake::new("deckschema", &body);
        let mut b = answering(backend(&fake, None), false);
        b.send_deck_building("Build", &["Moon Heron".to_string(), "Moon Heron".to_string()]);
        b.send_deck_building("Build", &["Chapel Geist".to_string()]);

        let calls = fake.calls();
        assert_eq!(arg_after(&calls[0], "--json-schema"), arg_after(&calls[1], "--json-schema"));
        let full: serde_json::Value = serde_json::from_str(
            calls[0].lines().find_map(|l| l.strip_prefix("ENV: ")).expect("the full schema is set")).unwrap();
        assert_eq!(full["properties"]["maindeck"]["prefixItems"],
            serde_json::json!([{"type": "integer", "enum": [0, 1, 2]}]));
    }

    #[test]
    fn a_model_suffix_is_passed_as_a_cli_alias_not_an_api_model_id() {
        use super::DraftBackend;
        let fake = Fake::new("model", OK_BODY);
        let mut b = backend(&fake, Some("opus"));
        b.send_pick("Pack 1, Pick 1", 2);
        assert_eq!(arg_after(&fake.calls()[0], "--model"), Some("opus"));
    }

    /// Issue #218: three attempts and six seconds of backoff gave up on the
    /// failure a long draft actually meets — a usage limit or a transient
    /// outage lasting minutes. The seat keeps trying for a wall-clock
    /// budget instead, and only then stops (rather than drafting card 0 for
    /// the rest of the run).
    #[test]
    fn a_failing_cli_is_retried_across_the_budget_and_then_fatal() {
        use super::DraftBackend;
        // Seconds, not the ten real minutes.
        std::env::set_var(super::RETRY_BUDGET_ENV, "3");
        let fake = Fake::new("fail", "echo 'boom' >&2; exit 3");
        let mut b = backend(&fake, None);
        let started = std::time::Instant::now();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            b.send_pick("Pack 1, Pick 1", 2);
        }));
        std::env::remove_var(super::RETRY_BUDGET_ENV);

        assert!(result.is_err(), "an unanswerable draft stops instead of drafting for the model");
        let payload = result.unwrap_err();
        let msg = payload.downcast_ref::<String>().expect("a message, not an opaque payload");
        assert!(
            msg.starts_with(super::FATAL_MARKER),
            "main recognises this prefix and reports it as one line: {msg}"
        );
        // The first 2s backoff fits in a 3s budget; the 4s one does not.
        assert!(fake.calls().len() >= 2, "it kept trying while the budget lasted");
        assert!(started.elapsed() >= std::time::Duration::from_secs(2),
            "it really waited between attempts rather than burning three in 6ms");
    }

    /// The default budget is sized for an outage, not for a hiccup.
    #[test]
    fn the_retry_budget_outlasts_a_transient_outage() {
        assert!(mtg_player::llm::RETRY_BUDGET >= std::time::Duration::from_secs(300));
        assert!(super::retry_backoff(1) < super::retry_backoff(3));
        assert_eq!(super::retry_backoff(20), mtg_player::llm::MAX_RETRY_BACKOFF, "backoff is capped");
    }

    #[test]
    fn an_error_result_is_retried_and_a_later_success_wins() {
        use super::DraftBackend;
        let body = r#"
if [ "$CALL" = "1" ]; then
  printf '{"type":"result","is_error":true,"result":"rate limited","session_id":"s"}\n'
else
  printf '{"type":"result","is_error":false,"result":"{\"pick\":1}","structured_output":{"pick":1},"session_id":"s","usage":{}}\n'
fi
"#;
        let fake = Fake::new("retry", body);
        let mut b = backend(&fake, None);
        let response = b.send_pick("Pack 1, Pick 1", 2);
        assert_eq!(serde_json::from_str::<serde_json::Value>(&response).unwrap()["pick"], 1);
        assert_eq!(fake.calls().len(), 2);
    }
}

