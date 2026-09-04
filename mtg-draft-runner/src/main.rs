use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

use mtg_draft::deckbuilding;
use mtg_draft::draft::DraftState;
use mtg_draft::pack::{generate_draft_packs, SheetData};
use mtg_draft::set_data::SetData;
use mtg_draft::tournament::{GameOutcome, MatchResult, Tournament, TournamentConfig};

use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::{self, Decklist, GameConfig};
use mtg_engine::ids::PlayerId;
use mtg_engine::state::GameState;
use mtg_engine::view::GameView;

use mtg_player::llm::LlmPlayer;
use mtg_player::Player;
use std::fmt::Write;

mod draft_log;
mod llm_client;

/// Per-seat configuration used by [`play_match`].
struct PlayerSpec<'a> {
    seat: usize,
    deck: &'a Decklist,
    model_spec: &'a str,
    guide: Option<&'a str>,
}

/// (seat, pack, pool, picks) for a single player at a single pick step.
type PickInput = (usize, Vec<String>, Vec<String>, Vec<mtg_draft::draft::DraftPick>);

// ─── CLI Argument Parsing ────────────────────────────────────────────

const USAGE: &str = "\
mtg-draft-runner — draft a set with LLM seats, then play a Swiss tournament

Usage: mtg-draft-runner [OPTIONS]

Options:
  --set <name>           Set to draft, from data/sets/<name>.json  (default isd)
  --players <N>          Number of drafters  (default 8)
  --model <spec>         Model for every seat  (default claude)
  --model-<N> <spec>     Model for seat N alone (0-based)
  --best-of <N>          Games per tournament match  (default 3)
  --guide <path>         Draft guide file prepended to every seat's prompt
  --guide-<N> <path>     Draft guide file for seat N alone (0-based)
  --log <path>           Write the run log here  (default draft.log)
  --quiet, -q            Suppress progress output
  --help, -h             Print this help and exit
  --version              Print the version and exit

Model spec: provider[:model[:draft_thinking[:game_thinking]]]. claude and gemini
seats call metered APIs (ANTHROPIC_API_KEY / GEMINI_API_KEY); claude-code (alias
cc) runs the same seat through `claude -p` on the CLI's own login, for both the
draft and the games.";

/// A user error: report it and exit without a Rust panic/backtrace.
fn die(msg: &str) -> ! {
    eprintln!("Error: {msg}");
    std::process::exit(1);
}

struct Args {
    set: String,
    players: usize,
    /// Per-player model specs. --model sets the default, --model-N overrides for player N.
    models: Vec<String>,
    best_of: usize,
    guides: Vec<Option<String>>,
    log: String,
    quiet: bool,
}

/// Refuse an argument vector `parse_args` wouldn't fully consume, and hand
/// back the `--model-N` / `--guide-N` flags it saw so their seat numbers can
/// be range-checked once `--players` is known. Every lookup below is an
/// exact-match position scan, so an unrecognized or misspelled flag was
/// silently dropped and its default silently used — a typo'd `--model` drafted
/// with a model nobody asked for, on a seat that bills per token.
fn validate_args(args: &[String]) -> Vec<(String, usize)> {
    const VALUE_FLAGS: &[&str] = &["--set", "--players", "--model", "--best-of", "--guide", "--log"];
    const BOOL_FLAGS: &[&str] = &["--quiet", "-q"];
    let mut indexed = Vec::new();
    let mut i = 1;
    while i < args.len() {
        let a = args[i].as_str();
        let per_seat = seat_flag(a);
        if VALUE_FLAGS.contains(&a) || per_seat.is_some() {
            if i + 1 >= args.len() {
                eprintln!("Error: {a} requires a value\n\n{USAGE}");
                std::process::exit(2);
            }
            if let Some(index) = per_seat {
                indexed.push((a.to_string(), index));
            }
            i += 2;
        } else if BOOL_FLAGS.contains(&a) {
            i += 1;
        } else {
            eprintln!("Error: unrecognized argument '{a}'\n\n{USAGE}");
            std::process::exit(2);
        }
    }
    indexed
}

/// The seat number of a `--model-N` / `--guide-N` flag, if it is one.
fn seat_flag(arg: &str) -> Option<usize> {
    let n = arg.strip_prefix("--model-").or_else(|| arg.strip_prefix("--guide-"))?;
    n.parse().ok()
}

