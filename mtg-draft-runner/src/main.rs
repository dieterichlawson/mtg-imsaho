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

use serde::Serialize;

mod llm_client;

// ─── CLI Argument Parsing ────────────────────────────────────────────

struct Args {
    set: String,
    players: usize,
    model: String,
    best_of: usize,
    guides: Vec<Option<String>>,
    output: String,
    quiet: bool,
}

fn parse_args() -> Args {
    let args: Vec<String> = env::args().collect();

    let get = |flag: &str| -> Option<String> {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .map(|s| s.to_string())
    };

    let set = get("--set").unwrap_or_else(|| "isd".to_string());
    let players: usize = get("--players")
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);
    let model = get("--model").unwrap_or_else(|| "claude".to_string());
    let best_of: usize = get("--best-of")
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    let output = get("--output").unwrap_or_else(|| "draft-results.json".to_string());
    let quiet = args.iter().any(|a| a == "--quiet" || a == "-q");

    // Load guides: --guide applies to all, --guide-N overrides for player N
    let global_guide = get("--guide").and_then(|path| fs::read_to_string(&path).ok());
    let mut guides: Vec<Option<String>> = vec![global_guide.clone(); players];
    for i in 0..players {
        let flag = format!("--guide-{}", i);
        if let Some(path) = get(&flag) {
            if let Ok(contents) = fs::read_to_string(&path) {
                guides[i] = Some(contents);
            }
        }
    }

    Args {
        set,
        players,
        model,
        best_of,
        guides,
        output,
        quiet,
    }
}

// ─── Output Types ────────────────────────────────────────────────────

#[derive(Serialize)]
struct DraftOutput {
    draft_log: DraftLog,
    tournament: TournamentLog,
}

#[derive(Serialize)]
struct DraftLog {
    set: String,
    pod_size: usize,
    original_packs: Vec<Vec<Vec<String>>>,
    players: Vec<PlayerLog>,
}

#[derive(Serialize)]
struct PlayerLog {
    seat: usize,
    guide: Option<String>,
    picks: Vec<PickLog>,
    pool: Vec<String>,
    deck_building: DeckBuildingLog,
}

#[derive(Serialize)]
struct PickLog {
    pack: usize,
    pick: usize,
    available: Vec<String>,
    chosen: String,
    prompt: String,
    response: String,
}

#[derive(Serialize)]
struct DeckBuildingLog {
    prompt: String,
    response: String,
    retries: usize,
    maindeck: Vec<String>,
    lands: HashMap<String, u32>,
    sideboard: Vec<String>,
}

#[derive(Serialize)]
struct TournamentLog {
    format: String,
    best_of: usize,
    rounds: Vec<serde_json::Value>,
    standings: Vec<serde_json::Value>,
}

// ─── Main ────────────────────────────────────────────────────────────

