//! Ask kept draft picks again, under this build and any seat.
//!
//! A pick is a pure function of what the seat is shown — the system prompt,
//! the pack, the pool, its place at the table and its notes from the pick
//! before — so a pick a real draft made can be asked again, of another
//! model or of the same model under another build's prompts, and the
//! choices compared. `mtg-draft-runner` keeps the positions when
//! `MTG_DRAFT_POSITIONS` names a file; this reads them and prints one JSON
//! line per position: the card chosen, whether it was substituted, the
//! tokens the pick cost and the bytes it sent. `scripts/decision-eval/`
//! shards a positions file across processes and scores the results.
//!
//! ```text
//! mtg-draft-replay --positions picks.jsonl --model cc:claude-haiku-5-5 [--guide FILE]
//! ```
//!
//! Every position is a real model call. A `claude` or `gemini` seat is a
//! metered one, refused unless `--metered` says it is meant.

use std::collections::HashMap;
use std::io::BufRead;

use mtg_draft::set_data::SetData;
use mtg_draft_runner::card_lines::CardLines;
use mtg_draft_runner::llm_client::{self, DraftLlmClient, Table};
use mtg_draft_runner::pick::parse_pick_response;
use mtg_engine::cards::CardRegistry;

fn die(msg: &str) -> ! {
    eprintln!("mtg-draft-replay: {msg}");
    std::process::exit(2);
}

/// A set as the draft runner loads it: the cards it can deal, their
/// reference and the lines a seat is shown.
struct LoadedSet {
    name: String,
    reference: String,
    lines: CardLines,
}

fn load_set(key: &str, registry: &CardRegistry) -> LoadedSet {
    let path = std::path::PathBuf::from(format!("data/sets/{key}.json"));
    let mut set = SetData::load(&path).unwrap_or_else(|e| die(&format!("cannot load {}: {e}", path.display())));
    set.filter_implemented(registry);
    LoadedSet {
        name: set.set_name.clone(),
        reference: llm_client::build_card_reference(&set.all_card_names(), registry),
        lines: CardLines::new(&set.all_card_names(), &set.rarities(), registry),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let get = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let positions = get("--positions").unwrap_or_else(|| die("--positions <file> is required"));
    let model = get("--model").unwrap_or_else(|| die("--model <spec> is required"));
    if !args.iter().any(|a| a == "--metered") && !matches!(model.split(':').next(), Some("cc" | "claude-code")) {
        die(&format!("{model} is a metered seat; pass --metered to spend on it"));
    }
    // The backend expects its key: a run with none panicked at the first
    // position (#903's review).
    if let Some(var) = mtg_draft_runner::llm_client::missing_api_key(model.split(':').next().unwrap_or("")) {
        die(&format!("{model} calls a metered API and {var} is not set"));
    }
    let guide = get("--guide").map(|p| std::fs::read_to_string(&p)
        .unwrap_or_else(|e| die(&format!("cannot read --guide {p}: {e}"))));

    let registry = CardRegistry::with_all_cards();
    let mut sets: HashMap<String, LoadedSet> = HashMap::new();
    let file = std::fs::File::open(&positions).unwrap_or_else(|e| die(&format!("cannot open {positions}: {e}")));
    for (id, line) in std::io::BufReader::new(file).lines().enumerate() {
        let line = line.unwrap_or_else(|e| die(&format!("cannot read {positions}: {e}")));
        if line.trim().is_empty() {
            continue;
        }
        let p: serde_json::Value = serde_json::from_str(&line)
            .unwrap_or_else(|e| die(&format!("{positions} line {}: {e}", id + 1)));
        let names = |key: &str| -> Vec<String> {
            p[key].as_array().into_iter().flatten().filter_map(|v| v.as_str().map(str::to_string)).collect()
        };
        let count = |key: &str| usize::try_from(p[key].as_u64().unwrap_or(0)).unwrap_or(0);
        let set_key = p["set"].as_str().unwrap_or("isd").to_string();
        let set = sets.entry(set_key.clone()).or_insert_with(|| load_set(&set_key, &registry));
        let table = Table { seat: count("seat"), pod_size: count("pod_size"), pack_size: count("pack_size") };
        let (available, pool) = (names("available"), names("pool"));
        if available.is_empty() {
            continue;
        }

        let mut client = DraftLlmClient::new(&model, &set.name, guide.as_deref(), &set.reference, table);
        client.restore_notes(p["notes"].as_str().map(str::to_string));
        let prompt = DraftLlmClient::notes_section(client.notes())
            + &DraftLlmClient::build_pick_prompt(table, count("pack_number"), count("pick_index"),
                &available, &pool, &set.lines);
        let before = llm_client::get_model_usage();
        let response = client.send_pick_message(&prompt, available.len());
        let after = llm_client::get_model_usage();
        let pick = parse_pick_response(&response, &available);

        let mut usage = serde_json::json!({"calls": 0, "input": 0, "output": 0, "cache_read": 0, "cache_write": 0});
        for (key, now) in &after {
            let was = before.get(key).cloned().unwrap_or_default();
            for (field, (n, w)) in [("calls", (now.calls, was.calls)), ("input", (now.input, was.input)),
                ("output", (now.output, was.output)), ("cache_read", (now.cache_read, was.cache_read)),
                ("cache_write", (now.cache_create, was.cache_create))] {
                usage[field] = serde_json::json!(usage[field].as_u64().unwrap_or(0) + n.saturating_sub(w));
            }
        }
        println!("{}", serde_json::json!({
            "id": id, "seed": p["seed"], "seat": table.seat, "pack_number": count("pack_number"),
            "pick_index": count("pick_index"), "chosen": pick.card(), "substituted": pick.was_substituted(),
            "live": p["chosen"], "usage": usage, "system_bytes": client.system_prompt().len(),
            "prompt_bytes": prompt.len(), "notes": client.notes(),
        }));
    }
}