fn parse_args() -> Args {
    let args: Vec<String> = env::args().collect();

    // --help used to start a real eight-seat draft, so a typo cost money on
    // a metered seat. Both of these answer and exit without drafting.
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        std::process::exit(0);
    }
    if args.iter().any(|a| a == "--version") {
        println!("mtg-draft-runner {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }
    let per_seat_flags = validate_args(&args);

    let get = |flag: &str| -> Option<String> {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1)).cloned()
    };
    let count = |flag: &str, default: usize| -> usize {
        get(flag).map_or(default, |s| {
            let n = s.parse().unwrap_or_else(|_| die(&format!("{flag} takes a number, got '{s}'")));
            if n == 0 {
                die(&format!("{flag} must be at least 1"));
            }
            n
        })
    };

    let set = get("--set").unwrap_or_else(|| "isd".to_string());
    let players = count("--players", 8);
    let default_model = get("--model").unwrap_or_else(|| "claude".to_string());
    let best_of = count("--best-of", 3);
    let log = get("--log").unwrap_or_else(|| "draft.log".to_string());
    let quiet = args.iter().any(|a| a == "--quiet" || a == "-q");

    // A --model-N or --guide-N naming a seat outside the pod used to be read
    // by nobody — the same silent no-op as a misspelled flag, so it is
    // refused the same way.
    for (flag, index) in &per_seat_flags {
        if *index >= players {
            die(&format!("{flag}: there is no seat {index} with --players {players}"));
        }
    }

    // Load per-player models: --model sets default, --model-N overrides for player N
    let mut models: Vec<String> = vec![default_model; players];
    for (i, model) in models.iter_mut().enumerate() {
        if let Some(m) = get(&format!("--model-{i}")) {
            *model = m;
        }
    }

    // Load guides: --guide applies to all, --guide-N overrides for player N.
    // An unreadable guide file is fatal: drafting without the guide the
    // caller asked for is a different draft than the one requested.
    let read_guide = |path: &str| -> String {
        fs::read_to_string(path)
            .unwrap_or_else(|e| die(&format!("failed to read guide file '{path}': {e}")))
    };
    let global_guide = get("--guide").map(|path| read_guide(&path));
    let mut guides: Vec<Option<String>> = vec![global_guide; players];
    for (i, guide) in guides.iter_mut().enumerate() {
        if let Some(path) = get(&format!("--guide-{i}")) {
            *guide = Some(read_guide(&path));
        }
    }

    Args {
        set,
        players,
        models,
        best_of,
        guides,
        log,
        quiet,
    }
}

/// One round-trip with the model during deck building.
struct DeckAttempt {
    prompt: String,
    response: String,
    error: Option<String>,
}

/// Return type for deck building LLM interaction.
struct DeckBuildResult {
    deck: deckbuilding::DraftDeck,
    attempts: Vec<DeckAttempt>,
    retries: usize,
}

/// Validate model specs before starting the draft. Catches invalid thinking
/// levels and other config errors so we fail fast rather than silently
/// falling back to defaults mid-draft.
fn validate_model_specs(models: &[String]) {
    // Known thinking-level constraints per model family.
    // Models not listed here accept any level the API supports.
    let restricted: &[(&str, &[&str])] = &[
        ("gemini-3-pro", &["low", "high"]),
        ("gemini-3.0-pro", &["low", "high"]),
        ("gemini-3.1-pro", &["low", "high"]),
    ];

    let valid_levels = ["minimal", "low", "medium", "high"];

    for (i, spec) in models.iter().enumerate() {
        let parts: Vec<&str> = spec.split(':').collect();
        let provider = parts[0];
        match provider {
            "claude" => continue,
            // Every decision a claude-code seat makes shells out to `claude`.
            // Unchecked, a run on a machine without the binary drafted the
            // whole pod first — billing the other seats — and only then
            // failed every game decision. Refuse before anything is spent.
            "claude-code" | "cc" => {
                if !mtg_player::llm::claude_code_available() {
                    die(&format!(
                        "seat {i} model '{spec}' needs the Claude Code CLI: `{}` is not runnable (set {} to its path)",
                        mtg_player::llm::claude_code_binary(),
                        mtg_player::llm::CLAUDE_CODE_BINARY_ENV
                    ));
                }
                continue;
            }
            "gemini" => {}
            // Defaulting an unknown provider to claude spent real API money
            // drafting with a model nobody asked for, then printed standings
            // and exited 0 — indistinguishable from the requested run.
            other => die(&format!(
                "seat {i} model '{spec}': unknown provider '{other}' (expected {})",
                llm_client::ACCEPTED_PROVIDERS
            )),
        }
        let model = parts.get(1).copied().unwrap_or("gemini-2.5-flash");
        let levels: Vec<&str> = parts.iter().skip(2).copied().collect();

        for level in &levels {
            if !valid_levels.contains(level) {
                eprintln!("ERROR: Seat {} model '{}': '{}' is not a valid thinking level (valid: {})",
                    i, spec, level, valid_levels.join(", "));
                std::process::exit(1);
            }
            // Check model-specific restrictions
            for (model_prefix, allowed) in restricted {
                if model.contains(model_prefix) && !allowed.contains(level) {
                    eprintln!("ERROR: Seat {} model '{}': '{}' is not supported by {} (allowed: {})",
                        i, spec, level, model, allowed.join(", "));
                    std::process::exit(1);
                }
            }
        }
    }
}