fn main() {
    let args = parse_args();
    let mut rng = rand::thread_rng();

    // Load set data
    let set_path = PathBuf::from(format!("data/sets/{}.json", args.set));
    let mut set_data = SetData::load(&set_path).unwrap_or_else(|e| {
        eprintln!("Failed to load set data: {}", e);
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
        eprintln!("Failed to build sheet data: {}", e);
        std::process::exit(1);
    });

    if !args.quiet {
        eprintln!(
            "=== {} Draft: {} players, best-of-{} ===",
            set_data.set_name, args.players, args.best_of
        );
    }

    // ── Phase 1: Generate packs ──
    if !args.quiet {
        eprintln!("Generating booster packs...");
    }
    let packs = generate_draft_packs(&sheets, args.players, &mut rng);

    // ── Phase 2: Draft ──
    if !args.quiet {
        eprintln!("Starting draft...");
    }
    let mut draft = DraftState::new(packs);

    // Create LLM clients for each drafter
    let mut clients: Vec<llm_client::DraftLlmClient> = (0..args.players)
        .map(|seat| {
            llm_client::DraftLlmClient::new(
                &args.model,
                &set_data.set_name,
                args.guides[seat].as_deref(),
            )
        })
        .collect();

    // Pick logs per player
    let mut pick_logs: Vec<Vec<PickLog>> = (0..args.players).map(|_| Vec::new()).collect();

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
            let pick_inputs: Vec<(usize, Vec<String>, Vec<String>, Vec<mtg_draft::draft::DraftPick>)> =
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
            let pick_results: Vec<(usize, String, String, String)> =
                std::thread::scope(|s| {
                    let handles: Vec<_> = pick_inputs
                        .iter()
                        .zip(clients.iter_mut())
                        .map(|((seat, available, pool, history), client)| {
                            let seat = *seat;
                            s.spawn(move || {
                                let prompt = client.build_pick_prompt(
                                    round + 1,
                                    pick_num + 1,
                                    available,
                                    pool,
                                    history,
                                );
                                let response = client.send_message(&prompt);
                                let chosen = parse_pick_response(&response, available);
                                client.record_pick(&chosen);
                                (seat, chosen, prompt, response)
                            })
                        })
                        .collect();

                    handles.into_iter().map(|h| h.join().unwrap()).collect()
                });

            // Apply picks sequentially (mutates draft state)
            for (seat, chosen, prompt, response) in pick_results {
                let available = draft.current_pack_for(seat).to_vec();

                draft.make_pick(seat, &chosen).unwrap_or_else(|e| {
                    eprintln!("\nDraft pick error for seat {}: {}", seat, e);
                    let first = draft.current_pack_for(seat)[0].clone();
                    draft.make_pick(seat, &first).unwrap();
                });

                pick_logs[seat].push(PickLog {
                    pack: round + 1,
                    pick: pick_num + 1,
                    available,
                    chosen,
                    prompt,
                    response,
                });
            }

            draft.rotate_packs();
        }
    }

    if !args.quiet {
        eprintln!("\nDraft complete!");
    }

    // ── Phase 3: Deck Building ──
    if !args.quiet {
        eprintln!("Building decks...");
    }

    // Build all decks in parallel
    let pools: Vec<Vec<String>> = draft.players.iter().map(|p| p.pool.clone()).collect();

    let deck_results: Vec<(deckbuilding::DraftDeck, DeckBuildingLog)> =
        std::thread::scope(|s| {
            let handles: Vec<_> = clients
                .iter_mut()
                .zip(pools.iter())
                .map(|(client, pool)| {
                    s.spawn(move || build_deck_with_llm(client, pool))
                })
                .collect();

            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });

    let mut deck_logs: Vec<DeckBuildingLog> = Vec::new();
    let mut decklists: Vec<Decklist> = Vec::new();

    for (deck, log) in deck_results {
        decklists.push(Decklist {
            entries: deckbuilding::to_decklist(&deck),
        });
        deck_logs.push(log);
    }

    if !args.quiet {
        eprintln!("\nDecks built!");
    }

    // ── Phase 4: Tournament ──
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
                eprintln!("  Seat {} gets a bye", a);
            }
        }

        if !args.quiet {
            for &(a, b) in &real_matches {
                eprintln!("  Seat {} vs Seat {}", a, b);
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
                    let model = &args.model;
                    let best_of = args.best_of;
                    let quiet = args.quiet;
                    s.spawn(move || play_match(a, b, deck_a, deck_b, reg, model, best_of, quiet))
                })
                .collect();

            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });

        if !args.quiet {
            for result in &results {
                eprintln!(
                    "  Seat {} vs Seat {}: {}-{} (winner: Seat {})",
                    result.player_a,
                    result.player_b,
                    result.wins_a,
                    result.wins_b,
                    result.winner().map(|w| w.to_string()).unwrap_or("draw".to_string())
                );
            }
        }

        tournament.record_round(pairings, results);
    }

    // ── Phase 5: Output ──
    if !args.quiet {
        eprintln!("\nFinal Standings:");
        for (rank, s) in tournament.sorted_standings().iter().enumerate() {
            eprintln!(
                "  {}. Seat {} — {} match wins, {} game wins",
                rank + 1,
                s.seat,
                s.match_wins,
                s.game_wins
            );
        }
    }

    // Assemble output
    let output = DraftOutput {
        draft_log: DraftLog {
            set: set_data.set_code.clone(),
            pod_size: args.players,
            original_packs: draft.original_packs.clone(),
            players: (0..args.players)
                .map(|seat| PlayerLog {
                    seat,
                    guide: args.guides[seat].clone(),
                    picks: std::mem::take(&mut pick_logs[seat]),
                    pool: draft.players[seat].pool.clone(),
                    deck_building: std::mem::replace(
                        &mut deck_logs[seat],
                        DeckBuildingLog {
                            prompt: String::new(),
                            response: String::new(),
                            retries: 0,
                            maindeck: Vec::new(),
                            lands: HashMap::new(),
                            sideboard: Vec::new(),
                        },
                    ),
                })
                .collect(),
        },
        tournament: TournamentLog {
            format: "swiss".to_string(),
            best_of: args.best_of,
            rounds: tournament
                .rounds
                .iter()
                .map(|r| serde_json::to_value(r).unwrap_or_default())
                .collect(),
            standings: tournament
                .sorted_standings()
                .iter()
                .map(|s| serde_json::to_value(s).unwrap_or_default())
                .collect(),
        },
    };

    let json = serde_json::to_string_pretty(&output).expect("Failed to serialize output");
    fs::write(&args.output, &json).expect("Failed to write output file");

    if !args.quiet {
        eprintln!("\nResults written to {}", args.output);
    }
}