// ─── Main ────────────────────────────────────────────────────────────

fn main() {
    let args = parse_args();
    validate_model_specs(&args.models);
    let mut rng = rand::thread_rng();

    // Load set data
    let set_path = PathBuf::from(format!("data/sets/{}.json", args.set));
    let mut set_data = SetData::load(&set_path).unwrap_or_else(|e| {
        eprintln!("Failed to load set data: {e}");
        std::process::exit(1);
    });

    let registry = CardRegistry::with_all_cards();
    let removed = set_data.filter_implemented(&registry);
    if !removed.is_empty() && !args.quiet {
        eprintln!(
            "Warning: {} cards not implemented, removed from draft pool",
            removed.len()
        );
    }

    let sheets = SheetData::from_set_data(&set_data).unwrap_or_else(|e| {
        eprintln!("Failed to build sheet data: {e}");
        std::process::exit(1);
    });

    // Create streaming log file
    let log = draft_log::DraftLogger::new(std::path::Path::new(&args.log));
    log_header!(log, &set_data.set_name, args.players, args.best_of, args.models.as_slice());

    if !args.quiet {
        eprintln!(
            "=== {} Draft: {} players, best-of-{} ===",
            set_data.set_name, args.players, args.best_of
        );
        eprintln!("Log file: {}", args.log);
    }

    // ── Phase 1: Generate packs ──
    if !args.quiet {
        eprintln!("Generating booster packs...");
    }
    let packs = generate_draft_packs(&sheets, args.players, &mut rng);

    // Log original pack contents
    log_section!(log, "BOOSTER PACKS");
    for (seat, player_packs) in packs.iter().enumerate() {
        for (pack_num, pack) in player_packs.iter().enumerate() {
            log_pack_contents!(log, seat, pack_num + 1, &pack.all_cards());
        }
    }

    // ── Phase 2: Draft ──
    log_section!(log, "DRAFT");
    if !args.quiet {
        eprintln!("Starting draft...");
    }
    let mut draft = DraftState::new(&packs);

    // Build card reference with oracle text for all cards in the set
    let card_reference = llm_client::build_card_reference(&set_data.all_card_names(), &registry);

    // Create LLM clients for each drafter (each may use a different model)
    let mut clients: Vec<llm_client::DraftLlmClient> = (0..args.players)
        .map(|seat| {
            llm_client::DraftLlmClient::new(
                &args.models[seat],
                &set_data.set_name,
                args.guides[seat].as_deref(),
                &card_reference,
            )
        })
        .collect();

    // Log the system prompt once. The shared draft rules / card reference
    // are identical across seats; only the per-backend response-format
    // suffix may differ. Logging seat 0's prompt is representative.
    log_system_prompt!(log, clients[0].system_prompt());

    // Picks the run had to make on a seat's behalf, per seat. Reported at
    // the end: a draft where a seat never made a choice must not present
    // its pools, decks and standings as if it had (issue #195).
    let mut substituted_picks = vec![0usize; args.players];

    // Run the draft — all players pick in parallel each round
    for round in 0..3 {
        if round > 0 {
            draft.start_next_pack_round();
        }

        let initial_cards = draft.cards_remaining(0);

        for pick_num in 0..initial_cards {
            if !args.quiet {
                eprint!("\rPack {} Pick {}/{}", round + 1, pick_num + 1, initial_cards);
            }

            // Gather inputs for each player before spawning threads
            let pick_inputs: Vec<PickInput> =
                (0..args.players)
                    .map(|seat| {
                        (
                            seat,
                            draft.current_pack_for(seat).to_vec(),
                            draft.players[seat].pool.clone(),
                            draft.players[seat].picks.clone(),
                        )
                    })
                    .collect();

            // All players pick in parallel
            let pick_results: Vec<(usize, Pick, String, String)> =
                std::thread::scope(|s| {
                    let handles: Vec<_> = pick_inputs
                        .iter()
                        .zip(clients.iter_mut())
                        .map(|((seat, available, pool, history), client)| {
                            let seat = *seat;
                            s.spawn(move || {
                                let prompt = crate::llm_client::DraftLlmClient::build_pick_prompt(
                                    round + 1,
                                    pick_num + 1,
                                    available,
                                    pool,
                                    history,
                                );
                                let response = client.send_pick_message(&prompt, available.len());
                                let chosen = parse_pick_response(&response, available);
                                crate::llm_client::DraftLlmClient::record_pick(chosen.card());
                                (seat, chosen, prompt, response)
                            })
                        })
                        .collect();

                    handles.into_iter().map(|h| h.join().unwrap()).collect()
                });

            // Apply picks sequentially (mutates draft state) and log
            if pick_num == 0 {
                log_subsection!(log, &format!("Pack {}", round + 1));
            }
            for (seat, pick, prompt, response) in pick_results {
                let available = draft.current_pack_for(seat).to_vec();

                if pick.was_substituted() {
                    // A seat whose answers never parse is a failed seat, and
                    // the run has to be able to say so: without this, 42
                    // unusable answers read exactly like 42 deliberate picks
                    // (issue #195).
                    substituted_picks[seat] += 1;
                    eprintln!("\nWARN: seat {} pack {} pick {}: could not use the response, \
substituting {} (the first card). Response: {}",
                        seat, round + 1, pick_num + 1, pick.card(),
                        response.trim().replace('\n', " "));
                    log_draft_warning!(log, seat, round + 1, pick_num + 1, pick.card(), &response);
                }
                let chosen = pick.into_card();

                draft.make_pick(seat, &chosen).unwrap_or_else(|e| {
                    eprintln!("\nDraft pick error for seat {seat}: {e}");
                    let first = draft.current_pack_for(seat)[0].clone();
                    draft.make_pick(seat, &first).unwrap();
                });

                log_draft_pick!(log, seat, round + 1, pick_num + 1, &available, &chosen, &prompt, &response);
            }

            draft.rotate_packs();
        }
    }

    if !args.quiet {
        eprintln!("\nDraft complete!");
    }

    // Log final pools
    log_section!(log, "DRAFT POOLS");
    for seat in 0..args.players {
        log_pool_summary!(log, seat, &draft.players[seat].pool);
    }

    // ── Phase 3: Deck Building ──
    log_section!(log, "DECK BUILDING");
    if !args.quiet {
        eprintln!("Building decks...");
    }

    // Build all decks in parallel. Each worker logs its own result as
    // soon as it finishes so progress shows up in the log in real time
    // rather than after the slowest worker blocks the batch.
    // game_log::write_at serializes writes via a global Mutex, so
    // concurrent writes from different workers are safe. Per-seat
    // entries may interleave in wall-clock order; each entry carries a
    // `[Seat N]` label so grep-by-seat still works.
    let pools: Vec<Vec<String>> = draft.players.iter().map(|p| p.pool.clone()).collect();

    let deck_results: Vec<DeckBuildResult> = std::thread::scope(|s| {
        let log_ref = &log;
        let handles: Vec<_> = clients
            .iter_mut()
            .zip(pools.iter())
            .enumerate()
            .map(|(seat, (client, pool))| s.spawn(move || {
                let result = build_deck_with_llm(client, pool);
                let attempts: Vec<(&str, &str, Option<&str>)> = result
                    .attempts
                    .iter()
                    .map(|a| (a.prompt.as_str(), a.response.as_str(), a.error.as_deref()))
                    .collect();
                log_deck_building!(log_ref,
                    seat,
                    &result.deck.maindeck,
                    &result.deck.lands,
                    &result.deck.sideboard,
                    &attempts,
                    result.retries,
                );
                result
            }))
            .collect();

        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    // Build the decklist collection in seat order now that all workers
    // have finished. No further logging — that already happened above.
    let decklists: Vec<Decklist> = deck_results
        .iter()
        .map(|result| Decklist {
            entries: deckbuilding::to_decklist(&result.deck),
        })
        .collect();

    if !args.quiet {
        eprintln!("\nDecks built!");
    }

    // ── Phase 4: Tournament ──
    log_section!(log, "TOURNAMENT");
    if !args.quiet {
        eprintln!("Starting Swiss tournament...");
    }

    let tournament_config = TournamentConfig {
        best_of: args.best_of,
    };
    let mut tournament = Tournament::new(args.players, tournament_config);

    while !tournament.is_complete() {
        let round_num = tournament.rounds.len() + 1;
        let pairings = tournament.generate_pairings();

        if !args.quiet {
            eprintln!("Round {}/{}", round_num, tournament.total_rounds());
        }

        // Separate byes from real matches
        let real_matches: Vec<(usize, usize)> = pairings
            .iter()
            .filter(|&&(_, b)| b != usize::MAX)
            .copied()
            .collect();

        for &(a, _) in pairings.iter().filter(|&&(_, b)| b == usize::MAX) {
            if !args.quiet {
                eprintln!("  Seat {a} gets a bye");
            }
        }

        if !args.quiet {
            for &(a, b) in &real_matches {
                eprintln!("  Seat {a} vs Seat {b}");
            }
        }

        // Play all matches in the round in parallel
        let results: Vec<MatchResult> = std::thread::scope(|s| {
            let handles: Vec<_> = real_matches
                .iter()
                .map(|&(a, b)| {
                    let deck_a = &decklists[a];
                    let deck_b = &decklists[b];
                    let reg = &registry;
                    let model_a = &args.models[a];
                    let model_b = &args.models[b];
                    let guide_a = args.guides[a].as_deref();
                    let guide_b = args.guides[b].as_deref();
                    let card_ref = &card_reference;
                    let best_of = args.best_of;
                    let quiet = args.quiet;
                    s.spawn(move || play_match(
                        &PlayerSpec { seat: a, deck: deck_a, model_spec: model_a, guide: guide_a },
                        &PlayerSpec { seat: b, deck: deck_b, model_spec: model_b, guide: guide_b },
                        reg,
                        best_of,
                        quiet,
                        card_ref,
                    ))
                })
                .collect();

            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });

        // Log byes
        for &(a, b) in &pairings {
            if b == usize::MAX {
                log_bye!(log, round_num, a);
            }
        }

        // Log match results and game logs
        for result in &results {
            log_match_result!(log, 
                round_num,
                result.player_a,
                result.player_b,
                result.wins_a,
                result.wins_b,
                result.winner(),
            );
            for (game_num, game) in result.games.iter().enumerate() {
                log_game_log!(log, 
                    round_num,
                    game_num + 1,
                    result.player_a,
                    result.player_b,
                    &game.game_log,
                );
            }

            if !args.quiet {
                eprintln!(
                    "  Seat {} vs Seat {}: {}-{} (winner: Seat {})",
                    result.player_a,
                    result.player_b,
                    result.wins_a,
                    result.wins_b,
                    result.winner().map_or("draw".to_string(), |w| w.to_string())
                );
            }
        }

        tournament.record_round(pairings, results);
    }

    // ── Phase 5: Output ──
    log_section!(log, "FINAL STANDINGS");
    let sorted = tournament.sorted_standings();
    let standings_data: Vec<(usize, usize, usize, usize)> = sorted
        .iter()
        .map(|s| (s.seat, s.match_wins, s.match_losses, s.game_wins))
        .collect();
    log_standings!(log, &standings_data);

    if !args.quiet {
        eprintln!("\nFinal Standings:");
        for (rank, s) in sorted.iter().enumerate() {
            eprintln!(
                "  {}. Seat {} — {} match wins, {} game wins",
                rank + 1,
                s.seat,
                s.match_wins,
                s.game_wins
            );
        }
    }

    // Count total games played
    let total_games: usize = tournament.rounds.iter()
        .flat_map(|r| r.results.iter())
        .map(|m| m.games.len())
        .sum();

    // Print token usage summary (draft client + game player combined)
    llm_client::print_usage_summary(total_games);

    // A run whose seats never picked must not look like one that did. This
    // is the last thing printed before "Done", next to the standings it
    // qualifies (issue #195).
    let substituted_total: usize = substituted_picks.iter().sum();
    if substituted_total > 0 {
        eprintln!("\n=== Substituted Picks ===");
        eprintln!("  {substituted_total} pick(s) were made by the runner, not by a seat:");
        for (seat, n) in substituted_picks.iter().enumerate() {
            if *n > 0 {
                eprintln!("    Seat {seat}: {n} pick(s) unusable — this seat's pool, \
deck and results are not a drafted one");
            }
        }
        eprintln!("  (grep the log for WARN to see each one)");
    }

    if !args.quiet {
        eprintln!("\nDone. Log written to {}", args.log);
    }
}

// ─── Draft Pick Parsing ──────────────────────────────────────────────

/// What a seat's answer amounted to: the card it picked, and whether that
/// card was actually chosen or substituted because the answer was unusable.
///
/// The substitution itself is deliberate — a draft has to continue — but it
/// used to be silent, so 42 unparsable answers produced 42 confident
/// "Chose:" lines and a tournament built on them (issue #195). The adjacent
/// backend code already treats this class of failure as loud; this carries
/// the same fact out of the parser so the caller can too.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Pick {
    /// The seat named this card.
    Chosen(String),
    /// The answer could not be used; this is the first card of the pack.
    Substituted(String),
}

impl Pick {
    fn card(&self) -> &str {
        match self {
            Pick::Chosen(c) | Pick::Substituted(c) => c,
        }
    }

    fn into_card(self) -> String {
        match self {
            Pick::Chosen(c) | Pick::Substituted(c) => c,
        }
    }

    fn was_substituted(&self) -> bool {
        matches!(self, Pick::Substituted(_))
    }
}

fn parse_pick_response(response: &str, available: &[String]) -> Pick {
    // Primary path: JSON response like `{"thoughts": "...", "pick": N}`.
    // Secondary path (legacy or stray wrappers): strip markdown code fences
    // and retry. Last resort: fall through to a text scan for "PICK: N".
    let try_json = |s: &str| -> Option<String> {
        let v: serde_json::Value = serde_json::from_str(s).ok()?;
        let idx = usize::try_from(v["pick"].as_u64()?).unwrap_or(usize::MAX);
        (idx < available.len()).then(|| available[idx].clone())
    };

    if let Some(pick) = try_json(response) {
        return Pick::Chosen(pick);
    }

    // Strip optional ```json ... ``` fencing that some models still add.
    let stripped = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    if let Some(pick) = try_json(stripped) {
        return Pick::Chosen(pick);
    }

    // Legacy text scan — kept for robustness against older responses.
    for line in response.lines().rev() {
        let trimmed = line.trim().to_uppercase();
        if let Some(rest) = trimmed.strip_prefix("PICK:") {
            if let Ok(idx) = rest.trim().trim_start_matches('"').trim_end_matches('"').trim_end_matches(',').parse::<usize>() {
                if idx < available.len() {
                    return Pick::Chosen(available[idx].clone());
                }
            }
        }
    }

    // Last resort: the draft must continue, so take the first card — but say
    // so, rather than letting it pass for a decision.
    Pick::Substituted(available[0].clone())
}

// ─── Deck Building ───────────────────────────────────────────────────

fn build_deck_with_llm(
    client: &mut llm_client::DraftLlmClient,
    pool: &[String],
) -> DeckBuildResult {
    let prompt = build_deck_prompt(pool);
    let mut last_error = String::new();
    let mut attempts: Vec<DeckAttempt> = Vec::new();
    let max_retries = 10;

    for attempt in 0..max_retries {
        if attempt > 0 {
            // Brief delay before retry (helps with transient network errors)
            std::thread::sleep(std::time::Duration::from_secs(2));
        }

        let msg = if attempt == 0 {
            prompt.clone()
        } else {
            format!(
                "Your previous deck was invalid: {last_error}. Please try again.\n\n{prompt}"
            )
        };

        let response = client.send_deck_building_message(&msg, pool);

        match deckbuilding::parse_deck_response(&response) {
            Ok((maindeck, lands)) => match deckbuilding::validate_deck(pool, &maindeck, &lands) {
                Ok(deck) => {
                    attempts.push(DeckAttempt { prompt: msg, response, error: None });
                    let retries = attempts.len() - 1;
                    return DeckBuildResult { deck, attempts, retries };
                }
                Err(e) => {
                    attempts.push(DeckAttempt { prompt: msg, response, error: Some(e.clone()) });
                    last_error = e;
                }
            },
            Err(e) => {
                attempts.push(DeckAttempt { prompt: msg, response, error: Some(e.clone()) });
                last_error = e;
            }
        }
    }

    // Fallback: include all cards, add 17 lands split by color
    eprintln!("Warning: deck building failed after {max_retries} attempts, using fallback");
    let mut lands = HashMap::new();
    lands.insert("Island".to_string(), 9);
    lands.insert("Swamp".to_string(), 8);

    let retries = attempts.len();
    DeckBuildResult {
        deck: deckbuilding::DraftDeck {
            maindeck: pool.to_vec(),
            lands,
            sideboard: Vec::new(),
        },
        attempts,
        retries,
    }
}

fn build_deck_prompt(pool: &[String]) -> String {
    // Count copies of each card
    let mut counts: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
    for card in pool {
        let name = card.split(" // ").next().unwrap_or(card);
        *counts.entry(name).or_insert(0) += 1;
    }
    let mut sorted: Vec<_> = counts.into_iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));

    let mut prompt = String::from(
        "Draft complete! Build a 40-card limited deck from your drafted pool.\n\n\
         Your pool:\n",
    );
    for (name, count) in &sorted {
        writeln!(prompt, "{count}x {name}").unwrap();
    }
    prompt
}

// ─── Tournament Game Execution ───────────────────────────────────────

fn play_match(
    a: &PlayerSpec<'_>,
    b: &PlayerSpec<'_>,
    registry: &CardRegistry,
    best_of: usize,
    _quiet: bool,
    card_reference: &str,
) -> MatchResult {
    let wins_needed = best_of / 2 + 1;
    let mut wins_a = 0;
    let mut wins_b = 0;
    let mut games = Vec::new();

    let seat_a = a.seat;
    let seat_b = b.seat;
    let deck_a = a.deck;
    let deck_b = b.deck;

    // Create LLM players once per match, reuse across games.
    // Set log file so all API prompts/responses are written to the draft log.
    let name_a = format!("Seat{seat_a}");
    let name_b = format!("Seat{seat_b}");
    let mut p1 = make_game_player(a.model_spec, &name_a, a.guide);
    let mut p2 = make_game_player(b.model_spec, &name_b, b.guide);

    // Play/draw per MTG tournament rules, delegated to the engine helpers:
    //   Game 1: engine::random_starting_player() — fair coin flip.
    //   Games 2+: engine::next_starter_loser_plays() — the loser of the
    //   previous game always elects to play first (the strategically
    //   dominant choice in Limited); drawn games keep the previous starter.
    let mut starter = engine::random_starting_player(2);

    while wins_a < wins_needed && wins_b < wins_needed {
        let outcome = play_game(
            seat_a,
            seat_b,
            deck_a,
            deck_b,
            registry,
            &mut p1,
            &mut p2,
            starter,
            card_reference,
        );

        // Engine's winner is a PlayerId (0 = seat_a, 1 = seat_b).
        let prev_winner: Option<mtg_engine::ids::PlayerId> = outcome.winner.map(|w| {
            if w == seat_a { mtg_engine::ids::PlayerId(0) } else { mtg_engine::ids::PlayerId(1) }
        });
        starter = engine::next_starter_loser_plays(starter, prev_winner, 2);

        match outcome.winner {
            Some(w) if w == seat_a => wins_a += 1,
            Some(_) => wins_b += 1,
            None => {}
        }

        games.push(outcome);
    }

    MatchResult {
        player_a: seat_a,
        player_b: seat_b,
        wins_a,
        wins_b,
        games,
    }
}