// ─── Draft Pick Parsing ──────────────────────────────────────────────

fn parse_pick_response(response: &str, available: &[String]) -> String {
    // Look for "PICK: <number>" on the last matching line
    for line in response.lines().rev() {
        let trimmed = line.trim().to_uppercase();
        if let Some(rest) = trimmed.strip_prefix("PICK:") {
            if let Ok(idx) = rest.trim().parse::<usize>() {
                if idx < available.len() {
                    return available[idx].clone();
                }
            }
        }
        // Also try just a bare number on the last line
        if let Ok(idx) = trimmed.parse::<usize>() {
            if idx < available.len() {
                return available[idx].clone();
            }
        }
    }

    // Fallback: pick the first card
    available[0].clone()
}

// ─── Deck Building ───────────────────────────────────────────────────

fn build_deck_with_llm(
    client: &mut llm_client::DraftLlmClient,
    pool: &[String],
) -> (deckbuilding::DraftDeck, DeckBuildingLog) {
    let prompt = build_deck_prompt(pool);
    let mut last_response = String::new();
    let mut retries = 0;

    for attempt in 0..3 {
        let msg = if attempt == 0 {
            prompt.clone()
        } else {
            format!(
                "Your previous deck was invalid: {}. Please try again.\n\n{}",
                last_response, prompt
            )
        };

        let response = client.send_deck_building_message(&msg);

        match deckbuilding::parse_deck_response(&response) {
            Ok((maindeck, lands)) => match deckbuilding::validate_deck(pool, &maindeck, &lands) {
                Ok(deck) => {
                    return (
                        deck.clone(),
                        DeckBuildingLog {
                            prompt: prompt.clone(),
                            response,
                            retries,
                            maindeck: deck.maindeck,
                            lands: deck.lands,
                            sideboard: deck.sideboard,
                        },
                    );
                }
                Err(e) => {
                    last_response = e;
                    retries += 1;
                }
            },
            Err(e) => {
                last_response = e;
                retries += 1;
            }
        }
    }

    // Fallback: include all cards, add 17 lands split by color
    eprintln!("Warning: deck building failed after 3 attempts, using fallback");
    let maindeck = pool.to_vec();
    let mut lands = HashMap::new();
    lands.insert("Island".to_string(), 9);
    lands.insert("Swamp".to_string(), 8);

    let deck = deckbuilding::DraftDeck {
        maindeck: maindeck.clone(),
        lands: lands.clone(),
        sideboard: Vec::new(),
    };

    (
        deck,
        DeckBuildingLog {
            prompt,
            response: format!("FALLBACK (all attempts failed: {})", last_response),
            retries,
            maindeck,
            lands,
            sideboard: Vec::new(),
        },
    )
}