fn play_game(
    seat_a: usize,
    seat_b: usize,
    deck_a: &Decklist,
    deck_b: &Decklist,
    registry: &CardRegistry,
    p1: &mut LlmPlayer,
    p2: &mut LlmPlayer,
    starting_player: mtg_engine::ids::PlayerId,
    card_reference: &str,
) -> GameOutcome {
    let config = GameConfig {
        player_names: vec![p1.name().to_string(), p2.name().to_string()],
        decklists: vec![deck_a.clone(), deck_b.clone()],
        starting_life: 20,
        starting_player: Some(starting_player),
        // A fresh seed per game.
        rng_seed: None,
    };

    let mut state = engine::setup_game(&config, registry);

    // Re-initialize conversations for this game (fresh context per game)
    p1.init_conversation(&deck_a.entries, card_reference, registry);
    p2.init_conversation(&deck_b.entries, card_reference, registry);

    let mut action_count: u64 = 0;
    let max_actions: u64 = 50_000;

    let mut game_callback =
        |game_state: &GameState,
         acting_player: PlayerId,
         legal: &engine::LegalActions|
         -> mtg_engine::actions::Action {
            action_count += 1;

            if action_count >= max_actions {
                if let Some(concede_idx) = legal
                    .actions
                    .iter()
                    .position(|a| matches!(a, mtg_engine::actions::Action::Concede))
                {
                    return legal.actions[concede_idx].clone();
                }
            }

            let view = GameView::for_player(game_state, acting_player, registry);

            let player: &mut LlmPlayer = if acting_player == PlayerId(0) {
                p1
            } else {
                p2
            };

            if let Some(prompt) = &legal.combat_prompt {
                return player.choose_combat(&view, prompt);
            }

            player.choose_action(&view, legal)
        };

    engine::run_game_loop(&mut state, registry, &mut game_callback);

    let winner = state.result.as_ref().and_then(|r| {
        match r {
            mtg_engine::state::GameResult::Winner(pid) => {
                if *pid == PlayerId(0) {
                    Some(seat_a)
                } else {
                    Some(seat_b)
                }
            }
            mtg_engine::state::GameResult::Draw => None,
        }
    });

    // Capture game log, filtering out Debug-level entries (priority passes etc.)
    // to keep the log readable
    let game_log: Vec<String> = state
        .game_log
        .iter()
        .filter(|entry| entry.level as u8 >= 1) // Info and above
        .map(|entry| entry.message.clone())
        .collect();

    GameOutcome {
        winner,
        turns: state.turn_number,
        game_log,
    }
}

fn make_game_player(model_spec: &str, name: &str, guide: Option<&str>) -> LlmPlayer {
    // Parse "provider:model:draft_thinking:game_thinking"
    let parts: Vec<&str> = model_spec.split(':').collect();
    let provider = parts[0];
    let model = parts.get(1).copied();
    // Game thinking is the 4th part, or falls back to 3rd, or defaults
    let game_thinking = parts.get(3).or(parts.get(2)).copied();

    let mut p = match provider {
        "gemini" => {
            let mut p = LlmPlayer::new_gemini(name);
            if let Some(m) = model {
                p = p.with_model(m);
            }
            p
        }
        "claude" => {
            let mut p = LlmPlayer::new(name);
            if let Some(m) = model {
                p = p.with_model(m);
            }
            p
        }
        "claude-code" | "cc" => {
            let mut p = LlmPlayer::new_claude_code(name);
            if let Some(m) = model {
                p = p.with_model(m);
            }
            p
        }
        // Unreachable once validate_model_specs has run, and fatal if it ever
        // is reached: substituting a seat plays a different game than the one
        // requested and still prints a winner.
        other => die(&format!(
            "unknown model provider '{other}' (expected {})",
            llm_client::ACCEPTED_PROVIDERS
        )),
    };
    if let Some(level) = game_thinking {
        p = p.with_thinking_level(level);
    }
    if let Some(g) = guide {
        p = p.with_guide(g.to_string());
    }
    p
}