fn build_deck_prompt(pool: &[String]) -> String {
    let mut prompt = String::from(
        "Draft complete! Build a 40-card deck from your pool.\n\n\
         Choose your best ~22-24 non-land cards and add basic lands to reach 40+ cards total.\n\
         You may use any number of basic lands (Plains, Island, Swamp, Mountain, Forest).\n\n\
         Your pool:\n",
    );
    for (i, card) in pool.iter().enumerate() {
        let name = card.split(" // ").next().unwrap_or(card);
        prompt.push_str(&format!("{}. {}\n", i, name));
    }
    prompt.push_str(
        "\nOutput your deck in this exact format:\n\n\
         MAINDECK:\n\
         Card Name\n\
         Card Name\n\
         ...\n\n\
         LANDS:\n\
         9 Island\n\
         8 Swamp\n",
    );
    prompt
}

// ─── Tournament Game Execution ───────────────────────────────────────

fn play_match(
    seat_a: usize,
    seat_b: usize,
    deck_a: &Decklist,
    deck_b: &Decklist,
    registry: &CardRegistry,
    model_spec: &str,
    best_of: usize,
    quiet: bool,
) -> MatchResult {
    let wins_needed = best_of / 2 + 1;
    let mut wins_a = 0;
    let mut wins_b = 0;
    let mut games = Vec::new();

    while wins_a < wins_needed && wins_b < wins_needed {
        let game_num = games.len() + 1;
        if !quiet {
            eprint!("    Game {}...", game_num);
        }

        let outcome = play_game(seat_a, seat_b, deck_a, deck_b, registry, model_spec);

        match outcome.winner {
            Some(w) if w == seat_a => wins_a += 1,
            Some(_) => wins_b += 1,
            None => {} // draw, no one gets a win
        }

        if !quiet {
            let winner_str = match outcome.winner {
                Some(w) => format!("Seat {} wins in {} turns", w, outcome.turns),
                None => format!("Draw after {} turns", outcome.turns),
            };
            eprintln!(" {}", winner_str);
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
    model_spec: &str,
) -> GameOutcome {
    let name_a = format!("Seat{}", seat_a);
    let name_b = format!("Seat{}", seat_b);

    let config = GameConfig {
        player_names: vec![name_a.clone(), name_b.clone()],
        decklists: vec![deck_a.clone(), deck_b.clone()],
        starting_life: 20,
    };

    let mut state = engine::setup_game(&config, registry);

    // Create LLM players for the game
    let mut p1 = make_game_player(model_spec, &name_a);
    let mut p2 = make_game_player(model_spec, &name_b);

    // Initialize conversations with decklists
    p1.init_conversation(&deck_a.entries, &deck_b.entries, registry);
    p2.init_conversation(&deck_b.entries, &deck_a.entries, registry);

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
                &mut p1
            } else {
                &mut p2
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

    GameOutcome {
        winner,
        turns: state.turn_number,
    }
}

fn make_game_player(model_spec: &str, name: &str) -> LlmPlayer {
    let parts: Vec<&str> = model_spec.splitn(2, ':').collect();
    let provider = parts[0];
    let model = parts.get(1).copied();

    match provider {
        "claude" => {
            let mut p = LlmPlayer::new(name);
            if let Some(m) = model {
                p = p.with_model(m);
            }
            p
        }
        "gemini" => {
            let mut p = LlmPlayer::new_gemini(name);
            if let Some(m) = model {
                p = p.with_model(m);
            }
            p
        }
        _ => {
            eprintln!("Unknown model provider '{}', defaulting to claude", provider);
            LlmPlayer::new(name)
        }
    }
}