#[cfg(test)]
mod pick_parsing_tests {
    use super::{parse_pick_response, Pick};

    fn pack() -> Vec<String> {
        ["Hysterical Blindness", "Voiceless Spirit", "Ambush Viper", "Delver of Secrets"]
            .iter().map(std::string::ToString::to_string).collect()
    }

    #[test]
    fn a_usable_answer_is_the_seats_own_pick() {
        let p = pack();
        assert_eq!(parse_pick_response(r#"{"pick": 2}"#, &p),
            Pick::Chosen("Ambush Viper".into()));
        assert_eq!(parse_pick_response("```json\n{\"pick\": 1}\n```", &p),
            Pick::Chosen("Voiceless Spirit".into()));
        assert_eq!(parse_pick_response("thinking...\nPICK: 3", &p),
            Pick::Chosen("Delver of Secrets".into()));
    }

    /// The four shapes from issue #195: each is a well-formed JSON object
    /// that never reaches the backend's loud "no structured object" path,
    /// so the parser is the only place that can notice. Each still yields a
    /// card — a draft has to continue — but it must be marked as the
    /// runner's substitution, not the seat's choice.
    #[test]
    fn an_unusable_answer_is_reported_as_a_substitution() {
        let p = pack();
        for response in [
            r#"{"pick": 9999}"#,            // out-of-range index
            r#"{"choice": 3}"#,             // right shape, wrong key
            "{}",                           // empty object
            r#"{"pick": "Ambush Viper"}"#,  // a name where an index goes
        ] {
            let got = parse_pick_response(response, &p);
            assert_eq!(got, Pick::Substituted("Hysterical Blindness".into()),
                "{response} is not a usable pick, so it must not pass for one");
            assert!(got.was_substituted(),
                "{response} must be reportable as a substitution");
            // The draft still gets a card to continue with.
            assert_eq!(got.card(), "Hysterical Blindness");
        }
    }
}
